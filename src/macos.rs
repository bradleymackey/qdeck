use anyhow::{Context, Result, anyhow};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};

pub const LAUNCH_AGENT_LABEL: &str = "local.qdeck";

static SHUTDOWN_REQUESTED: AtomicBool = AtomicBool::new(false);

pub fn is_shutdown_requested() -> bool {
    SHUTDOWN_REQUESTED.load(Ordering::SeqCst)
}

extern "C" fn handle_shutdown_signal(_sig: i32) {
    SHUTDOWN_REQUESTED.store(true, Ordering::SeqCst);
}

#[cfg(unix)]
unsafe extern "C" {
    fn signal(sig: i32, handler: extern "C" fn(i32)) -> usize;
}

pub fn install_signal_handlers() {
    #[cfg(unix)]
    {
        const SIGHUP: i32 = 1;
        const SIGINT: i32 = 2;
        const SIGQUIT: i32 = 3;
        const SIGTERM: i32 = 15;
        unsafe {
            signal(SIGHUP, handle_shutdown_signal);
            signal(SIGINT, handle_shutdown_signal);
            signal(SIGQUIT, handle_shutdown_signal);
            signal(SIGTERM, handle_shutdown_signal);
        }
    }
}

#[cfg(target_os = "macos")]
mod cg {
    use std::ffi::{c_char, c_void};

    #[link(name = "CoreGraphics", kind = "framework")]
    unsafe extern "C" {
        pub fn CGMainDisplayID() -> u32;
        pub fn CGDisplayIsAsleep(display: u32) -> u8;
        pub fn CGSessionCopyCurrentDictionary() -> *const c_void;
    }

    #[link(name = "CoreFoundation", kind = "framework")]
    unsafe extern "C" {
        pub fn CFStringCreateWithCString(
            alloc: *const c_void,
            c_str: *const c_char,
            encoding: u32,
        ) -> *const c_void;
        pub fn CFDictionaryGetValue(the_dict: *const c_void, key: *const c_void) -> *const c_void;
        pub fn CFBooleanGetValue(boolean: *const c_void) -> u8;
        pub fn CFRelease(cf: *const c_void);
    }

    pub const K_CF_STRING_ENCODING_UTF8: u32 = 0x0800_0100;
}

/// Checks via macOS CoreGraphics whether the main display is asleep or the user session screen is locked.
#[cfg(target_os = "macos")]
pub fn is_macos_screen_locked_or_asleep() -> bool {
    use cg::*;
    unsafe {
        if CGDisplayIsAsleep(CGMainDisplayID()) != 0 {
            return true;
        }

        let dict = CGSessionCopyCurrentDictionary();
        if dict.is_null() {
            return false;
        }

        let locked_key = CFStringCreateWithCString(
            std::ptr::null(),
            c"CGSSessionScreenIsLocked".as_ptr(),
            K_CF_STRING_ENCODING_UTF8,
        );
        let mut is_locked = false;
        if !locked_key.is_null() {
            let val = CFDictionaryGetValue(dict, locked_key);
            if !val.is_null() && CFBooleanGetValue(val) != 0 {
                is_locked = true;
            }
            CFRelease(locked_key);
        }

        if !is_locked {
            let console_key = CFStringCreateWithCString(
                std::ptr::null(),
                c"kCGSSessionOnConsoleKey".as_ptr(),
                K_CF_STRING_ENCODING_UTF8,
            );
            if !console_key.is_null() {
                let val = CFDictionaryGetValue(dict, console_key);
                if !val.is_null() && CFBooleanGetValue(val) == 0 {
                    is_locked = true;
                }
                CFRelease(console_key);
            }
        }

        CFRelease(dict);
        is_locked
    }
}

#[cfg(not(target_os = "macos"))]
pub fn is_macos_screen_locked_or_asleep() -> bool {
    false
}

pub fn launch_agent_plist_path() -> Result<PathBuf> {
    let home = env::var("HOME").context("HOME environment variable not set")?;
    Ok(PathBuf::from(home)
        .join("Library")
        .join("LaunchAgents")
        .join(format!("{LAUNCH_AGENT_LABEL}.plist")))
}

pub fn format_launch_agent_plist(exe_path: &Path, work_dir: &Path, log_path: &Path) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>{label}</string>
  <key>ProgramArguments</key>
  <array>
    <string>{exe}</string>
  </array>
  <key>WorkingDirectory</key>
  <string>{workdir}</string>
  <key>RunAtLoad</key>
  <true/>
  <key>KeepAlive</key>
  <true/>
  <key>ThrottleInterval</key>
  <integer>2</integer>
  <key>ProcessType</key>
  <string>Interactive</string>
  <key>StandardOutPath</key>
  <string>{log}</string>
  <key>StandardErrorPath</key>
  <string>{log}</string>
</dict>
</plist>
"#,
        label = LAUNCH_AGENT_LABEL,
        exe = exe_path.display(),
        workdir = work_dir.display(),
        log = log_path.display()
    )
}

pub fn install_launch_agent() -> Result<()> {
    let home = env::var("HOME").context("HOME environment variable not set")?;
    let exe_path = env::current_exe().context("Failed to resolve current executable path")?;
    let plist_path = launch_agent_plist_path()?;
    let log_path = PathBuf::from(&home)
        .join("Library")
        .join("Logs")
        .join("qdeck.log");
    let work_dir = exe_path
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.parent())
        .unwrap_or_else(|| Path::new(&home));

    if let Some(parent) = plist_path.parent() {
        fs::create_dir_all(parent)?;
    }
    if let Some(log_parent) = log_path.parent() {
        let _ = fs::create_dir_all(log_parent);
    }

    let plist_content = format_launch_agent_plist(&exe_path, work_dir, &log_path);
    fs::write(&plist_path, plist_content)
        .with_context(|| format!("Failed to write {}", plist_path.display()))?;

    let _ = Command::new("launchctl")
        .args(["unload", "-w"])
        .arg(&plist_path)
        .status();
    let status = Command::new("launchctl")
        .args(["load", "-w"])
        .arg(&plist_path)
        .status()
        .context("Failed to execute launchctl load")?;

    if !status.success() {
        return Err(anyhow!(
            "launchctl load exited with status {:?} for {}",
            status.code(),
            plist_path.display()
        ));
    }

    println!(
        "[QDECK] Installed & started macOS LaunchAgent: {}",
        plist_path.display()
    );
    println!("[QDECK] Binary: {}", exe_path.display());
    println!("[QDECK] Logs:   {}", log_path.display());
    Ok(())
}

pub fn uninstall_launch_agent() -> Result<()> {
    let plist_path = launch_agent_plist_path()?;
    if plist_path.exists() {
        let _ = Command::new("launchctl")
            .args(["unload", "-w"])
            .arg(&plist_path)
            .status();
        fs::remove_file(&plist_path)?;
        println!(
            "[QDECK] Removed macOS LaunchAgent: {}",
            plist_path.display()
        );
    } else {
        println!(
            "[QDECK] LaunchAgent not installed at {}",
            plist_path.display()
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_launch_agent_plist() {
        let plist = format_launch_agent_plist(
            Path::new("/usr/local/bin/qdeck"),
            Path::new("/Users/test"),
            Path::new("/Users/test/Library/Logs/qdeck.log"),
        );
        assert!(plist.contains("<string>local.qdeck</string>"));
        assert!(plist.contains("<string>/usr/local/bin/qdeck</string>"));
        assert!(plist.contains("<key>KeepAlive</key>"));
    }
}
