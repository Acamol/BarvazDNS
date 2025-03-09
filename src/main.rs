//! BarvazDNS is a Windows application designed to automatically update DuckDNS domains with your public IP address.
//!
//! It functions as both a command-line tool and a Windows service, providing flexibility and control.
//!
//! # Features
//!
//! * **Automatic DuckDNS Updates:** Regularly checks and updates your DuckDNS domains.
//! * **Single Executable:** All functionality, including service management and configuration, is contained within a single executable.
//! * **Command-Line Interface (CLI):** Provides extensive control over the service and configuration.
//! * **Windows Service:** Runs in the background for continuous, automated updates.
//! * **Human-Readable Interval:** Supports intervals in hours, minutes, and days (e.g., `5h`, `30m`, `1d`).
//! * **TOML Configuration:** Uses a TOML configuration file for easy setup and modification.
//! * **User-Specific Configuration:** Configuration file located in `%ProgramData%\BarvazDNS\config.toml`.
//! * **Logging:** Detailed logs are stored in `%ProgramData%\BarvazDNS\`.
//! * **IPv6 Support:** Option to enable or disable IPv6 updates.
//! * **Open Source:** Feel free to modify, contribute, and distribute.
//!
//! # Getting Started
//!
//! 1.  Download the latest release from the [Releases](https://github.com/acamol/BarvasDNS/releases) page.
//! 2.  Extract the downloaded executable.
//! 3.  Configure `config.toml` in `%ProgramData%\BarvazDNS\`.
//! 4.  Install and start the Windows service (recommended):
//!     * `BarvazDNS service install`
//!     * `BarvazDNS service start`
//!
//! # Command-Line Usage
//!
//! * `BarvazDNS`: Displays general help and available commands.
//! * `BarvazDNS domain add "yourdomain"`: Adds a subdomain.
//! * `BarvazDNS domain remove "yourdomain"`: Removes a subdomain.
//! * `BarvazDNS token "your_token"`: Sets the DuckDNS token.
//! * `BarvazDNS interval "5h"`: Sets the update interval.
//! * `BarvazDNS ipv6 enable`: Enables IPv6 updates.
//! * `BarvazDNS ipv6 disable`: Disables IPv6 updates.
//! * `BarvazDNS update`: Forces an immediate update.
//! * `BarvazDNS config`: Displays the current configuration.
//! * `BarvazDNS status`: Displays the last update attempt status.
//! * `BarvazDNS service`: Service related commands.
//!     * `BarvazDNS service install`: Installs the service.
//!     * `BarvazDNS service install --no-startup`: Installs the service without start on startup.
//!     * `BarvazDNS service uninstall`: Uninstalls the service.
//!     * `BarvazDNS service start`: Starts the service.
//!     * `BarvazDNS service stop`: Stops the service.
//!
//! # Logging
//!
//! Logs are stored in `%ProgramData%\BarvazDNS\`.
//!
//! # License
//!
//! MIT License.

mod service;
mod service_manager;
mod client;
mod common;
mod arg_parser;

use anyhow::Result;
use clap::{Parser, CommandFactory};
use std::process::exit;
use crate::arg_parser::*;


fn handle_service_result(result: Result<()>, command: &str) {
    if let Err(e) = result {
        eprintln!("Failed to execute {command}: {e}");
        exit(1);
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // check if we are in a start service context
    let num_args = std::env::args_os().count();
    if num_args == 1 {
        match service::service_dispatcher() {
            Err(s) if s.to_string().contains("code: 1063") => {
                Cli::command().print_help()?;
            }
            _ => {}
        }

        return Ok(());
    }

    let args = Cli::parse();

    match args.command {
        Command::Service(service_args) => 
            match service_args.command {
                ServiceSubcommands::Install(args) => handle_service_result(service_manager::install_service(args), "install"),
                ServiceSubcommands::Uninstall => handle_service_result(service_manager::uninstall_service(), "uninstall"),
                ServiceSubcommands::Start => handle_service_result(service_manager::start_service(), "start"),
                ServiceSubcommands::Stop => handle_service_result(service_manager::stop_service(), "stop"),
            }
        Command::Interval { interval } => client::set_interval(interval).await?,
        Command::Token { token } => client::set_token(token).await?,
        Command::Domain(DomainSubCommands::Add { domain }) => client::add_domain(domain).await?,
        Command::Domain(DomainSubCommands::Remove { domain }) => client::remove_domain(domain).await?,
        Command::Ipv6(IPv6SubCommands::Enable) => client::enable_ipv6().await?,
        Command::Ipv6(IPv6SubCommands::Disable) => client::disable_ipv6().await?,
        Command::Update => client::force_update().await?,
        Command::Debug { level } => client::update_debug_level(level.to_string()).await?,
        Command::Config => client::print_configuration().await?,
        Command::Status => client::get_last_status().await?,
    }

    Ok(())
}
