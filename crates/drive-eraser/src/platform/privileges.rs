//! Platform-specific administrative privilege detection and verification.
//!
//! Real hardware sanitization requires raw physical device access, volume extent locking,
//! and SCSI/ATA/NVMe pass-through IOCTL dispatch. These operations fail closed if the process
//! does not possess elevated administrator (Windows) or root (Linux) privileges.

use crate::models::DriveEraseFailureReason;

#[cfg(target_os = "windows")]
pub fn is_elevated_admin() -> bool {
    use std::ffi::c_void;
    use std::mem;
    use std::ptr;

    extern "system" {
        fn GetCurrentProcess() -> *mut c_void;
        fn OpenProcessToken(
            ProcessHandle: *mut c_void,
            DesiredAccess: u32,
            TokenHandle: *mut *mut c_void,
        ) -> i32;
        fn GetTokenInformation(
            TokenHandle: *mut c_void,
            TokenInformationClass: u32,
            TokenInformation: *mut c_void,
            TokenInformationLength: u32,
            ReturnLength: *mut u32,
        ) -> i32;
        fn CloseHandle(hObject: *mut c_void) -> i32;
    }

    const TOKEN_QUERY: u32 = 0x0008;
    const TOKEN_ELEVATION: u32 = 20;

    #[repr(C)]
    struct TokenElevation {
        token_is_elevated: u32,
    }

    unsafe {
        let mut token: *mut c_void = ptr::null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
            return false;
        }

        let mut elevation: TokenElevation = mem::zeroed();
        let mut ret_len: u32 = 0;
        let ok = GetTokenInformation(
            token,
            TOKEN_ELEVATION,
            &mut elevation as *mut _ as *mut c_void,
            mem::size_of::<TokenElevation>() as u32,
            &mut ret_len,
        );
        CloseHandle(token);

        ok != 0 && elevation.token_is_elevated != 0
    }
}

#[cfg(not(target_os = "windows"))]
pub fn is_elevated_admin() -> bool {
    unsafe {
        extern "C" {
            fn geteuid() -> u32;
        }
        geteuid() == 0
    }
}

/// Verifies that the current execution process possesses administrative / root privileges.
/// Fails closed with `PreExecutionCheckFailed` if not elevated.
pub fn verify_administrative_privileges() -> Result<(), DriveEraseFailureReason> {
    if !is_elevated_admin() {
        return Err(DriveEraseFailureReason::PreExecutionCheckFailed(
            "Administrative privileges required: Opening physical storage device handles for exclusive hardware access requires running with elevated administrator or root privileges. Safety restrictions cannot be bypassed.".to_string(),
        ));
    }
    Ok(())
}
