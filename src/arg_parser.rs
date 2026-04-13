use clap::{Args, Parser, Subcommand, ValueEnum};
use humantime::{format_duration, parse_duration};
use std::{fmt, time::Duration};

use crate::common;

#[derive(Parser, Debug)]
pub struct ServiceCommands {
    #[command(subcommand)]
    pub command: ServiceSubcommands,
}

#[derive(Subcommand, Debug)]
pub enum ServiceSubcommands {
    /// Installs the service.
    Install(InstallArgs),
    /// Uninstalls the service.
    Uninstall,
    /// Starts the service.
    Start,
    /// Stops the service.
    Stop,
    /// Retrieves the service version.
    Version,
    /// Internal entry point used by the Windows Service Control Manager (SCM).
    /// This is not intended to be invoked directly by users.
    /// It is automatically passed as a launch argument when the service is installed,
    /// so that the SCM starts the process with `BarvazDNS.exe service run-as-service`.
    #[clap(hide = true)]
    RunAsService,
}

#[derive(Args, Debug)]
pub struct InstallArgs {
    /// Disables startup on boot (enabled by default)
    #[arg(long, action = clap::ArgAction::SetTrue)]
    pub no_startup: bool,
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
    },
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

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Service related commands
    Service(ServiceCommands),
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
    /// Displays the time of the last successful update.
    Status,
    /// Checks if a newer version is available.
    CheckUpdate,
    /// Deletes all log files.
    ClearLogs,
    #[clap(hide = true)]
    Debug {
        #[arg(value_enum)]
        level: DebugLevelOption,
    },
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
        Err(format!(
            "Duration must be at least {}, got {}",
            format_duration(common::consts::MINIMAL_INTERVAL),
            format_duration(duration)
        ))
    } else {
        Ok(duration)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_durations() {
        assert_eq!(
            parse_humantime_duration("10s").unwrap(),
            Duration::from_secs(10)
        );
        assert_eq!(
            parse_humantime_duration("1m").unwrap(),
            Duration::from_secs(60)
        );
        assert_eq!(
            parse_humantime_duration("1h 30m").unwrap(),
            Duration::from_secs(5400)
        );
        assert_eq!(
            parse_humantime_duration("2h").unwrap(),
            Duration::from_secs(7200)
        );
    }

    #[test]
    fn parse_exact_minimum() {
        assert_eq!(
            parse_humantime_duration("5s").unwrap(),
            common::consts::MINIMAL_INTERVAL
        );
    }

    #[test]
    fn parse_below_minimum_is_rejected() {
        assert!(parse_humantime_duration("4s").is_err());
        assert!(parse_humantime_duration("1s").is_err());
    }

    #[test]
    fn parse_invalid_format_is_rejected() {
        assert!(parse_humantime_duration("").is_err());
        assert!(parse_humantime_duration("abc").is_err());
        assert!(parse_humantime_duration("-5s").is_err());
    }
}
