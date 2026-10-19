#[cfg(not(target_os = "macos"))]
use std::process::Command;

/// Represents a single macOS audio endpoint (Input or Output).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AudioEndpoint {
    /// Full device name reported by macOS CoreAudio (e.g. `"Bradley’s APP 2"`).
    pub raw_name: String,
    /// Compact uppercase label formatted for a 72x72 Stream Deck tile (max 10 chars, e.g. `"APP 2"`).
    pub short_name: String,
    /// Short transport badge (e.g. `"BT"`, `"USB"`, `"HDMI"`, `"DP"`, `"AIR"`, `"INT"`).
    pub transport: String,
}

/// Snapshot of the current macOS default audio output and input devices.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AudioDevices {
    pub output: AudioEndpoint,
    pub input: AudioEndpoint,
    pub available: bool,
}

#[cfg(target_os = "macos")]
mod coreaudio_ffi {
    use std::ffi::{CStr, c_char, c_void};

    #[repr(C)]
    pub struct AudioObjectPropertyAddress {
        pub m_selector: u32,
        pub m_scope: u32,
        pub m_element: u32,
    }

    pub const K_AUDIO_OBJECT_SYSTEM_OBJECT: u32 = 1;
    pub const K_AUDIO_HARDWARE_PROPERTY_DEFAULT_INPUT_DEVICE: u32 = u32::from_be_bytes(*b"dIn ");
    pub const K_AUDIO_HARDWARE_PROPERTY_DEFAULT_OUTPUT_DEVICE: u32 = u32::from_be_bytes(*b"dOut");
    pub const K_AUDIO_OBJECT_PROPERTY_NAME: u32 = u32::from_be_bytes(*b"lnam");
    pub const K_AUDIO_DEVICE_PROPERTY_TRANSPORT_TYPE: u32 = u32::from_be_bytes(*b"tran");
    pub const K_AUDIO_OBJECT_PROPERTY_SCOPE_GLOBAL: u32 = u32::from_be_bytes(*b"glob");
    pub const K_AUDIO_OBJECT_PROPERTY_ELEMENT_MAIN: u32 = 0;

    pub const K_CF_STRING_ENCODING_UTF8: u32 = 0x0800_0100;

    #[link(name = "CoreAudio", kind = "framework")]
    unsafe extern "C" {
        pub fn AudioObjectGetPropertyData(
            in_object_id: u32,
            in_address: *const AudioObjectPropertyAddress,
            in_qualifier_data_size: u32,
            in_qualifier_data: *const c_void,
            io_data_size: *mut u32,
            out_data: *mut c_void,
        ) -> i32;
    }

    #[link(name = "CoreFoundation", kind = "framework")]
    unsafe extern "C" {
        pub fn CFStringGetCString(
            the_string: *const c_void,
            buffer: *mut c_char,
            buffer_size: isize,
            encoding: u32,
        ) -> u8;
        pub fn CFRelease(cf: *const c_void);
    }

    pub fn get_default_device_id(selector: u32) -> Option<u32> {
        let addr = AudioObjectPropertyAddress {
            m_selector: selector,
            m_scope: K_AUDIO_OBJECT_PROPERTY_SCOPE_GLOBAL,
            m_element: K_AUDIO_OBJECT_PROPERTY_ELEMENT_MAIN,
        };
        let mut device_id: u32 = 0;
        let mut size = std::mem::size_of::<u32>() as u32;
        let status = unsafe {
            AudioObjectGetPropertyData(
                K_AUDIO_OBJECT_SYSTEM_OBJECT,
                &addr,
                0,
                std::ptr::null(),
                &mut size,
                (&mut device_id as *mut u32).cast::<c_void>(),
            )
        };
        if status == 0 && device_id != 0 {
            Some(device_id)
        } else {
            None
        }
    }

    pub fn get_device_name(device_id: u32) -> Option<String> {
        let addr = AudioObjectPropertyAddress {
            m_selector: K_AUDIO_OBJECT_PROPERTY_NAME,
            m_scope: K_AUDIO_OBJECT_PROPERTY_SCOPE_GLOBAL,
            m_element: K_AUDIO_OBJECT_PROPERTY_ELEMENT_MAIN,
        };
        let mut cf_str: *const c_void = std::ptr::null();
        let mut size = std::mem::size_of::<*const c_void>() as u32;
        let status = unsafe {
            AudioObjectGetPropertyData(
                device_id,
                &addr,
                0,
                std::ptr::null(),
                &mut size,
                (&mut cf_str as *mut *const c_void).cast::<c_void>(),
            )
        };
        if status != 0 || cf_str.is_null() {
            return None;
        }

        let mut buf = [0 as c_char; 256];
        let ok = unsafe {
            CFStringGetCString(
                cf_str,
                buf.as_mut_ptr(),
                buf.len() as isize,
                K_CF_STRING_ENCODING_UTF8,
            )
        };
        unsafe {
            CFRelease(cf_str);
        }
        if ok == 0 {
            return None;
        }
        let c_str = unsafe { CStr::from_ptr(buf.as_ptr()) };
        let name = c_str.to_string_lossy().trim().to_string();
        (!name.is_empty()).then_some(name)
    }

    pub fn get_device_transport_badge(device_id: u32) -> String {
        let addr = AudioObjectPropertyAddress {
            m_selector: K_AUDIO_DEVICE_PROPERTY_TRANSPORT_TYPE,
            m_scope: K_AUDIO_OBJECT_PROPERTY_SCOPE_GLOBAL,
            m_element: K_AUDIO_OBJECT_PROPERTY_ELEMENT_MAIN,
        };
        let mut transport: u32 = 0;
        let mut size = std::mem::size_of::<u32>() as u32;
        let status = unsafe {
            AudioObjectGetPropertyData(
                device_id,
                &addr,
                0,
                std::ptr::null(),
                &mut size,
                (&mut transport as *mut u32).cast::<c_void>(),
            )
        };
        if status != 0 {
            return String::new();
        }
        match &transport.to_be_bytes() {
            b"blue" | b"blea" => "BT".to_string(),
            b"usb " => "USB".to_string(),
            b"hdmi" => "HDMI".to_string(),
            b"dprt" => "DP".to_string(),
            b"airp" => "AIR".to_string(),
            b"bltn" => "INT".to_string(),
            b"thun" => "TB".to_string(),
            b"virt" => "VIRT".to_string(),
            b"grup" => "AGG".to_string(),
            b"ccwd" | b"ccwl" => "IOS".to_string(),
            _ => String::new(),
        }
    }
}

/// Queries the active macOS default audio output and input devices via CoreAudio.
#[cfg(target_os = "macos")]
pub fn query_macos_audio_devices() -> AudioDevices {
    use coreaudio_ffi::*;

    let out_endpoint = get_default_device_id(K_AUDIO_HARDWARE_PROPERTY_DEFAULT_OUTPUT_DEVICE)
        .and_then(|id| {
            let raw_name = get_device_name(id)?;
            let short_name = format_audio_device_short_name(&raw_name);
            let transport = get_device_transport_badge(id);
            Some(AudioEndpoint {
                raw_name,
                short_name,
                transport,
            })
        });

    let in_endpoint = get_default_device_id(K_AUDIO_HARDWARE_PROPERTY_DEFAULT_INPUT_DEVICE)
        .and_then(|id| {
            let raw_name = get_device_name(id)?;
            let short_name = format_audio_device_short_name(&raw_name);
            let transport = get_device_transport_badge(id);
            Some(AudioEndpoint {
                raw_name,
                short_name,
                transport,
            })
        });

    let available = out_endpoint.is_some() || in_endpoint.is_some();
    AudioDevices {
        output: out_endpoint.unwrap_or_else(|| AudioEndpoint {
            raw_name: "None".to_string(),
            short_name: "NONE".to_string(),
            transport: String::new(),
        }),
        input: in_endpoint.unwrap_or_else(|| AudioEndpoint {
            raw_name: "None".to_string(),
            short_name: "NONE".to_string(),
            transport: String::new(),
        }),
        available,
    }
}

#[cfg(not(target_os = "macos"))]
pub fn query_macos_audio_devices() -> AudioDevices {
    let Ok(output) = Command::new("system_profiler")
        .args(["SPAudioDataType", "-json"])
        .output()
    else {
        return AudioDevices::default();
    };
    let text = String::from_utf8_lossy(&output.stdout);
    parse_spaudio_json(&text).unwrap_or_default()
}

/// Formats a raw macOS audio device name into a compact uppercase string (`<= 10` chars)
/// suitable for rendering in the 5x7 tactical bitmap font on a 72x72 key.
pub fn format_audio_device_short_name(raw_name: &str) -> String {
    let normalized = raw_name
        .replace(['\u{2019}', '\u{2018}', '`'], "'")
        .trim()
        .to_string();

    if normalized.is_empty() {
        return "NONE".to_string();
    }

    // Strip leading possessive prefix like "Bradley's APP 2" -> "APP 2"
    let without_owner = strip_possessive_prefix(&normalized);
    let mut s = without_owner.to_ascii_uppercase();

    // Normalize whitespace
    s = s.split_whitespace().collect::<Vec<_>>().join(" ");

    if s.len() <= 10 {
        return s;
    }

    // Apply progressive tactical abbreviations when longer than 10 chars
    let replacements = [
        ("MACBOOK PRO ", "MBP "),
        ("MACBOOK AIR ", "MBA "),
        ("MAC STUDIO SPEAKERS", "MAC STUDIO"),
        ("MAC STUDIO ", "STUDIO "),
        ("MAC MINI SPEAKERS", "MAC MINI"),
        ("IMAC SPEAKERS", "IMAC"),
        ("EXTERNAL ", "EXT "),
        ("BUILT-IN ", "INT "),
        ("AIRPODS PRO", "AIRPOD PRO"),
        ("AIRPODS MAX", "AIRPOD MAX"),
        ("HEADPHONES", "HDPHN"),
        ("MICROPHONE", "MIC"),
        ("SPEAKERS", "SPKR"),
        ("SPEAKER", "SPKR"),
    ];

    for (from, to) in replacements {
        if s.contains(from) {
            s = s.replace(from, to);
            s = s.split_whitespace().collect::<Vec<_>>().join(" ");
            if s.len() <= 10 {
                return s;
            }
        }
    }

    // If still > 10 chars and ends with redundant role word, strip the suffix
    for suffix in [" SPKR", " MIC", " AUDIO"] {
        if let Some(stripped) = s.strip_suffix(suffix)
            && !stripped.is_empty()
            && stripped.len() <= 10
        {
            return stripped.to_string();
        }
    }

    s.chars()
        .take(10)
        .collect::<String>()
        .trim_end()
        .to_string()
}

fn strip_possessive_prefix(s: &str) -> &str {
    let lower = s.to_ascii_lowercase();
    if let Some(pos) = lower.find("'s ")
        && pos > 0
        && pos <= 18
        && !s[..pos].contains(' ')
    {
        let rest = s[pos + 3..].trim();
        if !rest.is_empty() {
            return rest;
        }
    }
    s
}

/// Parses `system_profiler SPAudioDataType -json` output into `AudioDevices`.
pub fn parse_spaudio_json(json: &str) -> Option<AudioDevices> {
    if !json.contains("SPAudioDataType") {
        return None;
    }

    let mut output: Option<AudioEndpoint> = None;
    let mut input: Option<AudioEndpoint> = None;
    let mut search_from = 0;

    while let Some(rel_pos) = json[search_from..].find("\"_name\"") {
        let pos = search_from + rel_pos;
        let after = &json[pos + "\"_name\"".len()..];
        let next_pos = after
            .find("\"_name\"")
            .map(|p| pos + "\"_name\"".len() + p)
            .unwrap_or(json.len());
        let chunk = &json[pos..next_pos];

        if let Some(name) = extract_json_str(chunk, "_name")
            && name != "coreaudio_device"
        {
            let transport = extract_json_str(chunk, "coreaudio_device_transport")
                .map(|t| spaudio_transport_badge(&t))
                .unwrap_or_default();

            if chunk.contains("\"coreaudio_default_audio_output_device\"")
                && chunk.contains("\"spaudio_yes\"")
            {
                output = Some(AudioEndpoint {
                    short_name: format_audio_device_short_name(&name),
                    raw_name: name.clone(),
                    transport: transport.clone(),
                });
            }
            if chunk.contains("\"coreaudio_default_audio_input_device\"")
                && chunk.contains("\"spaudio_yes\"")
            {
                input = Some(AudioEndpoint {
                    short_name: format_audio_device_short_name(&name),
                    raw_name: name,
                    transport,
                });
            }
        }

        search_from = next_pos;
    }

    if output.is_none() && input.is_none() {
        return None;
    }

    Some(AudioDevices {
        output: output.unwrap_or_else(|| AudioEndpoint {
            raw_name: "None".to_string(),
            short_name: "NONE".to_string(),
            transport: String::new(),
        }),
        input: input.unwrap_or_else(|| AudioEndpoint {
            raw_name: "None".to_string(),
            short_name: "NONE".to_string(),
            transport: String::new(),
        }),
        available: true,
    })
}

fn spaudio_transport_badge(raw: &str) -> String {
    let lower = raw.to_ascii_lowercase();
    if lower.contains("bluetooth") {
        "BT".to_string()
    } else if lower.contains("usb") {
        "USB".to_string()
    } else if lower.contains("hdmi") {
        "HDMI".to_string()
    } else if lower.contains("displayport") {
        "DP".to_string()
    } else if lower.contains("airplay") {
        "AIR".to_string()
    } else if lower.contains("builtin") {
        "INT".to_string()
    } else {
        String::new()
    }
}

fn extract_json_str(json: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\"");
    let pos = json.find(&needle)?;
    let after_key = &json[pos + needle.len()..];
    let colon_pos = after_key.find(':')?;
    let after_colon = after_key[colon_pos + 1..].trim_start();
    let after_quote = after_colon.strip_prefix('"')?;
    let end_quote = after_quote.find('"')?;
    Some(after_quote[..end_quote].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_audio_device_short_name() {
        assert_eq!(format_audio_device_short_name("Bradley’s APP 2"), "APP 2");
        assert_eq!(
            format_audio_device_short_name("Bradley's AirPods Pro"),
            "AIRPOD PRO"
        );
        assert_eq!(
            format_audio_device_short_name("Mac Studio Speakers"),
            "MAC STUDIO"
        );
        assert_eq!(
            format_audio_device_short_name("MacBook Pro Microphone"),
            "MBP MIC"
        );
        assert_eq!(
            format_audio_device_short_name("External Headphones"),
            "EXT HDPHN"
        );
        assert_eq!(format_audio_device_short_name("Brio 300"), "BRIO 300");
        assert_eq!(
            format_audio_device_short_name("DELL S2721QSA"),
            "DELL S2721"
        );
    }

    #[test]
    fn test_parse_spaudio_json() {
        let sample = r#"{
          "SPAudioDataType" : [
            {
              "_items" : [
                {
                  "_name" : "Bradley’s APP 2",
                  "coreaudio_default_audio_input_device" : "spaudio_yes",
                  "coreaudio_device_input" : 1,
                  "coreaudio_device_transport" : "coreaudio_device_type_bluetooth"
                },
                {
                  "_name" : "Bradley’s APP 2",
                  "coreaudio_default_audio_output_device" : "spaudio_yes",
                  "coreaudio_device_output" : 2,
                  "coreaudio_device_transport" : "coreaudio_device_type_bluetooth"
                },
                {
                  "_name" : "Mac Studio Speakers",
                  "coreaudio_default_audio_system_device" : "spaudio_yes",
                  "coreaudio_device_output" : 2,
                  "coreaudio_device_transport" : "coreaudio_device_type_builtin"
                }
              ],
              "_name" : "coreaudio_device"
            }
          ]
        }"#;

        let devs = parse_spaudio_json(sample).expect("Should parse SPAudioDataType JSON");
        assert!(devs.available);
        assert_eq!(devs.output.short_name, "APP 2");
        assert_eq!(devs.output.transport, "BT");
        assert_eq!(devs.input.short_name, "APP 2");
        assert_eq!(devs.input.transport, "BT");
    }

    #[test]
    fn test_query_macos_audio_devices_does_not_panic() {
        let devs = query_macos_audio_devices();
        assert!(!devs.output.short_name.is_empty());
        assert!(!devs.input.short_name.is_empty());
        assert!(devs.output.short_name.len() <= 10);
        assert!(devs.input.short_name.len() <= 10);
    }
}
