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
                ServiceSubcommands::Install => handle_service_result(service_manager::install_service(), "install"),
                ServiceSubcommands::Uninstall => handle_service_result(service_manager::uninstall_service(), "uninstall"),
                ServiceSubcommands::Start => handle_service_result(service_manager::start_service(), "start"),
                ServiceSubcommands::Stop => handle_service_result(service_manager::stop_service(), "stop"),
            }
        Command::Client(client_args) => {
            match client_args.command {
                ClientSubcommands::SetInterval { interval } => client::set_interval(interval).await?,
                ClientSubcommands::SetToken { token } => client::set_token(token).await?,
                ClientSubcommands::Domain(DomainSubCommands::Add { domain }) => client::add_domain(domain).await?,
                ClientSubcommands::Domain(DomainSubCommands::Remove { domain }) => client::remove_domain(domain).await?,
                ClientSubcommands::Set(_set_args) => todo!(),
                ClientSubcommands::Ipv6(IPv6SubCommands::Enable) => client::enable_ipv6().await?,
                ClientSubcommands::Ipv6(IPv6SubCommands::Disable) => client::disable_ipv6().await?,
                ClientSubcommands::Update => client::force_update().await?,
            }
        }
    }

    Ok(())
}
