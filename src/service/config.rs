use std::fs;

use anyhow::{Result, anyhow};

use crate::common::{self, config::Config};


fn create_config_directory(dir_path: &std::path::Path) -> Result<()> {
    if let Err(e) = fs::create_dir(dir_path) {
        return Err(anyhow!("Failed to create config directory: {}", e));
    }

    Ok(())
}

fn install_config_file(dir_path: &std::path::Path) -> Result<Config> {
	if !dir_path.is_dir() {
		create_config_directory(dir_path)?;
		log::info!("Created the config directory in {dir_path:?}");
	}

	let config: Config = toml::from_str(common::strings::DEFAULT_CONFIG_CONTENT)?;

	// store the configuration in the config file
	config.store()?;

    Ok(config)
}

pub fn read() -> Result<Config> {
	let config_dir_path = Config::get_config_directory_path()?;

	let config_file_path = Config::get_config_file_path()?;
	// check if we already have a config file,
	// if so, use it
	if config_dir_path.is_dir() && config_file_path.is_file() {
		let config_file = fs::read_to_string(&config_file_path)?;
		return match toml::from_str::<Config>(&config_file) {
			Ok(mut config) => {
				if config.service.interval < common::consts::MINIMAL_INTERVAL {
					let min_human_time = humantime::format_duration(common::consts::MINIMAL_INTERVAL);
					let interval_human_time = humantime::format_duration(config.service.interval);

					log::info!("Interval must be at least {}, got {}, using {}", min_human_time, interval_human_time, min_human_time);
					config.service.interval = common::consts::MINIMAL_INTERVAL;

					// we can now store the new interval, but we'll allow bad configuration -
					// just ignore it
				}
				// to be on the safe side, when we read from the config file,
				// better to clear addresses in case the IPv6 configuration was changed
				config.service.clear_ip_addresses = true;
				Ok(config)
			}
			Err(e) => Err(anyhow!("Failed to parse the configuration file: {e}")),
		};
	}

	// there is no config file, let's create it
	install_config_file(&config_dir_path)
}
