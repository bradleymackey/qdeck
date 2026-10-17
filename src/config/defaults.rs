pub const DEFAULT_CONFIG_TOML: &str = r##"# =============================================================================
# QDeck // Tactical Ops-Center Configuration (~/.config/qdeck/config.toml)
# Changes to this file are hot-reloaded automatically while qdeck is running.
# =============================================================================

brightness = 85
refresh_ms = 1000
vid = 0x0fd9
# pid = 0x0080          # Stream Deck MK.2 (auto-detected if omitted)
background_mode = "grid" # "grid" (tactical standby reticle) or "blank" (pure black)
default_mode = "dashboard"
modes = ["dashboard", "xplane"]
xplane_addr = "127.0.0.1:49000"

[theme]
background = "#000000"
grid_dim = "#09260e"
bar_track = "#061809"
phosphor_green = "#00ff41"
phosphor_dim = "#008f24"
amber = "#ffb000"
red = "#ff2a2a"
warn_threshold = 60.0
crit_threshold = 85.0

# -----------------------------------------------------------------------------
# Global Mode Selector (Top-Left across all modes)
# -----------------------------------------------------------------------------
[[tiles]]
type = "mode"
position = "top-left"
mode = "*"

# -----------------------------------------------------------------------------
# Mode: "dashboard" (System Telemetry)
# -----------------------------------------------------------------------------
[[tiles]]
type = "gpu"
position = "0,3"
mode = "dashboard"
label = "GPU"
app = "Activity Monitor"
warn_threshold = 60.0
crit_threshold = 85.0

[[tiles]]
type = "cpu"
position = "top-right"
mode = "dashboard"
label = "CPU"
app = "Activity Monitor"
warn_threshold = 60.0
crit_threshold = 85.0

[[tiles]]
type = "claude"
position = "1,3"
mode = "dashboard"
label = "CLAUDE"
warn_threshold = 60.0
crit_threshold = 85.0

[[tiles]]
type = "gemini"
position = "1,4"
mode = "dashboard"
label = "GEMINI"
warn_threshold = 60.0
crit_threshold = 85.0

# -----------------------------------------------------------------------------
# Mode: "xplane" — Secondary Panel Selector (Key 0,1)
# Toggles between:
#   1. "fcu" (Autopilot / FCU)
#   2. "apu" (Overhead Electrical, APU & Engine Start Prep)
#   3. "flt" (Gear, Autobrake, Flaps, Spoilers & Flight Controls)
#   4. "sys" (Lights, Anti-Ice, ADIRS, Seatbelts, Baro STD & TCAS)
# -----------------------------------------------------------------------------
[[tiles]]
type = "panel"
position = "0,1"
mode = "xplane"
label = "PANEL"

# =============================================================================
# Mode: "xplane" // PANEL 1: "fcu" (Airbus A320/A330 FCU & Glare-Shield)
# =============================================================================
# Row 0: MODE (0,0) | PANEL (0,1) | AP 1 (0,2) | AP 2 (0,3) | A/THR (0,4)
[[tiles]]
type = "a320_fcu"
control = "ap1"
position = "0,2"
mode = "xplane"
panel = "fcu"
label = "AP 1"

[[tiles]]
type = "a320_fcu"
control = "ap2"
position = "0,3"
mode = "xplane"
panel = "fcu"
label = "AP 2"

[[tiles]]
type = "a320_fcu"
control = "athr"
position = "0,4"
mode = "xplane"
panel = "fcu"
label = "A/THR"

# Row 1: FD 1 (1,0) | SPD (1,1) | HDG (1,2) | ALT (1,3) | APPR (1,4)
[[tiles]]
type = "a320_fcu"
control = "fd"
position = "1,0"
mode = "xplane"
panel = "fcu"
label = "FD 1"

[[tiles]]
type = "a320_fcu"
control = "spd"
position = "1,1"
mode = "xplane"
panel = "fcu"
label = "SPD"

[[tiles]]
type = "a320_fcu"
control = "hdg"
position = "1,2"
mode = "xplane"
panel = "fcu"
label = "HDG"

[[tiles]]
type = "a320_fcu"
control = "alt"
position = "1,3"
mode = "xplane"
panel = "fcu"
label = "ALT"

[[tiles]]
type = "a320_fcu"
control = "appr"
position = "1,4"
mode = "xplane"
panel = "fcu"
label = "APPR"

# Row 2: M WARN (2,0) | M CAUT (2,1) | LOC (2,2) | FPA (2,3) | V/S (2,4)
[[tiles]]
type = "a320_fcu"
control = "master_warn"
position = "2,0"
mode = "xplane"
panel = "fcu"

[[tiles]]
type = "a320_fcu"
control = "master_caut"
position = "2,1"
mode = "xplane"
panel = "fcu"

[[tiles]]
type = "a320_fcu"
control = "loc"
position = "2,2"
mode = "xplane"
panel = "fcu"
label = "LOC"

[[tiles]]
type = "a320_fcu"
control = "trk"
position = "2,3"
mode = "xplane"
panel = "fcu"
label = "FPA"

[[tiles]]
type = "a320_fcu"
control = "vs"
position = "2,4"
mode = "xplane"
panel = "fcu"
label = "V/S"

# =============================================================================
# Mode: "xplane" // PANEL 2: "apu" (Airbus Overhead Electrical, APU & Eng Prep)
# =============================================================================
# Row 0: MODE (0,0) | PANEL (0,1) | BAT 1 (0,2) | BAT 2 (0,3) | EXT PWR (0,4)
[[tiles]]
type = "a320_fcu"
control = "bat1"
position = "0,2"
mode = "xplane"
panel = "apu"
label = "BAT 1"

[[tiles]]
type = "a320_fcu"
control = "bat2"
position = "0,3"
mode = "xplane"
panel = "apu"
label = "BAT 2"

[[tiles]]
type = "a320_fcu"
control = "ext_pwr"
position = "0,4"
mode = "xplane"
panel = "apu"
label = "EXT PWR"

# Row 1: APU MST (1,0) | APU STR (1,1) | APU BLD (1,2) | APU GEN (1,3) | FUEL PMP (1,4)
[[tiles]]
type = "a320_fcu"
control = "apu_master"
position = "1,0"
mode = "xplane"
panel = "apu"
label = "APU MST"

[[tiles]]
type = "a320_fcu"
control = "apu_start"
position = "1,1"
mode = "xplane"
panel = "apu"
label = "APU STR"

[[tiles]]
type = "a320_fcu"
control = "apu_bleed"
position = "1,2"
mode = "xplane"
panel = "apu"
label = "APU BLD"

[[tiles]]
type = "a320_fcu"
control = "apu_gen"
position = "1,3"
mode = "xplane"
panel = "apu"
label = "APU GEN"

[[tiles]]
type = "a320_fcu"
control = "fuel_pumps"
position = "1,4"
mode = "xplane"
panel = "apu"
label = "FUEL PMP"

# Row 2: ENG MOD (2,0) | PACK 1/2 (2,1) | BEACON (2,2) | ECAM APU (2,3) | ECAM ENG (2,4)
[[tiles]]
type = "a320_fcu"
control = "eng_mode"
position = "2,0"
mode = "xplane"
panel = "apu"
label = "ENG MOD"

[[tiles]]
type = "a320_fcu"
control = "packs"
position = "2,1"
mode = "xplane"
panel = "apu"
label = "PACK 1/2"

[[tiles]]
type = "a320_fcu"
control = "beacon"
position = "2,2"
mode = "xplane"
panel = "apu"
label = "BEACON"

[[tiles]]
type = "a320_fcu"
control = "ecam_apu"
position = "2,3"
mode = "xplane"
panel = "apu"
label = "ECAM APU"

[[tiles]]
type = "a320_fcu"
control = "ecam_eng"
position = "2,4"
mode = "xplane"
panel = "apu"
label = "ECAM ENG"

# =============================================================================
# Mode: "xplane" // PANEL 3: "flt" (Gear, Autobrake, Flaps, Spoilers & F/CTL)
# =============================================================================
# Row 0: MODE (0,0) | PANEL (0,1) | L/G (0,2) | PARK BRK (0,3) | BRK FAN (0,4)
[[tiles]]
type = "a320_fcu"
control = "gear"
position = "0,2"
mode = "xplane"
panel = "flt"
label = "LDG GEAR"

[[tiles]]
type = "a320_fcu"
control = "park_brake"
position = "0,3"
mode = "xplane"
panel = "flt"
label = "PARK BRK"

[[tiles]]
type = "a320_fcu"
control = "brake_fan"
position = "0,4"
mode = "xplane"
panel = "flt"
label = "BRK FAN"

# Row 1: A/BRK LO (1,0) | A/BRK MED (1,1) | A/BRK MAX (1,2) | FLAPS UP (1,3) | FLAPS DN (1,4)
[[tiles]]
type = "a320_fcu"
control = "abrk_lo"
position = "1,0"
mode = "xplane"
panel = "flt"
label = "A/BRK LO"

[[tiles]]
type = "a320_fcu"
control = "abrk_med"
position = "1,1"
mode = "xplane"
panel = "flt"
label = "A/BRK MD"

[[tiles]]
type = "a320_fcu"
control = "abrk_max"
position = "1,2"
mode = "xplane"
panel = "flt"
label = "A/BRK MX"

[[tiles]]
type = "a320_fcu"
control = "flaps_up"
position = "1,3"
mode = "xplane"
panel = "flt"
label = "FLAPS UP"

[[tiles]]
type = "a320_fcu"
control = "flaps_dn"
position = "1,4"
mode = "xplane"
panel = "flt"
label = "FLAPS DN"

# Row 2: SPD BRK- (2,0) | SPD BRK+ (2,1) | T.O CFG (2,2) | ECAM FCTL (2,3) | ECAM WHL (2,4)
[[tiles]]
type = "a320_fcu"
control = "spdbrk_dn"
position = "2,0"
mode = "xplane"
panel = "flt"
label = "SPDBRK -"

[[tiles]]
type = "a320_fcu"
control = "spdbrk_up"
position = "2,1"
mode = "xplane"
panel = "flt"
label = "SPDBRK +"

[[tiles]]
type = "a320_fcu"
control = "to_config"
position = "2,2"
mode = "xplane"
panel = "flt"
label = "T.O CFG"

[[tiles]]
type = "a320_fcu"
control = "ecam_fctl"
position = "2,3"
mode = "xplane"
panel = "flt"
label = "ECAM F/C"

[[tiles]]
type = "a320_fcu"
control = "ecam_whl"
position = "2,4"
mode = "xplane"
panel = "flt"
label = "ECAM WHL"

# =============================================================================
# Mode: "xplane" // PANEL 4: "sys" (Lights, Anti-Ice, ADIRS, Baro STD & TCAS)
# =============================================================================
# Row 0: MODE (0,0) | PANEL (0,1) | STROBE (0,2) | LAND LT (0,3) | NOSE LT (0,4)
[[tiles]]
type = "a320_fcu"
control = "strobe"
position = "0,2"
mode = "xplane"
panel = "sys"
label = "STROBE"

[[tiles]]
type = "a320_fcu"
control = "land_lights"
position = "0,3"
mode = "xplane"
panel = "sys"
label = "LAND LT"

[[tiles]]
type = "a320_fcu"
control = "nose_light"
position = "0,4"
mode = "xplane"
panel = "sys"
label = "NOSE LT"

# Row 1: WING A/I (1,0) | ENG 1 A/I (1,1) | ENG 2 A/I (1,2) | PRB HEAT (1,3) | NAV LT (1,4)
[[tiles]]
type = "a320_fcu"
control = "wing_ai"
position = "1,0"
mode = "xplane"
panel = "sys"
label = "WING A/I"

[[tiles]]
type = "a320_fcu"
control = "eng1_ai"
position = "1,1"
mode = "xplane"
panel = "sys"
label = "ENG1 A/I"

[[tiles]]
type = "a320_fcu"
control = "eng2_ai"
position = "1,2"
mode = "xplane"
panel = "sys"
label = "ENG2 A/I"

[[tiles]]
type = "a320_fcu"
control = "probe_heat"
position = "1,3"
mode = "xplane"
panel = "sys"
label = "PRB HEAT"

[[tiles]]
type = "a320_fcu"
control = "nav_light"
position = "1,4"
mode = "xplane"
panel = "sys"
label = "NAV LT"

# Row 2: ADIRS (2,0) | SEATBLT (2,1) | BARO STD (2,2) | TCAS TARA (2,3) | ECAM ALL (2,4)
[[tiles]]
type = "a320_fcu"
control = "adirs"
position = "2,0"
mode = "xplane"
panel = "sys"
label = "ADIRS"

[[tiles]]
type = "a320_fcu"
control = "seatbelt"
position = "2,1"
mode = "xplane"
panel = "sys"
label = "SEATBLT"

[[tiles]]
type = "a320_fcu"
control = "baro_std"
position = "2,2"
mode = "xplane"
panel = "sys"
label = "BARO STD"

[[tiles]]
type = "a320_fcu"
control = "tcas"
position = "2,3"
mode = "xplane"
panel = "sys"
label = "TCAS"

[[tiles]]
type = "a320_fcu"
control = "ecam_all"
position = "2,4"
mode = "xplane"
panel = "sys"
label = "ECAM ALL"
"##;
