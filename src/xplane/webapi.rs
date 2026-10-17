/// Extracts the first numeric `"id": <u64>` value from an X-Plane 12 Local WebAPI JSON response.
pub fn extract_json_u64_id(json: &str) -> Option<u64> {
    let idx = json.find("\"id\":")?;
    let after = &json[idx + 5..];
    let digits: String = after
        .trim_start()
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    digits.parse::<u64>().ok()
}

/// Extracts the first numeric `"data": <f32>` value from an X-Plane 12 Local WebAPI JSON response.
pub fn extract_json_f32_data(json: &str) -> Option<f32> {
    let idx = json.find("\"data\":")?;
    let after = json[idx + 7..].trim_start().trim_start_matches('[');
    let num_str: String = after
        .trim_start()
        .chars()
        .take_while(|c| {
            c.is_ascii_digit() || *c == '.' || *c == '-' || *c == '+' || *c == 'e' || *c == 'E'
        })
        .collect();
    num_str.parse::<f32>().ok()
}

/// Extracts a numeric array `"data": [<f32>, ...]` (or single scalar `"data": <f32>`)
/// from an X-Plane 12 Local WebAPI JSON response.
pub fn extract_json_f32_array(json: &str) -> Option<Vec<f32>> {
    let idx = json.find("\"data\":")?;
    let after = json[idx + 7..].trim_start();
    if let Some(arr_body) = after.strip_prefix('[') {
        let end = arr_body.find(']')?;
        let inner = &arr_body[..end];
        let vals: Vec<f32> = inner
            .split(',')
            .filter_map(|s| s.trim().parse::<f32>().ok())
            .collect();
        if vals.is_empty() { None } else { Some(vals) }
    } else {
        extract_json_f32_data(json).map(|v| vec![v])
    }
}

/// Formats an HTTP/1.1 request targeting X-Plane 12's Local WebAPI (`127.0.0.1:8086`).
pub fn format_http_request(method: &str, path: &str, body: Option<&str>) -> String {
    if let Some(b) = body {
        format!(
            "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:8086\r\nAccept: application/json\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{b}",
            b.len()
        )
    } else {
        format!(
            "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:8086\r\nAccept: application/json\r\nConnection: close\r\n\r\n"
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_json_u64_id() {
        let sample = r#"{"data":[{"id":1948,"name":"sim/autopilot/servos_toggle","description":"Toggle servos."}]}"#;
        assert_eq!(extract_json_u64_id(sample), Some(1948));
        assert_eq!(extract_json_u64_id(r#"{"data":[]}"#), None);
    }

    #[test]
    fn test_extract_json_f32_data() {
        assert_eq!(extract_json_f32_data(r#"{"data":250.5}"#), Some(250.5));
        assert_eq!(extract_json_f32_data(r#"{"data": -1200}"#), Some(-1200.0));
        assert_eq!(extract_json_f32_data(r#"{"data": 1.5e3}"#), Some(1500.0));
        assert_eq!(extract_json_f32_data(r#"{"error":"not_found"}"#), None);
        assert_eq!(
            extract_json_f32_array(r#"{"data":[1,0.5,0,0,0]}"#),
            Some(vec![1.0, 0.5, 0.0, 0.0, 0.0])
        );
    }

    #[test]
    fn test_format_http_request() {
        let get_req = format_http_request("GET", "/api/v2/datarefs/10/value", None);
        assert!(get_req.starts_with("GET /api/v2/datarefs/10/value HTTP/1.1\r\n"));
        assert!(get_req.ends_with("\r\n\r\n"));

        let post_req = format_http_request(
            "POST",
            "/api/v2/command/42/activate",
            Some("{\"duration\":0}"),
        );
        assert!(post_req.contains("Content-Length: 14\r\n"));
        assert!(post_req.ends_with("\r\n\r\n{\"duration\":0}"));
    }
}
