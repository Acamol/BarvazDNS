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
//! * **Configuration:** Located in `%ProgramData%\BarvazDNS\config.toml`.
//! * **Logging:** Detailed logs are stored in `%ProgramData%\BarvazDNS\`.
//! * **IPv6 Support:** Option to enable or disable IPv6 updates.
//! * **Open Source:** Feel free to modify, contribute, and distribute.
//!
//! # Getting Started
//!
//! 1.  Download the latest release from the [Releases](https://github.com/acamol/BarvazDNS/releases) page.
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
//! * `BarvazDNS status`: Displays the last successful update time.
//! * `BarvazDNS check-update`: Checks if a newer version is available.
//! * `BarvazDNS service`: Service related commands.
//!     * `BarvazDNS service install`: Installs the service.
//!     * `BarvazDNS service install --no-startup`: Installs the service without start on startup.
//!     * `BarvazDNS service uninstall`: Uninstalls the service.
//!     * `BarvazDNS service start`: Starts the service.
//!     * `BarvazDNS service stop`: Stops the service.
//!     * `BarvazDNS service version`: Retrieves the running service version.
//!
//! # Logging
//!
//! Logs are stored in `%ProgramData%\BarvazDNS\`.
//!
//! # License
//!
//! MIT License.

mod arg_parser;
mod client;
mod common;
mod service;
mod service_manager;
mod tray;

use crate::arg_parser::*;
use anyhow::Result;
use clap::Parser;
use std::process::exit;

fn is_elevated() -> bool {
    unsafe { windows_sys::Win32::UI::Shell::IsUserAnAdmin() != 0 }
}

fn requires_elevation(command: &Command) -> bool {
    !matches!(command, Command::CheckUpdate | Command::Tray)
}

fn elevate_self() -> ! {
    let exe = std::env::current_exe().expect("Failed to determine executable path");
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args_str = format!("{} --elevated", args.join(" "));
    let verb = wide("runas");
    let file = wide(&exe.to_string_lossy());
    let params = wide(&args_str);

    unsafe {
        windows_sys::Win32::UI::Shell::ShellExecuteW(
            std::ptr::null_mut(),
            verb.as_ptr(),
            file.as_ptr(),
            params.as_ptr(),
            std::ptr::null(),
            windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL,
        );
    }

    exit(0);
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn wait_for_keypress() {
    use std::io::Write;
    print!("\nPress any key to continue...");
    let _ = std::io::stdout().flush();
    unsafe {
        use windows_sys::Win32::System::Console::*;
        let h = GetStdHandle(STD_INPUT_HANDLE);
        // Drain any buffered input, then wait for a fresh keypress.
        let _ = FlushConsoleInputBuffer(h);
        let mut mode = 0u32;
        let _ = GetConsoleMode(h, &mut mode);
        // Disable line-input so we don't need Enter.
        let _ = SetConsoleMode(h, mode & !ENABLE_LINE_INPUT & !ENABLE_ECHO_INPUT);
        let mut buf = [0u8; 1];
        let mut read = 0u32;
        let _ = ReadConsoleA(h, buf.as_mut_ptr().cast(), 1, &mut read, std::ptr::null());
        let _ = SetConsoleMode(h, mode);
    }
}

fn main() {
    let args = Cli::parse();

    if matches!(args.command, Command::Tray) {
        if let Err(e) = tray::run() {
            eprintln!("{e}");
            exit(1);
        }
        return;
    }

    if requires_elevation(&args.command) && !is_elevated() {
        elevate_self();
    }

    let elevated_flag = args.elevated;
    let result = tokio_main(args);

    if let Err(ref e) = result {
        eprintln!("Failed to execute: {e}.");
    }

    if elevated_flag {
        wait_for_keypress();
    }

    if result.is_err() {
        exit(1);
    }
}

#[tokio::main]
async fn tokio_main(args: Cli) -> Result<()> {
    match args.command {
        Command::Service(ServiceCommands {
            command: ServiceSubcommands::Install(args),
        }) => service_manager::install_service(args)?,
        Command::Service(ServiceCommands {
            command: ServiceSubcommands::Uninstall,
        }) => service_manager::uninstall_service()?,
        Command::Service(ServiceCommands {
            command: ServiceSubcommands::RunAsService,
        }) => {
            if let Err(e) = service::service_dispatcher() {
                eprintln!("Service error: {e}");
            }
        }
        Command::Service(ServiceCommands {
            command: ServiceSubcommands::Start,
        }) => service_manager::start_service()?,
        Command::Service(ServiceCommands {
            command: ServiceSubcommands::Stop,
        }) => service_manager::stop_service()?,
        Command::Service(ServiceCommands {
            command: ServiceSubcommands::Version,
        }) => service_manager::version().await?,
        Command::Interval { interval } => client::set_interval(interval).await?,
        Command::Token { token } => client::set_token(token).await?,
        Command::Domain(DomainSubCommands::Add { domain }) => client::add_domain(domain).await?,
        Command::Domain(DomainSubCommands::Remove { domain }) => {
            client::remove_domain(domain).await?
        }
        Command::Ipv6(IPv6SubCommands::Enable) => client::enable_ipv6().await?,
        Command::Ipv6(IPv6SubCommands::Disable) => client::disable_ipv6().await?,
        Command::Update => client::force_update().await?,
        Command::Debug { level } => client::update_debug_level(level.to_string()).await?,
        Command::Config => client::print_configuration().await?,
        Command::Status => client::get_last_status().await?,
        Command::CheckUpdate => client::check_update().await,
        Command::ClearLogs => client::clear_logs()?,
        Command::Tray => unreachable!(),
    }

    Ok(())
}
