use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::thread;
use std::time::Duration;

const AI_LIMITS_REFRESH_INTERVAL: Duration = Duration::from_secs(15);

/// Session (`5h`) and Weekly (`7d`) utilization percentages (`0.0..=100.0`) for a single quota group.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct QuotaWindowPair {
    /// 5-hour session utilization percentage (`0.0..=100.0`)
    pub session_pct: f32,
    /// 7-day weekly utilization percentage (`0.0..=100.0`)
    pub weekly_pct: f32,
    /// Whether live quota telemetry has been successfully retrieved
    pub available: bool,
}

pub type ClaudeLimits = QuotaWindowPair;

/// Utilization percentages across all 3 Google Gemini / Antigravity quota groups
/// returned by `RetrieveUserQuotaSummary`:
/// - Page 0 (`1/3`): `GEMINI` (`gemini-5h` + `gemini-weekly`)
/// - Page 1 (`2/3`): `ARGON` (Argon `5h` + `gemini-argon-wtu-weekly`)
/// - Page 2 (`3/3`): `CLD/GPT` (`3p-5h` + `3p-weekly`)
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct GeminiLimits {
    pub gemini: QuotaWindowPair,
    pub argon: QuotaWindowPair,
    pub claude_gpt: QuotaWindowPair,
    /// Active page on the Gemini tile (`0` = Gemini, `1` = Argon, `2` = Claude/GPT)
    pub page_index: u8,
    /// Whether live quota telemetry has been successfully retrieved
    pub available: bool,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct AiLimitsState {
    pub claude: QuotaWindowPair,
    pub gemini: GeminiLimits,
}

struct AiLimitsWorker {
    state: Mutex<AiLimitsState>,
    trigger: (Mutex<bool>, Condvar),
}

static AI_LIMITS_WORKER: OnceLock<Arc<AiLimitsWorker>> = OnceLock::new();
static GEMINI_PAGE_INDEX: AtomicU8 = AtomicU8::new(0);

/// Cycles the active Gemini tile page (`0`: Gemini -> `1`: Argon -> `2`: Claude/GPT -> `0`)
/// and returns the new page index (`0..3`).
pub fn toggle_gemini_view() -> u8 {
    let prev = GEMINI_PAGE_INDEX.load(Ordering::Relaxed);
    let next = (prev + 1) % 3;
    GEMINI_PAGE_INDEX.store(next, Ordering::Relaxed);
    next
}

fn get_or_start_worker() -> &'static Arc<AiLimitsWorker> {
    AI_LIMITS_WORKER.get_or_init(|| {
        let worker = Arc::new(AiLimitsWorker {
            state: Mutex::new(AiLimitsState::default()),
            trigger: (Mutex::new(true), Condvar::new()),
        });
        let worker_clone = Arc::clone(&worker);
        thread::spawn(move || {
            loop {
                let claude = fetch_claude_limits();
                let gemini = fetch_gemini_limits();
                if let Ok(mut guard) = worker_clone.state.lock() {
                    if let Some(c) = claude {
                        guard.claude = c;
                    }
                    if let Some(g) = gemini {
                        guard.gemini = g;
                    }
                }

                let (lock, cvar) = &worker_clone.trigger;
                if let Ok(mut requested) = lock.lock() {
                    *requested = false;
                    let _ = cvar.wait_timeout_while(
                        requested,
                        AI_LIMITS_REFRESH_INTERVAL,
                        |&mut req| !req,
                    );
                } else {
                    thread::sleep(AI_LIMITS_REFRESH_INTERVAL);
                }
            }
        });
        worker
    })
}

/// Returns the most recently cached Claude and Gemini usage limits without blocking the render thread,
/// starting the background 15-second poller if not already running.
pub fn current_ai_limits() -> AiLimitsState {
    let worker = get_or_start_worker();
    let mut state = worker.state.lock().map(|g| *g).unwrap_or_default();
    state.gemini.page_index = GEMINI_PAGE_INDEX.load(Ordering::Relaxed);
    state
}

/// Requests an immediate asynchronous refresh of Claude and Gemini usage limits.
pub fn request_ai_limits_refresh() {
    let worker = get_or_start_worker();
    let (lock, cvar) = &worker.trigger;
    if let Ok(mut requested) = lock.lock() {
        *requested = true;
        cvar.notify_one();
    }
}

/// Fetches Claude session (`five_hour`) and weekly (`seven_day`) utilization from Anthropic's OAuth usage endpoint.
pub fn fetch_claude_limits() -> Option<QuotaWindowPair> {
    let token = read_claude_oauth_token()?;
    let auth_header = format!("Authorization: Bearer {token}");
    let output = Command::new("curl")
        .args([
            "-sS",
            "--max-time",
            "4",
            "https://api.anthropic.com/api/oauth/usage",
            "-H",
            &auth_header,
            "-H",
            "anthropic-beta: oauth-2025-04-20",
            "-H",
            "User-Agent: claude-code/2.0.0",
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let body = String::from_utf8_lossy(&output.stdout);
    parse_claude_usage_json(&body)
}

fn read_claude_oauth_token() -> Option<String> {
    if let Ok(output) = Command::new("security")
        .args([
            "find-generic-password",
            "-s",
            "Claude Code-credentials",
            "-w",
        ])
        .output()
        && output.status.success()
    {
        let raw = String::from_utf8_lossy(&output.stdout);
        if let Some(tok) = extract_json_string_field(&raw, "accessToken") {
            return Some(tok);
        }
    }

    let home = std::env::var_os("HOME")?;
    let creds_path = PathBuf::from(home)
        .join(".claude")
        .join(".credentials.json");
    let raw = fs::read_to_string(creds_path).ok()?;
    extract_json_string_field(&raw, "accessToken")
}

/// Parses Anthropic `/api/oauth/usage` JSON for `five_hour.utilization` and `seven_day.utilization`.
pub fn parse_claude_usage_json(json: &str) -> Option<QuotaWindowPair> {
    let session_pct = extract_nested_f32(json, "\"five_hour\"", "\"utilization\"")?;
    let weekly_pct = extract_nested_f32(json, "\"seven_day\"", "\"utilization\"").unwrap_or(0.0);

    Some(QuotaWindowPair {
        session_pct: session_pct.clamp(0.0, 100.0),
        weekly_pct: weekly_pct.clamp(0.0, 100.0),
        available: true,
    })
}

/// Fetches live Google Gemini / Antigravity quota groups (`Gemini`, `Argon`, `Claude/GPT`)
/// with both `5h` session and `weekly` limits via `/exa.language_server_pb.LanguageServerService/RetrieveUserQuotaSummary`.
pub fn fetch_gemini_limits() -> Option<GeminiLimits> {
    let ps_out = Command::new("ps")
        .args(["-ax", "-o", "pid=,command="])
        .output()
        .ok()?;
    let ps_text = String::from_utf8_lossy(&ps_out.stdout);
    let (pid, csrf_token) = parse_ps_for_antigravity(&ps_text)?;

    let lsof_out = Command::new("lsof")
        .args(["-Pan", "-p", &pid.to_string(), "-iTCP", "-sTCP:LISTEN"])
        .output()
        .ok()?;
    let lsof_text = String::from_utf8_lossy(&lsof_out.stdout);
    let ports = parse_lsof_listen_ports(&lsof_text);

    let csrf_header = format!("x-codeium-csrf-token: {csrf_token}");
    for &port in ports.iter().take(4) {
        for scheme in ["http", "https"] {
            let url = format!(
                "{scheme}://127.0.0.1:{port}/exa.language_server_pb.LanguageServerService/RetrieveUserQuotaSummary"
            );
            let Ok(output) = Command::new("curl")
                .args([
                    "-sk",
                    "--max-time",
                    "2",
                    "-X",
                    "POST",
                    &url,
                    "-H",
                    "Content-Type: application/json",
                    "-H",
                    "Connect-Protocol-Version: 1",
                    "-H",
                    &csrf_header,
                    "-d",
                    r#"{"metadata":{"ideName":"antigravity","extensionName":"antigravity","locale":"en"}}"#,
                ])
                .output()
            else {
                continue;
            };

            if output.status.success() {
                let body = String::from_utf8_lossy(&output.stdout);
                if let Some(limits) = parse_gemini_quota_summary_json(&body) {
                    return Some(limits);
                }
            }
        }
    }

    None
}

/// Extracts `(pid, csrf_token)` from `ps -ax -o pid=,command=` output for the Antigravity `language_server`.
pub fn parse_ps_for_antigravity(ps_text: &str) -> Option<(u32, String)> {
    for line in ps_text.lines() {
        let trimmed = line.trim();
        if !trimmed.contains("language_server") || !trimmed.contains("--csrf_token") {
            continue;
        }
        let mut parts = trimmed.split_whitespace();
        let pid_str = parts.next()?;
        let pid = pid_str.parse::<u32>().ok()?;
        while let Some(arg) = parts.next() {
            if arg == "--csrf_token" {
                let token = parts.next()?;
                if !token.is_empty() {
                    return Some((pid, token.to_string()));
                }
            }
        }
    }
    None
}

/// Extracts `127.0.0.1:<port>` listening TCP ports from `lsof -Pan -iTCP -sTCP:LISTEN` output.
pub fn parse_lsof_listen_ports(lsof_text: &str) -> Vec<u16> {
    let mut ports = Vec::new();
    for line in lsof_text.lines() {
        if let Some(pos) = line.find("127.0.0.1:") {
            let after = &line[pos + "127.0.0.1:".len()..];
            let digits: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
            if let Ok(port) = digits.parse::<u16>()
                && !ports.contains(&port)
            {
                ports.push(port);
            }
        }
    }
    ports
}

/// Parses Antigravity `RetrieveUserQuotaSummary` JSON into the 3 quota pages:
/// - `gemini` (`gemini-5h` + `gemini-weekly`)
/// - `argon` (Argon `5h` + `gemini-argon-wtu-weekly`)
/// - `claude_gpt` (`3p-5h` + `3p-weekly`)
pub fn parse_gemini_quota_summary_json(json: &str) -> Option<GeminiLimits> {
    if !json.contains("\"bucketId\"") {
        return None;
    }

    let mut gemini = QuotaWindowPair {
        session_pct: 0.0,
        weekly_pct: 0.0,
        available: true,
    };
    let mut argon = QuotaWindowPair {
        session_pct: 0.0,
        weekly_pct: 0.0,
        available: true,
    };
    let mut claude_gpt = QuotaWindowPair {
        session_pct: 0.0,
        weekly_pct: 0.0,
        available: true,
    };
    let mut found_any = false;
    let mut search_from = 0;

    while let Some(rel_pos) = json[search_from..].find("\"bucketId\"") {
        let key_pos = search_from + rel_pos;
        let after_key = &json[key_pos + "\"bucketId\"".len()..];
        let next_pos = after_key
            .find("\"bucketId\"")
            .map(|p| key_pos + "\"bucketId\"".len() + p)
            .unwrap_or(json.len());
        let chunk = &json[key_pos..next_pos];

        if let Some(bucket_id) = extract_json_string_field(chunk, "bucketId") {
            let window = extract_json_string_field(chunk, "window").unwrap_or_default();
            let rem = if chunk.contains("\"disabled\":true") || chunk.contains("\"disabled\": true")
            {
                0.0
            } else {
                extract_f32_after_key(chunk, "\"remainingFraction\"")
                    .unwrap_or(1.0)
                    .clamp(0.0, 1.0)
            };
            let used_pct = ((1.0 - rem) * 100.0).clamp(0.0, 100.0);
            let is_5h = window.eq_ignore_ascii_case("5h") || bucket_id.ends_with("-5h");

            let lower_id = bucket_id.to_ascii_lowercase();
            if lower_id.contains("argon") {
                if is_5h {
                    argon.session_pct = used_pct;
                } else {
                    argon.weekly_pct = used_pct;
                }
                found_any = true;
            } else if lower_id.starts_with("gemini") {
                if is_5h {
                    gemini.session_pct = used_pct;
                } else {
                    gemini.weekly_pct = used_pct;
                }
                found_any = true;
            } else if lower_id.starts_with("3p")
                || lower_id.contains("claude")
                || lower_id.contains("gpt")
            {
                if is_5h {
                    claude_gpt.session_pct = used_pct;
                } else {
                    claude_gpt.weekly_pct = used_pct;
                }
                found_any = true;
            }
        }

        search_from = next_pos;
    }

    if !found_any {
        return None;
    }

    Some(GeminiLimits {
        gemini,
        argon,
        claude_gpt,
        page_index: GEMINI_PAGE_INDEX.load(Ordering::Relaxed),
        available: true,
    })
}

fn extract_json_string_field(json: &str, field_name: &str) -> Option<String> {
    let needle = format!("\"{field_name}\"");
    let pos = json.find(&needle)?;
    let after_key = &json[pos + needle.len()..];
    let colon_pos = after_key.find(':')?;
    let after_colon = after_key[colon_pos + 1..].trim_start();
    let after_quote = after_colon.strip_prefix('"')?;
    let end_quote = after_quote.find('"')?;
    Some(after_quote[..end_quote].to_string())
}

fn extract_f32_after_key(json: &str, key: &str) -> Option<f32> {
    let pos = json.find(key)?;
    let after_key = &json[pos + key.len()..];
    let colon_pos = after_key.find(':')?;
    let after_colon = after_key[colon_pos + 1..].trim_start();
    let num_str: String = after_colon
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.' || *c == '-')
        .collect();
    num_str.parse::<f32>().ok()
}

fn extract_nested_f32(json: &str, parent_key: &str, child_key: &str) -> Option<f32> {
    let parent_pos = json.find(parent_key)?;
    let after_parent = &json[parent_pos + parent_key.len()..];
    let close_brace = after_parent.find('}').unwrap_or(after_parent.len());
    let obj_slice = &after_parent[..close_brace];
    extract_f32_after_key(obj_slice, child_key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_claude_usage_json() {
        let sample = r#"{
            "five_hour": {"utilization": 14.5, "resets_at": "2026-10-17T20:10:00Z"},
            "seven_day": {"utilization": 42.0, "resets_at": "2026-10-24T15:00:00Z"}
        }"#;
        let limits = parse_claude_usage_json(sample).expect("Should parse Claude usage JSON");
        assert!(limits.available);
        assert!((limits.session_pct - 14.5).abs() < 0.01);
        assert!((limits.weekly_pct - 42.0).abs() < 0.01);
    }

    #[test]
    fn test_parse_gemini_quota_summary_json() {
        let sample = r#"{
            "response": {
                "groups": [
                    {
                        "displayName": "Gemini Models",
                        "buckets": [
                            {"bucketId": "gemini-weekly", "window": "weekly", "remainingFraction": 0.98},
                            {"bucketId": "gemini-5h", "window": "5h", "remainingFraction": 0.90}
                        ]
                    },
                    {
                        "displayName": "Claude and GPT models",
                        "buckets": [
                            {"bucketId": "3p-weekly", "window": "weekly", "remainingFraction": 0.75},
                            {"bucketId": "3p-5h", "window": "5h", "remainingFraction": 0.95}
                        ]
                    },
                    {
                        "displayName": "Gemini 4 Argon",
                        "buckets": [
                            {"bucketId": "gemini-argon-wtu-weekly", "window": "weekly", "remainingFraction": 0.28}
                        ]
                    }
                ]
            }
        }"#;
        let limits =
            parse_gemini_quota_summary_json(sample).expect("Should parse Gemini quota summary");
        assert!(limits.available);
        assert!((limits.gemini.session_pct - 10.0).abs() < 0.1);
        assert!((limits.gemini.weekly_pct - 2.0).abs() < 0.1);
        assert!((limits.argon.session_pct - 0.0).abs() < 0.1);
        assert!((limits.argon.weekly_pct - 72.0).abs() < 0.1);
        assert!((limits.claude_gpt.session_pct - 5.0).abs() < 0.1);
        assert!((limits.claude_gpt.weekly_pct - 25.0).abs() < 0.1);
    }

    #[test]
    fn test_parse_ps_and_lsof_helpers() {
        let ps = " 68140 /Applications/Antigravity.app/Contents/Resources/bin/language_server --standalone --csrf_token b6694d70-0019-4388 --app_data_dir antigravity\n";
        let (pid, token) = parse_ps_for_antigravity(ps).expect("Should extract PID and CSRF token");
        assert_eq!(pid, 68140);
        assert_eq!(token, "b6694d70-0019-4388");

        let lsof = "language_ 68140 bradley 7u IPv4 0x123 0t0 TCP 127.0.0.1:60730 (LISTEN)\nlanguage_ 68140 bradley 8u IPv4 0x124 0t0 TCP 127.0.0.1:60731 (LISTEN)\n";
        let ports = parse_lsof_listen_ports(lsof);
        assert_eq!(ports, vec![60730, 60731]);
    }
}
