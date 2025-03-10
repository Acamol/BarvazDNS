# BarvazDNS

BarvazDNS is a Windows application designed to automatically update your DuckDNS domain(s) with your current public IP address, ensuring your domain remains accessible even with a dynamic IP. It functions as both a command-line tool and a Windows service, providing flexibility and control.

## Features

* **Automatic DuckDNS Updates:** Regularly checks and updates your DuckDNS domains.
* **Single Executable:** All functionality, including service management and configuration, is contained within a single executable.
* **Command-Line Interface (CLI):** Provides extensive control over the service and configuration.
* **Windows Service:** Runs in the background for continuous, automated updates.
* **Human-Readable Interval:** Supports intervals in hours, minutes, and days and so on (e.g., `5h`, `30m`, `1d`).
* **TOML Configuration:** Uses a TOML configuration file for easy setup and modification.
* **User-Specific Configuration:** Configuration file located in `%ProgramData%\BarvazDNS\config.toml`.
* **Logging:** Logs are stored in `%ProgramData%\BarvazDNS\`.
* **IPv6 Support:** Option to enable or disable IPv6 updates.
* **Open Source:** Feel free to modify, contribute, and distribute.

## Getting Started

### Prerequisites

* A DuckDNS account and domain(s).
* Windows operating system (doh).
* **Administrator privileges are required to install and manage the Windows service.**
* **For building from source:**
    * Windows SDK installed and added to your system's PATH environment variable.

### Installation

**Option 1: Pre-built Executable**

1.  **Download:** Download the latest release from the [Releases](https://github.com/Acamol/BarvazDNS/releases/) page.
3.  **Configuration:**
    * The configuration file `config.toml` is automatically created in `%ProgramData%\BarvazDNS\` on the first run.
    * You can also manually create or modify the `config.toml` file.
    * Example `config.toml`:

    ```toml
    [service]
    token = "your-duckdns-token"
    domain = ["yoursubdomain", "anothersubdomain"]
    interval = "5h"
    ipv6 = false

    [client]
    # Currently not used
    ```

4.  **Windows Service Installation:**
    * **Open a command prompt or PowerShell as administrator.**
    * Navigate to the directory containing `BarvazDNS`.
    * Run `BarvazDNS service install` to install the service.
    * Run `BarvazDNS service start` to start the service.
    * Run `BarvazDNS service stop` to start the service.
    * Run `BarvazDNS service uninstall` to uninstall the service.

**Option 2: Building from Source**

1.  **Clone the Repository:** `git clone https://github.com/acamol/BarvazDNS.git`
2.  **Navigate to the Directory:** `cd BarvazDNS`
3.  **Build the Executable:** `cargo build --release`
4.  **The executable will be located in `target/release/BarvazDNS.exe`**
5.  **Follow the configuration and service installation steps from Option 1.**

### Command-Line Usage

BarvazDNS provides a comprehensive command-line interface for managing the service and configuration. **Many of these commands require administrator privileges.**

* `BarvazDNS`: Displays general help and available commands.
* `BarvazDNS domain add "yourdomain"`: Adds a subdomain.
* `BarvazDNS domain remove "yourdomain"`: Removes a subdomain.
* `BarvazDNS token "your_token"`: Sets the DuckDNS token.
* `BarvazDNS interval "5h"`: Sets the update interval.
* `BarvazDNS ipv6 enable`: Enables IPv6 updates.
* `BarvazDNS ipv6 disable`: Disables IPv6 updates.
* `BarvazDNS update`: Forces an immediate update.
* `BarvazDNS config`: Displays the current configuration.
* `BarvazDNS status`: Displays the last update attempt status.
* `BarvazDNS service`: Service related commands.
    * `BarvazDNS service install`: Installs the service.
    * `BarvazDNS service install --no-startup` Installs the service without start on startup.
    * `BarvazDNS service uninstall`: Uninstalls the service.
    * `BarvazDNS service start`: Starts the service.
    * `BarvazDNS service stop`: Stops the service.

### Logging

BarvazDNS logs its activity to `%ProgramData%\BarvazDNS\`. Check this log file for any errors or issues.

### License

This project is licensed under the [MIT License](LICENSE).

### Acknowledgments

* DuckDNS for their excellent service.

---