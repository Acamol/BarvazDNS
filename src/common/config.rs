use std::io::Write;
use std::{collections::HashSet, fmt};
use std::time::Duration;
use std::path::PathBuf;
use std::{env, fs};

use serde::{Deserialize, Serialize};
use anyhow::{Result, anyhow};

use crate::common;


#[derive(Deserialize, Serialize, Clone)]
pub struct ServiceConfig {
	pub token: Option<String>,
	#[serde(default)]
	pub domain: HashSet<String>,
	#[serde(with = "humantime_serde")]
	pub interval: Duration,
	pub ipv6: Option<bool>,
	#[serde(skip_serializing, default)]
	pub ipv6_config_changed: bool,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct ClientConfig { }

#[derive(Deserialize, Serialize, Clone)]
pub struct Config {
	pub service: ServiceConfig,
	pub client: ClientConfig,
}
impl fmt::Display for Config {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", toml::to_string_pretty(self).unwrap())
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

		// create the config file if it does not exist
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
