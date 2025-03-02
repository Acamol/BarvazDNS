use anyhow::{Result, anyhow};
use windows_service::{
    service::{ServiceAccess, ServiceState},
    service::{ServiceErrorControl, ServiceInfo, ServiceStartType, ServiceType},
    service_manager::{ServiceManager, ServiceManagerAccess},
};
use windows_sys::Win32::Foundation::ERROR_SERVICE_DOES_NOT_EXIST;
use std::ffi::OsString;
use std::{
    thread::sleep,
    time::{Duration, Instant},
};

use crate::common::{self, strings::SERVICE_NAME};

pub fn install_service() -> Result<()> {
    let manager_access = ServiceManagerAccess::CONNECT | ServiceManagerAccess::CREATE_SERVICE;
    let service_manager = ServiceManager::local_computer(None::<&str>, manager_access)?;

    let service_binary_path = ::std::env::current_exe().unwrap();

    let service_info = ServiceInfo {
        name: OsString::from(SERVICE_NAME),
        display_name: OsString::from(common::strings::SERVICE_DISPLAY_NAME),
        service_type: ServiceType::OWN_PROCESS,
        start_type: ServiceStartType::OnDemand, // TODO: later should be changed
        error_control: ServiceErrorControl::Normal,
        executable_path: service_binary_path,
        launch_arguments: vec![/*OsString::from("run_service")*/],
        dependencies: vec![],
        account_name: None, // run as System
        account_password: None,
    };
    let service = service_manager.create_service(&service_info, ServiceAccess::CHANGE_CONFIG)?;
    service.set_description(common::strings::SERVICE_DESCRIPTION)?;
    Ok(())
}

pub fn uninstall_service() -> Result<()> {
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
    // Explicitly close our open handle to the service. This is automatically called when `service` goes out of scope.
    drop(service);

    // Win32 API does not give us a way to wait for service deletion.
    // To check if the service is deleted from the database, we have to poll it ourselves.
    let start = Instant::now();
    let timeout = Duration::from_secs(5);
    while start.elapsed() < timeout {
        if let Err(windows_service::Error::Winapi(e)) =
            service_manager.open_service(SERVICE_NAME, ServiceAccess::QUERY_STATUS)
        {
            if e.raw_os_error() == Some(ERROR_SERVICE_DOES_NOT_EXIST as i32) {
                println!("{SERVICE_NAME} is deleted.");
                return Ok(());
            }
        }
        sleep(Duration::from_secs(1));
    }
    println!("{SERVICE_NAME} is marked for deletion.");

    Ok(())
}

pub fn start_service() -> Result<()> {
    let manager_access = ServiceManagerAccess::CONNECT;
    let service_manager = ServiceManager::local_computer(None::<&str>, manager_access)?;

    let service_access = ServiceAccess::START;
    let service = service_manager.open_service(SERVICE_NAME, service_access)?;

    let log_path = if let Some(path) = std::env::var_os("BARVAZ_LOG_FILE") {
        path
    } else {
        return Err(anyhow::anyhow!("BARVAZ_LOG_FILE is not configured"));
    };

    service.start::<OsString>(&[log_path]).map_err(|e| {
        match e {
            windows_service::Error::Winapi(win_err) => {
                anyhow!("code {}", win_err.raw_os_error().unwrap_or_default())
            }
            _ => anyhow!("{e:#?}")
        }
    })
}

pub fn stop_service() -> Result<()> {
    let manager_access = ServiceManagerAccess::CONNECT;
    let service_manager = ServiceManager::local_computer(None::<&str>, manager_access)?;

    let service_access = ServiceAccess::STOP | ServiceAccess::QUERY_STATUS;
    let service = service_manager.open_service(SERVICE_NAME, service_access)?;

    let status = service.stop();

    return match status {
        Ok(state) if state.current_state != ServiceState::Stopped && state.current_state != ServiceState::StopPending => {
            eprintln!("Cannot stop the service. Current state is {:?}", state.current_state);
            Ok(())
        }
        Ok(state) if state.current_state == ServiceState::StopPending => {
            println!("{SERVICE_NAME} is stop pending"); 

            let start = Instant::now();
            let timeout = Duration::from_secs(5);
            while start.elapsed() < timeout {
                if service.query_status()?.current_state == ServiceState::Stopped {
                    println!("{SERVICE_NAME} is stopped");
                    return Ok(())
                }
                sleep(Duration::from_secs(1));
            }
            Ok(())
        }
        Ok(_) => Ok(()),
        Err(windows_service::Error::Winapi(e)) => Err(anyhow!("code {}", e.raw_os_error().unwrap_or_default())),
        Err(_) => Ok(()),
    }
}