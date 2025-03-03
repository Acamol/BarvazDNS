use std::path::PathBuf;
use std::fs;
use std::io::Write;

use anyhow::{Result, anyhow};

use crate::common::{self, config::Config};


fn create_config_directory(config_dir_path: &PathBuf) -> Result<()> {
    if let Err(e) = fs::create_dir(&config_dir_path) {
        return Err(anyhow!("Failed to create config directory: {}", e));
    }

    Ok(())
}

fn install_config_file(config_path: &PathBuf) -> Result<Config> {
	if !config_path.is_dir() {
		create_config_directory(config_path)?;
		log::info!("Created the config directory in {config_path:?}");
	}

    let config_file_path = Config::get_config_file_path()?;
	let mut config_file = if !config_file_path.is_file() {
		fs::File::create(&config_file_path)
			.map_err(|e| anyhow!("Failed to create config file: {}", e))
			.map(|file| {
				log::info!("Created the config file at {config_file_path:?}");
				file
			})?
	} else {
		fs::File::open(&config_file_path)?
	};

	let config: Config = toml::from_str(common::strings::DEFAULT_CONFIG_CONTENT)?;

    config_file
        .write_all(toml::to_string_pretty(&config)?.as_bytes())
			.map_err(|e| anyhow!("Failed to write config file: {}", e))?;

    Ok(config)
}

pub fn read() -> Result<Config> {
	let config_dir_path = Config::get_config_directory_path()?;

	let config_file_path = Config::get_config_file_path()?;
	// check if we already have a config file,
	// if so, use it
	if config_dir_path.is_dir() && config_file_path.is_file() {
		let config_file = fs::read_to_string(&config_file_path)?;
		return Ok(
			if let Ok(config) = toml::from_str::<Config>(&config_file) {
				config
			} else {
				toml::from_str::<Config>(common::strings::DEFAULT_CONFIG_CONTENT)?
			});
	}

	// there is not config file, let's create it
	install_config_file(&config_dir_path)
}
