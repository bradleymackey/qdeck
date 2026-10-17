pub mod lock;
pub mod service;
pub mod signals;

pub use lock::is_macos_screen_locked_or_asleep;
pub use service::{
    LAUNCH_AGENT_LABEL, format_launch_agent_plist, install_launch_agent, launch_agent_plist_path,
    uninstall_launch_agent,
};
pub use signals::{install_signal_handlers, is_shutdown_requested};
