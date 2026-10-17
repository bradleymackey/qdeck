# AGENTS.md — Project Rules & Engineering Guidelines

This file documents the architectural conventions, hardware rules, cockpit design decisions, and operational workflows established for **`qdeck`**. Any agent or contributor modifying this repository must follow these rules.

---

## 1. Architecture & Code Organization

- **Modular Library + Thin Binary**:
  - Keep [`src/main.rs`](src/main.rs) minimal (CLI flags, signal handlers, top-level reconnect loop).
  - Place all domain logic inside the library crate ([`src/lib.rs`](src/lib.rs)) organized into focused sub-modules:
    - [`src/config/`](src/config/) — TOML schema, theme resolution, default config generation, and sub-panel tile lookup (`effective_tile_for_key`).
    - [`src/xplane/`](src/xplane/) — X-Plane 12 state (`state.rs`), command dispatch (`commands.rs`), Local WebAPI client (`webapi.rs`), and client coordinator (`mod.rs`).
    - [`src/widgets/`](src/widgets/) — Tile rendering split across [`system.rs`](src/widgets/system.rs), [`navigation.rs`](src/widgets/navigation.rs), [`airbus.rs`](src/widgets/airbus.rs), and [`icons.rs`](src/widgets/icons.rs).
    - [`src/font.rs`](src/font.rs) — `5×7` and `3×5` bitmap font rasterizers and basic drawing primitives.
    - [`src/telemetry.rs`](src/telemetry.rs) — macOS system metrics collection.
    - [`src/macos.rs`](src/macos.rs) — macOS CoreGraphics screen-lock / display-sleep detection.
    - [`src/deck.rs`](src/deck.rs) — Stream Deck event loop, dirty-tile tracking, and screen-blanking drop guard.
- **Avoid Monolithic Files**:
  - Do not combine state definitions, network protocols, and pixel rendering into a single file. When adding new cockpit controls or widgets, keep state in `src/xplane/state.rs`, command dispatch in `src/xplane/commands.rs`, vector icons in `src/widgets/icons.rs`, and tile layouts in `src/widgets/airbus.rs`.

---

## 2. Visual & Hardware UX Rules

- **1Hz (`1000ms`) Refresh Loop + Immediate Tactile Feedback**:
  - The default periodic refresh interval (`refresh_ms`) is **`1000` ms (1Hz)** to keep idle CPU and USB HID traffic low.
  - Button presses must feel instantaneous: poll button input at short intervals (`30ms`), mark affected tiles dirty immediately on press, and render a **120ms** phosphor-green highlight border flash (`TAP_HIGHLIGHT_DURATION`).
- **Screen Blanking on Exit & macOS Display Lock**:
  - The Stream Deck screen must **never** remain stuck on the last rendered frame when `qdeck` exits, crashes, or when the Mac screen is locked.
  - `ScreenBlankGuard` (`src/deck.rs`) blanks all keys, calls `deck.reset()`, and sets brightness to `0%` on normal exit, `SIGINT`/`SIGTERM`, or panic.
  - While `macos::is_screen_locked()` returns `true` (`CGSessionCopyCurrentDictionary` / `CGDisplayIsAsleep`), blank the Stream Deck once and pause rendering until unlocked, then restore full brightness and redraw all 15 tiles.
- **Iconography Required for Flight Controls**:
  - Do **not** rely on text alone (such as `"UP"` / `"DN"`) for flight control surface or brake tiles.
  - Always pair flight controls with clear at-a-glance icons in [`src/widgets/icons.rs`](src/widgets/icons.rs) and [`src/widgets/airbus.rs`](src/widgets/airbus.rs):
    - Up/Down directional arrows (`draw_arrow_up`, `draw_arrow_down`) for Flaps, Speedbrakes, and Landing Gear.
    - Visual 4-notch flap bar (`draw_flap_notches`), wing spoiler cross-section (`draw_speedbrake_icon`), 3-triangle landing gear formation, brake caliper/rotor (`draw_brake_icon`), 4-blade brake fan impeller (`draw_fan_icon`), and 3-bar autobrake deceleration meter (`draw_autobrake_meter`).
- **Strict `72×72` Tile Geometry**:
  - Stream Deck MK.2 keys are `72×72` pixels.
  - `5×7` font characters are `6px` wide (`5px` glyph + `1px` spacing); `3×5` font characters are `4px` wide. Verify coordinates so text and icons never overlap or clip tile borders.

---

## 3. X-Plane 12 & Airbus Cockpit Rules

- **4-Panel Layout (`fcu` → `apu` → `flt` → `sys`)**:
  - On the `xplane` page:
    - **Key `#1`** is always the `BACK` button (`target = "root"`).
    - **Key `#2`** is always the `panel_switch` button cycling `fcu` → `apu` → `flt` → `sys`.
    - **Keys `#3–#15`** are overlaid from `[[pages.panels]]` for the active sub-panel.
- **No Engine 1 / Engine 2 Master Switches**:
  - Do **not** add Engine 1 or Engine 2 master switches to the Stream Deck panels — the user has dedicated physical joystick/throttle hardware for Engine 1 & 2 master switches. Keep `ENG MOD` (`NORM` / `IGN/STR` / `CRANK`), APU, fuel pumps, bleed, and packs on Panel 2 (`apu`).
- **WebAPI + UDP Dual Transport**:
  - X-Plane 12's local RakNet UDP port (`49010`) can fail to initialize on macOS. Therefore, `qdeck` uses **X-Plane 12's Local WebAPI (`http://127.0.0.1:8086/api/v2/...`)** as its primary command and dataref transport, alongside native UDP (`127.0.0.1:49000`) as a secondary transport.
  - Only poll WebAPI datarefs needed for the currently active sub-panel (`datarefs_for_panel(active_panel)` in `src/xplane/state.rs`) to keep HTTP polling fast and lightweight.
- **Multi-Aircraft Command Compatibility & Optimistic State**:
  - In [`src/xplane/commands.rs`](src/xplane/commands.rs), emit Laminar A330 custom commands (`laminar/A333/...`) alongside standard X-Plane (`sim/...`) and ToLiss (`AirbusFBW/...`, `toliss_airbus/...`) commands.
  - Always update `A320FcuState` optimistically on button press so the Stream Deck UI updates immediately even if X-Plane is still animating a switch or is offline.
  - In [`src/xplane/state.rs`](src/xplane/state.rs) (`apply_dref_update`), use `if changed` edge-triggering so unchanged periodic dataref polls never overwrite an optimistic toggle on the Stream Deck.

---

## 4. Configuration, Testing & Deployment Workflow

- **User-Driven Configuration in `~/.config/qdeck/config.toml` Only**:
  - Do **not** store application configuration in `Cargo.toml` (`[package.metadata.qdeck]`) or repo-local `.toml` files.
  - Runtime configuration is driven purely by **`~/.config/qdeck/config.toml`** (hot-reloaded automatically on change, and bootstrapped on first run from [`src/config/defaults.rs`](src/config/defaults.rs) if missing).
  - When adding new default tiles or panels, update both [`src/config/defaults.rs`](src/config/defaults.rs) and the user's [`~/.config/qdeck/config.toml`](file:///Users/bradley/.config/qdeck/config.toml).
- **Build & Test Before Deploying**:
  - Build artifacts are written to `./build/` (configured in [`.cargo/config.toml`](.cargo/config.toml)).
  - Always run `cargo test` (unit + integration tests in [`tests/integration_test.rs`](tests/integration_test.rs)) and `cargo clippy --all-targets` before building the release binary (`cargo build --release`).
- **Rebuild & Restart Live Binary**:
  - Whenever code changes affect runtime behavior, rebuild the release binary (`cargo build --release`) and restart the running background `launchd` service (`killall qdeck`) so the physical Stream Deck immediately runs the updated binary.
- **Git & CI Discipline**:
  - Commit logical units of work with clear commit messages and push to `origin/main` (`git@github.com:bradleymackey/qdeck.git`).
  - GitHub Actions CI ([`.github/workflows/ci.yml`](.github/workflows/ci.yml)) verifies `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo build --release`, and `cargo test` on every push and pull request.
