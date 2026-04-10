use std::io;
use std::ptr;
use std::time::Duration;

use tokio::io::AsyncReadExt;
use tokio::net::windows::named_pipe::{NamedPipeServer, ServerOptions};
use tokio::time::timeout;
use windows_sys::Win32::Foundation::LocalFree;
use windows_sys::Win32::Security::SECURITY_ATTRIBUTES;
use windows_sys::Win32::Security::Authorization::ConvertStringSecurityDescriptorToSecurityDescriptorW;

/// SDDL security descriptor that restricts pipe access to privileged accounts.
///
/// The descriptor defines a Discretionary Access Control List (DACL) with two
/// Access Control Entries (ACEs):
///
/// - `(A;;GA;;;SY)` — **Allow** Generic All (full access) to **Local System**.
///   The service itself runs as SYSTEM, so it needs this to create and manage
///   the pipe.
///
/// - `(A;;GA;;;BA)` — **Allow** Generic All (full access) to **Built-in
///   Administrators**. Only elevated administrator processes (the CLI) can
///   connect to the pipe.
///
/// All other users and groups are implicitly denied access because no other
/// ACEs are present.
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

/// Creates a named pipe server with a security descriptor that only allows
/// SYSTEM and Administrators to connect.
///
/// This prevents unprivileged users from reading or writing to the pipe,
/// protecting sensitive data (such as the DuckDNS token) that flows through it.
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

pub trait NamedPipeServerWithTimeout {
    async fn read_with_timeout(&mut self, buf: &mut [u8], duration: Duration) -> io::Result<usize>;
    async fn connect_with_timeout(&self, duration: Duration) -> io::Result<()>;
}

impl NamedPipeServerWithTimeout for NamedPipeServer {
    async fn read_with_timeout(&mut self, buf: &mut [u8], duration: Duration) -> io::Result<usize> {
        timeout(duration, self.read(buf))
            .await
            .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "Read operation timed out"))?
    }

    async fn connect_with_timeout(&self, duration: Duration) -> io::Result<()> {
        timeout(duration, self.connect())
            .await
            .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "Connect operation timed out"))?
    }
}
