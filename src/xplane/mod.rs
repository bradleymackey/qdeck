pub mod commands;
pub mod state;
pub mod webapi;

pub use state::A320FcuState;

use commands::dispatch_control;
use state::{UDP_RREF_SUBSCRIPTIONS, apply_dref_update, datarefs_for_panel};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream, UdpSocket};
use std::time::{Duration, Instant};
use webapi::{extract_json_f32_data, extract_json_u64_id, format_http_request};

const WEB_API_ADDR: ([u8; 4], u16) = ([127, 0, 0, 1], 8086);

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
        if let Some(until) = self.web_api_backoff_until
            && Instant::now() < until
        {
            return None;
        }

        let addr = SocketAddr::from(WEB_API_ADDR);
        let Ok(mut stream) = TcpStream::connect_timeout(&addr, Duration::from_millis(35)) else {
            self.web_api_backoff_until = Some(Instant::now() + Duration::from_secs(3));
            return None;
        };
        let _ = stream.set_read_timeout(Some(Duration::from_millis(60)));
        let _ = stream.set_write_timeout(Some(Duration::from_millis(35)));

        let req = format_http_request(method, path, body);
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

        for &(id, path) in UDP_RREF_SUBSCRIPTIONS {
            self.send_rref(id, 2, path);
        }
    }

    /// Drains any incoming `RREF` UDP packets OR polls X-Plane 12's Local WebAPI (`127.0.0.1:8086`)
    /// for the active panel (`"fcu"`, `"apu"`, `"flt"`, or `"sys"`) and updates `self.state`.
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
                        apply_dref_update(&mut self.state, &mut self.prev_dref_vals, id, val);
                        offset += 8;
                    }
                }
            }
        }

        if !got_udp {
            let list = datarefs_for_panel(active_panel);
            for &(key_id, dref_name) in list {
                let Some(web_id) = self.resolve_web_dref_id(dref_name) else {
                    if self
                        .web_api_backoff_until
                        .is_some_and(|u| Instant::now() < u)
                    {
                        break;
                    }
                    continue;
                };
                let path = format!("/api/v2/datarefs/{web_id}/value");
                if let Some(body) = self.web_api_request("GET", &path, None)
                    && let Some(val) = extract_json_f32_data(&body)
                {
                    self.state.last_rx = Some(Instant::now());
                    apply_dref_update(&mut self.state, &mut self.prev_dref_vals, key_id, val);
                }
            }
        }
    }

    /// Executes an Airbus A320/A330 control press across Panels 1-4 via WebAPI + UDP.
    pub fn trigger_a320_control(&mut self, control_name: &str, custom_cmd: Option<&str>) {
        let mut cmds_to_send: Vec<String> = Vec::new();
        dispatch_control(&mut self.state, control_name, custom_cmd, |cmd| {
            cmds_to_send.push(cmd.to_string());
        });
        for cmd in cmds_to_send {
            self.send_command(&cmd);
        }
    }
}
