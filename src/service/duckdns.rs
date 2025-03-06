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

/// Updates DuckDNS with the provided configuration.
///
/// This function generates a DuckDNS update request based on the provided `config`,
/// sends the request, and handles the response. If the configuration specifies, it
/// also clears existing IP addresses on DuckDNS before sending the update.
///
/// # Arguments
///
/// * `config`: The configuration used to generate the DuckDNS update request.
///
/// # Returns
///
/// * `Ok(())` if the update was successful.
/// * `Err(e)` if an error occurred during the update process, including request
///   generation, clearing IP addresses, or sending the update.
pub async fn update(config: &Config) -> Result<()> {
	let url = match generate_request(&config).await {
		Ok(u) => u,
		Err(e) => {
			log::error!("{e}");
			return Err(anyhow!("{e}"));
		}
	};

	if config.service.clear_ip_addresses {
		// the ipv6 confiugration might have been changed to false,
		// in which case we need to clear the ipv6 addrress
		match clear_ip_addresses(config) {
			Ok(res) => {
				let body = res.as_str()?;
				match body {
					"OK" => log::debug!("Cleared"),
					_ => return Err(anyhow!("Failed to clear. Bad response")),

				}
				log::debug!("Clear sent. Response: {res:?}");
			}
			Err(e) => {
				log::error!("Failed to clear IP addresses on DuckDNS");
				log::debug!("Error is {e}");
				return Err(anyhow!("{e}"));
			}
		}
	}

	log::debug!("Request url: {url}");
	match minreq::get(url).send() {
		Ok(res) => {
			let body = res.as_str()?;
			log::debug!("Update sent. Response: {body}");
			match body {
				"OK" => Ok(()),
				_ => Err(anyhow!("Bad response")),
			}
		}
		Err(e) => {
			log::error!("Failed to update DuckDNS");
			log::debug!("Error is {e}");
			return Err(anyhow!("{e}"));
		}
	}
}