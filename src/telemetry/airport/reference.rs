//! Reference data for the airport tile, compiled into the binary so it can never fail at runtime:
//! - Runway geometry from OurAirports `runways.csv` (public domain).
//! - Callsign → route table from the VRS standing data `routes.csv` (CC0).
//!
//! Both live in `data/airport/` and are refreshed by `scripts/update-airport-data.sh`.

use super::geo::bearing_deg;
use std::collections::HashMap;

const EMBEDDED_RUNWAYS_CSV: &str = include_str!("../../../data/airport/runways.csv");
const EMBEDDED_ROUTES_CSV: &str = include_str!("../../../data/airport/routes.csv");

/// One usable end of a runway (e.g. `27` at London City, heading 273° true).
#[derive(Debug, Clone, PartialEq)]
pub struct RunwayEnd {
    pub ident: String,
    pub heading_deg: f64,
    pub lat: f64,
    pub lon: f64,
}

/// Airport reference point and runway layout.
#[derive(Debug, Clone, PartialEq)]
pub struct AirportInfo {
    pub icao: String,
    pub lat: f64,
    pub lon: f64,
    pub elevation_ft: f64,
    /// Runways as `(low end, high end)` pairs, e.g. `(09, 27)`.
    pub runways: Vec<(RunwayEnd, RunwayEnd)>,
}

impl AirportInfo {
    pub fn runway_ends(&self) -> impl Iterator<Item = &RunwayEnd> {
        self.runways.iter().flat_map(|(le, he)| [le, he])
    }
}

/// How a callsign's published route touches the tracked airport.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteRole {
    /// Previous stop before the tracked airport (set when the flight can arrive here).
    pub from: Option<String>,
    /// Next stop after the tracked airport (set when the flight can depart from here).
    pub to: Option<String>,
}

/// Parses bundled runway rows (`airport_ident,closed,le_ident,le_latitude_deg,...`, see
/// `scripts/update-airport-data.sh`) for `icao`. Closed runways and runways without threshold
/// coordinates are skipped. The airport reference point is the mean of the runway ends.
pub fn parse_runway_rows(csv: &str, icao: &str) -> Option<AirportInfo> {
    let mut runways = Vec::new();
    let mut elevations = Vec::new();
    for line in csv.lines() {
        // Cheap prefix check first: the bundled table has ~48k rows.
        let Some(rest) = line.strip_prefix(icao).and_then(|r| r.strip_prefix(',')) else {
            continue;
        };
        let f: Vec<&str> = rest.split(',').map(str::trim).collect();
        if f.len() < 11 || f[0] == "1" {
            continue;
        }
        let (Ok(le_lat), Ok(le_lon), Ok(he_lat), Ok(he_lon)) = (
            f[2].parse::<f64>(),
            f[3].parse::<f64>(),
            f[7].parse::<f64>(),
            f[8].parse::<f64>(),
        ) else {
            continue;
        };
        let le_hdg = f[5]
            .parse::<f64>()
            .unwrap_or_else(|_| bearing_deg(le_lat, le_lon, he_lat, he_lon));
        let he_hdg = f[10]
            .parse::<f64>()
            .unwrap_or_else(|_| (le_hdg + 180.0).rem_euclid(360.0));
        elevations.extend([f[4], f[9]].iter().filter_map(|e| e.parse::<f64>().ok()));
        runways.push((
            RunwayEnd {
                ident: f[1].to_ascii_uppercase(),
                heading_deg: le_hdg,
                lat: le_lat,
                lon: le_lon,
            },
            RunwayEnd {
                ident: f[6].to_ascii_uppercase(),
                heading_deg: he_hdg,
                lat: he_lat,
                lon: he_lon,
            },
        ));
    }
    if runways.is_empty() {
        return None;
    }
    let n = (runways.len() * 2) as f64;
    let lat = runways.iter().map(|(a, b)| a.lat + b.lat).sum::<f64>() / n;
    let lon = runways.iter().map(|(a, b)| a.lon + b.lon).sum::<f64>() / n;
    let elevation_ft = if elevations.is_empty() {
        0.0
    } else {
        elevations.iter().sum::<f64>() / elevations.len() as f64
    };
    Some(AirportInfo {
        icao: icao.to_string(),
        lat,
        lon,
        elevation_ft,
        runways,
    })
}

/// Parses bundled route rows (`Callsign,AirportCodes`) into the routes that touch `icao`.
/// `AirportCodes` is a dash-separated list of stops, e.g. `EGLC-LEIB`.
pub fn parse_route_rows<'a>(
    lines: impl Iterator<Item = &'a str>,
    icao: &str,
) -> HashMap<String, RouteRole> {
    let mut routes = HashMap::new();
    // Cheap substring check first: the bundled table has ~620k rows.
    for line in lines.filter(|l| l.contains(icao)) {
        let Some((callsign, stops)) = line.trim().split_once(',') else {
            continue;
        };
        let stops: Vec<&str> = stops.split('-').collect();
        let Some(idx) = stops.iter().position(|s| *s == icao) else {
            continue;
        };
        let role = RouteRole {
            from: idx.checked_sub(1).map(|i| stops[i].to_string()),
            to: stops.get(idx + 1).map(|s| s.to_string()),
        };
        routes.insert(callsign.to_string(), role);
    }
    routes
}

/// Runway geometry for `icao` from the bundled OurAirports table (`None` for an unknown
/// ICAO code or an airport without runway coordinates).
pub fn embedded_airport_info(icao: &str) -> Option<AirportInfo> {
    parse_runway_rows(EMBEDDED_RUNWAYS_CSV, &icao.trim().to_ascii_uppercase())
}

/// Callsign → route table for `icao` from the bundled VRS table.
pub fn embedded_routes(icao: &str) -> HashMap<String, RouteRole> {
    parse_route_rows(
        EMBEDDED_ROUTES_CSV.lines(),
        &icao.trim().to_ascii_uppercase(),
    )
}

#[cfg(test)]
pub(super) mod fixtures {
    pub const EGLC_RUNWAYS_CSV: &str = "airport_ident,closed,le_ident,le_latitude_deg,le_longitude_deg,le_elevation_ft,le_heading_degT,he_ident,he_latitude_deg,he_longitude_deg,he_elevation_ft,he_heading_degT
EGLC,0,09,51.505576,0.044333,16,93,27,51.504894,0.066026,19,273
EGLC,1,04,51.5,0.04,16,40,22,51.51,0.05,16,220
EGLCX,0,18,51.5,0.04,16,180,36,51.51,0.04,16,360
EGLL,0,09L,51.47749,-0.489439,79,90,27R,51.477681,-0.433227,78,270
";

    pub const EGLC_ROUTES_CSV: &str = "Callsign,AirportCodes
CFE63BB,EGLC-LEIB
CFE8NV,EHAM-EGLC
KLM45A,EHAM-EGLC-EHAM
DLH2PN,EGLL-EDDF
XEGLC1,EGLCX-EDDF
";
}

#[cfg(test)]
mod tests {
    use super::fixtures::*;
    use super::*;

    #[test]
    fn test_parse_runway_rows_for_eglc() {
        let info = parse_runway_rows(EGLC_RUNWAYS_CSV, "EGLC").expect("EGLC runways");
        assert_eq!(info.runways.len(), 1);
        let (le, he) = &info.runways[0];
        assert_eq!((le.ident.as_str(), le.heading_deg), ("09", 93.0));
        assert_eq!((he.ident.as_str(), he.heading_deg), ("27", 273.0));
        assert!((info.lat - 51.5052).abs() < 0.001);
        assert!((info.lon - 0.0552).abs() < 0.001);
        assert!((info.elevation_ft - 17.5).abs() < 0.01);
        assert!(parse_runway_rows(EGLC_RUNWAYS_CSV, "KJFK").is_none());
    }

    #[test]
    fn test_parse_route_rows_roles() {
        let routes = parse_route_rows(EGLC_ROUTES_CSV.lines(), "EGLC");
        assert_eq!(routes.len(), 3);
        assert_eq!(
            routes["CFE63BB"],
            RouteRole {
                from: None,
                to: Some("LEIB".into())
            }
        );
        assert_eq!(
            routes["CFE8NV"],
            RouteRole {
                from: Some("EHAM".into()),
                to: None
            }
        );
        assert_eq!(
            routes["KLM45A"],
            RouteRole {
                from: Some("EHAM".into()),
                to: Some("EHAM".into())
            }
        );
        assert!(!routes.contains_key("DLH2PN"));
    }

    #[test]
    fn test_embedded_data_covers_london_city() {
        let info = embedded_airport_info("EGLC").expect("EGLC is bundled");
        let idents: Vec<_> = info.runway_ends().map(|e| e.ident.as_str()).collect();
        assert_eq!(idents, vec!["09", "27"]);
        let routes = embedded_routes("EGLC");
        assert!(
            routes.len() > 100,
            "only {} EGLC routes bundled",
            routes.len()
        );
        assert!(routes.values().any(|r| r.from.is_some()));
        assert!(routes.values().any(|r| r.to.is_some()));
        assert!(embedded_airport_info("ZZZZ").is_none());
    }
}
