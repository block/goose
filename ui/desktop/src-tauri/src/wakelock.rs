use std::sync::Mutex;

/// Global wakelock state — platform-specific assertion handle.
pub struct WakelockState {
    pub enabled: Mutex<bool>,
    #[cfg(target_os = "macos")]
    pub assertion_id: Mutex<u32>,
}

impl Default for WakelockState {
    fn default() -> Self {
        Self {
            enabled: Mutex::new(false),
            #[cfg(target_os = "macos")]
            assertion_id: Mutex::new(0),
        }
    }
}

#[cfg(target_os = "macos")]
pub fn set_wakelock_platform(state: &WakelockState, enable: bool) -> Result<bool, String> {
    use std::ffi::CString;

    // IOPMAssertionCreateWithName / IOPMAssertionRelease via CoreFoundation
    extern "C" {
        fn IOPMAssertionCreateWithName(
            assertion_type: core_foundation::string::CFStringRef,
            level: u32,
            name: core_foundation::string::CFStringRef,
            assertion_id: *mut u32,
        ) -> i32;
        fn IOPMAssertionRelease(assertion_id: u32) -> i32;
    }

    const K_IOPM_ASSERTION_LEVEL_ON: u32 = 255;

    let mut enabled = state.enabled.lock().map_err(|e| e.to_string())?;
    let mut assertion_id = state.assertion_id.lock().map_err(|e| e.to_string())?;

    if enable && !*enabled {
        use core_foundation::string::CFString;

        let assertion_type = CFString::new("PreventUserIdleSystemSleep");
        let reason = CFString::new("Goose wakelock active");
        let mut new_id: u32 = 0;

        let result = unsafe {
            IOPMAssertionCreateWithName(
                assertion_type.as_concrete_TypeRef(),
                K_IOPM_ASSERTION_LEVEL_ON,
                reason.as_concrete_TypeRef(),
                &mut new_id,
            )
        };

        if result == 0 {
            *assertion_id = new_id;
            *enabled = true;
            log::info!("Wakelock enabled (assertion_id: {})", new_id);
        } else {
            return Err(format!("IOPMAssertionCreateWithName failed: {}", result));
        }
    } else if !enable && *enabled {
        let result = unsafe { IOPMAssertionRelease(*assertion_id) };
        if result == 0 {
            *enabled = false;
            log::info!("Wakelock disabled");
        } else {
            return Err(format!("IOPMAssertionRelease failed: {}", result));
        }
    }

    Ok(*enabled)
}

#[cfg(target_os = "windows")]
pub fn set_wakelock_platform(state: &WakelockState, enable: bool) -> Result<bool, String> {
    // SetThreadExecutionState
    extern "system" {
        fn SetThreadExecutionState(flags: u32) -> u32;
    }

    const ES_CONTINUOUS: u32 = 0x80000000;
    const ES_SYSTEM_REQUIRED: u32 = 0x00000001;
    const ES_DISPLAY_REQUIRED: u32 = 0x00000002;

    let mut enabled = state.enabled.lock().map_err(|e| e.to_string())?;

    if enable {
        unsafe {
            SetThreadExecutionState(ES_CONTINUOUS | ES_SYSTEM_REQUIRED | ES_DISPLAY_REQUIRED);
        }
        *enabled = true;
        log::info!("Wakelock enabled");
    } else {
        unsafe {
            SetThreadExecutionState(ES_CONTINUOUS);
        }
        *enabled = false;
        log::info!("Wakelock disabled");
    }

    Ok(*enabled)
}

#[cfg(target_os = "linux")]
pub fn set_wakelock_platform(state: &WakelockState, enable: bool) -> Result<bool, String> {
    // On Linux, a full D-Bus implementation would be ideal, but for now just track state
    let mut enabled = state.enabled.lock().map_err(|e| e.to_string())?;
    *enabled = enable;
    log::info!("Wakelock set to: {} (Linux stub)", enable);
    Ok(*enabled)
}
