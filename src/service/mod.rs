use anyhow::{anyhow, Result};
use flexi_logger::{Cleanup, FileSpec, LogSpecification, Logger, LoggerHandle, WriteMode};
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;
use std::ffi::OsString;
use std::time::{Duration, Instant};
use std::sync::{mpsc, Arc};
use std::io::Write;
use tokio::runtime::Runtime;
use tokio::net::windows::named_pipe::{NamedPipeServer, ServerOptions};
use windows_service::service::{ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus, ServiceType};
use windows_service::{
    define_windows_service,
    service_control_handler::{self, ServiceControlHandlerResult, ServiceStatusHandle},
	service_dispatcher,
};

use flexi_logger::{DeferredNow, Record};

use crate::common::{self,
    config::Config,
    message::{ServiceRequest, Request, Response, Serialize, Deserialize},
};

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
        .rotate(
            flexi_logger::Criterion::Size(1024 * 1024), // 1MB
            flexi_logger::Naming::Timestamps,
            Cleanup::KeepLogFiles(3)
        )
        .write_mode(WriteMode::Direct)
        .format_for_files(log_formatter)
        .append()
        .start()
        .map_err(|e| anyhow!("{e}"))
}

fn set_service_status(status_handle: &ServiceStatusHandle, current_state: ServiceState, exit_code: u32) -> Result<(), windows_service::Error> {
    let next_status = ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state,
        controls_accepted: if current_state == ServiceState::Running
            { ServiceControlAccept::STOP
            } else { ServiceControlAccept::empty() },
        exit_code: ServiceExitCode::Win32(exit_code),
        checkpoint: 0,
        wait_hint: if current_state == ServiceState::StartPending
        { Duration::from_secs(10) } else { Duration::default() },
        process_id: None,
    };

    status_handle.set_service_status(next_status)
}

struct ServiceContext {
    logger_handle: LoggerHandle,
    status_handle: ServiceStatusHandle,
    config: Config,
    last_update_succeeded: Arc<Mutex<bool>>,
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

    set_service_status(&status_handle, ServiceState::StartPending, 0).unwrap();

    // also creates the app directory if it does not exist yet
    let config = match config::read() {
        Ok(c) => c,
        Err(_) => {
            set_service_status(&status_handle, ServiceState::Stopped, 1).unwrap();
            return;
        }
    };

	let logger_handle = match logger_init() {
        Err(_) => {
            set_service_status(&status_handle, ServiceState::Stopped, 2).unwrap();
            return;
        }
        Ok(handle) => handle,
    };

    log::debug!("Service is running with the following configuration:\n{config}");

    let context = ServiceContext {
        status_handle,
        logger_handle,
        config,
        last_update_succeeded: Arc::new(Mutex::new(false)),
    };

    if let Err(e) = run_service(context, shutdown_rx) {
        log::error!("Service failed: {:?}", e);
    }
}

async fn handle_message(msg: &Request, context: &mut ServiceContext, update_tx: &mpsc::Sender<Config>) -> Result<Response> {
    log::debug!("Received: {:?}", msg);

    let res = match msg {
        Request::Interval(interval) => {
            if *interval < common::consts::MINIMAL_INTERVAL {
                Err(anyhow!("Got interval of {}, minimal interval is {}", humantime::format_duration(*interval), humantime::format_duration(common::consts::MINIMAL_INTERVAL)))
            } else {
                context.config.service.interval = *interval;
                Ok(Response::Ok)
            }
        }
        Request::AddDomain(domain) => {
            if context.config.service.domain.len() >= common::consts::DOMAIN_LENGTH_LIMIT {
                Err(anyhow!("The number of domain to update is limited to {}", common::consts::DOMAIN_LENGTH_LIMIT))
            } else if context.config.service.domain.insert(domain.clone()) {
                Ok(Response::Ok)
            } else {
                Err(anyhow!("Domain {domain} already exists"))
            }
        }
        Request::RemoveDomain(domain) => {
            if context.config.service.domain.remove(domain) {
                Ok(Response::Ok)
            } else {
                Err(anyhow!("Domain {domain} does not exist"))
            }
        }
        Request::Token(token) => {
            context.config.service.token.replace(token.clone());
            Ok(Response::Ok)
        }
        Request::Ipv6(enable) => {
            if context.config.service.ipv6.is_some_and(|v| v != *enable) {
                context.config.service.ipv6 = Some(*enable);
                context.config.service.clear_ip_addresses = true;
            }
            Ok(Response::Ok)
        }
        Request::ForceUpdate => {
            context.config = config::read()?;
            Ok(Response::Ok)
        }
        Request::DebugLevel(level) => {
            let new_spec = LogSpecification::parse(level)?;
            context.logger_handle.set_new_spec(new_spec);
            log::info!("debug level changed to {level}");
            return Ok(Response::Ok)
        }
        Request::GetConfig => {
            return Ok(Response::Config(context.config.service.clone()));
        }
        Request::GetStatus => {
            let status = context.last_update_succeeded.lock().await;
            return Ok(Response::Status(*status));
        }
    }?;

    context.config.store()?;
    update_tx.send(context.config.clone())?;

    log::debug!("New config:\n{}", context.config);
    Ok(res)
}

async fn send_response(pipe: &mut NamedPipeServer, response: Response) -> Result<()> {
    log::info!("response is {response:?}");
    let encoded = response.serialize()?;
    pipe.write_all(&encoded)
        .await
        .map_err(|e| anyhow!("{e}"))
}

async fn service_listening_loop(mut context: ServiceContext, update_tx: mpsc::Sender<Config>) {
    // force an update when the service has just started
    if let Err(_) = update_tx.send(context.config.clone()) {
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

                let mut buffer = vec![0; std::mem::size_of::<ServiceRequest>()];
                match pipe.read_with_timeout(&mut buffer, Duration::from_secs(common::consts::PIPE_TIMEOUT_IN_SEC)).await {
                    Ok(0) => {
                        log::debug!("Client disconnected");
                    }
                    Ok(_bytes_read) => {
                        let msg = match ServiceRequest::deserialize(&buffer) {
                            Ok(m) => m,
                            Err(e) => {
                                log::error!("Failed to deserialize message, error: {e}");
                                continue;
                            }
                        };
                        if !msg.is_compatiable() {
                            log::error!("Client version incompatible. Client version: {}, Service version: {}", msg.version(), common::strings::VERSION);
                            let res = Response::Err("Client version incompatible".to_string());
                            if let Err(e) = send_response(&mut pipe, res).await {
                                log::error!("Failed to send response: {e}");
                            }
                            continue;
                        }
                        match handle_message(msg.request(), &mut context, &update_tx).await {
                            Err(e) => log::error!("Failed to handle {msg:?}, error: {e}"),
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
                log::error!("Falied to create pipe: {e:?}")
            }
        }
    }
}

async fn update_ip_loop(receiver: mpsc::Receiver<Config>, initial_config: Config, last_update_succeeded: Arc<Mutex<bool>>) {
    let mut config = initial_config;
    let mut last_run = Instant::now();
    let mut already_warned_token = false;
    let mut already_warned_domain = false;

    loop {
        let mut force_update = false;
        if let Ok(c) = receiver.try_recv() {
            config = c;
            force_update = true;
        }

        if config.service.token.is_none() {
            if !already_warned_token {
                already_warned_token = true;
                log::warn!("No token is configured");
            }
            continue;
        }
        already_warned_token = false;

        if config.service.domain.is_empty() {
            if !already_warned_domain {
                already_warned_domain = true;
                log::warn!("No domain is conifgured");
            }
            continue;
        }
        already_warned_domain = false;

        if last_run.elapsed() >= config.service.interval || force_update {
            let mut succeeded = last_update_succeeded.lock().await;
            *succeeded = duckdns::update(&config)
                .await
                .is_ok();
            log::info!("Update {}",
            if *succeeded { "succeeded" } else { "failed" });
            last_run = Instant::now();
            config.service.clear_ip_addresses = false;
        }

        // let other tasks a chance to advance too
        tokio::task::yield_now().await;
    }
}

fn run_service(context: ServiceContext, shutdown_rx: mpsc::Receiver<()>) -> Result<()> {
    let (update_tx, update_rx) = mpsc::channel();
    let status_handle = context.status_handle.clone();

    // Tell the system that the service is running now
    set_service_status(&context.status_handle, ServiceState::Running, 0)?;
    log::info!("Service has started");

    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        let update_ip_handle = tokio::spawn(update_ip_loop(update_rx, context.config.clone(), context.last_update_succeeded.clone()));
        let listening_loop_handle = tokio::spawn(service_listening_loop(context, update_tx));
        let shutdown_handle = tokio::spawn(async move {shutdown_rx.recv().unwrap();});

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
