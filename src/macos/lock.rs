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
