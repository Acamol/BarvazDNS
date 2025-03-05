use std::time::Duration;

use anyhow::{Result, anyhow};
use tokio::{io::AsyncReadExt, net::windows::named_pipe::ClientOptions};
use tokio::io::AsyncWriteExt;

use crate::common::{self, message::{Request, Response, Serialize, Deserialize}};

async fn send_message(msg: Request) -> Result<Response> {
    // Try to connect to the named pipe
    let mut client = ClientOptions::new()
        .open(common::strings::PIPE_NAME)
        .map_err(|_| anyhow!("Failed to communicate with the service. Verify it is running."))?;

    let encode = msg.serialize()?;
    client.write_all(&encode).await?;

    match msg {
        Request::GetConfig => {
            let mut buf = vec![0; std::mem::size_of::<Response>()];
            client.read(&mut buf).await?;
            let res = Response::deserialize(&buf);
            res
        }
        _ => Ok(Response::Nothing),
    }
}

pub async fn set_interval(duration: Duration) -> Result<()> {
    let msg = Request::Interval(duration);
    send_message(msg).await.map(|_| ())
}

pub async fn set_token(token: String) -> Result<()> {
    let msg = Request::Token(token);
    send_message(msg).await.map(|_| ())
}

pub async fn add_domain(domain: String) -> Result<()> {
    let msg = Request::AddDomain(domain);
    send_message(msg).await.map(|_| ())
}

pub async fn remove_domain(domain: String) -> Result<()> {
    let msg = Request::RemoveDomain(domain);
    send_message(msg).await.map(|_| ())
}

pub async fn enable_ipv6() -> Result<()> {
    let msg = Request::Ipv6(true);
    send_message(msg).await.map(|_| ())
}

pub async fn disable_ipv6() -> Result<()> {
    let msg = Request::Ipv6(false);
    send_message(msg).await.map(|_| ())
}

pub async fn force_update() -> Result<()> {
    let msg = Request::ForceUpdate;
    send_message(msg).await.map(|_| ())
}

pub async fn update_debug_level(level: String) -> Result<()> {
    let msg = Request::DebugLevel(level);
    send_message(msg).await.map(|_| ())
}

pub async fn print_configuration() -> Result<()> {
    let msg = Request::GetConfig;
    match send_message(msg).await? {
        Response::Config(config) => println!("{config}"),
        _ => eprintln!("Bad response"),
    }

    Ok(())
}