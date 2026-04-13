use anyhow::{Result, anyhow};
use windows_service::{
    service::{ServiceAccess, ServiceState},
    service::{ServiceErrorControl, ServiceInfo, ServiceStartType, ServiceType},
    service_manager::{ServiceManager, ServiceManagerAccess},
};
use windows_sys::Win32::Foundation::ERROR_SERVICE_DOES_NOT_EXIST;

use std::{ffi::OsString, io::Write};
use std::{
    process::Command,
    thread::sleep,
    time::{Duration, Instant},
};

use crate::{
    arg_parser::InstallArgs,
    common::{
        self,
        config::Config,
        message::{Request, Response},
        strings::{SERVICE_DESCRIPTION, SERVICE_DISPLAY_NAME, SERVICE_NAME, VERSION},
    },
};

enum Answer {
    No,
    Yes,
}

fn yes_no_question(question: &str) -> Result<Answer> {
    print!("{question} [Yes/No] ");
    std::io::stdout().flush()?;

    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;

    match input.trim().to_lowercase().as_str() {
        "yes" | "y" => Ok(Answer::Yes),
        "no" | "n" => Ok(Answer::No),
        _ => Err(anyhow!("Please enter Yes or No")),
    }
}

pub fn service_is_running() -> Result<bool> {
    let manager_access = ServiceManagerAccess::CONNECT;
    let service_manager = ServiceManager::local_computer(None::<&str>, manager_access)?;

    let service_access = ServiceAccess::QUERY_STATUS;
    let service = service_manager.open_service(SERVICE_NAME, service_access)?;

    Ok(matches!(
        service.query_status()?.current_state,
        ServiceState::Running | ServiceState::StartPending
    ))
}

fn service_is_installed() -> Result<bool> {
    let manager_access = ServiceManagerAccess::CONNECT;
    let service_manager = ServiceManager::local_computer(None::<&str>, manager_access)?;

    let service_access = ServiceAccess::QUERY_STATUS;
    Ok(service_manager
        .open_service(SERVICE_NAME, service_access)
        .is_ok())
}

fn spawn_tray() -> Result<()> {
    let exe =
        std::env::current_exe().map_err(|e| anyhow!("Failed to determine executable path: {e}"))?;
    Command::new(exe)
        .arg("tray")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| anyhow!("Failed to spawn tray icon process: {e}"))?;
    Ok(())
}

fn register_tray_startup() -> Result<()> {
    let exe =
        std::env::current_exe().map_err(|e| anyhow!("Failed to determine executable path: {e}"))?;
    let task_run = format!("\"{}\" tray", exe.display());
    Command::new("schtasks")
        .args([
            "/Create",
            "/TN",
            "BarvazDNS Tray",
            "/TR",
            &task_run,
            "/SC",
            "ONLOGON",
            "/RL",
            "HIGHEST",
            "/F",
        ])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map_err(|e| anyhow!("Failed to register tray startup task: {e}"))?;
    Ok(())
}

fn unregister_tray_startup() {
    let _ = Command::new("schtasks")
        .args(["/Delete", "/TN", "BarvazDNS Tray", "/F"])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
}

/// Installs the Windows service.
///
/// This function attempts to install the application as a Windows service.
/// Upon successful installation, it prints a success message to the console.
/// If an error occurs during the installation process, it returns an `Err`
/// containing the error details, and no success message is printed.
///
/// # Returns
///
/// * `Ok(())` if the service was successfully installed.
/// * `Err(e)` where `e` is an error type describing the failure, if the service installation failed.
pub fn install_service(args: InstallArgs) -> Result<()> {
    if service_is_installed()? {
        println!(
            "An existing installation of {SERVICE_DISPLAY_NAME} was detected.\nTo install version {VERSION}, the existing installation must be uninstalled."
        );

        loop {
            match yes_no_question("Proceed with uninstallation and installation?") {
                Ok(Answer::Yes) => {
                    uninstall_service()?;
                    break;
                }
                Ok(Answer::No) => {
                    println!("Installation aborted.");
                    return Ok(());
                }
                Err(e) => println!("{e}"),
            }
        }
    }

    let manager_access = ServiceManagerAccess::CONNECT | ServiceManagerAccess::CREATE_SERVICE;
    let service_manager = ServiceManager::local_computer(None::<&str>, manager_access)?;

    let service_binary_path = ::std::env::current_exe()
        .map_err(|e| anyhow!("Failed to determine executable path: {e}"))?;

    let service_info = ServiceInfo {
        name: OsString::from(SERVICE_NAME),
        display_name: OsString::from(SERVICE_DISPLAY_NAME),
        service_type: ServiceType::OWN_PROCESS,
        start_type: if args.no_startup {
            ServiceStartType::OnDemand
        } else {
            ServiceStartType::AutoStart
        },
        error_control: ServiceErrorControl::Normal,
        executable_path: service_binary_path,
        launch_arguments: vec![OsString::from("service"), OsString::from("run-as-service")],
        dependencies: vec![],
        account_name: None,
        account_password: None,
    };
    let service = service_manager.create_service(&service_info, ServiceAccess::CHANGE_CONFIG)?;
    service.set_description(SERVICE_DESCRIPTION)?;

    if let Err(e) = register_tray_startup() {
        log::warn!("Failed to register tray startup: {e}");
    }

    println!("{} is installed.", SERVICE_DISPLAY_NAME);
    Ok(())
}

/// Uninstalls the Windows service.
///
/// Attempts to stop and delete the Windows service.
/// If the service is running, it tries to stop it. Then, it marks the service for deletion.
/// It polls for up to 5 seconds to confirm the service is uninstalled, printing a success
/// message if successful or a "marked for deletion" message if the timeout is reached.
///
/// # Returns
///
/// * `Ok(())` on successful uninstallation or marking for deletion.
/// * `Err(e)` if an error occurs during the process.
pub fn uninstall_service() -> Result<()> {
    if !service_is_installed()? {
        return Err(anyhow!("{SERVICE_DISPLAY_NAME} is not installed"));
    }

    let manager_access = ServiceManagerAccess::CONNECT;
    let service_manager = ServiceManager::local_computer(None::<&str>, manager_access)?;

    let service_access = ServiceAccess::QUERY_STATUS | ServiceAccess::STOP | ServiceAccess::DELETE;
    let service = service_manager.open_service(SERVICE_NAME, service_access)?;

    // The service will be marked for deletion as long as this function call succeeds.
    // However, it will not be deleted from the database until it is stopped and all open handles to it are closed.
    service.delete()?;

    // Our handle to it is not closed yet. So we can still query it.
    if service.query_status()?.current_state != ServiceState::Stopped {
        // If the service cannot be stopped, it will be deleted when the system restarts.
        service.stop()?;
    }

    unregister_tray_startup();
    // Explicitly close our open handle to the service. This is automatically called when `service` goes out of scope.
    drop(service);

    let start = Instant::now();
    let timeout = common::consts::SERVICE_POLL_TIMEOUT;
    while start.elapsed() < timeout {
        if let Err(windows_service::Error::Winapi(e)) =
            service_manager.open_service(SERVICE_NAME, ServiceAccess::QUERY_STATUS)
            && e.raw_os_error() == Some(ERROR_SERVICE_DOES_NOT_EXIST as i32)
        {
            println!("{SERVICE_DISPLAY_NAME} is uninstalled.");
            prompt_delete_config_directory();
            return Ok(());
        }
        sleep(Duration::from_secs(1));
    }

    println!("{SERVICE_DISPLAY_NAME} is marked for deletion.");
    prompt_delete_config_directory();
    Ok(())
}

fn prompt_delete_config_directory() {
    match Config::get_config_directory_path() {
        Ok(path) if path.is_dir() => loop {
            match yes_no_question("Do you want to delete the configuration directory?") {
                Ok(Answer::Yes) => {
                    if let Err(e) = std::fs::remove_dir_all(&path) {
                        eprintln!("Failed to delete configuration directory: {e}");
                    } else {
                        println!("Configuration directory deleted.");
                    }
                    break;
                }
                Ok(Answer::No) => break,
                Err(e) => println!("{e}"),
            }
        },
        _ => {}
    }
}

/// Starts the Windows service.
///
/// Attempts to start the Windows service.
/// Returns an error if the service is already running.
/// If successful, prints a message indicating the service is running.
///
/// # Returns
///
/// * `Ok(())` if the service started successfully.
/// * `Err(e)` if the service failed to start or was already running.
pub fn start_service() -> Result<()> {
    if !service_is_installed()? {
        loop {
            match yes_no_question(&format!(
                "{SERVICE_DISPLAY_NAME} is not currently installed. Would you like to install it now?"
            )) {
                Ok(Answer::Yes) => {
                    install_service(InstallArgs { no_startup: false })?;
                    break;
                }
                Ok(Answer::No) => {
                    println!("Start command aborted.");
                    return Ok(());
                }
                Err(e) => println!("{e}."),
            }
        }
    }

    if service_is_running()? {
        return Err(anyhow!("{SERVICE_DISPLAY_NAME} is already running"));
    }

    let manager_access = ServiceManagerAccess::CONNECT;
    let service_manager = ServiceManager::local_computer(None::<&str>, manager_access)?;

    let service_access = ServiceAccess::START;
    let service = service_manager.open_service(SERVICE_NAME, service_access)?;

    service.start::<OsString>(&[]).map_err(|e| match e {
        windows_service::Error::Winapi(win_err) => {
            anyhow!("code {}", win_err.raw_os_error().unwrap_or_default())
        }
        _ => anyhow!("{e:#?}"),
    })?;

    if service_is_running()? {
        println!("{SERVICE_DISPLAY_NAME} is running.");
        if let Err(e) = spawn_tray() {
            if let Err(stop_err) = stop_service() {
                return Err(anyhow!(
                    "{e}. Additionally, failed to stop the service: {stop_err}"
                ));
            }
            return Err(e);
        }
        Ok(())
    } else {
        Err(anyhow!("Failed to start {SERVICE_DISPLAY_NAME}"))
    }
}

/// Stops the Windows service.
///
/// Attempts to stop the Windows service.
/// Returns an error if the service is not running.
/// Polls for up to 5 seconds to confirm the service has stopped, printing
/// appropriate messages.
///
/// # Returns
///
/// * `Ok(())` if the service stopped successfully.
/// * `Err(e)` if the service failed to stop or was not running.
pub fn stop_service() -> Result<()> {
    if !service_is_installed()? {
        return Err(anyhow!("{SERVICE_DISPLAY_NAME} is not installed"));
    }

    if !service_is_running()? {
        return Err(anyhow!("{SERVICE_DISPLAY_NAME} is not running"));
    }

    let manager_access = ServiceManagerAccess::CONNECT;
    let service_manager = ServiceManager::local_computer(None::<&str>, manager_access)?;

    let service_access = ServiceAccess::STOP | ServiceAccess::QUERY_STATUS;
    let service = service_manager.open_service(SERVICE_NAME, service_access)?;

    let status = service.stop();

    match status {
        Ok(state)
            if matches!(
                state.current_state,
                ServiceState::StopPending | ServiceState::Running
            ) =>
        {
            println!("{SERVICE_DISPLAY_NAME} is stop pending");

            let start = Instant::now();
            let timeout = common::consts::SERVICE_POLL_TIMEOUT;
            while start.elapsed() < timeout {
                if service.query_status()?.current_state == ServiceState::Stopped {
                    println!("{SERVICE_DISPLAY_NAME} has stopped");
                    return Ok(());
                }
                sleep(Duration::from_secs(1));
            }
            Ok(())
        }
        Ok(state) if state.current_state == ServiceState::Stopped => {
            println!("{SERVICE_DISPLAY_NAME} has stopped");
            Ok(())
        }
        Ok(state) => Err(anyhow!(
            "{SERVICE_DISPLAY_NAME} is in an invalid state {:?}",
            state.current_state
        )),
        Err(windows_service::Error::Winapi(e)) => {
            Err(anyhow!("code {}", e.raw_os_error().unwrap_or_default()))
        }
        Err(_) => Ok(()),
    }
}

pub async fn version() -> Result<()> {
    if !service_is_installed()? {
        return Err(anyhow!("{SERVICE_DISPLAY_NAME} is not installed"));
    }

    if !service_is_running()? {
        return Err(anyhow!("{SERVICE_DISPLAY_NAME} is not running"));
    }

    let req = Request::Version;
    match req.send().await? {
        Response::Version(ver) => {
            println!("Version {ver}");
            Ok(())
        }
        _ => Err(anyhow!("Failed to receive response from the service")),
    }
}
