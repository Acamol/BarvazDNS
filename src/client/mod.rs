use std::time::Duration;

use anyhow::{Result, anyhow};


use crate::common::message::{Request, Response};

pub async fn set_interval(duration: Duration) -> Result<()> {
    let msg = Request::Interval(duration);
    msg.send().await.map(|_| ())
}

pub async fn set_token(token: String) -> Result<()> {
    let msg = Request::Token(token);
    msg.send().await.map(|_| ())
}

pub async fn add_domain(domain: String) -> Result<()> {
    let msg = Request::AddDomain(domain);
    msg.send().await.map(|_| ())
}

pub async fn remove_domain(domain: String) -> Result<()> {
    let msg = Request::RemoveDomain(domain);
    msg.send().await.map(|_| ())
}

pub async fn enable_ipv6() -> Result<()> {
    let msg = Request::Ipv6(true);
    msg.send().await.map(|_| ())
}

pub async fn disable_ipv6() -> Result<()> {
    let msg = Request::Ipv6(false);
    msg.send().await.map(|_| ())
}

pub async fn force_update() -> Result<()> {
    let msg = Request::ForceUpdate;
    msg.send().await.map(|_| ())
}

pub async fn update_debug_level(level: String) -> Result<()> {
    let msg = Request::DebugLevel(level);
    msg.send().await.map(|_| ())
}

pub async fn print_configuration() -> Result<()> {
    let msg = Request::GetConfig;
    match msg.send().await? {
        Response::Config(config) => println!("{config}"),
        Response::Err(e) => return Err(anyhow!("Bad response: {e}")),
        _ => return Err(anyhow!("Failed to send request")),
    }

    Ok(())
}

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
