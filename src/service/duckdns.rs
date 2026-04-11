use anyhow::{Result, anyhow};

use crate::common::config::Config;


async fn generate_request(config: &Config) -> Result<String> {
	let ip = 
		public_ip::addr_v4()
			.await
			.ok_or(anyhow!("Failed to get the public IP address"))?;
	
	let mut url = format!("https://www.duckdns.org/update?domains={}&token={}&ip={}",
		config.service.domains_csv(),
		config.service.token.as_ref().ok_or(anyhow!("No token configured"))?.as_str(),
		ip
	);

	if config.service.ipv6 == Some(true) {
		let ipv6 = public_ip::addr_v6()
			.await
			.ok_or(anyhow!("Failed to get the public IPv6 address"))?;

		url.push_str(&format!("&ipv6={}", ipv6));
	}

	Ok(url)
}

fn clear_ip_addresses(config: &Config) -> Result<minreq::Response> {
	let url = format!("https://www.duckdns.org/update?domains={}&token={}&clear=true",
		config.service.domains_csv(),
		config.service.token.as_ref().ok_or(anyhow!("No token configured"))?.as_str()
	);

	Ok(minreq::get(url).send()?)
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
	let url = generate_request(config).await?;

	if config.service.clear_ip_addresses {
		// the ipv6 configuration might have been changed to false,
		// in which case we need to clear the ipv6 address
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
				return Err(anyhow!("Failed to clear IP addresses: {e}"));
			}
		}
	}

	log::debug!("Sending update request for domains: {}", config.service.domains_csv());
	match minreq::get(url).send() {
		Ok(res) => {
			let body = res.as_str()?;
			log::debug!("Update sent. Response: {body}");
			match body {
				"OK" => Ok(()),
				_ => Err(anyhow!("Bad response")),
			}
		}
		Err(e) => Err(anyhow!("Failed to update DuckDNS: {e}")),
	}
}