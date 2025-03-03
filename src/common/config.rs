use std::fmt;
use std::time::Duration;
use std::path::PathBuf;
use std::env;

use serde::{Deserialize, Serialize};
use anyhow::{Result, anyhow};

use crate::common;


#[derive(Deserialize, Serialize)]
pub struct ServiceConfig {
	pub token: Option<String>,
	#[serde(default)]
	pub domain: Vec<String>,
	#[serde(with = "humantime_serde")]
	pub interval: Duration,
}

#[derive(Deserialize, Serialize)]
pub struct ClientConfig { }

#[derive(Deserialize, Serialize)]
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
}
