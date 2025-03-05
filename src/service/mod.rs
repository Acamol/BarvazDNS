use anyhow::{anyhow, Result};
use flexi_logger::{FileSpec, LogSpecification, Logger, LoggerHandle, WriteMode};
use std::ffi::OsString;
use std::time::{Duration, Instant};
use std::sync::mpsc;
use std::io::Write;
use tokio::runtime::Runtime;
use tokio::net::windows::named_pipe::ServerOptions;
use windows_service::service::{ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus, ServiceType};
use windows_service::{
    define_windows_service,
    service_control_handler::{self, ServiceControlHandlerResult, ServiceStatusHandle},
	service_dispatcher,
};

use flexi_logger::{DeferredNow, Record};

use crate::common::{self, config::Config, message::Message};

mod duckdns;
mod config;
mod named_pipe_extension;
use named_pipe_extension::NamedPipeServerWithTimeout;

define_windows_service!(duckdns_service_main, service_main);

pub fn service_dispatcher() -> Result<()> {

    service_dispatcher::start(common::strings::SERVICE_NAME, duckdns_service_main)
		.map_err(|e| anyhow!("Dispatching error: {e:#?}"))
}

fn logger_init() -> Result<LoggerHandle> {
    let log_formatter = |w: &mut dyn Write, now: &mut DeferredNow, record: &Record| -> Result<(), std::io::Error> {
        write!(w,
            "[{}] {}: {}",
            now.now().format("%Y-%m-%d %H:%M:%S"),
            record.level(),
            record.args()
        )
    };

    let path = Config::get_config_directory_path()?;

    if !path.is_dir() {
        return Err(anyhow!("{} is not a directory", path.to_str().unwrap_or_default()));
    }

    let level = std::env::var(common::strings::ENV_VAR_LOG_LEVEL).unwrap_or("info".to_string());

    Logger::try_with_str(&level).unwrap()
        .log_to_file(FileSpec::default()
            .directory(path)
            .basename(common::strings::LOG_FILE_BASENAME)
            .suppress_timestamp())
        .write_mode(WriteMode::Direct)
        .format_for_files(log_formatter)
        .append()
        .start()
        .map_err(|e| anyhow!("{e}"))
}

fn service_main(_args: Vec<OsString>) {
    let (shutdown_tx, shutdown_rx) = mpsc::channel();

    let event_handler = move |control_event| -> ServiceControlHandlerResult {
        match control_event {
            ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
            ServiceControl::Stop => {
                shutdown_tx.send(()).unwrap();
                ServiceControlHandlerResult::NoError
            }
            _ => ServiceControlHandlerResult::NotImplemented,
        }
    };

    // Register system service event handler
    let status_handle = service_control_handler::register(common::strings::SERVICE_NAME, event_handler).unwrap();

    let next_status = ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::StartPending,
        controls_accepted: ServiceControlAccept::empty(),
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::from_secs(10),
        process_id: None,
    };

	status_handle.set_service_status(next_status).unwrap();

    // also creates the app directory if it does not exist yet
    let config = match config::read() {
        Ok(c) => c,
        Err(_) => {
            todo!("update status")
        }
    };

	let logger_handle = match logger_init() {
        Err(_) => {
            todo!("update status");
            #[allow(unreachable_code)]
            return;
        }
        Ok(handle) => handle,
    };

    log::debug!("Service is running with the following configuration:\n{config}");

    if let Err(e) = run_service(status_handle, shutdown_rx, config, logger_handle) {
        log::error!("Service failed: {:?}", e);
    }
}

fn handle_message(msg: &Message, config: &mut Config, update_tx: &mpsc::Sender<Config>, logger_handle: &LoggerHandle) -> Result<()> {
    log::debug!("Received: {:?}", msg);

    match msg {
        Message::Interval(interval) => {
            if *interval < common::consts::MINIMAL_INTERVAL {
                Err(anyhow!("Got interval of {}, minimal interval is {}", humantime::format_duration(*interval), humantime::format_duration(common::consts::MINIMAL_INTERVAL)))
            } else {
                config.service.interval = *interval;
                Ok(())
            }
        }
        Message::AddDomain(domain) => {
            if config.service.domain.len() >= common::consts::DOMAIN_LENGTH_LIMIT {
                Err(anyhow!("The number of domain to update is limited to {}", common::consts::DOMAIN_LENGTH_LIMIT))
            } else if config.service.domain.insert(domain.clone()) {
                Ok(())
            } else {
                Err(anyhow!("Domain {domain} already exists"))
            }
        }
        Message::RemoveDomain(domain) => {
            if config.service.domain.remove(domain) {
                Ok(())
            } else {
                Err(anyhow!("Domain {domain} does not exist"))
            }
        }
        Message::Token(token) => {
            config.service.token.replace(token.clone());
            Ok(())
        }
        Message::Ipv6(enable) => {
            if config.service.ipv6.is_some_and(|v| v != *enable) {
                config.service.ipv6 = Some(*enable);
                config.service.clear_ip_addresses = true;
            }
            Ok(())
        }
        Message::ForceUpdate => {
            *config = config::read()?;
            Ok(())
        }
        Message::DebugLevel(level) => {
            let new_spec = LogSpecification::parse(level)?;
            logger_handle.set_new_spec(new_spec);
            log::info!("debug level changed to {level}");
            return Ok(())
        }
    }?;

    config.store()?;
    update_tx.send(config.clone())?;

    log::debug!("New config:\n{config}");
    Ok(())
}

async fn service_listening_loop(mut config: Config, update_tx: mpsc::Sender<Config>, logger_handle: LoggerHandle) {
    // force an update when the service has just started
    if let Err(_) = update_tx.send(config.clone()) {
        log::error!("Failed to request an update");
    }

    loop {
        match ServerOptions::new().create(common::strings::PIPE_NAME) {
            Ok(mut pipe) => {
                log::debug!("Waiting for a client...");
                if let Err(e) = pipe.connect_with_timeout(Duration::from_secs(common::consts::PIPE_TIMEOUT_IN_SEC)).await {
                    log::debug!("Pipe connection error: {:?}", e);
                    continue;
                } 
                log::debug!("Client connected");

                let mut buffer = vec![0; std::mem::size_of::<Message>()];
                match pipe.read_with_timeout(&mut buffer, Duration::from_secs(common::consts::PIPE_TIMEOUT_IN_SEC)).await {
                    Ok(0) => {
                        log::debug!("Client disconnected");
                    }
                    Ok(_bytes_read) => {
                        let msg = match Message::deserialize(&buffer) {
                            Ok(m) => m,
                            Err(e) => {
                                log::error!("Failed to deserialize message, error: {e}");
                                continue;
                            }
                        };
                        if let Err(e) = handle_message(&msg, &mut config, &update_tx, &logger_handle) {
                            log::error!("Failed to handle {msg:?}, error: {e}");
                        }
                    }
                    Err(e) => {
                        log::error!("Read error: {:?}", e);
                    }
                }
            }
            Err(e) => {
                log::info!("Falied to create pipe: {e:?}")
            }
        }
    }
}

async fn update_ip_loop(receiver: mpsc::Receiver<Config>, initial_config: Config) {
    let mut config = initial_config;
    let mut last_run = Instant::now();

    loop {
        let mut force_update = false;
        if let Ok(c) = receiver.try_recv() {
            config = c;
            force_update = true;
        }

        if config.service.token.is_none() {
            log::debug!("No token is configured");
            continue;
        }

        if config.service.domain.is_empty() {
            log::debug!("No domain is conifgured");
            continue;
        }

        if last_run.elapsed() >= config.service.interval || force_update {
            duckdns::update(&config).await;
            last_run = Instant::now();
            config.service.clear_ip_addresses = false;
        }

        // let other tasks a chance to advance too
        tokio::task::yield_now().await;
    }
}

fn run_service(status_handle: ServiceStatusHandle, shutdown_rx: mpsc::Receiver<()>, config: Config, logger_handle: LoggerHandle) -> Result<()> {
    let (update_tx, update_rx) = mpsc::channel();
    let next_status = ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::Running,
        controls_accepted: ServiceControlAccept::STOP,
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None,
    };

    // Tell the system that the service is running now
    status_handle.set_service_status(next_status)?;
    log::info!("Service has started");

    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        let listening_loop_handle = tokio::spawn(service_listening_loop(config.clone(), update_tx, logger_handle));
        let shutdown_handle = tokio::spawn(async move {shutdown_rx.recv().unwrap();});
        let update_ip_handle = tokio::spawn(update_ip_loop(update_rx, config.clone()));

        tokio::select! {
            _ = listening_loop_handle => {
                log::error!("listening loop has ended unexpectedly");
            }
            _ = shutdown_handle => {
                log::debug!("shutdown has been initiated");
            }
            _ = update_ip_handle => {
                log::error!("Cannot update DuckDNS")
            }
        }
    });

    let next_status = ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::Stopped,
        controls_accepted: ServiceControlAccept::empty(),
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None,
    };

    status_handle.set_service_status(next_status)?;
    log::info!("Service has stopped");

    Ok(())
}
