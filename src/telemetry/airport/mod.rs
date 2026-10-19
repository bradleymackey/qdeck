//! Background provider for the `airport` tile: runway direction in use plus expected arrivals
//! and departures for one airport (London City / `EGLC` by default), built only from open data:
//! - Live traffic: adsb.fi open data API (adsb.lol as fallback).
//! - Surface wind (runway fallback): aviationweather.gov METAR (NOAA).
//! - Callsign routes (VRS standing data) and runway geometry (OurAirports): bundled at build
//!   time from `data/airport/`, see `scripts/update-airport-data.sh`.
//!
//! The poller only runs while an airport tile is on screen.

mod geo;
mod reference;
mod runway;
mod traffic;

pub use reference::{
    AirportInfo, RouteRole, RunwayEnd, embedded_airport_info, embedded_routes, parse_route_rows,
    parse_runway_rows,
};
pub use runway::{
    RunwayInUse, RunwaySource, RunwayTracker, Wind, parse_metar_json, runway_from_wind,
};
pub use traffic::{
    Aircraft, AirportFlight, RECENT_DEPARTURE_MINUTES, TrafficPicture, classify_traffic,
    parse_adsb_json,
};

use std::collections::HashMap;
use std::process::Command;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

pub const DEFAULT_AIRPORT_ICAO: &str = "EGLC";
/// Number of pages on the airport tile (runway, arrivals, departures).
pub const AIRPORT_PAGES: u8 = 3;

const TRAFFIC_REFRESH: Duration = Duration::from_secs(30);
const METAR_REFRESH: Duration = Duration::from_secs(10 * 60);
/// The poller pauses once no airport tile has been on screen for this long.
const ACTIVE_GRACE: Duration = Duration::from_secs(90);

/// Latest airport picture shared with the renderer.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AirportStatus {
    pub icao: String,
    /// The bundled runway table knows this airport (`false` for an unknown ICAO code).
    pub known_airport: bool,
    pub runway: Option<RunwayInUse>,
    pub wind: Option<Wind>,
    pub arrivals: Vec<AirportFlight>,
    pub departures: Vec<AirportFlight>,
    /// The last traffic poll succeeded.
    pub traffic_available: bool,
    /// Active tile page (`0` runway, `1` arrivals, `2` departures).
    pub page_index: u8,
}

struct Control {
    icao: String,
    last_active: Option<Instant>,
    generation: u64,
}

struct AirportWorker {
    status: Mutex<AirportStatus>,
    control: Mutex<Control>,
    wake: Condvar,
}

static AIRPORT_WORKER: OnceLock<Arc<AirportWorker>> = OnceLock::new();
static AIRPORT_PAGE_INDEX: AtomicU8 = AtomicU8::new(0);

/// Cycles the airport tile page (runway → arrivals → departures) and returns the new index.
pub fn toggle_airport_page() -> u8 {
    let next = (AIRPORT_PAGE_INDEX.load(Ordering::Relaxed) + 1) % AIRPORT_PAGES;
    AIRPORT_PAGE_INDEX.store(next, Ordering::Relaxed);
    next
}

/// Marks an airport tile for `icao` as on screen, starting or waking the background poller.
pub fn note_airport_tile_active(icao: &str) {
    let icao = icao.trim().to_ascii_uppercase();
    let worker = AIRPORT_WORKER.get_or_init(start_worker);
    let Ok(mut ctl) = worker.control.lock() else {
        return;
    };
    let was_idle = ctl.last_active.is_none_or(|t| t.elapsed() > ACTIVE_GRACE);
    let changed = ctl.icao != icao;
    ctl.icao = icao;
    ctl.last_active = Some(Instant::now());
    if was_idle || changed {
        ctl.generation += 1;
        worker.wake.notify_one();
    }
}

/// Returns the latest airport picture without blocking (empty until a tile has been shown).
pub fn current_airport_status() -> AirportStatus {
    let mut status = AIRPORT_WORKER
        .get()
        .and_then(|w| w.status.lock().ok().map(|s| s.clone()))
        .unwrap_or_default();
    status.page_index = AIRPORT_PAGE_INDEX.load(Ordering::Relaxed);
    status
}

fn start_worker() -> Arc<AirportWorker> {
    let worker = Arc::new(AirportWorker {
        status: Mutex::new(AirportStatus::default()),
        control: Mutex::new(Control {
            icao: DEFAULT_AIRPORT_ICAO.to_string(),
            last_active: None,
            generation: 0,
        }),
        wake: Condvar::new(),
    });
    let w = Arc::clone(&worker);
    thread::spawn(move || run_worker(&w));
    worker
}

fn run_worker(worker: &AirportWorker) {
    let mut session: Option<AirportSession> = None;
    loop {
        let (icao, active, generation) = match worker.control.lock() {
            Ok(ctl) => (
                ctl.icao.clone(),
                ctl.last_active.is_some_and(|t| t.elapsed() <= ACTIVE_GRACE),
                ctl.generation,
            ),
            Err(_) => return,
        };

        let wait = if active {
            if session.as_ref().is_none_or(|s| s.icao != icao) {
                println!("[QDECK] [Airport] Tracking {icao}");
                session = Some(AirportSession::new(&icao));
            }
            if let Some(s) = session.as_mut() {
                let status = s.refresh();
                if let Ok(mut guard) = worker.status.lock() {
                    *guard = status;
                }
            }
            TRAFFIC_REFRESH
        } else {
            ACTIVE_GRACE
        };

        if let Ok(ctl) = worker.control.lock() {
            let _ = worker
                .wake
                .wait_timeout_while(ctl, wait, |c| c.generation == generation);
        }
    }
}

/// Per-airport state kept between polls.
struct AirportSession {
    icao: String,
    info: Option<AirportInfo>,
    routes: HashMap<String, RouteRole>,
    tracker: RunwayTracker,
    wind: Option<Wind>,
    metar_due_at: Instant,
    picture: TrafficPicture,
    /// Take-off time per airborne departure, fixed when the flight is first seen.
    departed_at: HashMap<String, Instant>,
}

impl AirportSession {
    fn new(icao: &str) -> Self {
        let info = embedded_airport_info(icao);
        if info.is_none() {
            eprintln!("[QDECK] [Airport] No runway data bundled for {icao}");
        }
        Self {
            icao: icao.to_string(),
            info,
            routes: embedded_routes(icao),
            tracker: RunwayTracker::default(),
            wind: None,
            metar_due_at: Instant::now(),
            picture: TrafficPicture::default(),
            departed_at: HashMap::new(),
        }
    }

    fn refresh(&mut self) -> AirportStatus {
        let now = Instant::now();
        if now >= self.metar_due_at {
            let url = format!(
                "https://aviationweather.gov/api/data/metar?ids={}&format=json",
                self.icao
            );
            if let Some(wind) = curl_text(&url).as_deref().and_then(parse_metar_json) {
                self.wind = Some(wind);
            }
            self.metar_due_at = now + METAR_REFRESH;
        }

        let mut traffic_available = false;
        if let Some(info) = &self.info {
            let fetched = traffic::traffic_urls(info.lat, info.lon)
                .iter()
                .find_map(|url| curl_text(url).as_deref().and_then(parse_adsb_json));
            if let Some(aircraft) = fetched {
                self.picture = classify_traffic(info, &self.routes, &aircraft);
                stamp_departures(&mut self.picture, &mut self.departed_at, now);
                for (ident, heading) in &self.picture.runway_votes {
                    self.tracker.record(ident, *heading, now);
                }
                traffic_available = true;
            }
        }

        let runway = self.tracker.current(now).or_else(|| {
            let info = self.info.as_ref()?;
            runway_from_wind(info, self.wind.as_ref()?)
        });

        AirportStatus {
            icao: self.icao.clone(),
            known_airport: self.info.is_some(),
            runway,
            wind: self.wind,
            arrivals: self.picture.arrivals.clone(),
            departures: self.picture.departures.clone(),
            traffic_available,
            page_index: 0,
        }
    }
}

/// Replaces each airborne departure's estimated minutes with the time since its take-off,
/// fixed the first time the flight is seen, so the count ticks up steadily between polls.
fn stamp_departures(
    picture: &mut TrafficPicture,
    departed_at: &mut HashMap<String, Instant>,
    now: Instant,
) {
    let mut seen = Vec::new();
    for flight in picture.departures.iter_mut().filter(|f| !f.on_ground) {
        let at = *departed_at
            .entry(flight.callsign.clone())
            .or_insert_with(|| {
                now.checked_sub(Duration::from_secs(u64::from(flight.minutes) * 60))
                    .unwrap_or(now)
            });
        flight.minutes = (now.saturating_duration_since(at).as_secs() / 60) as u32;
        seen.push(flight.callsign.clone());
    }
    departed_at.retain(|callsign, _| seen.contains(callsign));
    picture
        .departures
        .retain(|f| f.on_ground || f.minutes <= RECENT_DEPARTURE_MINUTES);
}

/// Runs `curl` and returns the response body on HTTP success.
fn curl_text(url: &str) -> Option<String> {
    let output = Command::new("curl")
        .args([
            "-sSfL",
            "--compressed",
            "--max-time",
            "10",
            "-A",
            "qdeck/0.1 (Stream Deck airport tile)",
            url,
        ])
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stamp_departures_counts_up_from_first_sighting() {
        let flight = |minutes, on_ground| AirportFlight {
            callsign: "CFE63BB".into(),
            other_airport: "LEIB".into(),
            minutes,
            on_ground,
        };
        let t0 = Instant::now();
        let mut departed_at = HashMap::new();
        let mut picture = TrafficPicture {
            departures: vec![flight(3, false)],
            ..TrafficPicture::default()
        };
        stamp_departures(&mut picture, &mut departed_at, t0);
        assert_eq!(picture.departures[0].minutes, 3);

        // Ten minutes later the distance-based estimate (here 9) is ignored.
        picture.departures = vec![flight(9, false)];
        stamp_departures(
            &mut picture,
            &mut departed_at,
            t0 + Duration::from_secs(600),
        );
        assert_eq!(picture.departures[0].minutes, 13);

        // Past the listing window the flight drops off; ground departures always stay.
        picture.departures = vec![flight(20, false)];
        stamp_departures(
            &mut picture,
            &mut departed_at,
            t0 + Duration::from_secs(43 * 60),
        );
        assert!(picture.departures.is_empty());
        picture.departures = vec![flight(0, true)];
        stamp_departures(
            &mut picture,
            &mut departed_at,
            t0 + Duration::from_secs(50 * 60),
        );
        assert_eq!(picture.departures.len(), 1);
        assert!(departed_at.is_empty());
    }

    #[test]
    fn test_toggle_airport_page_cycles() {
        let start = AIRPORT_PAGE_INDEX.load(Ordering::Relaxed);
        let seen: Vec<u8> = (0..AIRPORT_PAGES).map(|_| toggle_airport_page()).collect();
        assert_eq!(*seen.last().unwrap(), start);
        assert!(seen.iter().all(|p| *p < AIRPORT_PAGES));
    }
}
