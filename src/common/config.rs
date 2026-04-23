use std::io::Write;
use std::path::PathBuf;
use std::time::Duration;
use std::{collections::BTreeSet, fmt};
use std::{env, fs};

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};

use crate::common;

#[derive(Serialize, Deserialize, Clone)]
pub struct Token(String);

impl Token {
    pub fn new(value: String) -> Self {
        Self(value)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "***")
    }
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "***")
    }
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct ServiceConfig {
    pub token: Option<Token>,
    #[serde(default)]
    pub domain: BTreeSet<String>,
    #[serde(with = "humantime_serde")]
    pub interval: Duration,
    pub ipv6: Option<bool>,
    #[serde(default = "default_log_level")]
    pub log_level: String,
    #[serde(skip, default)]
    pub clear_ip_addresses: bool,
}

fn default_log_level() -> String {
    "info".to_string()
}

impl ServiceConfig {
    pub fn domains_csv(&self) -> String {
        self.domain
            .iter()
            .cloned()
            .collect::<Vec<String>>()
            .join(",")
    }

    /// Returns a string representation that includes the plaintext token.
    /// Only use this for direct display to an authenticated, privileged user.
    pub fn to_string_with_token(&self) -> String {
        format!(
            "token: {}\ndomains: {}\ninterval: {}\nipv6: {}",
            self.token.as_ref().map_or("<not set>", |t| t.as_str()),
            self.domains_csv(),
            humantime::format_duration(self.interval),
            if self.ipv6 == Some(true) {
                "enabled"
            } else {
                "disabled"
            }
        )
    }
}

impl fmt::Display for ServiceConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "token: {}\ndomains: {}\ninterval: {}\nipv6: {}",
            self.token
                .as_ref()
                .map_or("<not set>".to_string(), |t| t.to_string()),
            self.domains_csv(),
            humantime::format_duration(self.interval),
            if self.ipv6 == Some(true) {
                "enabled"
            } else {
                "disabled"
            }
        )
    }
}

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct DashboardConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Config {
    pub service: ServiceConfig,
    #[serde(default)]
    pub dashboard: Option<DashboardConfig>,
}

impl fmt::Display for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "domains: {}\ninterval: {}\nipv6: {}",
            self.service.domains_csv(),
            humantime::format_duration(self.service.interval),
            if self.service.ipv6 == Some(true) {
                "enabled"
            } else {
                "disabled"
            }
        )
    }
}

impl Config {
    pub fn effective_dashboard_port(&self) -> u16 {
        self.dashboard
            .as_ref()
            .and_then(|d| d.port)
            .unwrap_or(crate::common::consts::WEB_DASHBOARD_PORT)
    }

    fn get_programdata_path() -> Result<PathBuf> {
        env::var("ProgramData")
            .map(PathBuf::from)
            .map_err(|e| anyhow!("Failed to get ProgramData environment variable: {e}"))
    }

    pub fn get_config_directory_path() -> Result<PathBuf> {
        let mut path = Self::get_programdata_path()?;
        path.push(common::strings::CONFIG_DIR);
        Ok(path)
    }

    pub fn get_config_file_path() -> Result<PathBuf> {
        let mut path = Self::get_programdata_path()?;
        path.push(common::strings::CONFIG_DIR);
        path.push(common::strings::CONFIG_FILE_NAME);
        Ok(path)
    }

    pub fn store(&self) -> Result<()> {
        let config_file_path = Self::get_config_file_path()?;

        // write the current configuration to disk, creating the file if needed
        let mut config_file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&config_file_path)?;

        config_file
            .write_all(toml::to_string_pretty(self)?.as_bytes())
            .map_err(|e| anyhow!("Failed to write config file: {e}"))?;

        Ok(())
    }

    pub fn read() -> Result<Self> {
        let config_dir_path = Self::get_config_directory_path()?;
        let config_file_path = Self::get_config_file_path()?;

        if config_dir_path.is_dir() && config_file_path.is_file() {
            let config_file = fs::read_to_string(&config_file_path)?;
            return match toml::from_str::<Config>(&config_file) {
                Ok(mut config) => {
                    Self::clamp_interval(&mut config);
                    config.service.clear_ip_addresses = true;
                    Ok(config)
                }
                Err(e) => Err(anyhow!("Failed to parse the configuration file: {e}")),
            };
        }

        Self::install_config_file(&config_dir_path)
    }

    fn clamp_interval(config: &mut Config) {
        if config.service.interval < common::consts::MINIMAL_INTERVAL {
            let min_human_time = humantime::format_duration(common::consts::MINIMAL_INTERVAL);
            let interval_human_time = humantime::format_duration(config.service.interval);

            log::info!(
                "Interval must be at least {}, got {}, using {}",
                min_human_time,
                interval_human_time,
                min_human_time
            );
            config.service.interval = common::consts::MINIMAL_INTERVAL;
        }
    }

    fn install_config_file(dir_path: &std::path::Path) -> Result<Self> {
        if !dir_path.is_dir() {
            fs::create_dir(dir_path)
                .map_err(|e| anyhow!("Failed to create config directory: {e}"))?;
            log::info!("Created the config directory in {dir_path:?}");
        }

        let config: Config = toml::from_str(common::strings::DEFAULT_CONFIG_CONTENT)?;
        config.store()?;
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_service_config(
        token: Option<&str>,
        domains: &[&str],
        interval_secs: u64,
        ipv6: Option<bool>,
    ) -> ServiceConfig {
        ServiceConfig {
            token: token.map(|t| Token::new(t.to_string())),
            domain: domains.iter().map(|d| d.to_string()).collect(),
            interval: Duration::from_secs(interval_secs),
            ipv6,
            log_level: "info".to_string(),
            clear_ip_addresses: false,
        }
    }

    fn make_config() -> Config {
        Config {
            service: make_service_config(None, &[], 60, None),
            dashboard: None,
        }
    }

    #[test]
    fn domains_csv_empty() {
        let config = make_service_config(None, &[], 60, None);
        assert_eq!(config.domains_csv(), "");
    }

    #[test]
    fn domains_csv_single() {
        let config = make_service_config(None, &["myhost"], 60, None);
        assert_eq!(config.domains_csv(), "myhost");
    }

    #[test]
    fn domains_csv_multiple() {
        let config = make_service_config(None, &["a", "b", "c"], 60, None);
        let csv = config.domains_csv();
        let mut parts: Vec<&str> = csv.split(',').collect();
        parts.sort();
        assert_eq!(parts, vec!["a", "b", "c"]);
    }

    #[test]
    fn to_string_with_token_present() {
        let config = make_service_config(Some("secret123"), &["home"], 300, Some(true));
        let s = config.to_string_with_token();
        assert!(s.contains("secret123"));
        assert!(s.contains("home"));
        assert!(s.contains("enabled"));
    }

    #[test]
    fn to_string_with_token_absent() {
        let config = make_service_config(None, &[], 60, None);
        let s = config.to_string_with_token();
        assert!(s.contains("<not set>"));
        assert!(s.contains("disabled"));
    }

    #[test]
    fn display_masks_token() {
        let config = make_service_config(Some("secret123"), &["home"], 300, None);
        let displayed = format!("{config}");
        assert!(!displayed.contains("secret123"));
        assert!(displayed.contains("***"));
    }

    #[test]
    fn clamp_interval_below_minimum() {
        let mut config = make_config();
        config.service.interval = Duration::from_secs(1);
        Config::clamp_interval(&mut config);
        assert_eq!(
            config.service.interval,
            crate::common::consts::MINIMAL_INTERVAL
        );
    }

    #[test]
    fn clamp_interval_at_minimum() {
        let mut config = make_config();
        config.service.interval = Duration::from_secs(5);
        Config::clamp_interval(&mut config);
        assert_eq!(config.service.interval, Duration::from_secs(5));
    }

    #[test]
    fn clamp_interval_above_minimum() {
        let mut config = make_config();
        config.service.interval = Duration::from_secs(3600);
        Config::clamp_interval(&mut config);
        assert_eq!(config.service.interval, Duration::from_secs(3600));
    }

    #[test]
    fn dashboard_port_returns_default_when_no_section() {
        let config = make_config();
        assert_eq!(
            config.effective_dashboard_port(),
            crate::common::consts::WEB_DASHBOARD_PORT
        );
    }

    #[test]
    fn dashboard_port_returns_default_when_port_is_none() {
        let mut config = make_config();
        config.dashboard = Some(DashboardConfig { port: None });
        assert_eq!(
            config.effective_dashboard_port(),
            crate::common::consts::WEB_DASHBOARD_PORT
        );
    }

    #[test]
    fn dashboard_port_returns_custom_when_set() {
        let mut config = make_config();
        config.dashboard = Some(DashboardConfig { port: Some(9999) });
        assert_eq!(config.effective_dashboard_port(), 9999);
    }

    #[test]
    fn dashboard_not_serialized_when_none() {
        let config = make_config();
        let serialized = toml::to_string_pretty(&config).unwrap();
        assert!(!serialized.contains("dashboard"));
        assert!(!serialized.contains("port"));
    }

    #[test]
    fn dashboard_port_deserialized_when_present() {
        let toml_str = r#"
[service]
interval = "1 day"
log_level = "info"

[dashboard]
port = 8080
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.effective_dashboard_port(), 8080);
    }

    #[test]
    fn dashboard_port_defaults_when_section_absent() {
        let toml_str = r#"
[service]
interval = "1 day"
log_level = "info"
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert!(config.dashboard.is_none());
        assert_eq!(
            config.effective_dashboard_port(),
            crate::common::consts::WEB_DASHBOARD_PORT
        );
    }
}
