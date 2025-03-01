use anyhow::Result;
use flexi_logger::{FileSpec, Logger, WriteMode};
use std::ffi::OsString;
use std::time::Duration;
use std::sync::mpsc;
use tokio::runtime::Runtime;
use tokio::net::windows::named_pipe::ServerOptions;
use windows_service::service::{ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus, ServiceType};
use windows_service::{
    define_windows_service,
    service_control_handler::{self, ServiceControlHandlerResult},
    service_dispatcher,
};

mod named_pipe_extension;
use named_pipe_extension::*;

const SERVICE_NAME: &str = "Barvas DNS Service";
const PIPE_NAME: &str = r"\\.\pipe\barvas-dns-service";

fn main() -> windows_service::Result<()> {
    service_dispatcher::start(SERVICE_NAME, duckdns_service_main)
}

define_windows_service!(duckdns_service_main, service_main);

fn service_main(_arguments: Vec<OsString>) {
    // Initialize the logger
    Logger::try_with_str("info").unwrap()
    .log_to_file(FileSpec::default()
        .directory("C:\\Users\\aviad\\Source\\duckdns-updater1")
        .basename("serivce")
        .suppress_timestamp())
    .write_mode(WriteMode::Direct)
    .append()
    .start().unwrap();

    if let Err(e) = run_service() {
        log::error!("Service failed: {:?}", e);
    }
}

async fn service_listening_loop() {
    loop {
        match ServerOptions::new().create(PIPE_NAME) {
            Ok(mut pipe) => {
                log::debug!("Waiting for a client...");
                if let Err(e) = pipe.connect_with_timeout(Duration::from_secs(5)).await {
                    log::debug!("Pipe connection error: {:?}", e);
                    continue;
                } 
                log::debug!("Client connected");

                let mut buffer = vec![0; 1024];
                match pipe.read_with_timeout(&mut buffer, Duration::from_secs(5)).await {
                    Ok(0)  => {
                        log::debug!("Client disconnected");
                    }
                    Ok(bytes_read) => {
                        let message = String::from_utf8_lossy(&buffer[..bytes_read]);
                        log::debug!("Received: {}", message);
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

fn run_service() -> Result<()> {
    let (shutdown_tx, shutdown_rx) = mpsc::channel();

    log::debug!("service has started");

    //let mut stop_sender_opt = Some(shutdown_tx);
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
    let status_handle = service_control_handler::register(SERVICE_NAME, event_handler)?;

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

    Ok(())
}
