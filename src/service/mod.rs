use anyhow::{anyhow, Result};
use flexi_logger::{FileSpec, Logger, WriteMode};
use std::ffi::OsString;
use std::time::Duration;
use std::sync::mpsc;
use tokio::runtime::Runtime;
use tokio::net::windows::named_pipe::ServerOptions;
use windows_service::service::{ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus, ServiceType};
use windows_service::{
    define_windows_service,
    service_control_handler::{self, ServiceControlHandlerResult, ServiceStatusHandle},
	service_dispatcher,
};

use crate::common::{self, config::Config, message::Message};

mod config;
mod named_pipe_extension;
use named_pipe_extension::*;

define_windows_service!(duckdns_service_main, service_main);

pub fn service_dispatcher() -> Result<()> {

    service_dispatcher::start(common::strings::SERVICE_NAME, duckdns_service_main)
		.map_err(|e| anyhow!("Dispatching error: {e:#?}"))
}

use flexi_logger::{DeferredNow, Record};
use std::io::Write;


fn logger_init() -> Result<()> {
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
    .start().map(|_| ()).map_err(|e| anyhow!("{e}"))
}

fn service_main(_args: Vec<OsString>) {
    let (shutdown_tx, shutdown_rx) = mpsc::channel(); // TODO: switch to oneshot

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
        Err(_) => return, // TODO: update status
    };

	if let Err(_) = logger_init() {
        return; // TODO: update status
    }
    
    log::debug!("Service is running with the following configuration:\n{config}");

    if let Err(e) = run_service(status_handle, shutdown_rx, config) {
        log::error!("Service failed: {:?}", e);
    }
}

fn handle_message(msg: &Message, config: &mut Config) -> Result<()> {
    log::debug!("Received: {:?}", msg);

    match msg {
        Message::Interval(interval) => {
            config.service.interval = interval.clone();
            Ok(())
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
    }?;

    log::debug!("New config:\n{config}");
    Ok(())
}

async fn service_listening_loop(mut config: Config) {
    loop {
        match ServerOptions::new().create(common::strings::PIPE_NAME) {
            Ok(mut pipe) => {
                log::debug!("Waiting for a client...");
                if let Err(e) = pipe.connect_with_timeout(Duration::from_secs(5)).await {
                    log::debug!("Pipe connection error: {:?}", e);
                    continue;
                } 
                log::debug!("Client connected");

                let mut buffer = vec![0; std::mem::size_of::<Message>()];
                match pipe.read_with_timeout(&mut buffer, Duration::from_secs(5)).await {
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
                        if let Err(e) = handle_message(&msg, &mut config) {
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

fn run_service(status_handle: ServiceStatusHandle, shutdown_rx: mpsc::Receiver<()>, config: Config) -> Result<()> {
    let next_status = ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::Running,
        controls_accepted: ServiceControlAccept::STOP,
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::from_secs(60),
        process_id: None,
    };

    // Tell the system that the service is running now
    status_handle.set_service_status(next_status)?;
    log::info!("Service has started");

    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        let listening_loop_handle = tokio::spawn(service_listening_loop(config));
        let shutdown_handle = tokio::spawn(async move {shutdown_rx.recv().unwrap();});

        tokio::select! {
            _ = listening_loop_handle => {
                log::error!("listening loop has ended unexpectedly");
            }
            _ = shutdown_handle => {
                log::debug!("shutdown has been initiated");
            }
        }
    });

    let next_status = ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::Stopped,
        // Accept stop events when running
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
