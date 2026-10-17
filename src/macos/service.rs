use anyhow::{Context, Result, anyhow};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub const LAUNCH_AGENT_LABEL: &str = "local.qdeck";

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
