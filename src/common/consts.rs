use std::time::Duration;

pub const MAX_DOMAIN_COUNT: usize = 5;
pub const PIPE_BUFFER_SIZE: usize = 4096;
pub const PIPE_TIMEOUT: Duration = Duration::from_secs(5);
pub const MINIMAL_INTERVAL: Duration = Duration::from_secs(5);
pub const MAX_STARTUP_BOOT_DELAY: Duration = Duration::from_secs(30);
pub const LOG_ROTATION_SIZE: u64 = 5 * 1024 * 1024; // 5MB
pub const LOG_KEEP_FILES: usize = 5;
pub const SERVICE_POLL_TIMEOUT: Duration = Duration::from_secs(5);
pub const LATEST_RELEASE_URL: &str =
    "https://api.github.com/repos/acamol/BarvazDNS/releases/latest";
