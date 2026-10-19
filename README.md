# qdeck

A tactical, phosphor-green operations-center controller for the **Elgato Stream Deck MK.2** (`15` keys, `72×72` LCD tiles) written in Rust.

`qdeck` combines a real-time macOS system telemetry dashboard with a multi-panel **Airbus A320/A330 Flight Deck Controller** for **X-Plane 12**, rendered entirely in custom pixel-perfect bitmap typography and vector cockpit iconography.

---

## Features

- **Tactical Phosphor CRT Aesthetic**: Custom `5×7` and `3×5` bitmap fonts (`src/font/`) and crisp cockpit vector icons (`src/widgets/icons.rs`) rendered in phosphor green (`#00FF66`), caution/selected amber (`#FFB000`), and master-warning red (`#FF3333`) on a deep `#060A08` background.
- **System Telemetry & AI Quota Dashboard (`dashboard` mode)**:
  - Live CPU per-core bar graph and GPU core-grid utilization tiles (`0,3` and `0,4`).
  - **Claude Usage Limits (`1,3` / Tile `09`)**: Displays both 5-hour Session (`SES`) and 7-day Weekly (`WK`) utilization percentages and 15-segment progress bars via Anthropic's OAuth usage API.
  - **Google Gemini Usage Limits (`1,4` / Tile `10`)**: Cycles across 3 pages on click — `GEMINI` (`1/3`), `ARGON` (`2/3`), and `CLD/GPT` (`3/3`) — with each page showing both 5-hour Session (`SES`) and 7-day Weekly (`WK`) utilization percentages and 15-segment progress bars via the local Antigravity language server (`RetrieveUserQuotaSummary`).
  - **Airport Ops (`2,1`, London City `EGLC` by default)**: Cycles across 3 pages on click — runway in use (`1/3`: ident, direction arrow, `EASTERLY`/`WESTERLY` operations, arrival/departure counts, METAR wind), expected arrivals (`2/3`: callsign, origin, ETA in minutes) and departures (`3/3`: `GND` for aircraft taxiing or at the stand, otherwise minutes since take-off). Set `airport = "<ICAO>"` on the tile for any other airport. Built only from open data:
    - Live traffic from the [adsb.fi open data API](https://github.com/adsbfi/opendata) (adsb.lol as fallback), polled every 30 s only while the tile is on screen.
    - The runway in use comes from aircraft seen landing or taking off in the last 30 minutes (`ADS-B`); with no recent traffic it falls back to the runway end with the most headwind in the [aviationweather.gov](https://aviationweather.gov/data/api/) METAR (`WIND`, shown in amber).
    - Runway geometry ([OurAirports](https://ourairports.com/data/), public domain) and callsign routes ([VRS standing data](https://github.com/vradarserver/standing-data), CC0) for every airport are bundled into the binary from [`data/airport/`](data/airport/), so they never fail at runtime. Refresh them with `./scripts/update-airport-data.sh` every few weeks and commit the result.
- **4-Panel Airbus Cockpit Integration for X-Plane 12 (`xplane` page)**:
  - **Key `#1` (`BACK`)**: Returns to the `root` system telemetry page.
  - **Key `#2` (`PNL <ACTIVE> / NEXT: <NEXT>`)**: Cycles through 4 Airbus sub-panels across Keys `3–15`:
    1. **`FCU` (Glare-Shield & Autopilot)**:
       - Master Warning (`MASTER WARN`) & Master Caution (`MASTER CAUT`) push-to-cancel annunciators.
       - Digital LCD windows for `SPD`, `HDG`/`TRK`, `ALT`, `FPA` ("Bird" mode), and `V/S` (`0000` level-off push), displaying Airbus Managed (`---` + green dot) vs. Selected (`SEL` amber digits) states.
       - Classic 3-bar Korry switches for `AP 1`, `AP 2`, `A/THR`, `FD 1`, `LOC`, `EXPED`, and `APPR`.
    2. **`APU` (Overhead Electrical, APU & Engine Start Prep)**:
       - `BAT 1`, `BAT 2`, `EXT PWR` (`AVAIL` / `ON`), `APU MST`, `APU STR` (live `N xx%` spool-up readout and green `AVAIL`), `APU BLD`, `APU GEN`, `FUEL PMP` (all tank pumps), `ENG MOD` (`NORM` / `IGN/STR` / `CRANK` cycle), `PACK 1/2`, `BEACON`, `ECAM APU`, and `ECAM ENG`.
       - *Note*: Engine 1 & 2 master switches are intentionally omitted because they are bound to dedicated physical throttle-quadrant hardware.
    3. **`FLT` (Landing Gear, Brakes, Flaps & Flight Control Surfaces)**:
       - `GEAR` with 3-triangle nose/main landing gear indicator (`DOWN` green / `UNLK` red in transit / `UP` dim) and directional `↑` / `↓` arrows.
       - `FLAPS ↑` and `FLAPS ↓` with prominent directional arrows and a 4-notch (`0`, `1`, `2`, `3`, `FULL`) visual indicator bar.
       - `SPDBRK ↓` and `SPDBRK ↑` with a wing spoiler cross-section icon, live extension percentage, and directional arrows.
       - `PARK BRK` with a brake rotor/caliper icon (`((O))`), `BRK FAN` with a 4-blade impeller icon, and `A/BRK LO` / `MD` / `MX` with a 3-bar deceleration meter.
       - `T.O CFG` test pushbutton and `ECAM F/C` / `ECAM WHL` page selectors.
    4. **`SYS` (Exterior Lights, Anti-Ice, ADIRS, Baro & Transponder)**:
       - `STROBE` (`AUTO`/`ON`), `LAND LT` (`RET`/`ON`), 3-position `NOSE LT` (`OFF` → `TAXI` → `T.O`), and `NAV LT`.
       - `WING A/I`, `ENG1 A/I`, `ENG2 A/I`, and `PRB HEAT`.
       - `ADIRS` instant align, `SEATBLT` signs, `BARO STD` (`QNH` ↔ `STD`), `TCAS` (`STBY` ↔ `TA/RA`), and `ECAM ALL`.
- **Dual X-Plane 12 Transport (`src/xplane/`)**:
  - **Primary — X-Plane 12 Local WebAPI (`127.0.0.1:8086`)**: Uses REST v2 (`/api/v2/datarefs` and `/api/v2/command/.../activate`) so commands and telemetry work seamlessly even if X-Plane's UDP RakNet port (`49010`) fails to initialize.
  - **Secondary — Native X-Plane UDP (`127.0.0.1:49000`)**: Emits standard `CMND\0` and subscribes via `RREF\0` in parallel.
- **Hardware Power & Screen Lock Management**:
  - Runs a lightweight **1Hz (`1000ms`)** periodic refresh loop with instant dirty-key redraws on button press and a **120ms** tactile border flash.
  - Automatically blanks the Stream Deck (`reset()` + `0%` brightness) on `SIGINT`/`SIGTERM`, panic, process exit, or whenever the macOS display is locked or asleep (`src/macos/`).
- **Live Config Hot-Reloading**:
  - Watches `~/.config/qdeck/config.toml` and reloads layout or theme changes automatically without restarting.

---

## Project Structure

```text
qdeck/
├── .cargo/config.toml        # Build target directory configuration (build/)
├── .github/workflows/ci.yml  # GitHub Actions CI (fmt, clippy, build, test)
├── Cargo.toml                # Package manifest (lib + bin crate)
├── AGENTS.md                 # Project rules and conventions for AI coding agents
├── src/
│   ├── lib.rs                # Library crate root exporting modules
│   ├── main.rs               # Binary entrypoint, CLI flags, signal handling
│   ├── config/
│   │   ├── mod.rs            # Config loader (~/.config/qdeck/config.toml) & tile lookup
│   │   ├── defaults.rs       # Initial default config template for ~/.config/qdeck/config.toml
│   │   ├── theme.rs          # Hex color parsing and ResolvedTheme palette
│   │   └── tile.rs           # TileConfig and mode/sub-panel matching
│   ├── deck/
│   │   ├── mod.rs            # Stream Deck session loop & active telemetry refresh
│   │   ├── device.rs         # USB discovery, hardware geometry, and sleep/blanking
│   │   ├── input.rs          # Button edge detection, ClickAction execution, tap highlights
│   │   └── render.rs         # Key and full-layout rendering to Stream Deck LCDs
│   ├── font/
│   │   ├── mod.rs            # 5x7 & 3x5 bitmap text rendering and unit tests
│   │   ├── glyphs.rs         # 5x7 & 3x5 bitmap glyph lookup tables
│   │   └── primitives.rs     # Color scaling, lines, rectangles, and triangle primitives
│   ├── macos/
│   │   ├── mod.rs            # macOS platform module re-exports
│   │   ├── lock.rs           # CoreGraphics / IOKit screen-lock & display-sleep detection
│   │   ├── service.rs        # launchd LaunchAgent plist generation & installer
│   │   └── signals.rs        # POSIX SIGINT/SIGTERM/SIGHUP shutdown flag handling
│   ├── telemetry/
│   │   ├── mod.rs            # macOS CPU, RAM, GPU collectors & TelemetrySnapshot builder
│   │   ├── ai_limits.rs      # Background Claude & Google Gemini session/weekly quota poller
│   │   └── airport/
│   │       ├── mod.rs        # Background airport poller (ADS-B + METAR) and AirportStatus
│   │       ├── reference.rs  # Bundled runway & route tables (data/airport/) and parsers
│   │       ├── runway.rs     # Runway-in-use from traffic votes, METAR wind fallback
│   │       ├── traffic.rs    # ADS-B parsing and arrival/departure/runway classification
│   │       └── geo.rs        # Distance, bearing, and centreline maths
│   ├── widgets/
│   │   ├── mod.rs            # Tile dispatcher (system, AI limits, navigation, Airbus controls)
│   │   ├── ai_limits.rs      # Dual-bar Session (SES) & Weekly (WK) Claude/Gemini tiles
│   │   ├── airport.rs        # 3-page airport tile (runway in use, arrivals, departures)
│   │   ├── icons.rs          # Cockpit vector icons (arrows, brake, fan, spoiler, notches)
│   │   ├── navigation.rs     # Mode selector and sub-panel switcher tiles
│   │   ├── system.rs         # System telemetry tiles (CPU, GPU, Memory, Ops Status)
│   │   └── airbus/
│   │       ├── mod.rs        # Airbus tile dispatcher
│   │       ├── fcu.rs        # Glare-shield Master Warn/Caut & FCU digital LCD windows
│   │       ├── apu.rs        # APU Start (AVAIL/N1%) & Engine Mode Selector tiles
│   │       ├── surfaces.rs   # 3-strut Landing Gear, Flaps, and Speedbrake tiles
│   │       ├── brakes.rs     # Parking Brake, Brake Fan, and Autobrake tiles
│   │       └── korry.rs      # 2-window & 3-bar Korry switches and 3-pos Nose Light
│   └── xplane/
│       ├── mod.rs            # XPlaneClient combining UDP socket + WebAPI polling
│       ├── webapi.rs         # X-Plane 12 Local WebAPI HTTP v2 client (port 8086)
│       ├── state/
│       │   ├── mod.rs        # A320FcuState & edge-triggered dataref state updates
│       │   └── datarefs.rs   # Dataref subscription tables and per-panel WebAPI filters
│       └── commands/
│           ├── mod.rs        # Control dispatcher & unit tests across Panels 1–4
│           ├── fcu.rs        # Panel 1 (FCU & Glare-Shield) command handlers
│           ├── apu.rs        # Panel 2 (Electrical, APU & Engine Prep) command handlers
│           ├── flt.rs        # Panel 3 (Gear, Brakes, Flaps & Spoilers) command handlers
│           └── sys.rs        # Panel 4 (Lights, Anti-Ice, ADIRS, Baro & TCAS) command handlers
├── data/airport/             # Bundled worldwide runways.csv & routes.csv for the airport tile
├── scripts/
│   └── update-airport-data.sh # Refreshes data/airport/ from OurAirports and VRS standing data
└── tests/
    └── integration_test.rs   # End-to-end config, panel cycling, rendering & command tests
```

---

## Building & Running

### Prerequisites
- Rust (edition 2024 / stable toolchain)
- macOS with an Elgato Stream Deck MK.2 connected via USB

### Build & Test
```bash
# Run unit and integration tests
cargo test

# Build optimized release binary (outputs to ./build/release/qdeck)
cargo build --release
```

### Configuration (`~/.config/qdeck/config.toml`)
`qdeck` is configured purely via **`~/.config/qdeck/config.toml`** (created automatically from [`src/config/defaults.rs`](src/config/defaults.rs) on first run if missing, and hot-reloaded whenever saved).

```bash
# Run normally (loads ~/.config/qdeck/config.toml)
./build/release/qdeck

# Install or uninstall the macOS launchd background service
./build/release/qdeck --install-service
./build/release/qdeck --uninstall-service
```

### Background Service (`launchd`)
On macOS, `qdeck` runs as a persistent user agent via `~/Library/LaunchAgents/local.qdeck.plist` pointing to `/Users/bradley/Developer/Repos/qdeck/build/release/qdeck` with logs at `~/Library/Logs/qdeck.log`. After rebuilding the release binary, restart the service with:
```bash
killall qdeck
```
`launchd` will automatically respawn the updated binary within 2 seconds.
