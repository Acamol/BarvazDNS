use std::time::Duration;

use anyhow::{Result, anyhow};
use chrono::{DateTime, Local};

use crate::common;
use crate::common::message::{Request, Response, Token};

fn expect_ok(response: Response) -> Result<()> {
    match response {
        Response::Ok => Ok(()),
        Response::Err(e) => Err(anyhow!("{e}")),
        other => Err(anyhow!("Unexpected response: {other:?}")),
    }
}

fn set_dashboard_port(port: u16) -> Result<()> {
    let config_path = common::config::Config::get_config_file_path()?;
    let content = std::fs::read_to_string(&config_path)?;
    let mut config: common::config::Config = toml::from_str(&content)?;
    let dashboard = config.dashboard.get_or_insert_with(Default::default);
    dashboard.port = Some(port);
    config.store()?;
    Ok(())
}

fn get_effective_dashboard_port() -> u16 {
    common::config::Config::get_config_file_path()
        .ok()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| toml::from_str::<common::config::Config>(&s).ok())
        .map(|c| c.effective_dashboard_port())
        .unwrap_or(common::consts::WEB_DASHBOARD_PORT)
}

fn reload_dashboard(port: u16) {
    let url = format!("http://127.0.0.1:{port}/api/reload");
    let _ = minreq::post(&url).with_timeout(3).send();
}

/// Sets the DuckDNS update interval on the service.
///
/// Sends a request to the service to set its update interval to the specified `duration`.
///
/// # Arguments
///
/// * `duration`: The desired update interval.
///
/// # Returns
///
/// * `Ok(())` if the service accepted the new interval.
/// * `Err(e)` if the service rejected the request or communication failed.
pub async fn set_interval(duration: Duration) -> Result<()> {
    let msg = Request::Interval(duration);
    msg.send().await.and_then(expect_ok)
}

/// Sets the DuckDNS token on the service.
///
/// Sends a request to the service to set its DuckDNS token to the provided `token`.
///
/// # Arguments
///
/// * `token`: The DuckDNS token to be set.
///
/// # Returns
///
/// * `Ok(())` if the service accepted the new token.
/// * `Err(e)` if the service rejected the request or communication failed.
pub async fn set_token(token: String) -> Result<()> {
    let msg = Request::Token(Token::new(token));
    msg.send().await.and_then(expect_ok)
}

/// Adds a domain to the DuckDNS update list on the service.
///
/// Sends a request to the service to add the specified `domain` to the list of domains
/// that the service will update on DuckDNS.
///
/// # Arguments
///
/// * `domain`: The domain to add to the update list.
///
/// # Returns
///
/// * `Ok(())` if the service added the domain.
/// * `Err(e)` if the service rejected the request or communication failed.
pub async fn add_domain(domain: String) -> Result<()> {
    let msg = Request::AddDomain(domain);
    msg.send().await.and_then(expect_ok)
}

/// Removes a domain from the DuckDNS update list on the service.
///
/// Sends a request to the service to remove the specified `domain` from the list of
/// domains that the service will update on DuckDNS.
///
/// # Arguments
///
/// * `domain`: The domain to remove from the update list.
///
/// # Returns
///
/// * `Ok(())` if the service removed the domain.
/// * `Err(e)` if the service rejected the request or communication failed.
pub async fn remove_domain(domain: String) -> Result<()> {
    let msg = Request::RemoveDomain(domain);
    msg.send().await.and_then(expect_ok)
}

/// Enables IPv6 updates on the service.
///
/// Sends a request to the service to enable IPv6 address updates for DuckDNS.
///
/// # Returns
///
/// * `Ok(())` if the service enabled IPv6 updates.
/// * `Err(e)` if the service rejected the request or communication failed.
pub async fn enable_ipv6() -> Result<()> {
    let msg = Request::Ipv6(true);
    msg.send().await.and_then(expect_ok)
}

/// Disables IPv6 updates on the service.
///
/// Sends a request to the service to disable IPv6 address updates for DuckDNS.
///
/// # Returns
///
/// * `Ok(())` if the service disabled IPv6 updates.
/// * `Err(e)` if the service rejected the request or communication failed.
pub async fn disable_ipv6() -> Result<()> {
    let msg = Request::Ipv6(false);
    msg.send().await.and_then(expect_ok)
}

/// Forces an immediate DuckDNS update on the service.
///
/// Sends a request to the service to initiate an immediate DuckDNS update, regardless
/// of the configured update interval.
///
/// # Returns
///
/// * `Ok(())` if the update succeeded.
/// * `Err(e)` if the update failed or communication failed.
pub async fn force_update() -> Result<()> {
    let msg = Request::ForceUpdate;
    msg.send().await.and_then(expect_ok)
}

/// Updates the service's debug logging level.
///
/// Sends a request to the service to set its debug logging level to the specified `level`.
///
/// # Arguments
///
/// * `level`: The desired debug logging level. Possible values: error, warn, info, debug.
///
/// # Returns
///
/// * `Ok(())` if the service accepted the new level.
/// * `Err(e)` if the service rejected the request or communication failed.
pub async fn update_debug_level(level: String) -> Result<()> {
    let msg = Request::DebugLevel(level);
    msg.send().await.and_then(expect_ok)
}

/// Changes the dashboard port after user confirmation.
///
/// Prompts the user for confirmation, updates the port in the configuration file,
/// and sends a reload request to the running dashboard.
///
/// # Arguments
///
/// * `port`: The new port number for the dashboard.
///
/// # Returns
///
/// * `Ok(())` if the port was changed or the user cancelled.
/// * `Err(e)` if writing the configuration failed.
pub fn change_dashboard_port(port: u16) -> Result<()> {
    let old_port = get_effective_dashboard_port();
    loop {
        match common::prompt::yes_no_question(&format!(
            "Change the dashboard port to {port}? This will reload the dashboard."
        )) {
            Ok(common::prompt::Answer::Yes) => break,
            Ok(common::prompt::Answer::No) => {
                println!("Port change cancelled.");
                return Ok(());
            }
            Err(e) => println!("{e}"),
        }
    }
    set_dashboard_port(port)?;
    reload_dashboard(old_port);
    println!("Dashboard port set to {port}.");
    Ok(())
}

/// Prints the service's current configuration to the console.
///
/// Sends a request to the service to retrieve its current configuration and then
/// prints the configuration to the console.
///
/// # Returns
///
/// * `Ok(())` if the configuration was successfully retrieved and printed.
/// * `Err(e)` if an error occurred while retrieving or printing the configuration.
pub async fn print_configuration() -> Result<()> {
    let msg = Request::GetConfig;
    match msg.send().await? {
        Response::Config(config) => println!("{}", config.to_string_with_token()),
        Response::Err(e) => return Err(anyhow!("Bad response: {e}")),
        _ => return Err(anyhow!("Failed to send request")),
    }

    Ok(())
}

/// Prints the time of the last successful DuckDNS update.
///
/// Sends a request to the service to retrieve the timestamp of the last successful
/// DuckDNS update and prints it to the console.
///
/// # Returns
///
/// * `Ok(())` if the status was successfully retrieved and printed.
/// * `Err(e)` if an error occurred while retrieving or printing the status.
pub async fn get_last_status() -> Result<()> {
    let msg = Request::GetStatus;
    match msg.send().await? {
        Response::Status(status) => {
            if let Some((time, domains)) = status.last_success {
                let datetime: DateTime<Local> = time.into();
                let formatted_time = datetime.format("%Y-%m-%d %H:%M:%S");
                println!("Last successful update: {formatted_time}");
                println!("Updated domains: {}", domains.join(", "));
            } else {
                println!("No successful updates yet.");
            }
        }
        Response::Err(e) => return Err(anyhow!("Bad response: {e}")),
        _ => return Err(anyhow!("Failed to send request")),
    }

    Ok(())
}

/// Checks if a newer version of BarvazDNS is available.
///
/// Queries for the latest released version and prints a message
/// indicating whether an update is available or the current version is up to date.
/// Also queries the running service version and warns if it differs from the CLI.
/// If a newer version is found, prompts the user to install it.
pub async fn check_update(is_elevated: bool) {
    match common::version_check::check_for_update() {
        Some(latest) => {
            print_update_notice(&latest);
            prompt_install_update(is_elevated);
        }
        None => println!(
            "You are running the latest version ({}).",
            common::strings::VERSION
        ),
    }

    check_service_version_mismatch().await;
}

/// Prompts the user to install the update. If accepted and not already elevated,
/// re-launches the process with `install-update` in an elevated context.
fn prompt_install_update(is_elevated: bool) {
    let accepted = loop {
        match common::prompt::yes_no_question("Would you like to install it now?") {
            Ok(common::prompt::Answer::Yes) => break true,
            Ok(common::prompt::Answer::No) => break false,
            Err(e) => println!("{e}"),
        }
    };

    if !accepted {
        return;
    }

    if is_elevated {
        if let Err(e) = do_install_update() {
            eprintln!("Update failed: {e}");
        }
    } else {
        elevate_with_command("install-update");
    }
}

fn elevate_with_command(command: &str) {
    let exe = std::env::current_exe().expect("Failed to determine executable path");
    let args_str = format!("{command} --elevated");
    let verb: Vec<u16> = "runas".encode_utf16().chain(std::iter::once(0)).collect();
    let file: Vec<u16> = exe
        .to_string_lossy()
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    let params: Vec<u16> = args_str.encode_utf16().chain(std::iter::once(0)).collect();

    let result = unsafe {
        windows_sys::Win32::UI::Shell::ShellExecuteW(
            std::ptr::null_mut(),
            verb.as_ptr(),
            file.as_ptr(),
            params.as_ptr(),
            std::ptr::null(),
            windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL,
        )
    };

    if (result as isize) <= 32 {
        eprintln!(
            "Failed to request administrator privileges (error code {}).",
            result as isize
        );
    }
}

/// Downloads and installs the latest release, replacing the current executable.
pub fn do_install_update() -> Result<()> {
    println!("Checking for the latest release...");
    let info = common::version_check::get_update_info()
        .ok_or_else(|| anyhow!("No newer version found or failed to fetch release info."))?;

    println!("Downloading {}...", info.tag);
    let current_exe =
        std::env::current_exe().map_err(|e| anyhow!("Failed to determine executable path: {e}"))?;
    let parent = current_exe
        .parent()
        .ok_or_else(|| anyhow!("Cannot determine executable directory"))?;
    let new_exe = parent.join("BarvazDNS_new.exe");
    let old_exe = current_exe.with_extension("exe.old");

    common::version_check::download_release(&info.download_url, &new_exe)?;

    // Stop the service if it is running
    let was_running = crate::service_manager::service_is_running().unwrap_or(false);
    if was_running {
        println!("Stopping the service...");
        crate::service_manager::stop_service()?;
    }

    // Swap executables
    if old_exe.exists() {
        std::fs::remove_file(&old_exe).map_err(|e| anyhow!("Failed to remove old backup: {e}"))?;
    }
    std::fs::rename(&current_exe, &old_exe)
        .map_err(|e| anyhow!("Failed to rename current executable: {e}"))?;
    std::fs::rename(&new_exe, &current_exe)
        .map_err(|e| anyhow!("Failed to move new executable into place: {e}"))?;

    // Reinstall the service so the SCM references the updated binary
    println!("Reinstalling the service...");
    crate::service_manager::reinstall_service()?;

    if was_running {
        println!("Starting the service...");
        // Don't spawn a new tray — the caller (CLI or dashboard) manages its own tray.
        crate::service_manager::start_service(false, false)?;
    }

    println!("\nBarvazDNS has been updated to {}.", info.tag);
    Ok(())
}

/// Queries the running service version and warns if it differs from the CLI version.
async fn check_service_version_mismatch() {
    let cli_version = common::strings::VERSION;

    let service_version = Request::Version.send().await.ok().and_then(|r| match r {
        Response::Version(v) => Some(v),
        _ => None,
    });

    if let Some(sv) = service_version
        && sv != cli_version
    {
        eprintln!(
            "Warning: version mismatch — CLI is v{cli_version} \
             but the running service is v{sv}. \
             Reinstall the service to ensure both use the same version."
        );
    }
}

/// Prints an update notice for the given version.
fn print_update_notice(latest: &str) {
    eprintln!(
        "\nA new version of BarvazDNS is available: {latest} (current: {}).\n\
         Download it from: https://github.com/acamol/BarvazDNS/releases/latest",
        common::strings::VERSION
    );
}

/// Deletes all log files from the log directory.
pub fn clear_logs() -> Result<usize> {
    let path = common::config::Config::get_config_directory_path()?;

    let mut deleted = 0;
    for entry in std::fs::read_dir(&path)? {
        let entry = entry?;
        let file_name = entry.file_name();
        let name = file_name.to_string_lossy();
        if name.starts_with(common::strings::LOG_FILE_BASENAME) && name.ends_with(".log") {
            std::fs::remove_file(entry.path())?;
            deleted += 1;
        }
    }

    Ok(deleted)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expect_ok_with_ok_response() {
        assert!(expect_ok(Response::Ok).is_ok());
    }

    #[test]
    fn expect_ok_with_err_response() {
        let result = expect_ok(Response::Err("something failed".to_string()));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("something failed"));
    }

    #[test]
    fn expect_ok_with_unexpected_response() {
        let result = expect_ok(Response::Version("1.0.0".to_string()));
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Unexpected response")
        );
    }
}
