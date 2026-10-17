use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream, UdpSocket};
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct A320FcuState {
    // Panel 1: FCU / Autopilot
    pub fd_on: bool,
    pub ap1_on: bool,
    pub ap2_on: bool,
    pub athr_on: bool,
    pub loc_on: bool,
    pub exped_on: bool,
    pub appr_on: bool,
    pub trk_fpa_mode: bool,
    pub master_warn: bool,
    pub master_caut: bool,
    pub hdg_managed: bool,
    pub alt_managed: bool,
    pub spd_managed: bool,
    pub hdg_deg: f32,
    pub alt_ft: f32,
    pub spd_kts: f32,
    pub vs_fpm: f32,

    // Panel 2: Overhead Electrical, APU & Engine Start Prep
    pub bat1_on: bool,
    pub bat2_on: bool,
    pub ext_pwr_on: bool,
    pub apu_master_on: bool,
    pub apu_start_on: bool,
    pub apu_avail: bool,
    pub apu_n1_pct: f32,
    pub apu_bleed_on: bool,
    pub apu_gen_on: bool,
    pub fuel_pumps_on: bool,
    pub packs_on: bool,
    pub beacon_on: bool,
    /// Engine Mode Selector: -1 = CRANK, 0 = NORM, 1 = IGN/START
    pub eng_mode: i8,
    pub ecam_page: &'static str,

    pub last_rx: Option<Instant>,
}

impl Default for A320FcuState {
    fn default() -> Self {
        Self {
            fd_on: true,
            ap1_on: false,
            ap2_on: false,
            athr_on: false,
            loc_on: false,
            exped_on: false,
            appr_on: false,
            trk_fpa_mode: false,
            master_warn: false,
            master_caut: false,
            hdg_managed: true,
            alt_managed: true,
            spd_managed: true,
            hdg_deg: 254.0,
            alt_ft: 15000.0,
            spd_kts: 250.0,
            vs_fpm: 0.0,
            bat1_on: false,
            bat2_on: false,
            ext_pwr_on: false,
            apu_master_on: false,
            apu_start_on: false,
            apu_avail: false,
            apu_n1_pct: 0.0,
            apu_bleed_on: false,
            apu_gen_on: true,
            fuel_pumps_on: false,
            packs_on: true,
            beacon_on: false,
            eng_mode: 0,
            ecam_page: "APU",
            last_rx: None,
        }
    }
}

const DREF_SIM_FD: i32 = 1;
const DREF_SIM_AP1: i32 = 2;
const DREF_SIM_AP2: i32 = 3;
const DREF_SIM_ATHR: i32 = 4;
const DREF_SIM_LOC: i32 = 5;
const DREF_SIM_APPR: i32 = 6;
const DREF_SIM_HDG: i32 = 7;
const DREF_SIM_ALT: i32 = 8;
const DREF_SIM_SPD: i32 = 9;
const DREF_SIM_VS: i32 = 10;
const DREF_SIM_WARN: i32 = 11;
const DREF_SIM_CAUT: i32 = 12;
const DREF_SIM_TRK: i32 = 13;
const DREF_TOLISS_AP1: i32 = 20;
const DREF_TOLISS_AP2: i32 = 21;
const DREF_TOLISS_ATHR: i32 = 22;
const DREF_TOLISS_LOC: i32 = 23;
const DREF_TOLISS_APPR: i32 = 24;
const DREF_TOLISS_EXPED: i32 = 25;
const DREF_A333_HDG_OPEN: i32 = 30;

// Panel 2 Dataref IDs
const DREF_APU_N1: i32 = 40;
const DREF_APU_MASTER: i32 = 41;
const DREF_APU_START: i32 = 42;
const DREF_APU_AVAIL: i32 = 43;
const DREF_APU_BLEED: i32 = 44;
const DREF_BAT1: i32 = 45;
const DREF_BAT2: i32 = 46;
const DREF_FUEL_PUMP: i32 = 47;
const DREF_ENG_MODE: i32 = 48;
const DREF_BEACON: i32 = 49;

const WEB_API_ADDR: ([u8; 4], u16) = ([127, 0, 0, 1], 8086);

fn extract_json_u64_id(json: &str) -> Option<u64> {
    let idx = json.find("\"id\":")?;
    let after = &json[idx + 5..];
    let digits: String = after
        .trim_start()
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    digits.parse::<u64>().ok()
}

fn extract_json_f32_data(json: &str) -> Option<f32> {
    let idx = json.find("\"data\":")?;
    let after = &json[idx + 7..];
    let num_str: String = after
        .trim_start()
        .chars()
        .take_while(|c| {
            c.is_ascii_digit() || *c == '.' || *c == '-' || *c == '+' || *c == 'e' || *c == 'E'
        })
        .collect();
    num_str.parse::<f32>().ok()
}

pub struct XPlaneClient {
    socket: Option<UdpSocket>,
    target_addr: String,
    subscribed: bool,
    last_sub_attempt: Instant,
    cmd_id_cache: HashMap<String, Option<u64>>,
    dref_id_cache: HashMap<String, Option<u64>>,
    prev_dref_vals: HashMap<i32, f32>,
    web_api_backoff_until: Option<Instant>,
    pub state: A320FcuState,
}

impl XPlaneClient {
    pub fn new(target_addr: &str) -> Self {
        let socket = UdpSocket::bind("0.0.0.0:0").ok();
        if let Some(ref s) = socket {
            let _ = s.set_nonblocking(true);
        }
        Self {
            socket,
            target_addr: target_addr.to_string(),
            subscribed: false,
            last_sub_attempt: Instant::now() - Duration::from_secs(60),
            cmd_id_cache: HashMap::new(),
            dref_id_cache: HashMap::new(),
            prev_dref_vals: HashMap::new(),
            web_api_backoff_until: None,
            state: A320FcuState::default(),
        }
    }

    pub fn set_target_addr(&mut self, addr: &str) {
        if self.target_addr != addr {
            self.target_addr = addr.to_string();
            self.subscribed = false;
        }
    }

    /// Performs a fast loopback HTTP request to X-Plane 12's Local WebAPI (`127.0.0.1:8086`).
    fn web_api_request(&mut self, method: &str, path: &str, body: Option<&str>) -> Option<String> {
        if let Some(until) = self.web_api_backoff_until {
            if Instant::now() < until {
                return None;
            }
        }

        let addr = SocketAddr::from(WEB_API_ADDR);
        let Ok(mut stream) = TcpStream::connect_timeout(&addr, Duration::from_millis(35)) else {
            self.web_api_backoff_until = Some(Instant::now() + Duration::from_secs(3));
            return None;
        };
        let _ = stream.set_read_timeout(Some(Duration::from_millis(60)));
        let _ = stream.set_write_timeout(Some(Duration::from_millis(35)));

        let req = if let Some(b) = body {
            format!(
                "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:8086\r\nAccept: application/json\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{b}",
                b.len()
            )
        } else {
            format!(
                "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:8086\r\nAccept: application/json\r\nConnection: close\r\n\r\n"
            )
        };

        if stream.write_all(req.as_bytes()).is_err() {
            return None;
        }

        let mut resp = String::new();
        if stream.read_to_string(&mut resp).is_err() {
            return None;
        }

        let (headers, json_body) = resp.split_once("\r\n\r\n")?;
        if !headers.starts_with("HTTP/1.1 200") {
            return None;
        }
        Some(json_body.to_string())
    }

    fn resolve_web_cmd_id(&mut self, cmd_path: &str) -> Option<u64> {
        if let Some(&cached) = self.cmd_id_cache.get(cmd_path) {
            return cached;
        }
        let path = format!("/api/v2/commands?filter[name]={cmd_path}");
        let body = self.web_api_request("GET", &path, None)?;
        let id = extract_json_u64_id(&body);
        self.cmd_id_cache.insert(cmd_path.to_string(), id);
        id
    }

    fn resolve_web_dref_id(&mut self, dref_path: &str) -> Option<u64> {
        if let Some(&cached) = self.dref_id_cache.get(dref_path) {
            return cached;
        }
        let path = format!("/api/v2/datarefs?filter[name]={dref_path}");
        let body = self.web_api_request("GET", &path, None)?;
        let id = extract_json_u64_id(&body);
        self.dref_id_cache.insert(dref_path.to_string(), id);
        id
    }

    /// Sends a command to X-Plane via BOTH X-Plane 12's Local WebAPI (`127.0.0.1:8086`)
    /// AND legacy UDP (`127.0.0.1:49000` `CMND\0`).
    pub fn send_command(&mut self, cmd_path: &str) {
        if let Some(ref sock) = self.socket {
            let mut pkt = Vec::with_capacity(6 + cmd_path.len());
            pkt.extend_from_slice(b"CMND\0");
            pkt.extend_from_slice(cmd_path.as_bytes());
            pkt.push(0);
            let _ = sock.send_to(&pkt, &self.target_addr);
        }

        if let Some(id) = self.resolve_web_cmd_id(cmd_path) {
            let path = format!("/api/v2/command/{id}/activate");
            if self
                .web_api_request("POST", &path, Some("{\"duration\":0}"))
                .is_some()
            {
                self.state.last_rx = Some(Instant::now());
            }
        }
    }

    /// Requests X-Plane to stream a dataref at `freq_hz` Hz via `RREF\0`.
    fn send_rref(&self, id: i32, freq_hz: i32, dref_path: &str) {
        let Some(ref sock) = self.socket else {
            return;
        };
        let mut pkt = vec![0u8; 413];
        pkt[0..5].copy_from_slice(b"RREF\0");
        pkt[5..9].copy_from_slice(&freq_hz.to_le_bytes());
        pkt[9..13].copy_from_slice(&id.to_le_bytes());
        let bytes = dref_path.as_bytes();
        let copy_len = bytes.len().min(399);
        pkt[13..(13 + copy_len)].copy_from_slice(&bytes[..copy_len]);
        let _ = sock.send_to(&pkt, &self.target_addr);
    }

    /// Subscribes to standard X-Plane + ToLiss/AirbusFBW A320 FCU & APU datarefs at 2 Hz.
    pub fn ensure_subscribed(&mut self) {
        let is_live = self
            .state
            .last_rx
            .is_some_and(|t| t.elapsed() < Duration::from_secs(5));
        if self.subscribed && is_live {
            return;
        }
        if self.last_sub_attempt.elapsed() < Duration::from_secs(5) {
            return;
        }
        self.last_sub_attempt = Instant::now();
        self.subscribed = true;

        let subs = [
            (DREF_SIM_FD, "sim/cockpit2/autopilot/flight_director_mode"),
            (DREF_SIM_AP1, "sim/cockpit2/autopilot/servos_on"),
            (DREF_SIM_AP2, "sim/cockpit2/autopilot/servos2_on"),
            (DREF_SIM_ATHR, "sim/cockpit2/autopilot/autothrottle_enabled"),
            (DREF_SIM_LOC, "sim/cockpit2/autopilot/nav_status"),
            (DREF_SIM_APPR, "sim/cockpit2/autopilot/approach_status"),
            (
                DREF_SIM_HDG,
                "sim/cockpit2/autopilot/heading_dial_deg_mag_pilot",
            ),
            (DREF_SIM_ALT, "sim/cockpit2/autopilot/altitude_dial_ft"),
            (DREF_SIM_SPD, "sim/cockpit2/autopilot/airspeed_dial_kts_mach"),
            (DREF_SIM_VS, "sim/cockpit2/autopilot/vvi_dial_fpm"),
            (DREF_SIM_WARN, "sim/cockpit2/annunciators/master_warning"),
            (DREF_SIM_CAUT, "sim/cockpit2/annunciators/master_caution"),
            (DREF_TOLISS_AP1, "AirbusFBW/AP1Engage"),
            (DREF_TOLISS_AP2, "AirbusFBW/AP2Engage"),
            (DREF_TOLISS_ATHR, "AirbusFBW/ATHRmode"),
            (DREF_TOLISS_LOC, "AirbusFBW/LOCilluminated"),
            (DREF_TOLISS_APPR, "AirbusFBW/APPRilluminated"),
            (DREF_TOLISS_EXPED, "AirbusFBW/EXPEDilluminated"),
            (DREF_APU_N1, "sim/cockpit2/electrical/APU_N1_percent"),
        ];

        for (id, path) in subs {
            self.send_rref(id, 2, path);
        }
    }

    /// Drains any incoming `RREF` UDP packets OR polls X-Plane 12's Local WebAPI (`127.0.0.1:8086`)
    /// for the active panel (`"fcu"` or `"apu"`) and updates `self.state`.
    pub fn poll_incoming(&mut self, active_panel: &str) {
        let mut got_udp = false;
        if let Some(ref sock) = self.socket {
            let mut buf = [0u8; 2048];
            while let Ok((len, _addr)) = sock.recv_from(&mut buf) {
                if len >= 13 && &buf[0..4] == b"RREF" {
                    got_udp = true;
                    self.state.last_rx = Some(Instant::now());
                    let mut offset = 5;
                    while offset + 8 <= len {
                        let id = i32::from_le_bytes([
                            buf[offset],
                            buf[offset + 1],
                            buf[offset + 2],
                            buf[offset + 3],
                        ]);
                        let val = f32::from_le_bytes([
                            buf[offset + 4],
                            buf[offset + 5],
                            buf[offset + 6],
                            buf[offset + 7],
                        ]);
                        Self::apply_dref_update(
                            &mut self.state,
                            &mut self.prev_dref_vals,
                            id,
                            val,
                        );
                        offset += 8;
                    }
                }
            }
        }

        if !got_udp {
            let fcu_drefs: &[(i32, &str)] = &[
                (DREF_SIM_FD, "sim/cockpit2/autopilot/flight_director_mode"),
                (DREF_SIM_AP1, "sim/cockpit2/autopilot/servos_on"),
                (DREF_SIM_AP2, "sim/cockpit2/autopilot/servos2_on"),
                (DREF_SIM_ATHR, "sim/cockpit2/autopilot/autothrottle_on"),
                (DREF_SIM_LOC, "sim/cockpit2/autopilot/nav_status"),
                (DREF_SIM_APPR, "sim/cockpit2/autopilot/approach_status"),
                (
                    DREF_SIM_HDG,
                    "sim/cockpit2/autopilot/heading_dial_deg_mag_pilot",
                ),
                (DREF_SIM_ALT, "sim/cockpit2/autopilot/altitude_dial_ft"),
                (DREF_SIM_SPD, "sim/cockpit2/autopilot/airspeed_dial_kts_mach"),
                (DREF_SIM_VS, "sim/cockpit2/autopilot/vvi_dial_fpm"),
                (DREF_SIM_TRK, "sim/cockpit2/autopilot/trk_fpa"),
                (DREF_SIM_WARN, "sim/cockpit2/annunciators/master_warning"),
                (DREF_SIM_CAUT, "sim/cockpit2/annunciators/master_caution"),
                (DREF_A333_HDG_OPEN, "laminar/A333/autopilot/hdg_window_open"),
            ];

            let apu_drefs: &[(i32, &str)] = &[
                (DREF_APU_N1, "sim/cockpit2/electrical/APU_N1_percent"),
                (DREF_APU_MASTER, "laminar/A333/annun/apu_master_on"),
                (DREF_APU_START, "laminar/A333/annun/apu_start_on"),
                (DREF_APU_AVAIL, "laminar/A333/annun/apu_avail"),
                (DREF_APU_BLEED, "laminar/A333/annun/apu_bleed_on"),
                (DREF_BAT1, "laminar/A333/buttons/batt1_ctct_on_off"),
                (DREF_BAT2, "laminar/A333/buttons/batt2_ctct_on_off"),
                (DREF_FUEL_PUMP, "laminar/A333/fuel/buttons/left1_pump_pos"),
                (
                    DREF_ENG_MODE,
                    "sim/cockpit2/engine/actuators/eng_mode_selector",
                ),
                (DREF_BEACON, "sim/cockpit2/switches/beacon_on"),
            ];

            let list = if active_panel.eq_ignore_ascii_case("apu") {
                apu_drefs
            } else {
                fcu_drefs
            };

            for &(key_id, dref_name) in list {
                let Some(web_id) = self.resolve_web_dref_id(dref_name) else {
                    if self.web_api_backoff_until.is_some_and(|u| Instant::now() < u) {
                        break;
                    }
                    continue;
                };
                let path = format!("/api/v2/datarefs/{web_id}/value");
                if let Some(body) = self.web_api_request("GET", &path, None) {
                    if let Some(val) = extract_json_f32_data(&body) {
                        self.state.last_rx = Some(Instant::now());
                        Self::apply_dref_update(
                            &mut self.state,
                            &mut self.prev_dref_vals,
                            key_id,
                            val,
                        );
                    }
                }
            }
        }
    }

    fn apply_dref_update(
        state: &mut A320FcuState,
        prev_vals: &mut HashMap<i32, f32>,
        id: i32,
        val: f32,
    ) {
        let changed = prev_vals
            .insert(id, val)
            .is_none_or(|old| (old - val).abs() > 0.001);

        match id {
            DREF_SIM_FD if changed => state.fd_on = val >= 1.0,
            DREF_SIM_AP1 | DREF_TOLISS_AP1 if changed => state.ap1_on = val > 0.5,
            DREF_SIM_AP2 | DREF_TOLISS_AP2 if changed => state.ap2_on = val > 0.5,
            DREF_SIM_ATHR | DREF_TOLISS_ATHR if changed => state.athr_on = val > 0.5,
            DREF_SIM_LOC | DREF_TOLISS_LOC if changed => state.loc_on = val >= 1.0,
            DREF_SIM_APPR | DREF_TOLISS_APPR if changed => state.appr_on = val >= 1.0,
            DREF_TOLISS_EXPED if changed => state.exped_on = val > 0.5,
            DREF_SIM_TRK if changed => state.trk_fpa_mode = val > 0.5,
            DREF_A333_HDG_OPEN if changed => state.hdg_managed = val < 0.5,
            DREF_SIM_HDG => state.hdg_deg = val.rem_euclid(360.0),
            DREF_SIM_ALT => state.alt_ft = val,
            DREF_SIM_SPD => state.spd_kts = val,
            DREF_SIM_VS => state.vs_fpm = val,
            DREF_SIM_WARN if changed => state.master_warn = val > 0.5,
            DREF_SIM_CAUT if changed => state.master_caut = val > 0.5,
            // Panel 2 (APU / Overhead / Eng Mode)
            DREF_APU_N1 => {
                state.apu_n1_pct = val.clamp(0.0, 100.0);
                if state.apu_n1_pct >= 95.0 {
                    state.apu_avail = true;
                    state.apu_start_on = false;
                }
            }
            DREF_APU_MASTER if changed => state.apu_master_on = val > 0.1,
            DREF_APU_START if changed => state.apu_start_on = val > 0.1,
            DREF_APU_AVAIL if changed => state.apu_avail = val > 0.1,
            DREF_APU_BLEED if changed => state.apu_bleed_on = val > 0.1,
            DREF_BAT1 if changed => state.bat1_on = val > 0.5,
            DREF_BAT2 if changed => state.bat2_on = val > 0.5,
            DREF_FUEL_PUMP if changed => state.fuel_pumps_on = val > 0.5,
            DREF_ENG_MODE if changed => state.eng_mode = val.round() as i8,
            DREF_BEACON if changed => state.beacon_on = val > 0.5,
            _ => {}
        }
    }

    /// Executes an Airbus A320/A330 FCU or Overhead APU/Start control press via WebAPI + UDP.
    pub fn trigger_a320_control(&mut self, control_name: &str, custom_cmd: Option<&str>) {
        if let Some(cmd) = custom_cmd {
            if !cmd.trim().is_empty() {
                for single in cmd.split(';') {
                    let trimmed = single.trim();
                    if !trimmed.is_empty() {
                        self.send_command(trimmed);
                    }
                }
            }
        }

        let norm = control_name.trim().to_ascii_lowercase();
        match norm.as_str() {
            // --- PANEL 1: FCU / Autopilot ---
            "ap1" | "ap_1" => {
                self.send_command("sim/autopilot/servos_toggle");
                self.send_command("toliss_airbus/ap1_push");
                self.send_command("AirbusFBW/AP1button");
                let was_on = self.state.ap1_on;
                self.state.ap1_on = !was_on;
                if was_on && !self.state.ap2_on {
                    self.state.master_warn = true;
                }
            }
            "ap2" | "ap_2" => {
                self.send_command("sim/autopilot/servos2_toggle");
                self.send_command("toliss_airbus/ap2_push");
                self.send_command("AirbusFBW/AP2button");
                let was_on = self.state.ap2_on;
                self.state.ap2_on = !was_on;
                if was_on && !self.state.ap1_on {
                    self.state.master_warn = true;
                }
            }
            "athr" | "a/thr" | "autothrottle" => {
                self.send_command("laminar/A333/autopilot/a_thr_toggle");
                self.send_command("sim/autopilot/autothrottle_toggle");
                self.send_command("toliss_airbus/athr_push");
                self.send_command("AirbusFBW/ATHRbutton");
                let was_on = self.state.athr_on;
                self.state.athr_on = !was_on;
                if was_on {
                    self.state.master_caut = true;
                }
            }
            "fd" | "fd1" | "flight_director" => {
                self.send_command("sim/autopilot/fdir_toggle");
                self.send_command("toliss_airbus/fd1_push");
                self.send_command("AirbusFBW/FD1button");
                self.state.fd_on = !self.state.fd_on;
            }
            "loc" | "localizer" => {
                self.send_command("sim/autopilot/NAV");
                self.send_command("AirbusFBW/LOCbutton");
                self.state.loc_on = !self.state.loc_on;
                if self.state.loc_on {
                    self.state.appr_on = false;
                }
            }
            "appr" | "approach" => {
                self.send_command("sim/autopilot/approach");
                self.send_command("AirbusFBW/APPRbutton");
                self.state.appr_on = !self.state.appr_on;
                if self.state.appr_on {
                    self.state.loc_on = true;
                }
            }
            "exped" | "expedite" => {
                self.send_command("sim/autopilot/level_change");
                self.send_command("AirbusFBW/EXPEDbutton");
                self.state.exped_on = !self.state.exped_on;
            }
            "hdg" | "heading" => {
                if self.state.hdg_managed {
                    self.send_command("laminar/A333/autopilot/heading_knob_pull");
                    self.send_command("sim/autopilot/heading");
                    self.send_command("AirbusFBW/PullHDGSel");
                } else {
                    self.send_command("laminar/A333/autopilot/heading_knob_push");
                    self.send_command("sim/autopilot/NAV");
                    self.send_command("AirbusFBW/PushHDGSel");
                }
                self.state.hdg_managed = !self.state.hdg_managed;
            }
            "alt" | "altitude" => {
                if self.state.alt_managed {
                    self.send_command("laminar/A333/autopilot/altitude_knob_pull");
                    self.send_command("sim/autopilot/altitude_hold");
                    self.send_command("AirbusFBW/PullAltitude");
                } else {
                    self.send_command("laminar/A333/autopilot/altitude_knob_push");
                    self.send_command("AirbusFBW/PushAltitude");
                }
                self.state.alt_managed = !self.state.alt_managed;
            }
            "spd" | "speed" | "spd_mach" => {
                if self.state.spd_managed {
                    self.send_command("laminar/A333/autopilot/speed_knob_pull");
                    self.send_command("sim/autopilot/airspeed_sync");
                    self.send_command("AirbusFBW/PullSPDSel");
                } else {
                    self.send_command("laminar/A333/autopilot/speed_knob_push");
                    self.send_command("AirbusFBW/PushSPDSel");
                }
                self.state.spd_managed = !self.state.spd_managed;
            }
            "trk" | "hdg_trk" | "trkfpa" | "bird" => {
                self.send_command("sim/autopilot/trkfpa");
                self.send_command("AirbusFBW/HDGTRKbutton");
                self.state.trk_fpa_mode = !self.state.trk_fpa_mode;
            }
            "vs" | "vs_zero" | "lvl_ch" => {
                self.send_command("laminar/A333/autopilot/vertical_knob_push");
                self.send_command("sim/autopilot/vertical_speed");
                self.send_command("AirbusFBW/PushVSSel");
                self.state.vs_fpm = 0.0;
                self.state.exped_on = false;
            }
            "master_warn" | "warn" => {
                self.send_command("sim/annunciator/clear_master_warning");
                self.send_command("AirbusFBW/CaptWarnPush");
                self.state.master_warn = false;
            }
            "master_caut" | "caut" | "caution" => {
                self.send_command("sim/annunciator/clear_master_caution");
                self.send_command("AirbusFBW/CaptCautPush");
                self.state.master_caut = false;
            }

            // --- PANEL 2: Overhead Electrical, APU & Engine Start Prep ---
            "bat1" | "battery1" => {
                self.send_command("sim/electrical/battery_1_toggle");
                self.send_command("AirbusFBW/Bat1Toggle");
                self.state.bat1_on = !self.state.bat1_on;
            }
            "bat2" | "battery2" => {
                self.send_command("sim/electrical/battery_2_toggle");
                self.send_command("laminar/A333/buttons/APU_batt_toggle");
                self.send_command("AirbusFBW/Bat2Toggle");
                self.state.bat2_on = !self.state.bat2_on;
            }
            "ext_pwr" | "external_power" | "gpu" => {
                self.send_command("laminar/A333/buttons/external_powerA_toggle");
                self.send_command("sim/electrical/gpu_toggle");
                self.send_command("AirbusFBW/ExtPwrToggle");
                self.state.ext_pwr_on = !self.state.ext_pwr_on;
            }
            "apu_master" | "apu_mst" => {
                self.send_command("laminar/A333/buttons/APU_master");
                if self.state.apu_master_on {
                    self.send_command("sim/electrical/APU_off");
                } else {
                    self.send_command("sim/electrical/APU_on");
                }
                self.send_command("AirbusFBW/APUMaster");
                self.state.apu_master_on = !self.state.apu_master_on;
                if !self.state.apu_master_on {
                    self.state.apu_start_on = false;
                    self.state.apu_avail = false;
                }
            }
            "apu_start" | "apu_str" => {
                self.send_command("laminar/A333/buttons/APU_start");
                self.send_command("sim/electrical/APU_start");
                self.send_command("AirbusFBW/APUStarter");
                if !self.state.apu_avail {
                    self.state.apu_master_on = true;
                    self.state.apu_start_on = true;
                }
            }
            "apu_bleed" | "apu_bld" => {
                self.send_command("sim/bleed_air/apu_toggle");
                self.send_command("AirbusFBW/APUBleedSwitch");
                self.state.apu_bleed_on = !self.state.apu_bleed_on;
            }
            "apu_gen" => {
                self.send_command("laminar/A333/buttons/APU_gen_toggle");
                self.send_command("sim/electrical/APU_generator_toggle");
                self.state.apu_gen_on = !self.state.apu_gen_on;
            }
            "fuel_pumps" | "fuel" | "pumps" => {
                for pump_cmd in [
                    "laminar/A333/switches/left1_pump_toggle",
                    "laminar/A333/switches/left2_pump_toggle",
                    "laminar/A333/switches/right1_pump_toggle",
                    "laminar/A333/switches/right2_pump_toggle",
                    "laminar/A333/switches/center_left_pump_toggle",
                    "laminar/A333/switches/center_right_pump_toggle",
                ] {
                    self.send_command(pump_cmd);
                }
                self.state.fuel_pumps_on = !self.state.fuel_pumps_on;
            }
            "eng_mode" | "ign_start" => {
                // Cycle: NORM (0) -> IGN/START (1) -> CRANK (-1) -> NORM (0)
                match self.state.eng_mode {
                    0 => {
                        self.send_command("laminar/A333/switch/eng_mode_right");
                        self.state.eng_mode = 1;
                    }
                    1 => {
                        self.send_command("laminar/A333/switch/eng_mode_left");
                        self.send_command("laminar/A333/switch/eng_mode_left");
                        self.state.eng_mode = -1;
                    }
                    _ => {
                        self.send_command("laminar/A333/switch/eng_mode_right");
                        self.state.eng_mode = 0;
                    }
                }
            }
            "packs" | "pack" => {
                self.send_command("sim/bleed_air/pack_left_toggle");
                self.send_command("sim/bleed_air/pack_right_toggle");
                self.state.packs_on = !self.state.packs_on;
            }
            "beacon" => {
                self.send_command("sim/lights/beacon_lights_toggle");
                self.state.beacon_on = !self.state.beacon_on;
            }
            "ecam_apu" => {
                self.send_command("laminar/A333/button/ecam/apu_mode");
                self.state.ecam_page = "APU";
            }
            "ecam_eng" => {
                self.send_command("laminar/A333/button/ecam/eng_mode");
                self.state.ecam_page = "ENG";
            }
            other => {
                if custom_cmd.is_none() && !other.is_empty() {
                    self.send_command(other);
                }
            }
        }
    }
}
