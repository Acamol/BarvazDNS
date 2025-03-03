use std::time::Duration;

use anyhow::{Result, anyhow};
use tokio::net::windows::named_pipe::ClientOptions;
use tokio::io::AsyncWriteExt;

use crate::common::{self, message};


pub async fn set_interval(duration: Duration) -> Result<()> {
    // Try to connect to the named pipe
    let mut client = ClientOptions::new()
        .open(common::strings::PIPE_NAME)
        .map_err(|_| anyhow!("Failed to communicate with the service. Verify it is running."))?;

    let msg = message::new(duration);
    let encode = msg.serialize()?;

    client.write_all(&encode).await?;

    println!("Message sent to service.");
    Ok(())
}
