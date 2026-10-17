# qdeck

A tactical, phosphor-green operations-center controller for the **Elgato Stream Deck MK.2** (`15` keys, `72×72` LCD tiles) written in Rust.

`qdeck` combines a real-time macOS system telemetry dashboard with a multi-panel **Airbus A320/A330 Flight Deck Controller** for **X-Plane 12**, rendered entirely in custom pixel-perfect bitmap typography and vector cockpit iconography.

---

## Features

- **Tactical Phosphor CRT Aesthetic**: Custom `5×7` and `3×5` bitmap fonts (`src/font.rs`) and crisp cockpit vector icons (`src/widgets/icons.rs`) rendered in phosphor green (`#00FF66`), caution/selected amber (`#FFB000`), and master-warning red (`#FF3333`) on a deep `#060A08` background.
- **System Telemetry Dashboard (`root` page)**:
  - Live CPU, Memory, Swap, GPU utilization, Disk capacity, Battery/Thermal state, Process count, Clock, Uptime, and Network RX/TX throughput with rolling sparklines.
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
       - `GEAR` with 3-triangle nose/main landing gear indicator and directional `↑` / `↓` arrows.
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
  - Automatically blanks the Stream Deck (`reset()` + `0%` brightness) on `SIGINT`/`SIGTERM`, panic, process exit, or whenever the macOS display is locked or asleep (`src/macos.rs`).
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
│   ├── deck.rs               # Stream Deck hardware loop, input handling, blanking guard
│   ├── font.rs               # 5x7 & 3x5 bitmap fonts, geometric primitives, color scaling
│   ├── macos.rs              # CoreGraphics screen-lock / display-sleep detection
│   ├── telemetry.rs          # macOS CPU, RAM, GPU, Network, Disk, Battery collectors
│   ├── config/
│   │   ├── mod.rs            # Config loader (~/.config/qdeck/config.toml) & tile lookup
│   │   ├── defaults.rs       # Initial default config template for ~/.config/qdeck/config.toml
│   │   ├── theme.rs          # Hex color parsing and ResolvedTheme palette
│   │   └── tile.rs           # TileConfig and mode/sub-panel matching
│   ├── widgets/
│   │   ├── mod.rs            # Tile dispatcher (system, navigation, Airbus controls)
│   │   ├── airbus.rs         # Airbus FCU, Korry switches, gear, flaps, speedbrakes
│   │   ├── icons.rs          # Cockpit vector icons (arrows, brake, fan, spoiler, notches)
│   │   ├── navigation.rs     # Mode selector and sub-panel switcher tiles
│   │   └── system.rs         # System telemetry tiles & sparklines
│   └── xplane/
│       ├── mod.rs            # XPlaneClient combining UDP socket + WebAPI polling
│       ├── state.rs          # A320FcuState, dataref tables, edge-triggered state updates
│       ├── commands.rs       # Multi-aircraft command dispatch (Laminar A333, ToLiss, sim/*)
│       └── webapi.rs         # X-Plane 12 Local WebAPI HTTP v2 client (port 8086)
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
