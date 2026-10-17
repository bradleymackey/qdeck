use crate::config::QDeckConfig;
use anyhow::{Context, Result, anyhow};
use std::fs;
use std::path::Path;
use std::time::SystemTime;
use streamdeck::{Colour, Kind, StreamDeck, pids};

/// Returns the number of columns on the given Stream Deck hardware model.
pub fn cols_for_kind(kind: Kind) -> u8 {
    match kind {
        Kind::Mini | Kind::RevisedMini | Kind::Module6Keys => 3,
        Kind::Original | Kind::OriginalV2 | Kind::Mk2 | Kind::Module15Keys => 5,
        Kind::Xl | Kind::Module32Keys => 8,
        Kind::Plus => 4,
    }
}

/// Blanks all keys to pure black (`#000000`) and turns Stream Deck brightness down to `0%`.
pub fn blank_and_sleep_deck(deck: &mut StreamDeck, total_keys: u8) {
    let black = Colour { r: 0, g: 0, b: 0 };
    for k in 0..total_keys {
        let _ = deck.set_button_rgb(k, &black);
    }
    let _ = deck.set_brightness(0);
}

/// Connects to an Elgato Stream Deck via explicit PID, USB auto-probe, or MK.2 fallback.
pub fn connect_deck(cfg: &QDeckConfig) -> Result<StreamDeck> {
    if let Some(pid) = cfg.pid {
        return StreamDeck::connect(cfg.vid, pid, cfg.serial.clone()).with_context(|| {
            format!(
                "Failed to connect to StreamDeck (VID {:04x}, PID {:04x})",
                cfg.vid, pid
            )
        });
    }

    if let Ok(found) = StreamDeck::probe() {
        for item in found.into_iter().flatten() {
            let (kind, pid) = item;
            if let Ok(deck) = StreamDeck::connect(cfg.vid, pid, cfg.serial.clone()) {
                println!(
                    "[QDECK] Auto-detected Stream Deck {:?} (VID: {:04x}, PID: {:04x})",
                    kind, cfg.vid, pid
                );
                return Ok(deck);
            }
        }
    }

    StreamDeck::connect(cfg.vid, pids::MK2, cfg.serial.clone()).map_err(|e| {
        anyhow!(
            "Could not connect to Stream Deck: {e}. Make sure the Stream Deck is plugged in and the official Elgato app is closed."
        )
    })
}

/// Returns the last modification timestamp of a configuration file, if accessible.
pub fn file_mtime(path: &Path) -> Option<SystemTime> {
    fs::metadata(path).ok().and_then(|m| m.modified().ok())
}
