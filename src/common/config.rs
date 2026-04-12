use std::io::Write;
use std::path::PathBuf;
use std::time::Duration;
use std::{collections::HashSet, fmt};
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
    pub domain: HashSet<String>,
    #[serde(with = "humantime_serde")]
    pub interval: Duration,
    pub ipv6: Option<bool>,
    #[serde(skip_serializing, default)]
    pub clear_ip_addresses: bool,
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

#[derive(Deserialize, Serialize, Clone)]
pub struct ClientConfig {}

#[derive(Deserialize, Serialize, Clone)]
pub struct Config {
    pub service: ServiceConfig,
    pub client: Option<ClientConfig>,
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
}
