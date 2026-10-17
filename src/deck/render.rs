use crate::config::{QDeckConfig, ResolvedTheme};
use crate::widgets::{TelemetrySnapshot, render_standby_tile, render_tile_image};
use anyhow::Result;
use image::DynamicImage;
use std::collections::HashSet;
use streamdeck::{Colour, StreamDeck};

#[allow(clippy::too_many_arguments)]
pub fn render_key_to_deck(
    deck: &mut StreamDeck,
    cfg: &QDeckConfig,
    theme: &ResolvedTheme,
    cols: u8,
    total_keys: u8,
    img_w: usize,
    img_h: usize,
    key_idx: u8,
    snapshot: &TelemetrySnapshot,
    is_highlighted: bool,
) -> Result<()> {
    if let Some(tile) = cfg.tile_for_key(
        key_idx,
        cols,
        total_keys,
        snapshot.active_mode,
        snapshot.active_panel,
    ) {
        let img = render_tile_image(
            img_w as u32,
            img_h as u32,
            key_idx,
            tile,
            theme,
            snapshot,
            is_highlighted,
        );
        deck.set_button_image(key_idx, DynamicImage::ImageRgb8(img))?;
    } else if cfg.background_mode == "blank" && !is_highlighted {
        let bg = Colour {
            r: theme.background.0[0],
            g: theme.background.0[1],
            b: theme.background.0[2],
        };
        deck.set_button_rgb(key_idx, &bg)?;
    } else {
        let standby_img =
            render_standby_tile(img_w as u32, img_h as u32, key_idx, theme, is_highlighted);
        deck.set_button_image(key_idx, DynamicImage::ImageRgb8(standby_img))?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn apply_mode_layout(
    deck: &mut StreamDeck,
    cfg: &QDeckConfig,
    theme: &ResolvedTheme,
    cols: u8,
    total_keys: u8,
    img_w: usize,
    img_h: usize,
    snapshot: &TelemetrySnapshot,
) -> Result<()> {
    deck.set_brightness(cfg.brightness)?;
    let mut configured_keys = HashSet::new();

    for k in 0..total_keys {
        if let Some(tile) = cfg.tile_for_key(
            k,
            cols,
            total_keys,
            snapshot.active_mode,
            snapshot.active_panel,
        ) {
            configured_keys.insert(k);
            println!(
                "[QDECK] [mode={}/panel={}] Key {} (row {}, col {}) -> '{}' (type='{}') | click={:?}",
                snapshot.active_mode,
                snapshot.active_panel,
                k,
                k / cols,
                k % cols,
                tile.label
                    .as_deref()
                    .or(tile.control.as_deref())
                    .unwrap_or(&tile.tile_type),
                tile.tile_type,
                tile.click_action()
            );
            render_key_to_deck(
                deck, cfg, theme, cols, total_keys, img_w, img_h, k, snapshot, false,
            )?;
        }
    }

    for k in 0..total_keys {
        if !configured_keys.contains(&k) {
            if cfg.background_mode == "blank" {
                let bg = Colour {
                    r: theme.background.0[0],
                    g: theme.background.0[1],
                    b: theme.background.0[2],
                };
                deck.set_button_rgb(k, &bg)?;
            } else {
                let standby_img = render_standby_tile(img_w as u32, img_h as u32, k, theme, false);
                deck.set_button_image(k, DynamicImage::ImageRgb8(standby_img))?;
            }
        }
    }
    Ok(())
}
