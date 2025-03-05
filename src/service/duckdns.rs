use anyhow::{Result, anyhow};

use crate::common::config::Config;


async fn generate_request(config: &Config) -> Result<String> {
	let ip = 
		public_ip::addr_v4()
			.await
			.ok_or(anyhow!("Failed to get the public IP address"))?;
	
	let mut url = format!("https://www.duckdns.org/update?domains={}&token={}&ip={}",
		config.service.domain.iter().cloned().collect::<Vec<String>>().join(","),
		config.service.token.as_ref().unwrap(),
		ip
	);

	if config.service.ipv6.is_some_and(|enable| enable == true) {
		let ipv6 = public_ip::addr_v6()
			.await
			.ok_or(anyhow!("Failed to get the public IPv6 address"))?;

		url.push_str(&format!("&ipv6={}", ipv6));
	}

	Ok(url)
}

fn clear_ip_addresses(config: &Config) -> Result<minreq::Response, minreq::Error> {
	let url = format!("https://www.duckdns.org/update?domains={}&token={}&clear=true",
		config.service.domain.iter().cloned().collect::<Vec<String>>().join(","),
		config.service.token.as_ref().unwrap()
	);

	minreq::get(url) .send()
}

pub async fn update(config: &Config) {
	let url = match generate_request(&config).await {
		Ok(u) => u,
		Err(e) => {
			log::error!("{e}");
			return;
		}
	};

	if config.service.clear_ip_addresses {
		log::debug!("Clear IP addresses");
		// the ipv6 confiugration might have been changed to false,
		// in which case we need to clear the ipv6 addrress
		match clear_ip_addresses(config) {
			Ok(res) => log::debug!("Clear sent. Response: {res:?}"),
			Err(e) => {
				log::error!("Failed to clear IP addresses on DuckDNS");
				log::debug!("Error is {e}");
			}
		}
	}

	log::debug!("Request url: {url}");
	match minreq::get(url).send() {
		Ok(res) => log::debug!("Update sent. Response: {res:?}"),
		Err(e) => {
			log::error!("Failed to update DuckDNS");
			log::debug!("Error is {e}");
		}
	}
}