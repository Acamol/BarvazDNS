use anyhow::{anyhow, Result};
use flexi_logger::{FileSpec, Logger, WriteMode};
use std::ffi::OsString;
use std::path::Path;
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

use crate::common::{self, message};

mod named_pipe_extension;
use named_pipe_extension::*;

define_windows_service!(duckdns_service_main, service_main);

pub fn service_dispatcher() -> Result<()> {

    service_dispatcher::start(common::strings::SERVICE_NAME, duckdns_service_main)
		.map_err(|e| anyhow!("Dispatching error: {e:#?}"))
}

use flexi_logger::{DeferredNow, Record};
use std::io::Write;


fn logger_init(path: &Path) -> Result<()> {
    let log_formatter = |w: &mut dyn Write, now: &mut DeferredNow, record: &Record| -> Result<(), std::io::Error> {
        write!(w,
            "[{}] {}: {}",
            now.now().format("%Y-%m-%d %H:%M:%S"),
            record.level(),
            record.args()
        )
    };

    if !path.is_dir() {
        return Err(anyhow!("{} is not a directory", path.to_str().unwrap_or_default()));
    }

    Logger::try_with_str("debug").unwrap()
    .log_to_file(FileSpec::default()
        .directory(path)
        .basename("barvaz")
        .suppress_timestamp())
    .write_mode(WriteMode::Direct)
    .format_for_files(log_formatter)
    .append()
    .start().map(|_| ()).map_err(|e| anyhow!("{e}"))
}

fn service_main(args: Vec<OsString>) {
    if args.iter().count() < 2 {
        // missing logger path
        return;
    }

	if let Err(_) = logger_init(Path::new(&args[1])) {
        return;
    }

    log::info!("Service has started.");

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
        wait_hint: Duration::from_secs(30),
        process_id: None,
    };

	status_handle.set_service_status(next_status).unwrap();

    if let Err(e) = run_service(status_handle, shutdown_rx) {
        log::error!("Service failed: {:?}", e);
    }
}

async fn service_listening_loop() {
    loop {
        match ServerOptions::new().create(common::strings::PIPE_NAME) {
            Ok(mut pipe) => {
                log::debug!("Waiting for a client...");
                if let Err(e) = pipe.connect_with_timeout(Duration::from_secs(5)).await {
                    log::debug!("Pipe connection error: {:?}", e);
                    continue;
                } 
                log::debug!("Client connected");

                let mut buffer = vec![0; std::mem::size_of::<message>()];
                match pipe.read_with_timeout(&mut buffer, Duration::from_secs(5)).await {
                    Ok(0) => {
                        log::debug!("Client disconnected");
                    }
                    Ok(_bytes_read) => {
                        let message = message::deserialize(&buffer).unwrap();
                        log::debug!("Received: {:?}", message);
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

fn run_service(status_handle: ServiceStatusHandle, shutdown_rx: mpsc::Receiver<()>) -> Result<()> {
    log::debug!("service has started");

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

    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        let listening_loop_handle = tokio::spawn(service_listening_loop());
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
