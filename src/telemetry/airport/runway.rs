//! Works out which runway direction an airport is operating, from live traffic first and
//! from the METAR surface wind when no recent traffic has been seen.

use super::reference::AirportInfo;
use std::time::{Duration, Instant};

/// Traffic votes older than this no longer count towards the runway in use.
const VOTE_WINDOW: Duration = Duration::from_secs(30 * 60);
/// Below this wind speed the METAR gives no useful runway preference.
const CALM_WIND_KT: f64 = 3.0;

/// Surface wind from the latest METAR (`dir_deg` is `None` for variable wind).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Wind {
    pub dir_deg: Option<u16>,
    pub speed_kt: u16,
    pub gust_kt: Option<u16>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunwaySource {
    /// Aircraft seen landing on or departing from the runway in the last 30 minutes.
    Traffic,
    /// No recent traffic: the runway end with the most headwind in the METAR.
    Wind,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RunwayInUse {
    pub ident: String,
    pub heading_deg: f64,
    pub source: RunwaySource,
}

impl RunwayInUse {
    /// Operations are named after the direction aircraft take off and land towards,
    /// e.g. runway 27 (heading 273°) is "westerly" operations.
    pub fn ops_direction(&self) -> &'static str {
        match self.heading_deg.rem_euclid(360.0) {
            h if (45.0..135.0).contains(&h) => "EASTERLY",
            h if (135.0..225.0).contains(&h) => "SOUTHERLY",
            h if (225.0..315.0).contains(&h) => "WESTERLY",
            _ => "NORTHERLY",
        }
    }
}

/// Parses the aviationweather.gov `/api/data/metar?format=json` response.
pub fn parse_metar_json(json: &str) -> Option<Wind> {
    let value: serde_json::Value = serde_json::from_str(json).ok()?;
    let obs = value.as_array()?.first()?;
    let as_u16 = |v: &serde_json::Value| v.as_f64().map(|n| n.round().clamp(0.0, 999.0) as u16);
    Some(Wind {
        dir_deg: obs.get("wdir").and_then(as_u16),
        speed_kt: obs.get("wspd").and_then(as_u16)?,
        gust_kt: obs.get("wgst").and_then(as_u16),
    })
}

/// Picks the runway end with the strongest headwind component, or `None` in calm/variable wind.
pub fn runway_from_wind(info: &AirportInfo, wind: &Wind) -> Option<RunwayInUse> {
    let dir = f64::from(wind.dir_deg?);
    if f64::from(wind.speed_kt) < CALM_WIND_KT {
        return None;
    }
    info.runway_ends()
        .map(|end| (end, (dir - end.heading_deg).to_radians().cos()))
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(end, _)| RunwayInUse {
            ident: end.ident.clone(),
            heading_deg: end.heading_deg,
            source: RunwaySource::Wind,
        })
}

/// Rolling record of which runway end recent arrivals and departures were aligned with.
#[derive(Debug, Default)]
pub struct RunwayTracker {
    votes: Vec<(String, f64, Instant)>,
}

impl RunwayTracker {
    pub fn record(&mut self, ident: &str, heading_deg: f64, now: Instant) {
        self.votes.push((ident.to_string(), heading_deg, now));
    }

    /// The most-voted runway end in the last 30 minutes (ties go to the most recent vote).
    pub fn current(&mut self, now: Instant) -> Option<RunwayInUse> {
        self.votes
            .retain(|(_, _, at)| now.saturating_duration_since(*at) <= VOTE_WINDOW);
        let mut best: Option<(&str, f64, usize, Instant)> = None;
        for (ident, heading, at) in &self.votes {
            let count = self.votes.iter().filter(|v| &v.0 == ident).count();
            let better = best.is_none_or(|(_, _, c, last)| count > c || (count == c && *at > last));
            if better {
                best = Some((ident, *heading, count, *at));
            }
        }
        best.map(|(ident, heading_deg, _, _)| RunwayInUse {
            ident: ident.to_string(),
            heading_deg,
            source: RunwaySource::Traffic,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::reference::{fixtures::EGLC_RUNWAYS_CSV, parse_runway_rows};
    use super::*;

    fn eglc() -> AirportInfo {
        parse_runway_rows(EGLC_RUNWAYS_CSV, "EGLC").unwrap()
    }

    #[test]
    fn test_parse_metar_json() {
        let json =
            r#"[{"icaoId":"EGLC","wdir":110,"wspd":10,"rawOb":"METAR EGLC 191250Z AUTO 11010KT"}]"#;
        let wind = parse_metar_json(json).unwrap();
        assert_eq!(
            wind,
            Wind {
                dir_deg: Some(110),
                speed_kt: 10,
                gust_kt: None
            }
        );
        let vrb = parse_metar_json(r#"[{"wdir":"VRB","wspd":2,"wgst":12}]"#).unwrap();
        assert_eq!(vrb.dir_deg, None);
        assert_eq!(vrb.gust_kt, Some(12));
        assert!(parse_metar_json("[]").is_none());
    }

    #[test]
    fn test_runway_from_wind_prefers_headwind() {
        let info = eglc();
        let easterly_wind = Wind {
            dir_deg: Some(110),
            speed_kt: 10,
            gust_kt: None,
        };
        let rwy = runway_from_wind(&info, &easterly_wind).unwrap();
        assert_eq!(rwy.ident, "09");
        assert_eq!(rwy.ops_direction(), "EASTERLY");
        assert_eq!(rwy.source, RunwaySource::Wind);

        let westerly_wind = Wind {
            dir_deg: Some(240),
            speed_kt: 15,
            gust_kt: Some(25),
        };
        let rwy = runway_from_wind(&info, &westerly_wind).unwrap();
        assert_eq!(rwy.ident, "27");
        assert_eq!(rwy.ops_direction(), "WESTERLY");

        let calm = Wind {
            dir_deg: Some(270),
            speed_kt: 2,
            gust_kt: None,
        };
        assert!(runway_from_wind(&info, &calm).is_none());
    }

    #[test]
    fn test_tracker_majority_and_expiry() {
        let t0 = Instant::now();
        let mut tracker = RunwayTracker::default();
        tracker.record("27", 273.0, t0);
        tracker.record("27", 273.0, t0 + Duration::from_secs(60));
        tracker.record("09", 93.0, t0 + Duration::from_secs(120));
        let rwy = tracker.current(t0 + Duration::from_secs(180)).unwrap();
        assert_eq!(rwy.ident, "27");
        assert_eq!(rwy.source, RunwaySource::Traffic);

        // After 31 minutes only the later 09 vote is left.
        let rwy = tracker
            .current(t0 + Duration::from_secs(61 + 30 * 60))
            .unwrap();
        assert_eq!(rwy.ident, "09");
        assert!(
            tracker
                .current(t0 + Duration::from_secs(3 * 3600))
                .is_none()
        );
    }
}
