# BarvazDNS

BarvazDNS is a Windows application designed to automatically update your DuckDNS domain(s) with your current public IP address, ensuring your domain remains accessible even with a dynamic IP. It functions as both a command-line tool and a Windows service, providing flexibility and control.

## Features

* **Automatic DuckDNS Updates:** Regularly checks and updates your DuckDNS domains.
* **Single Executable:** All functionality, including service management and configuration, is contained within a single executable.
* **Command-Line Interface (CLI):** Provides extensive control over the service and configuration.
* **Windows Service:** Runs in the background for continuous, automated updates.
* **System Tray Icon:** Displays a tray icon while the service is running for at-a-glance status.
* **Human-Readable Interval:** Supports intervals in hours, minutes, and days (e.g., `5h`, `30m`, `1d`).
* **TOML Configuration:** Uses a TOML configuration file (`%ProgramData%\BarvazDNS\config.toml`) for easy setup and modification.
* **Logging:** Logs are stored in `%ProgramData%\BarvazDNS\`.
* **IPv6 Support:** Option to enable or disable IPv6 updates.
* **Open Source:** Feel free to modify, contribute, and distribute.

## Getting Started

### Prerequisites

* Windows operating system.
* Administrator privileges are required to install and manage the Windows service.

### Installation

**Option 1: Pre-built Executable**

1.  **Download:** Download the latest release from the [Releases](https://github.com/Acamol/BarvazDNS/releases/) page.
    > **Note:** When running from a non-elevated prompt, Windows may block the app from requesting administrator privileges if the executable was downloaded from the internet. If this happens, right-click the file → Properties → check **Unblock** → OK. This is standard Windows behavior for unsigned applications. Alternatively, you can run the app from an elevated prompt directly.
2.  **Configuration:**
    * The configuration file `config.toml` is automatically created in `%ProgramData%\BarvazDNS\` on the first run.
    * You can also manually create or modify the `config.toml` file.
    * Example `config.toml`:

    ```toml
    [service]
    token = "your-duckdns-token"
    domain = ["yoursubdomain", "anothersubdomain"]
    interval = "5h"
    ipv6 = false
    log_level = "info"
    ```

3.  **Windows Service Installation:**
    * **Open a command prompt or PowerShell as administrator.**
    * Navigate to the directory containing `BarvazDNS`.
    * Run `BarvazDNS service install` to install the service.
    * Run `BarvazDNS service start` to start the service.
    * Run `BarvazDNS service stop` to stop the service.
    * Run `BarvazDNS service uninstall` to uninstall the service.

**Option 2: Building from Source**

Requires the Rust toolchain and Windows SDK installed and added to your system's PATH.

1.  **Clone the Repository:** `git clone https://github.com/acamol/BarvazDNS.git`
2.  **Navigate to the Directory:** `cd BarvazDNS`
3.  **Build the Executable:** `cargo build --release`
    The executable will be located in `target/release/BarvazDNS.exe`.
4.  **Follow the configuration and service installation steps from Option 1.**

### Command-Line Usage

BarvazDNS provides a comprehensive command-line interface for managing the service and configuration.

* `BarvazDNS`: Displays general help and available commands.
* `BarvazDNS token "<token>"`: Sets the DuckDNS token.
* `BarvazDNS domain <add|remove> "<domain>"`: Adds or removes a subdomain.
* `BarvazDNS interval "<duration>"`: Sets the update interval (e.g., `5h`, `30m`, `1d`).
* `BarvazDNS ipv6 <enable|disable>`: Enables or disables IPv6 updates.
* `BarvazDNS config`: Displays the current configuration.
* `BarvazDNS update`: Forces an immediate update.
* `BarvazDNS status`: Displays the last update attempt status.
* `BarvazDNS check-update`: Checks if a newer version is available.
* `BarvazDNS clear-logs`: Deletes all log files.
* `BarvazDNS service`: Service related commands.
    * `BarvazDNS service install [--no-startup]`: Installs the service. Use `--no-startup` to disable start on boot.
    * `BarvazDNS service uninstall`: Uninstalls the service.
    * `BarvazDNS service start [--no-tray]`: Starts the service. Use `--no-tray` to start without the system tray icon.
    * `BarvazDNS service stop`: Stops the service.
    * `BarvazDNS service version`: Displays the running service version.

### Logging

BarvazDNS logs its activity to `%ProgramData%\BarvazDNS\`. The log level can be configured via the `log_level` field in `config.toml` (defaults to `info`). The `BARVAZ_LOG_LEVEL` environment variable overrides the config file setting.

### License

This project is licensed under the [MIT License](LICENSE).

### Acknowledgments

* DuckDNS for their excellent service.

---