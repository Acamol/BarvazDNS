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
* **Logging:** Detailed logs are stored in `%ProgramData%\BarvazDNS\`.
* **IPv6 Support:** Option to enable or disable IPv6 updates.
* **Open Source:** Feel free to modify, contribute, and distribute.

## Getting Started

### Prerequisites

* A DuckDNS account and domain(s).
* Windows operating system (doh).

### Installation

1.  **Download:** Download the latest release from the [Releases](https://github.com/acamol/BarvasDNS/releases) page.
2.  **Extract:** Extract the downloaded executable to a folder of your choice.
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

4.  **Windows Service Installation (Recommended):**
    * Open a command prompt or PowerShell as administrator.
    * Navigate to the directory containing `BarvazDNS`.
    * Run `BarvazDNS service install` to install the service.
    * Run `BarvazDNS service start` to start the service.
    * Run `BarvazDNS service uninstall` to uninstall the service.

### Command-Line Usage

BarvazDNS provides a comprehensive command-line interface for managing the service and configuration.


* `BarvazDNS`: Displays general help and available commands.
* `BarvazDNS service`: Service related commands.
    * `BarvazDNS service install`: Installs the service.
    * `BarvazDNS service uninstall`: Uninstalls the service.
    * `BarvazDNS service start`: Starts the service.
    * `BarvazDNS service stop`: Stops the service.
* `BarvazDNS client`: Client related commands.
    * `BarvazDNS client domain add "yourdomain"`: Adds a subdomain.
    * `BarvazDNS client domain remove "yourdomain"`: Removes a subdomain.
    * `BarvazDNS client token "your_token"`: Sets the DuckDNS token.
    * `BarvazDNS client interval "5h"`: Sets the update interval.
    * `BarvazDNS client ipv6 enable`: Enables IPv6 updates.
    * `BarvazDNS client ipv6 disable`: Disables IPv6 updates.
    * `BarvazDNS client update`: Forces an immediate update.
    * `BarvazDNS client config`: Displays the current configuration.
    * `BarvazDNS client status`: Displays the last update attempt status.

### Logging

BarvazDNS logs its activity to `%ProgramData%\BarvazDNS\`. Check this log file for any errors or issues.

### License

This project is licensed under the [MIT License](LICENSE).

### Acknowledgments

* DuckDNS for their excellent service.

---