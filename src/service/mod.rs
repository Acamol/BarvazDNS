use anyhow::{Result, anyhow};
use flexi_logger::{Cleanup, FileSpec, LogSpecification, Logger, LoggerHandle, WriteMode};
use std::ffi::OsString;
use std::io::Write;
use std::sync::{Arc, mpsc};
use std::time::{Duration, SystemTime};
use tokio::io::AsyncWriteExt;
use tokio::net::windows::named_pipe::NamedPipeServer;
use tokio::runtime::Runtime;
use tokio::sync::Mutex;
use windows_service::service::{
    ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus, ServiceType,
};
use windows_service::{
    define_windows_service,
    service_control_handler::{self, ServiceControlHandlerResult, ServiceStatusHandle},
    service_dispatcher,
};
use windows_sys::Win32::System::SystemInformation::GetTickCount64;

use flexi_logger::{DeferredNow, Record};

use crate::common::strings::VERSION;
use crate::common::{
    self,
    config::Config,
    message::{self, Request, Response, ServiceRequest, UpdateStatus},
};

mod duckdns;
mod named_pipe;
use named_pipe::{NamedPipeServerWithTimeout, create_admin_pipe};

define_windows_service!(duckdns_service_main, service_main);

pub fn service_dispatcher() -> Result<()> {
    service_dispatcher::start(common::strings::SERVICE_NAME, duckdns_service_main)
        .map_err(|e| anyhow!("Dispatching error: {e:#?}"))
}

fn logger_init(log_level: &str) -> Result<LoggerHandle> {
    let log_formatter =
        |w: &mut dyn Write, now: &mut DeferredNow, record: &Record| -> Result<(), std::io::Error> {
            write!(
                w,
                "[{}] {} [{}]: {}",
                now.now().format("%Y-%m-%d %H:%M:%S"),
                record.level(),
                record.module_path().unwrap_or("<unknown>"),
                record.args()
            )
        };

    let path = Config::get_config_directory_path()?;

    if !path.is_dir() {
        return Err(anyhow!(
            "{} is not a directory",
            path.to_str().unwrap_or_default()
        ));
    }

    Logger::try_with_str(log_level)
        .map_err(|e| anyhow!("Invalid log level '{log_level}': {e}"))?
        .log_to_file(
            FileSpec::default()
                .directory(path)
                .basename(common::strings::LOG_FILE_BASENAME)
                .suppress_timestamp(),
        )
        .rotate(
            flexi_logger::Criterion::Size(common::consts::LOG_ROTATION_SIZE),
            flexi_logger::Naming::Timestamps,
            Cleanup::KeepLogFiles(common::consts::LOG_KEEP_FILES),
        )
        .write_mode(WriteMode::Direct)
        .format_for_files(log_formatter)
        .append()
        .start()
        .map_err(|e| anyhow!("{e}"))
}

fn set_service_status(
    status_handle: &ServiceStatusHandle,
    current_state: ServiceState,
    exit_code: u32,
) -> Result<(), windows_service::Error> {
    let next_status = ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state,
        controls_accepted: if current_state == ServiceState::Running {
            ServiceControlAccept::STOP
        } else {
            ServiceControlAccept::empty()
        },
        exit_code: ServiceExitCode::Win32(exit_code),
        checkpoint: 0,
        wait_hint: if current_state == ServiceState::StartPending {
            Duration::from_secs(10)
        } else {
            Duration::default()
        },
        process_id: None,
    };

    status_handle.set_service_status(next_status)
}

struct ServiceContext {
    logger_handle: LoggerHandle,
    status_handle: ServiceStatusHandle,
    config: Config,
    update_status: Arc<Mutex<UpdateStatus>>,
}

fn log_config_warnings(config: &Config) {
    if config.service.token.is_none() {
        log::warn!("No token is configured");
    }
    if config.service.domain.is_empty() {
        log::warn!("No domain is configured");
    }
}

fn ensure_config_directory() -> Result<()> {
    let path = Config::get_config_directory_path()?;
    if !path.is_dir() {
        std::fs::create_dir_all(&path)
            .map_err(|e| anyhow!("Failed to create config directory: {e}"))?;
    }
    Ok(())
}

fn service_main(_args: Vec<OsString>) {
    let (shutdown_tx, shutdown_rx) = mpsc::channel();

    let event_handler = move |control_event| -> ServiceControlHandlerResult {
        match control_event {
            ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
            ServiceControl::Stop => {
                let _ = shutdown_tx.send(());
                ServiceControlHandlerResult::NoError
            }
            _ => ServiceControlHandlerResult::NotImplemented,
        }
    };

    // Register system service event handler
    let status_handle =
        service_control_handler::register(common::strings::SERVICE_NAME, event_handler).unwrap();

    set_service_status(&status_handle, ServiceState::StartPending, 0).unwrap();

    // Ensure config directory exists
    if let Err(e) = ensure_config_directory() {
        eprintln!("{e}");
        set_service_status(&status_handle, ServiceState::Stopped, 3).unwrap();
        return;
    }

    // Initialize logger with default level first so early log messages are captured
    let logger_handle = match logger_init("info") {
        Err(e) => {
            eprintln!("Failed to initialize logger: {e}");
            set_service_status(&status_handle, ServiceState::Stopped, 2).unwrap();
            return;
        }
        Ok(handle) => handle,
    };

    // Read config (may emit log messages)
    let config = match Config::read() {
        Ok(c) => c,
        Err(e) => {
            log::error!("{e}");
            set_service_status(&status_handle, ServiceState::Stopped, 1).unwrap();
            return;
        }
    };

    // Apply the configured log level (env var overrides config)
    let level = std::env::var(common::strings::ENV_VAR_LOG_LEVEL)
        .unwrap_or_else(|_| config.service.log_level.clone());
    if let Ok(spec) = LogSpecification::parse(&level) {
        logger_handle.set_new_spec(spec);
    }

    log::debug!("Service is running with the following configuration:\n{config}");
    log_config_warnings(&config);

    let context = ServiceContext {
        status_handle,
        logger_handle,
        config,
        update_status: Arc::new(Mutex::new(UpdateStatus::default())),
    };

    if let Err(e) = run_service(context, shutdown_rx) {
        log::error!("Service failed: {:?}", e);
    }
}

// Validates that a domain name only contains characters safe for use in a DuckDNS subdomain.
// Prevents URL parameter injection via crafted domain strings.
fn is_valid_domain(domain: &str) -> bool {
    !domain.is_empty()
        && domain.len() <= 63
        && domain
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-')
        && !domain.starts_with('-')
        && !domain.ends_with('-')
}

fn validate_interval(interval: &Duration) -> Result<()> {
    if *interval < common::consts::MINIMAL_INTERVAL {
        Err(anyhow!(
            "Got interval of {}, minimal interval is {}",
            humantime::format_duration(*interval),
            humantime::format_duration(common::consts::MINIMAL_INTERVAL)
        ))
    } else {
        Ok(())
    }
}

fn validate_add_domain(domain: &str, existing: &std::collections::BTreeSet<String>) -> Result<()> {
    if !is_valid_domain(domain) {
        Err(anyhow!("Invalid domain name: {domain}"))
    } else if existing.len() >= common::consts::MAX_DOMAIN_COUNT {
        Err(anyhow!(
            "The number of domains to update is limited to {}",
            common::consts::MAX_DOMAIN_COUNT
        ))
    } else if existing.contains(domain) {
        Err(anyhow!("Domain {domain} already exists"))
    } else {
        Ok(())
    }
}

fn validate_remove_domain(
    domain: &str,
    existing: &std::collections::BTreeSet<String>,
) -> Result<()> {
    if existing.contains(domain) {
        Ok(())
    } else {
        Err(anyhow!("Domain {domain} does not exist"))
    }
}

async fn handle_message(
    msg: &Request,
    context: &mut ServiceContext,
    update_tx: &tokio::sync::mpsc::Sender<Config>,
) -> Result<Response> {
    log::debug!("Received: {:?}", msg);

    let res = match msg {
        Request::Interval(interval) => {
            validate_interval(interval)?;
            context.config.service.interval = *interval;
            Ok(Response::Ok)
        }
        Request::AddDomain(domain) => {
            validate_add_domain(domain, &context.config.service.domain)?;
            context.config.service.domain.insert(domain.clone());
            Ok(Response::Ok)
        }
        Request::RemoveDomain(domain) => {
            validate_remove_domain(domain, &context.config.service.domain)?;
            context.config.service.domain.remove(domain);
            Ok(Response::Ok)
        }
        Request::Token(token) => {
            context.config.service.token.replace(token.clone());
            Ok(Response::Ok)
        }
        Request::Ipv6(enable) => {
            context.config.service.ipv6.replace(*enable);
            context.config.service.clear_ip_addresses = true;
            Ok(Response::Ok)
        }
        Request::ForceUpdate => {
            context.config = Config::read()?;
            let domains: Vec<String> = context.config.service.domain.iter().cloned().collect();
            match duckdns::update(&context.config).await {
                Ok(()) => {
                    let mut status = context.update_status.lock().await;
                    status.last_success = Some((SystemTime::now(), domains));
                    log::info!("Force update succeeded");
                    return Ok(Response::Ok);
                }
                Err(e) => Err(anyhow!("Update failed: {e}")),
            }
        }
        Request::DebugLevel(level) => {
            let new_spec = LogSpecification::parse(level)?;
            context.logger_handle.set_new_spec(new_spec);
            log::info!("debug level changed to {level}");
            return Ok(Response::Ok);
        }
        Request::GetConfig => {
            return Ok(Response::Config(context.config.service.clone()));
        }
        Request::GetStatus => {
            let status = context.update_status.lock().await;
            return Ok(Response::Status(status.clone()));
        }
        Request::Version => {
            return Ok(Response::Version(VERSION.to_string()));
        }
    }?;

    context.config.store()?;
    log_config_warnings(&context.config);
    update_tx
        .send(context.config.clone())
        .await
        .map_err(|e| anyhow!("Failed to notify update loop: {e}"))?;

    log::debug!("New config:\n{}", context.config);
    Ok(res)
}

async fn send_response(pipe: &mut NamedPipeServer, response: Response) -> Result<()> {
    log::debug!("response is {response:?}");
    let encoded = message::encode(&response)?;
    pipe.write_all(&encoded).await?;
    Ok(())
}

async fn force_update_on_service_start(
    update_tx: &tokio::sync::mpsc::Sender<Config>,
    config: &Config,
    max_delay: Duration,
) {
    let ms_since_boot = unsafe { GetTickCount64() };
    let uptime = Duration::from_millis(ms_since_boot);

    if uptime < max_delay {
        let to_sleep = max_delay - uptime;
        log::info!(
            "System just booted (uptime {}, delaying update by {})",
            uptime.as_secs(),
            to_sleep.as_secs()
        );
        tokio::time::sleep(to_sleep).await;
    }

    if let Err(e) = update_tx.send(config.clone()).await {
        log::error!("Failed to request an update: {e}");
    }
}

async fn service_listening_loop(
    mut context: ServiceContext,
    update_tx: tokio::sync::mpsc::Sender<Config>,
) {
    force_update_on_service_start(
        &update_tx,
        &context.config,
        common::consts::MAX_STARTUP_BOOT_DELAY,
    )
    .await;

    loop {
        match create_admin_pipe(common::strings::PIPE_NAME) {
            Ok(mut pipe) => {
                log::debug!("Waiting for a client...");
                if let Err(e) = pipe
                    .connect_with_timeout(common::consts::PIPE_TIMEOUT)
                    .await
                {
                    log::debug!("Pipe connection error: {:?}", e);
                    continue;
                }
                log::debug!("Client connected");

                let mut buffer = vec![0; common::consts::PIPE_BUFFER_SIZE];
                match pipe
                    .read_with_timeout(&mut buffer, common::consts::PIPE_TIMEOUT)
                    .await
                {
                    Ok(0) => {
                        log::debug!("Client disconnected");
                    }
                    Ok(bytes_read) => {
                        let msg: ServiceRequest = match message::decode(&buffer) {
                            Ok(m) => m,
                            Err(e) => {
                                log::error!("Failed to deserialize message, error: {e}");
                                log::debug!(
                                    "read {bytes_read} bytes, request size: {} bytes",
                                    std::mem::size_of::<ServiceRequest>()
                                );
                                continue;
                            }
                        };
                        if !msg.is_compatible() {
                            log::error!(
                                "Client version incompatible. Client version: {}, Service version: {}",
                                msg.version(),
                                common::strings::VERSION
                            );
                            let res = Response::Err("Client version incompatible".to_string());
                            if let Err(e) = send_response(&mut pipe, res).await {
                                log::error!("Failed to send response: {e}");
                            }
                            continue;
                        }
                        match handle_message(msg.request(), &mut context, &update_tx).await {
                            Err(e) => {
                                log::error!("Failed to handle request, error: {e}");
                                if let Err(e) =
                                    send_response(&mut pipe, Response::Err(e.to_string())).await
                                {
                                    log::error!("Failed to send error response: {e}");
                                }
                            }
                            Ok(res) => {
                                if let Err(e) = send_response(&mut pipe, res).await {
                                    log::error!("Failed to send response: {e}");
                                }
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("Read error: {:?}", e);
                    }
                }
            }
            Err(e) => {
                log::error!("Failed to create pipe: {e:?}")
            }
        }
    }
}

async fn update_ip_loop(
    mut receiver: tokio::sync::mpsc::Receiver<Config>,
    initial_config: Config,
    update_status: Arc<Mutex<UpdateStatus>>,
) {
    let mut config = initial_config;
    let mut interval = tokio::time::interval(config.service.interval);
    // The first tick completes immediately, which triggers the initial update.
    // Subsequent ticks follow the configured interval.

    loop {
        tokio::select! {
            Some(c) = receiver.recv() => {
                config = c;
                interval = tokio::time::interval(config.service.interval);
                interval.reset();
            }
            _ = interval.tick() => {},
        };

        let ready = config.service.token.is_some() && !config.service.domain.is_empty();

        if ready {
            let domains: Vec<String> = config.service.domain.iter().cloned().collect();
            match duckdns::update(&config).await {
                Ok(()) => {
                    let mut status = update_status.lock().await;
                    status.last_success = Some((SystemTime::now(), domains));
                    log::info!("Update succeeded");
                }
                Err(e) => log::error!("Update failed: {e}"),
            }

            config.service.clear_ip_addresses = false;
        }
    }
}

fn run_service(context: ServiceContext, shutdown_rx: mpsc::Receiver<()>) -> Result<()> {
    let (update_tx, update_rx) = tokio::sync::mpsc::channel(8);
    let status_handle = context.status_handle;

    // Tell the system that the service is running now
    set_service_status(&context.status_handle, ServiceState::Running, 0)?;
    log::info!("Service has started");

    let rt = Runtime::new().map_err(|e| anyhow!("Failed to create tokio runtime: {e}"))?;
    rt.block_on(async {
        let update_ip_handle = tokio::spawn(update_ip_loop(
            update_rx,
            context.config.clone(),
            context.update_status.clone(),
        ));
        let listening_loop_handle = tokio::spawn(service_listening_loop(context, update_tx));
        let shutdown_handle = tokio::spawn(async move {
            let _ = shutdown_rx.recv();
        });

        tokio::select! {
            _ = listening_loop_handle => {
                log::error!("listening loop has ended unexpectedly");
            }
            _ = shutdown_handle => {
                log::debug!("shutdown has been initiated");
            }
            _ = update_ip_handle => {
                log::error!("Cannot update DuckDNS");
            }
        }
    });

    set_service_status(&status_handle, ServiceState::Stopped, 0)?;
    log::info!("Service has stopped");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn valid_domains() {
        assert!(is_valid_domain("mydomain"));
        assert!(is_valid_domain("test123"));
        assert!(is_valid_domain("my-domain"));
        assert!(is_valid_domain("a"));
        assert!(is_valid_domain("a-b-c"));
    }

    #[test]
    fn valid_domain_max_length() {
        let domain = "a".repeat(63);
        assert!(is_valid_domain(&domain));
    }

    #[test]
    fn empty_domain_is_invalid() {
        assert!(!is_valid_domain(""));
    }

    #[test]
    fn domain_exceeding_max_length_is_invalid() {
        let domain = "a".repeat(64);
        assert!(!is_valid_domain(&domain));
    }

    #[test]
    fn domain_starting_with_hyphen_is_invalid() {
        assert!(!is_valid_domain("-domain"));
    }

    #[test]
    fn domain_ending_with_hyphen_is_invalid() {
        assert!(!is_valid_domain("domain-"));
    }

    #[test]
    fn domain_with_special_chars_is_invalid() {
        assert!(!is_valid_domain("my.domain"));
        assert!(!is_valid_domain("my domain"));
        assert!(!is_valid_domain("my@domain"));
        assert!(!is_valid_domain("my$domain"));
        assert!(!is_valid_domain("dom&ain"));
    }

    #[test]
    fn domain_with_url_injection_is_invalid() {
        assert!(!is_valid_domain("test&token=stolen"));
        assert!(!is_valid_domain("test?token=stolen"));
    }

    #[test]
    fn validate_interval_at_minimum() {
        assert!(validate_interval(&common::consts::MINIMAL_INTERVAL).is_ok());
    }

    #[test]
    fn validate_interval_above_minimum() {
        assert!(validate_interval(&Duration::from_secs(3600)).is_ok());
    }

    #[test]
    fn validate_interval_below_minimum() {
        assert!(validate_interval(&Duration::from_secs(1)).is_err());
    }

    #[test]
    fn validate_add_domain_success() {
        let existing = BTreeSet::new();
        assert!(validate_add_domain("myhost", &existing).is_ok());
    }

    #[test]
    fn validate_add_domain_invalid_name() {
        let existing = BTreeSet::new();
        assert!(validate_add_domain("-bad", &existing).is_err());
    }

    #[test]
    fn validate_add_domain_duplicate() {
        let existing: BTreeSet<String> = ["myhost".to_string()].into();
        assert!(validate_add_domain("myhost", &existing).is_err());
    }

    #[test]
    fn validate_add_domain_at_limit() {
        let existing: BTreeSet<String> = (0..common::consts::MAX_DOMAIN_COUNT)
            .map(|i| format!("host{i}"))
            .collect();
        assert!(validate_add_domain("onemore", &existing).is_err());
    }

    #[test]
    fn validate_remove_domain_exists() {
        let existing: BTreeSet<String> = ["myhost".to_string()].into();
        assert!(validate_remove_domain("myhost", &existing).is_ok());
    }

    #[test]
    fn validate_remove_domain_not_found() {
        let existing = BTreeSet::new();
        assert!(validate_remove_domain("missing", &existing).is_err());
    }
}
