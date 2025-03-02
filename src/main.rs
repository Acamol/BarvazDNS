mod service;
mod service_manager;
mod cli;
mod common;

use anyhow::Result;
use clap::{Parser, CommandFactory, Subcommand};
use std::process::exit;

#[derive(Subcommand, Debug)]
enum Command {
    /// Service related commands
    Service(ServiceCommands),
    /// Client related commands
    Client(ClientCommands),
}

#[derive(Parser, Debug)]
struct ServiceCommands {
    #[command(subcommand)]
    command: ServiceSubcommands,
}

#[derive(Subcommand, Debug)]
enum ServiceSubcommands {
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
enum ClientSubcommands {
    /// Sets the interval at which the service checks for and updates your public IP address, using human-readable time.
    SetInterval {
        #[arg(value_parser)]
        interval: String,
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

fn default_config_file() -> String {
    std::env::var("USERPROFILE").unwrap()
}

#[derive(Parser, Debug)]
struct ClientSetArgs {
    /// Sets the interval at which the service checks for and updates your public IP address, using human-readable time.
    #[arg(short, long)]
    interval: Option<String>,
    /// Sets the authentication token used to update your DuckDNS domain.
    #[arg(short, long)]
    token: Option<String>,
    /// Sets the DuckDNS domain name that will be updated.
    #[arg(short, long)]
    domain: Option<String>,
}

#[derive(Parser, Debug)]
struct ClientCommands {
    #[command(subcommand)]
    command: ClientSubcommands,
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

fn handle_service_result(result: Result<()>, command: &str) {
    if let Err(e) = result {
        eprintln!("Failed to execute {command}: {e}");
        exit(1);
    }
}

fn main() -> Result<()> {
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
            ServiceSubcommands::Install => handle_service_result(service_manager::install_service(), "install"),
            ServiceSubcommands::Uninstall => handle_service_result(service_manager::uninstall_service(), "uninstall"),
            ServiceSubcommands::Start => handle_service_result(service_manager::start_service(), "start"),
            ServiceSubcommands::Stop => handle_service_result(service_manager::stop_service(), "stop"),
        }
        _ => unimplemented!(),
    }

    Ok(())
}
