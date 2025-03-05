use clap::{Parser, Subcommand};
use std::time::Duration;
use humantime::{format_duration, parse_duration};

use crate::common;



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
pub enum DomainSubCommands {
    /// Add a DuckDNS domain to the service.
    Add {
        #[arg(value_parser)]
        domain: String,
    },
    /// Remove a DuckDNS domain from the service.
    Remove {
        #[arg(value_parser)]
        domain: String,
    }
}

#[derive(Subcommand, Debug)]
pub enum IPv6SubCommands {
    /// Enables IPv6 address updates for your DuckDNS domains.
    Enable,
    /// Disables IPv6 address updates for your DuckDNS domains.
    Disable,
}

#[derive(Parser, Debug)]
pub enum ClientSubcommands {
    /// Sets the interval at which the service checks for and updates your public IP address, using human-readable time.
    SetInterval {
        #[arg(value_parser = parse_humantime_duration)]
        interval: Duration,
    },
    /// Sets the authentication token used to update your DuckDNS domains.
    SetToken {
        #[arg(value_parser)]
        token: String,
    },
    /// Adds or remove a DuckDNS domain name.
    #[command(subcommand)]
    Domain(DomainSubCommands),
    /// Sets the token, domain or interval.
    Set(ClientSetArgs),
    /// Enables or disables IPv6 address updates for your DuckDNS domains.
    #[command(subcommand)]
    Ipv6(IPv6SubCommands),
    /// Forces an update based on the configuration file.
    Update,
}

#[derive(Parser, Debug)]
pub struct ClientSetArgs {
    /// Sets the interval at which the service checks for and updates your public IP address, using human-readable time.
    #[arg(short, long, value_parser = parse_humantime_duration)]
    interval: Option<Duration>,
    /// Sets the authentication token used to update your DuckDNS domains.
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

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Service related commands
    Service(ServiceCommands),
    /// Client related commands
    Client(ClientCommands),
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None, before_help = common::strings::LOGO)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

fn parse_humantime_duration(s: &str) -> Result<Duration, String> {
    let duration = parse_duration(s).map_err(|e| e.to_string())?;
    if duration < common::consts::MINIMAL_INTERVAL {
        Err(format!("Duration must be at least {}, got {}", format_duration(common::consts::MINIMAL_INTERVAL), format_duration(duration)))
    } else {
        Ok(duration)
    }
}
