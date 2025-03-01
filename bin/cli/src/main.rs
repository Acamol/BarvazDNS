use anyhow::Result;
use clap::Parser;
use tokio::io::AsyncWriteExt;
use tokio::net::windows::named_pipe::ClientOptions;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[arg(short, long)]
    message: String,
}

const PIPE_NAME: &str = r"\\.\pipe\barvas-dns-service";

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    // Try to connect to the named pipe
    let mut client = ClientOptions::new()
        .open(PIPE_NAME)?;

    // Write message to pipe
    client.write_all(cli.message.as_bytes()).await?;

    println!("Message sent to service.");
    Ok(())
}
