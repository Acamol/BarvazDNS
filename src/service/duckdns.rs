use anyhow::{Result, anyhow};

use crate::common::config::Config;


async fn generate_request(config: &Config) -> Result<String> {

	let ip = 
		public_ip::addr_v4()
			.await
			.ok_or(anyhow!("Failed to get the public IP"))?;
	
	Ok(format!("https://www.duckdns.org/update?domains={}&token={}&ip={}&verbose=true",
		config.service.domain.iter().cloned().collect::<Vec<String>>().join(","),
		config.service.token.as_ref().unwrap(),
		ip
	))
}

pub async fn update(config: &Config) {
	let url = match generate_request(&config).await {
		Ok(u) => u,
		Err(e) => {
			log::error!("{e}");
			return;
		}
	};

	log::debug!("request url: {url}");
	match minreq::get(url).send() {
		Ok(res) => log::debug!("Update sent. Response: {res:?}"),
		Err(e) => {
			log::error!("Failed to update DuckDNS");
			log::debug!("Error is {e}");
		}
	}
}