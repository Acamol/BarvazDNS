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

pub fn read() -> Result<Config> {
    let config_dir_path = Config::get_config_directory_path()?;

    let config_file_path = Config::get_config_file_path()?;
    // check if we already have a config file,
    // if so, use it
    if config_dir_path.is_dir() && config_file_path.is_file() {
        let config_file = fs::read_to_string(&config_file_path)?;
        return match toml::from_str::<Config>(&config_file) {
            Ok(mut config) => {
                clamp_interval(&mut config);
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    use crate::common::config::{ServiceConfig, Token};
    use std::collections::HashSet;

    fn make_config(interval_secs: u64) -> Config {
        Config {
            service: ServiceConfig {
                token: Some(Token::new("test".to_string())),
                domain: HashSet::new(),
                interval: Duration::from_secs(interval_secs),
                ipv6: None,
                clear_ip_addresses: false,
            },
            client: None,
        }
    }

    #[test]
    fn clamp_interval_below_minimum() {
        let mut config = make_config(1);
        clamp_interval(&mut config);
        assert_eq!(config.service.interval, common::consts::MINIMAL_INTERVAL);
    }

    #[test]
    fn clamp_interval_at_minimum() {
        let mut config = make_config(5);
        clamp_interval(&mut config);
        assert_eq!(config.service.interval, Duration::from_secs(5));
    }

    #[test]
    fn clamp_interval_above_minimum() {
        let mut config = make_config(3600);
        clamp_interval(&mut config);
        assert_eq!(config.service.interval, Duration::from_secs(3600));
    }
}
