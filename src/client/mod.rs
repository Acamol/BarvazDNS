use std::time::Duration;

use anyhow::{Result, anyhow};
use chrono::{DateTime, Local};

use crate::common::message::{Request, Response, Token};
use crate::common;


fn expect_ok(response: Response) -> Result<()> {
    match response {
        Response::Ok => Ok(()),
        Response::Err(e) => Err(anyhow!("{e}")),
        other => Err(anyhow!("Unexpected response: {other:?}")),
    }
}

/// Sets the update interval of the service to DuckDNS.
///
/// Sends a request to the service to set its update interval to the specified `duration`.
///
/// # Arguments
///
/// * `duration`: The desired update interval.
///
/// # Returns
///
/// * `Ok(())` if the request was successfully sent.
/// * `Err(e)` if an error occurred while sending the request.
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
/// * `Ok(())` if the request was successfully sent.
/// * `Err(e)` if an error occurred while sending the request.
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
/// * `Ok(())` if the request was successfully sent.
/// * `Err(e)` if an error occurred while sending the request.
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
/// * `Ok(())` if the request was successfully sent.
/// * `Err(e)` if an error occurred while sending the request.
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
/// * `Ok(())` if the request was successfully sent.
/// * `Err(e)` if an error occurred while sending the request.
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
/// * `Ok(())` if the request was successfully sent.
/// * `Err(e)` if an error occurred while sending the request.
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
/// * `Ok(())` if the request was successfully sent.
/// * `Err(e)` if an error occurred while sending the request.
pub async fn force_update() -> Result<()> {
    let msg = Request::ForceUpdate;
    msg.send().await.and_then(|res| {
        expect_ok(res)?;
        println!("Update succeeded.");
        Ok(())
    })
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
/// * `Ok(())` if the request was successfully sent.
/// * `Err(e)` if an error occurred while sending the request.
pub async fn update_debug_level(level: String) -> Result<()> {
    let msg = Request::DebugLevel(level);
    msg.send().await.and_then(expect_ok)
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

/// Prints the status of the last DuckDNS update attempt.
///
/// Sends a request to the service to retrieve the status of the last DuckDNS update
/// attempt and then prints whether the update succeeded or failed to the console.
///
/// # Returns
///
/// * `Ok(())` if the status was successfully retrieved and printed.
/// * `Err(e)` if an error occurred while retrieving or printing the status.
pub async fn get_last_status() -> Result<()> {
    let msg = Request::GetStatus;
    match msg.send().await? {
        Response::Status(Some(last_update_time)) => {
            let datetime: DateTime<Local> = last_update_time.into();
            let formatted_time = datetime.format("%Y-%m-%d %H:%M:%S");
            println!("Last update: {formatted_time}");
        },
        Response::Status(None) => println!("No updates have been performed yet."),
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
pub async fn check_update() {
    match common::version_check::check_for_update() {
        Some(latest) => print_update_notice(&latest),
        None => println!("You are running the latest version ({}).", common::strings::VERSION),
    }

    check_service_version_mismatch().await;
}

/// Queries the running service version and warns if it differs from the CLI version.
async fn check_service_version_mismatch() {
    let cli_version = common::strings::VERSION;

    let service_version = Request::Version.send().await.ok().and_then(|r| match r {
        Response::Version(v) => Some(v),
        _ => None,
    });

    if let Some(sv) = service_version {
        if sv != cli_version {
            eprintln!(
                "Warning: version mismatch — CLI is v{cli_version} \
                 but the running service is v{sv}. \
                 Reinstall the service to ensure both use the same version."
            );
        }
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
