use std::io;
use std::ptr;

use tokio::net::windows::named_pipe::{NamedPipeServer, ServerOptions};
use windows_sys::Win32::Foundation::LocalFree;
use windows_sys::Win32::Security::SECURITY_ATTRIBUTES;
use windows_sys::Win32::Security::Authorization::ConvertStringSecurityDescriptorToSecurityDescriptorW;

// SDDL string that grants full access to SYSTEM (SY) and Administrators (BA) only.
// D:      - DACL
// (A;;GA;;;SY) - Allow Generic All to Local System
// (A;;GA;;;BA) - Allow Generic All to Built-in Administrators
const ADMIN_ONLY_SDDL: &[u16] = &{
    const S: &str = "D:(A;;GA;;;SY)(A;;GA;;;BA)";
    const LEN: usize = S.len() + 1;
    let mut buf = [0u16; LEN];
    let bytes = S.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        buf[i] = bytes[i] as u16;
        i += 1;
    }
    buf
};

const SDDL_REVISION_1: u32 = 1;

pub fn create_admin_pipe(name: &str) -> io::Result<NamedPipeServer> {
    let mut sd = ptr::null_mut();

    let ok = unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            ADMIN_ONLY_SDDL.as_ptr(),
            SDDL_REVISION_1,
            &mut sd,
            ptr::null_mut(),
        )
    };

    if ok == 0 {
        return Err(io::Error::last_os_error());
    }

    let mut sa = SECURITY_ATTRIBUTES {
        nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: sd,
        bInheritHandle: 0,
    };

    let result = unsafe {
        ServerOptions::new()
            .create_with_security_attributes_raw(name, &mut sa as *mut _ as *mut _)
    };

    unsafe { LocalFree(sd) };

    result
}
