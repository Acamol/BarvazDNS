use std::time::Duration;

use anyhow::{Result, anyhow};
use tokio::net::windows::named_pipe::ClientOptions;
use tokio::io::AsyncWriteExt;

use crate::common::{self, Message};

async fn send_message(msg: Message) -> Result<()> {
    // Try to connect to the named pipe
    let mut client = ClientOptions::new()
        .open(common::strings::PIPE_NAME)
        .map_err(|_| anyhow!("Failed to communicate with the service. Verify it is running."))?;

    let encode = msg.serialize()?;
    client.write_all(&encode).await?;

    Ok(())
}

pub async fn set_interval(duration: Duration) -> Result<()> {

    let msg = Message::Interval(duration);
    send_message(msg).await
}

pub async fn set_token(token: String) -> Result<()> {
    let msg = Message::Token(token);
    send_message(msg).await
}

pub async fn add_domain(domain: String) -> Result<()> {
    let msg = Message::AddDomain(domain);
    send_message(msg).await
}

pub async fn remove_domain(domain: String) -> Result<()> {
    let msg = Message::RemoveDomain(domain);
    send_message(msg).await
}
