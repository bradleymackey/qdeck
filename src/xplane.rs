use std::net::UdpSocket;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct A320FcuState {
    pub fd_on: bool,
    pub ap1_on: bool,
    pub ap2_on: bool,
    pub athr_on: bool,
    pub loc_on: bool,
    pub exped_on: bool,
    pub appr_on: bool,
    pub trk_fpa_mode: bool,
    pub mach_mode: bool,
    pub master_warn: bool,
    pub master_caut: bool,
    pub hdg_managed: bool,
    pub alt_managed: bool,
    pub spd_managed: bool,
    pub hdg_deg: f32,
    pub alt_ft: f32,
    pub spd_kts: f32,
    pub vs_fpm: f32,
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
            mach_mode: false,
            master_warn: false,
            master_caut: false,
            hdg_managed: true,
            alt_managed: true,
            spd_managed: true,
            hdg_deg: 254.0,
            alt_ft: 15000.0,
            spd_kts: 250.0,
            vs_fpm: 0.0,
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
const DREF_TOLISS_AP1: i32 = 20;
const DREF_TOLISS_AP2: i32 = 21;
const DREF_TOLISS_ATHR: i32 = 22;
const DREF_TOLISS_LOC: i32 = 23;
const DREF_TOLISS_APPR: i32 = 24;
const DREF_TOLISS_EXPED: i32 = 25;

pub struct XPlaneClient {
    socket: Option<UdpSocket>,
    target_addr: String,
    subscribed: bool,
    last_sub_attempt: Instant,
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
            state: A320FcuState::default(),
        }
    }

    pub fn set_target_addr(&mut self, addr: &str) {
        if self.target_addr != addr {
            self.target_addr = addr.to_string();
            self.subscribed = false;
        }
    }

    /// Sends an X-Plane `CMND\0<path>\0` UDP datagram to X-Plane (port 49000).
    pub fn send_command(&self, cmd_path: &str) {
        let Some(ref sock) = self.socket else {
            return;
        };
        let mut pkt = Vec::with_capacity(6 + cmd_path.len());
        pkt.extend_from_slice(b"CMND\0");
        pkt.extend_from_slice(cmd_path.as_bytes());
        pkt.push(0);
        let _ = sock.send_to(&pkt, &self.target_addr);
    }

    /// Sends an X-Plane `DREF\0<f32><path[500]>` UDP datagram to set a dataref directly.
    #[allow(dead_code)]
    pub fn send_dataref(&self, dref_path: &str, value: f32) {
        let Some(ref sock) = self.socket else {
            return;
        };
        let mut pkt = vec![0u8; 509];
        pkt[0..5].copy_from_slice(b"DREF\0");
        pkt[5..9].copy_from_slice(&value.to_le_bytes());
        let bytes = dref_path.as_bytes();
        let copy_len = bytes.len().min(499);
        pkt[9..(9 + copy_len)].copy_from_slice(&bytes[..copy_len]);
        let _ = sock.send_to(&pkt, &self.target_addr);
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

    /// Subscribes to standard X-Plane + ToLiss/AirbusFBW A320 FCU datarefs at 2 Hz.
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
            (DREF_SIM_HDG, "sim/cockpit2/autopilot/heading_dial_deg_mag_pilot"),
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
        ];

        for (id, path) in subs {
            self.send_rref(id, 2, path);
        }
    }

    /// Drains any incoming `RREF` UDP packets from X-Plane and updates `self.state`.
    pub fn poll_incoming(&mut self) {
        let Some(ref sock) = self.socket else {
            return;
        };
        let mut buf = [0u8; 2048];
        while let Ok((len, _addr)) = sock.recv_from(&mut buf) {
            if len >= 13 && &buf[0..4] == b"RREF" {
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
                    Self::apply_dref_update(&mut self.state, id, val);
                    offset += 8;
                }
            }
        }
    }

    fn apply_dref_update(state: &mut A320FcuState, id: i32, val: f32) {
        match id {
            DREF_SIM_FD => state.fd_on = val >= 1.0,
            DREF_SIM_AP1 | DREF_TOLISS_AP1 => state.ap1_on = val > 0.5,
            DREF_SIM_AP2 | DREF_TOLISS_AP2 => state.ap2_on = val > 0.5,
            DREF_SIM_ATHR | DREF_TOLISS_ATHR => state.athr_on = val > 0.5,
            DREF_SIM_LOC | DREF_TOLISS_LOC => state.loc_on = val >= 1.0,
            DREF_SIM_APPR | DREF_TOLISS_APPR => state.appr_on = val >= 1.0,
            DREF_TOLISS_EXPED => state.exped_on = val > 0.5,
            DREF_SIM_HDG => state.hdg_deg = val.rem_euclid(360.0),
            DREF_SIM_ALT => state.alt_ft = val,
            DREF_SIM_SPD => state.spd_kts = val,
            DREF_SIM_VS => state.vs_fpm = val,
            DREF_SIM_WARN => state.master_warn = val > 0.5,
            DREF_SIM_CAUT => state.master_caut = val > 0.5,
            _ => {}
        }
    }

    /// Executes an Airbus A320 FCU control press: sends both standard X-Plane and AirbusFBW/ToLiss
    /// UDP commands to X-Plane AND updates optimistic FCU state for immediate cockpit feedback.
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
            "ap1" | "ap_1" => {
                self.send_command("sim/autopilot/servos_toggle");
                self.send_command("toliss_airbus/ap1_push");
                self.send_command("AirbusFBW/AP1button");
                let was_on = self.state.ap1_on;
                self.state.ap1_on = !was_on;
                if was_on && !self.state.ap2_on {
                    // Disconnecting AP triggers Master Warning (Red) just like a real A320!
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
                self.send_command("sim/autopilot/autothrottle_toggle");
                self.send_command("toliss_airbus/athr_push");
                self.send_command("AirbusFBW/ATHRbutton");
                let was_on = self.state.athr_on;
                self.state.athr_on = !was_on;
                if was_on {
                    // Disconnecting A/THR triggers Master Caution (Amber) like a real A320!
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
                self.send_command("sim/autopilot/heading");
                self.send_command("AirbusFBW/PullHDGSel");
                self.state.hdg_managed = !self.state.hdg_managed;
            }
            "alt" | "altitude" => {
                self.send_command("sim/autopilot/altitude_hold");
                self.send_command("AirbusFBW/PullAltitude");
                self.state.alt_managed = !self.state.alt_managed;
            }
            "spd" | "speed" | "spd_mach" => {
                self.send_command("sim/autopilot/airspeed_sync");
                self.send_command("AirbusFBW/SPDabovemach");
                self.state.spd_managed = !self.state.spd_managed;
            }
            "trk" | "hdg_trk" | "trkfpa" | "bird" => {
                self.send_command("sim/autopilot/trkfpa");
                self.send_command("AirbusFBW/HDGTRKbutton");
                self.state.trk_fpa_mode = !self.state.trk_fpa_mode;
            }
            "vs" | "vs_zero" | "lvl_ch" => {
                self.send_command("sim/autopilot/vertical_speed");
                self.send_command("AirbusFBW/PushVSSel");
                self.state.vs_fpm = 0.0;
                self.state.exped_on = false;
            }
            "master_warn" | "warn" => {
                self.send_command("sim/annunciator/clear_master_warning");
                self.send_command("AirbusFBW/CaptWarnPush");
                self.state.master_warn = !self.state.master_warn;
            }
            "master_caut" | "caut" | "caution" => {
                self.send_command("sim/annunciator/clear_master_caution");
                self.send_command("AirbusFBW/CaptCautPush");
                self.state.master_caut = !self.state.master_caut;
            }
            other => {
                if custom_cmd.is_none() && !other.is_empty() {
                    self.send_command(other);
                }
            }
        }
    }
}
