use clap::{Parser, Subcommand};
use std::time::Duration;
use humantime::parse_duration;

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Service related commands
    Service(ServiceCommands),
    /// Client related commands
    Client(ClientCommands),
}

#[derive(Parser, Debug)]
pub struct ServiceCommands {
    #[command(subcommand)]
    pub command: ServiceSubcommands,
}

#[derive(Subcommand, Debug)]
pub enum ServiceSubcommands {
    /// Install the service.
    Install,
    /// Uninstall the service.
    Uninstall,
    /// Start the service.
    Start,
    /// Stop the service.
    Stop,
}

#[derive(Subcommand, Debug)]
pub enum ClientSubcommands {
    /// Sets the interval at which the service checks for and updates your public IP address, using human-readable time.
    SetInterval {
        #[arg(value_parser = parse_humantime_duration)]
        interval: Duration,
    },
    /// Sets the authentication token used to update your DuckDNS domain.
    SetToken {
        #[arg(value_parser)]
        token: String,
    },
    /// Adds a DuckDNS domain name that will be updated.
    AddDomain {
        #[arg(value_parser)]
        domain: String,
    },
    RemoveDomain {
        #[arg(value_parser)]
        domain: String,
    },
    /// Sets the token, domain or interval.
    Set(ClientSetArgs),
    /// Sets configuration file path for persistent configuration (Defaults to $HOME/barvaz.toml).
    SetConfigFile {
        #[arg(value_parser, default_value_t = default_config_file())]
        path: String,
    },
}

#[derive(Parser, Debug)]
pub struct ClientSetArgs {
    /// Sets the interval at which the service checks for and updates your public IP address, using human-readable time.
    #[arg(short, long, value_parser = parse_humantime_duration)]
    interval: Option<Duration>,
    /// Sets the authentication token used to update your DuckDNS domain.
    #[arg(short, long)]
    token: Option<String>,
    /// Sets the DuckDNS domain name that will be updated.
    #[arg(short, long)]
    domain: Option<String>,
}

#[derive(Parser, Debug)]
pub struct ClientCommands {
    #[command(subcommand)]
    pub command: ClientSubcommands,
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

fn default_config_file() -> String {
    std::env::var("USERPROFILE").unwrap()
}

fn parse_humantime_duration(s: &str) -> Result<Duration, String> {
    parse_duration(s).map_err(|e| e.to_string())
}
