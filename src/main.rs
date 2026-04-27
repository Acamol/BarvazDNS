mod arg_parser;
mod client;
mod common;
mod dashboard;
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
    !matches!(command, Command::CheckUpdate | Command::Tray(_))
}

fn elevate_self() -> ! {
    let exe = std::env::current_exe().expect("Failed to determine executable path");
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args_str = format!("{} --elevated", args.join(" "));
    let verb = wide("runas");
    let file = wide(&exe.to_string_lossy());
    let params = wide(&args_str);

    let result = unsafe {
        windows_sys::Win32::UI::Shell::ShellExecuteW(
            std::ptr::null_mut(),
            verb.as_ptr(),
            file.as_ptr(),
            params.as_ptr(),
            std::ptr::null(),
            windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL,
        )
    };

    // ShellExecuteW returns an HINSTANCE; values <= 32 indicate an error.
    if (result as isize) <= 32 {
        eprintln!(
            "Failed to request administrator privileges (error code {}).\n\
             If you downloaded this executable from the internet, Windows may be\n\
             blocking it. Right-click the file → Properties → check \"Unblock\" → OK,\n\
             then try again.",
            result as isize
        );
        exit(1);
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

async fn handle_service_command(svc: ServiceCommands) -> Result<()> {
    match svc.command {
        ServiceSubcommands::Install(args) => service_manager::install_service(args)?,
        ServiceSubcommands::Uninstall => service_manager::uninstall_service()?,
        ServiceSubcommands::RunAsService => {
            if let Err(e) = service::service_dispatcher() {
                eprintln!("Service error: {e}");
            }
        }
        ServiceSubcommands::Start(args) => {
            service_manager::start_service(!args.no_tray, !args.no_web)?
        }
        ServiceSubcommands::Stop => service_manager::stop_service()?,
        ServiceSubcommands::Version => service_manager::version().await?,
    }
    Ok(())
}

fn main() {
    let args = Cli::parse();

    if matches!(args.command, Command::Tray(_)) {
        let no_web = match &args.command {
            Command::Tray(tray_args) => tray_args.no_web,
            _ => unreachable!(),
        };
        if let Err(e) = tray::run(!no_web) {
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
        Command::Service(svc) => handle_service_command(svc).await?,
        Command::Interval { interval } => {
            client::set_interval(interval).await?;
            println!("Interval updated.");
        }
        Command::Token { token } => {
            client::set_token(token).await?;
            println!("Token updated.");
        }
        Command::Domain(DomainSubCommands::Add { domain }) => {
            client::add_domain(domain.clone()).await?;
            println!("Domain '{domain}' added.");
        }
        Command::Domain(DomainSubCommands::Remove { domain }) => {
            client::remove_domain(domain.clone()).await?;
            println!("Domain '{domain}' removed.");
        }
        Command::Ipv6(IPv6SubCommands::Enable) => {
            client::enable_ipv6().await?;
            println!("IPv6 enabled.");
        }
        Command::Ipv6(IPv6SubCommands::Disable) => {
            client::disable_ipv6().await?;
            println!("IPv6 disabled.");
        }
        Command::Update => {
            client::force_update().await?;
            println!("Update succeeded.");
        }
        Command::Debug { level } => {
            client::update_debug_level(level.to_string()).await?;
            println!("Debug level set to '{level}'.");
        }
        Command::Config => client::print_configuration().await?,
        Command::Status => client::get_last_status().await?,
        Command::CheckUpdate => client::check_update(args.elevated).await,
        Command::InstallUpdate => client::do_install_update()?,
        Command::ClearLogs => {
            let deleted = client::clear_logs()?;
            match deleted {
                0 => println!("No log files found."),
                1 => println!("Deleted 1 log file."),
                n => println!("Deleted {n} log files."),
            }
        }
        Command::DashboardPort { port } => {
            client::change_dashboard_port(port)?;
        }
        Command::Tray(_) => unreachable!(),
    }

    Ok(())
}
