use std::time::Duration;

use anyhow::{Result, anyhow};

use crate::common::message::{Request, Response};


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
    msg.send().await.map(|_| ())
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
    let msg = Request::Token(token);
    msg.send().await.map(|_| ())
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
    msg.send().await.map(|_| ())
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
    msg.send().await.map(|_| ())
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
    msg.send().await.map(|_| ())
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
    msg.send().await.map(|_| ())
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
    msg.send().await.map(|_| ())
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
    msg.send().await.map(|_| ())
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
        Response::Config(config) => println!("{config}"),
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
        Response::Status(status) =>
            println!("Last update {}.",
                if status { "succeeded" } else { "failed" }),
        Response::Err(e) => return Err(anyhow!("Bad response: {e}")),
        _ => return Err(anyhow!("Failed to send request")),
    }

    Ok(())
}
