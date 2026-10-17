use std::sync::atomic::{AtomicBool, Ordering};

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
