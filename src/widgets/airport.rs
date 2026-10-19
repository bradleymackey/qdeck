use super::TelemetrySnapshot;
use super::icons::draw_heading_arrow;
use crate::config::{ResolvedTheme, TileConfig};
use crate::font::{
    draw_hline, draw_rect_border, draw_text_3x5, draw_text_5x7, fill_rect, scale_color,
    text_width_5x7,
};
use crate::telemetry::{AirportFlight, AirportStatus, RunwaySource, Wind};
use image::{Rgb, RgbImage};

/// Arrivals landing within this many minutes are drawn at full brightness.
const IMMINENT_MINUTES: u32 = 5;
const LIST_ROWS: usize = 6;

/// Renders the airport tile with 3 pages cycled on click:
/// - Page 0 (`1/3`): runway in use (ident, direction arrow, EASTERLY/WESTERLY ops), counts, wind
/// - Page 1 (`2/3`): expected arrivals with origin and ETA in minutes
/// - Page 2 (`3/3`): departures on the ground (`GND`) and minutes since recent take-offs
pub fn render_airport_tile(
    width: u32,
    height: u32,
    tile: &TileConfig,
    theme: &ResolvedTheme,
    telemetry: &TelemetrySnapshot,
    is_highlighted: bool,
) -> RgbImage {
    let w = width as i32;
    let h = height as i32;
    let bg = if is_highlighted {
        scale_color(theme.phosphor_green, 0.20)
    } else {
        theme.background
    };
    let mut img = RgbImage::from_pixel(width, height, bg);
    if is_highlighted {
        draw_rect_border(&mut img, 0, 0, w, h, 2, theme.phosphor_green);
    }

    let icao = tile.airport_icao();
    let status = &telemetry.airport;
    let page = status.page_index % 3;
    let header = match page {
        0 => tile.label.clone().unwrap_or_else(|| icao.clone()),
        1 => format!("ARR {}", status.arrivals.len()),
        _ => format!("DEP {}", status.departures.len()),
    };
    let live = status.icao == icao && status.traffic_available;
    let header_col = if live {
        theme.phosphor_green
    } else {
        theme.phosphor_dim
    };
    draw_text_5x7(&mut img, 5, 5, &header, 1, header_col);
    let badge = format!("{}/3", page + 1);
    let bw = (badge.len() as i32) * 4 - 1;
    draw_text_3x5(&mut img, w - 12 - bw, 6, &badge, 1, theme.phosphor_dim);
    fill_rect(
        &mut img,
        w - 9,
        7,
        4,
        4,
        if live {
            theme.phosphor_green
        } else {
            theme.grid_dim
        },
    );
    draw_hline(&mut img, 5, 14, w - 10, theme.grid_dim);

    if status.icao != icao {
        draw_centered_3x5(&mut img, w, 34, "LOADING", theme.phosphor_dim);
        return img;
    }
    if !status.known_airport {
        draw_centered_3x5(&mut img, w, 30, "UNKNOWN ICAO", theme.amber);
        draw_centered_3x5(&mut img, w, 40, &icao, theme.phosphor_dim);
        return img;
    }

    match page {
        0 => draw_runway_page(&mut img, w, status, theme),
        1 => draw_flight_list(&mut img, w, &status.arrivals, status, "NO INBOUND", theme),
        _ => draw_flight_list(&mut img, w, &status.departures, status, "NO DEPS", theme),
    }
    img
}

fn draw_runway_page(img: &mut RgbImage, w: i32, status: &AirportStatus, theme: &ResolvedTheme) {
    match &status.runway {
        Some(rwy) => {
            // A runway picked from the wind alone is a best guess, so it is shown in amber.
            let (rwy_col, source) = match rwy.source {
                RunwaySource::Traffic => (theme.phosphor_green, "ADS-B"),
                RunwaySource::Wind => (theme.amber, "WIND"),
            };
            draw_text_5x7(img, 5, 18, &rwy.ident, 2, rwy_col);
            let ident_w = text_width_5x7(&rwy.ident, 2);
            let arrow_cx = (5 + ident_w + w - 4) / 2;
            draw_heading_arrow(img, arrow_cx, 24, 22, rwy.heading_deg, rwy_col);
            draw_text_3x5(img, 5, 35, rwy.ops_direction(), 1, rwy_col);
            let sw = (source.len() as i32) * 4 - 1;
            draw_text_3x5(img, w - 5 - sw, 35, source, 1, theme.phosphor_dim);
        }
        None => {
            draw_text_5x7(img, 5, 18, "--", 2, theme.phosphor_dim);
            draw_text_3x5(img, 5, 35, "RWY UNKNOWN", 1, theme.phosphor_dim);
        }
    }
    draw_hline(img, 5, 43, w - 10, theme.grid_dim);

    let value_col = if status.traffic_available {
        theme.phosphor_green
    } else {
        theme.phosphor_dim
    };
    draw_label_value(
        img,
        5,
        47,
        "ARR",
        &status.arrivals.len().to_string(),
        value_col,
        theme,
    );
    if let Some(next) = status.arrivals.first() {
        let text = format!("NEXT {}M", next.minutes.min(99));
        let tw = (text.len() as i32) * 4 - 1;
        draw_text_3x5(img, w - 5 - tw, 47, &text, 1, value_col);
    }
    let ground = status.departures.iter().filter(|f| f.on_ground).count();
    draw_label_value(
        img,
        5,
        55,
        "DEP",
        &status.departures.len().to_string(),
        value_col,
        theme,
    );
    if ground > 0 {
        let text = format!("GND {ground}");
        let tw = (text.len() as i32) * 4 - 1;
        draw_text_3x5(img, w - 5 - tw, 55, &text, 1, value_col);
    }
    draw_text_3x5(img, 5, 63, &format_wind(status.wind), 1, theme.phosphor_dim);
}

fn draw_flight_list(
    img: &mut RgbImage,
    w: i32,
    flights: &[AirportFlight],
    status: &AirportStatus,
    empty_text: &str,
    theme: &ResolvedTheme,
) {
    if flights.is_empty() {
        let text = if status.traffic_available {
            empty_text
        } else {
            "NO DATA"
        };
        draw_centered_3x5(img, w, 34, text, theme.phosphor_dim);
        return;
    }
    for (row, flight) in flights.iter().take(LIST_ROWS).enumerate() {
        let y = 18 + (row as i32) * 8;
        let col = if flight.on_ground || flight.minutes <= IMMINENT_MINUTES {
            theme.phosphor_green
        } else {
            theme.phosphor_dim
        };
        let callsign: String = flight.callsign.chars().take(8).collect();
        draw_text_3x5(img, 3, y, &callsign, 1, col);
        draw_text_3x5(img, 37, y, &flight.other_airport, 1, theme.phosphor_dim);
        let time = if flight.on_ground {
            "GND".to_string()
        } else {
            format!("{}M", flight.minutes.min(99))
        };
        let tw = (time.len() as i32) * 4 - 1;
        draw_text_3x5(img, w - 4 - tw, y, &time, 1, col);
    }
}

fn draw_label_value(
    img: &mut RgbImage,
    x: i32,
    y: i32,
    label: &str,
    value: &str,
    value_col: Rgb<u8>,
    theme: &ResolvedTheme,
) {
    draw_text_3x5(img, x, y, label, 1, theme.phosphor_dim);
    draw_text_3x5(
        img,
        x + (label.len() as i32) * 4 + 3,
        y,
        value,
        1,
        value_col,
    );
}

fn draw_centered_3x5(img: &mut RgbImage, w: i32, y: i32, text: &str, col: Rgb<u8>) {
    let tw = (text.len() as i32) * 4 - 1;
    draw_text_3x5(img, ((w - tw) / 2).max(2), y, text, 1, col);
}

/// Formats METAR wind as e.g. `WIND 110/10`, `WIND 240/15G25`, `WIND VRB/03` or `WIND --`.
pub fn format_wind(wind: Option<Wind>) -> String {
    match wind {
        None => "WIND --".to_string(),
        Some(wd) => {
            let dir = wd
                .dir_deg
                .map_or_else(|| "VRB".to_string(), |d| format!("{d:03}"));
            let gust = wd.gust_kt.map(|g| format!("G{g}")).unwrap_or_default();
            format!("WIND {dir}/{:02}{gust}", wd.speed_kt)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::QDeckConfig;
    use crate::telemetry::{AirportStatus, RunwayInUse};

    #[test]
    fn test_format_wind() {
        assert_eq!(format_wind(None), "WIND --");
        let wind = Wind {
            dir_deg: Some(90),
            speed_kt: 7,
            gust_kt: None,
        };
        assert_eq!(format_wind(Some(wind)), "WIND 090/07");
        let gusty = Wind {
            dir_deg: None,
            speed_kt: 12,
            gust_kt: Some(25),
        };
        assert_eq!(format_wind(Some(gusty)), "WIND VRB/12G25");
    }

    #[test]
    fn test_airport_pages_render_distinct_frames() {
        let cfg = QDeckConfig::builtin_default();
        let theme = cfg.theme.resolve();
        let tile = cfg
            .tiles
            .iter()
            .find(|t| t.tile_type == "airport")
            .expect("default config has an airport tile");
        let fcu = crate::xplane::A320FcuState::default();
        let flight = |cs: &str, other: &str, minutes: u32, on_ground: bool| AirportFlight {
            callsign: cs.into(),
            other_airport: other.into(),
            minutes,
            on_ground,
        };
        let mut status = AirportStatus {
            icao: "EGLC".into(),
            known_airport: true,
            runway: Some(RunwayInUse {
                ident: "27".into(),
                heading_deg: 273.0,
                source: RunwaySource::Traffic,
            }),
            wind: Some(Wind {
                dir_deg: Some(250),
                speed_kt: 12,
                gust_kt: None,
            }),
            arrivals: vec![
                flight("CFE8NV", "EHAM", 4, false),
                flight("KLM45A", "EHAM", 17, false),
            ],
            departures: vec![
                flight("CFE63BB", "LEIB", 0, true),
                flight("SWR455", "LSZH", 6, false),
            ],
            traffic_available: true,
            page_index: 0,
        };

        let mut frames = Vec::new();
        for page in 0..3 {
            status.page_index = page;
            let snap = super::super::tests::snapshot_with_airport(&fcu, status.clone());
            let img = render_airport_tile(72, 72, tile, &theme, &snap, false);
            let lit = img.pixels().filter(|p| p.0 != [0, 0, 0]).count();
            assert!(lit > 60, "page {page} rendered only {lit} lit pixels");
            frames.push(img.into_raw());
        }
        assert_ne!(frames[0], frames[1]);
        assert_ne!(frames[1], frames[2]);
    }
}
