use anyhow::Result;
use tokio::net::windows::named_pipe::ClientOptions;
use tokio::io::AsyncWriteExt;
use clap::Parser;

use crate::common;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[arg(short, long)]
    message: String,
}

async fn send_message() -> Result<()> {
    let cli = Cli::parse();
    // Try to connect to the named pipe
    let mut client = ClientOptions::new()
        .open(common::strings::PIPE_NAME)?;

    // Write message to pipe
    client.write_all(cli.message.as_bytes()).await?;

    println!("Message sent to service.");
    Ok(())
}

