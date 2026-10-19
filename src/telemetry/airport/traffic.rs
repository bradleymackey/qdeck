//! Live ADS-B traffic around the airport (adsb.fi open data, adsb.lol as fallback), sorted into
//! expected arrivals, departures, and runway-alignment observations.

use super::geo::{angle_diff_deg, bearing_deg, cross_track_nm, distance_nm};
use super::reference::{AirportInfo, RouteRole};
use std::collections::HashMap;

/// Search radius around the airport. At typical cruise speeds this shows arrivals ~30 min out.
pub const TRAFFIC_RADIUS_NM: u32 = 200;
/// Departures are listed for this long after take-off.
pub const RECENT_DEPARTURE_MINUTES: u32 = 45;
/// Most entries kept per list.
const MAX_FLIGHTS: usize = 12;

#[derive(Debug, Clone, PartialEq)]
pub struct Aircraft {
    pub callsign: Option<String>,
    pub lat: f64,
    pub lon: f64,
    /// Barometric altitude in feet (`None` while on the ground).
    pub alt_ft: Option<f64>,
    pub on_ground: bool,
    pub gs_kt: Option<f64>,
    pub track_deg: Option<f64>,
    pub vert_rate_fpm: Option<f64>,
    pub rotorcraft: bool,
}

/// One row on the arrivals or departures page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AirportFlight {
    pub callsign: String,
    /// Origin (arrivals) or destination (departures), as an ICAO code.
    pub other_airport: String,
    /// Arrivals: estimated minutes to landing. Departures: minutes since take-off (estimated
    /// from distance flown and climb when first seen).
    pub minutes: u32,
    /// Departures only: still on the ground at the airport (taxiing or at the stand).
    pub on_ground: bool,
}

/// Traffic sorted for one poll.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TrafficPicture {
    pub arrivals: Vec<AirportFlight>,
    pub departures: Vec<AirportFlight>,
    /// Runway ends (ident, heading) that low traffic near the airport was aligned with.
    pub runway_votes: Vec<(String, f64)>,
}

/// URLs to try in order for traffic around `(lat, lon)`.
pub fn traffic_urls(lat: f64, lon: f64) -> [String; 2] {
    let nm = TRAFFIC_RADIUS_NM;
    [
        format!("https://opendata.adsb.fi/api/v2/lat/{lat:.4}/lon/{lon:.4}/dist/{nm}"),
        format!("https://api.adsb.lol/v2/point/{lat:.4}/{lon:.4}/{nm}"),
    ]
}

/// Parses a readsb-style JSON response (`aircraft` on adsb.fi, `ac` on adsb.lol).
pub fn parse_adsb_json(json: &str) -> Option<Vec<Aircraft>> {
    let value: serde_json::Value = serde_json::from_str(json).ok()?;
    let list = value
        .get("aircraft")
        .or_else(|| value.get("ac"))?
        .as_array()?;
    Some(
        list.iter()
            .filter_map(|a| {
                let num = |key: &str| a.get(key).and_then(serde_json::Value::as_f64);
                let on_ground = a.get("alt_baro").and_then(|v| v.as_str()) == Some("ground");
                Some(Aircraft {
                    callsign: a
                        .get("flight")
                        .and_then(|v| v.as_str())
                        .map(|s| s.trim().to_ascii_uppercase())
                        .filter(|s| !s.is_empty()),
                    lat: num("lat")?,
                    lon: num("lon")?,
                    alt_ft: if on_ground { None } else { num("alt_baro") },
                    on_ground,
                    gs_kt: num("gs"),
                    track_deg: num("track").or_else(|| num("true_heading")),
                    vert_rate_fpm: num("baro_rate").or_else(|| num("geom_rate")),
                    rotorcraft: a.get("category").and_then(|v| v.as_str()) == Some("A7"),
                })
            })
            .collect(),
    )
}

/// Sorts aircraft into arrivals and departures using each callsign's published route, and
/// collects runway-alignment votes from aircraft low on approach or just after take-off.
pub fn classify_traffic(
    info: &AirportInfo,
    routes: &HashMap<String, RouteRole>,
    aircraft: &[Aircraft],
) -> TrafficPicture {
    let mut picture = TrafficPicture::default();

    for ac in aircraft {
        let dist = distance_nm(ac.lat, ac.lon, info.lat, info.lon);
        let role = ac.callsign.as_ref().and_then(|c| routes.get(c));

        if let Some(end) = aligned_runway_end(info, ac, dist, role.is_some()) {
            picture.runway_votes.push(end);
        }

        let (Some(callsign), Some(role)) = (&ac.callsign, role) else {
            continue;
        };

        if ac.on_ground {
            // Only aircraft on the ground here matter: these are departures getting ready.
            if dist <= 2.5
                && let Some(to) = &role.to
            {
                picture.departures.push(AirportFlight {
                    callsign: callsign.clone(),
                    other_airport: to.clone(),
                    minutes: 0,
                    on_ground: true,
                });
            }
            continue;
        }

        let gs = ac.gs_kt.unwrap_or(0.0);
        if gs < 40.0 {
            continue;
        }
        let inbound = match (&role.from, &role.to) {
            (Some(_), None) => true,
            (None, Some(_)) => false,
            _ => is_heading_towards(info, ac, dist),
        };

        if inbound {
            if let Some(from) = &role.from
                && (dist <= 15.0 || is_heading_towards(info, ac, dist))
            {
                // Straight-line time plus ~15% for descent, vectoring and approach speeds.
                let minutes = (dist / gs.max(140.0) * 60.0 * 1.15).round().max(1.0) as u32;
                picture.arrivals.push(AirportFlight {
                    callsign: callsign.clone(),
                    other_airport: from.clone(),
                    minutes,
                    on_ground: false,
                });
            }
        } else if let Some(to) = &role.to {
            // Straight-line time at current speed plus ~25% for the slower climb.
            let minutes = (dist / gs.max(160.0) * 60.0 * 1.25).round().max(1.0) as u32;
            if minutes <= RECENT_DEPARTURE_MINUTES && !is_heading_towards(info, ac, dist) {
                picture.departures.push(AirportFlight {
                    callsign: callsign.clone(),
                    other_airport: to.clone(),
                    minutes,
                    on_ground: false,
                });
            }
        }
    }

    picture
        .arrivals
        .sort_by(|a, b| a.minutes.cmp(&b.minutes).then(a.callsign.cmp(&b.callsign)));
    picture.departures.sort_by(|a, b| {
        b.on_ground
            .cmp(&a.on_ground)
            .then(a.minutes.cmp(&b.minutes))
            .then(a.callsign.cmp(&b.callsign))
    });
    picture.arrivals.truncate(MAX_FLIGHTS);
    picture.departures.truncate(MAX_FLIGHTS);
    picture
}

fn is_heading_towards(info: &AirportInfo, ac: &Aircraft, dist: f64) -> bool {
    if dist < 1.0 {
        return true;
    }
    ac.track_deg.is_some_and(|trk| {
        angle_diff_deg(trk, bearing_deg(ac.lat, ac.lon, info.lat, info.lon)) < 90.0
    })
}

/// Returns the runway end an aircraft is flying along, if it is low and close enough to be
/// landing or taking off. Aircraft without a known route must sit on the extended centreline,
/// which keeps helicopters and overflights from voting.
fn aligned_runway_end(
    info: &AirportInfo,
    ac: &Aircraft,
    dist: f64,
    has_route: bool,
) -> Option<(String, f64)> {
    if ac.rotorcraft {
        return None;
    }
    let track = ac.track_deg?;
    let gs = ac.gs_kt.unwrap_or(0.0);
    let height = ac.alt_ft.map(|a| a - info.elevation_ft);
    let close_and_low = if ac.on_ground {
        // Take-off or landing roll.
        dist <= 1.5 && gs >= 40.0
    } else if has_route {
        dist <= 8.0 && height.is_some_and(|h| h < 4000.0) && gs >= 70.0
    } else {
        dist <= 5.0 && height.is_some_and(|h| h < 2500.0) && gs >= 90.0
    };
    if !close_and_low {
        return None;
    }

    info.runways
        .iter()
        .filter(|(le, he)| {
            has_route
                || ac.on_ground
                || cross_track_nm((ac.lat, ac.lon), (le.lat, le.lon), (he.lat, he.lon)) <= 0.6
        })
        .flat_map(|(le, he)| [le, he])
        .map(|end| (end, angle_diff_deg(track, end.heading_deg)))
        .filter(|(_, diff)| *diff <= 25.0)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(end, _)| (end.ident.clone(), end.heading_deg))
}

#[cfg(test)]
mod tests {
    use super::super::reference::fixtures::{EGLC_ROUTES_CSV, EGLC_RUNWAYS_CSV};
    use super::super::reference::{parse_route_rows, parse_runway_rows};
    use super::*;

    fn aircraft(
        callsign: &str,
        lat: f64,
        lon: f64,
        alt: Option<f64>,
        gs: f64,
        trk: f64,
    ) -> Aircraft {
        Aircraft {
            callsign: Some(callsign.to_string()),
            lat,
            lon,
            alt_ft: alt,
            on_ground: alt.is_none(),
            gs_kt: Some(gs),
            track_deg: Some(trk),
            vert_rate_fpm: None,
            rotorcraft: false,
        }
    }

    #[test]
    fn test_parse_adsb_json_both_feeds() {
        let fi = r#"{"now":1,"aircraft":[
            {"hex":"1","flight":"CFE63BB ","lat":51.48,"lon":0.28,"alt_baro":10725,"gs":257.2,"track":142.1,"baro_rate":1024,"category":"A3"},
            {"hex":"2","flight":"HLE60","lat":51.5,"lon":0.0,"alt_baro":"ground","gs":0.0,"category":"A7"},
            {"hex":"3","alt_baro":5000}
        ]}"#;
        let list = parse_adsb_json(fi).unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].callsign.as_deref(), Some("CFE63BB"));
        assert_eq!(list[0].alt_ft, Some(10725.0));
        assert!(list[1].on_ground && list[1].rotorcraft && list[1].alt_ft.is_none());

        let lol = r#"{"ac":[{"flight":"ITY227","lat":51.56,"lon":0.2,"alt_baro":6750,"gs":261.7,"track":67.1}]}"#;
        assert_eq!(parse_adsb_json(lol).unwrap().len(), 1);
        assert!(parse_adsb_json(r#"{"msg":"rate limited"}"#).is_none());
    }

    #[test]
    fn test_classify_arrivals_departures_and_runway() {
        let info = parse_runway_rows(EGLC_RUNWAYS_CSV, "EGLC").unwrap();
        let routes = parse_route_rows(EGLC_ROUTES_CSV.lines(), "EGLC");
        let traffic = [
            // Inbound from Amsterdam, ~60 nm east, heading west towards London.
            aircraft("CFE8NV", 51.55, 1.65, Some(14000.0), 300.0, 268.0),
            // Short final for runway 27: 3 nm east of the field, tracking 273.
            aircraft("KLM45A", 51.503, 0.135, Some(1100.0), 135.0, 273.0),
            // Just departed for Ibiza, climbing out westbound off runway 27.
            aircraft("CFE63BB", 51.507, 0.0, Some(1800.0), 170.0, 271.0),
            // Not an EGLC route, so ignored.
            aircraft("DLH2PN", 51.4, 0.2, Some(8000.0), 263.0, 90.0),
        ];
        let picture = classify_traffic(&info, &routes, &traffic);

        let arr: Vec<_> = picture
            .arrivals
            .iter()
            .map(|f| f.callsign.as_str())
            .collect();
        assert_eq!(arr, vec!["KLM45A", "CFE8NV"]);
        assert_eq!(picture.arrivals[0].other_airport, "EHAM");
        assert!(picture.arrivals[0].minutes <= 2);
        assert!((12..=16).contains(&picture.arrivals[1].minutes));

        assert_eq!(picture.departures.len(), 1);
        assert_eq!(picture.departures[0].other_airport, "LEIB");
        assert!(!picture.departures[0].on_ground);

        assert_eq!(picture.runway_votes.len(), 2);
        assert!(picture.runway_votes.iter().all(|(id, _)| id == "27"));
    }

    #[test]
    fn test_ground_departures_listed_first() {
        let info = parse_runway_rows(EGLC_RUNWAYS_CSV, "EGLC").unwrap();
        let routes = parse_route_rows(EGLC_ROUTES_CSV.lines(), "EGLC");
        let traffic = [
            aircraft("KLM45A", 51.62, -0.2, Some(9000.0), 300.0, 300.0),
            aircraft("CFE63BB", 51.5049, 0.06, None, 12.0, 90.0),
        ];
        let picture = classify_traffic(&info, &routes, &traffic);
        assert_eq!(picture.departures.len(), 2);
        assert!(picture.departures[0].on_ground);
        assert_eq!(picture.departures[0].callsign, "CFE63BB");
        assert_eq!(picture.departures[1].callsign, "KLM45A");
        // Slow taxi on the ground does not vote for a runway.
        assert!(picture.runway_votes.is_empty());
    }

    #[test]
    fn test_unrouted_traffic_needs_centreline() {
        let info = parse_runway_rows(EGLC_RUNWAYS_CSV, "EGLC").unwrap();
        let routes = HashMap::new();
        let mut on_centreline = aircraft("N123", 51.5055, 0.0, Some(1200.0), 130.0, 93.0);
        let off_centreline = aircraft("N456", 51.53, 0.0, Some(1200.0), 130.0, 93.0);
        let mut heli = on_centreline.clone();
        heli.rotorcraft = true;
        let picture = classify_traffic(
            &info,
            &routes,
            &[on_centreline.clone(), off_centreline, heli],
        );
        assert_eq!(picture.runway_votes, vec![("09".to_string(), 93.0)]);

        on_centreline.track_deg = Some(150.0);
        assert!(
            classify_traffic(&info, &routes, &[on_centreline])
                .runway_votes
                .is_empty()
        );
    }
}
