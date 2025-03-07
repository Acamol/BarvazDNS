use clap::{Parser, Subcommand, ValueEnum};
use std::{fmt, time::Duration};
use humantime::{format_duration, parse_duration};

use crate::common;



#[derive(Parser, Debug)]
pub struct ServiceCommands {
    #[command(subcommand)]
    pub command: ServiceSubcommands,
}

#[derive(Subcommand, Debug)]
pub enum ServiceSubcommands {
    /// Installs the service.
    Install,
    /// Uninstalls the service.
    Uninstall,
    /// Starts the service.
    Start,
    /// Stops the service.
    Stop,
}

#[derive(Subcommand, Debug)]
pub enum DomainSubCommands {
    /// Adds a subdomain.
    Add {
        #[arg(value_parser)]
        domain: String,
    },
    /// Removes a subdomain.
    Remove {
        #[arg(value_parser)]
        domain: String,
    }
}

#[derive(Subcommand, Debug)]
pub enum IPv6SubCommands {
    /// Enables IPv6 updates.
    Enable,
    /// Disables IPv6 updates.
    Disable,
}

#[derive(ValueEnum, Clone, Debug)]
pub enum DebugLevelOption {
    Error,
    Warn,
    Info,
    Debug,
}

impl fmt::Display for DebugLevelOption {
 fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
        Self::Error => write!(f, "error"),
        Self::Warn => write!(f, "warn"),
        Self::Info => write!(f, "info"),
        Self::Debug => write!(f, "debug"),
    }
 }
}

#[derive(Parser, Debug)]
pub enum ClientSubcommands {
    /// Adds or removes a DuckDNS domain name from the service.
    #[command(subcommand)]
    Domain(DomainSubCommands),
    /// Sets the DuckDNS token.
    Token {
        #[arg(value_parser)]
        token: String,
    },
    /// Sets the update interval (in human-readable form. For example: 1h 30m, for every 1 hour and 30 minutes).
    Interval {
        #[arg(value_parser = parse_humantime_duration)]
        interval: Duration,
    },
    /// Enables or disables IPv6 updates.
    #[command(subcommand)]
    Ipv6(IPv6SubCommands),
    /// Forces an immediate update (based on the configuration file).
    Update,
    /// Displays the current configuration.
    Config,
    /// Displays the last update attempt status.
    Status,
    #[clap(hide = true)]
    Debug {
        #[arg(value_enum)]
        level: DebugLevelOption,
    },
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
