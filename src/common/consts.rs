use std::time::Duration;

pub const DOMAIN_LENGTH_LIMIT: usize = 5;
pub const PIPE_TIMEOUT: Duration = Duration::from_secs(5);
pub const UPDATE_IP_SLEEP_TIME: Duration = Duration::from_secs(1);
pub const MINIMAL_INTERVAL: Duration = Duration::from_secs(5);