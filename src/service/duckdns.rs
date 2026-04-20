use std::net::{Ipv4Addr, Ipv6Addr};

use anyhow::{Result, anyhow};

use crate::common::config::Config;

fn build_update_url(
    domains_csv: &str,
    token: &str,
    ipv4: Ipv4Addr,
    ipv6: Option<Ipv6Addr>,
) -> String {
    let mut url =
        format!("https://www.duckdns.org/update?domains={domains_csv}&token={token}&ip={ipv4}");

    if let Some(v6) = ipv6 {
        url.push_str(&format!("&ipv6={v6}"));
    }

    url
}

fn build_clear_url(domains_csv: &str, token: &str) -> String {
    format!("https://www.duckdns.org/update?domains={domains_csv}&token={token}&clear=true")
}

async fn generate_request(config: &Config) -> Result<String> {
    let ip = public_ip::addr_v4()
        .await
        .ok_or(anyhow!("Failed to get the public IP address"))?;

    let token = config
        .service
        .token
        .as_ref()
        .ok_or(anyhow!("No token configured"))?;

    let ipv6 = if config.service.ipv6 == Some(true) {
        Some(
            public_ip::addr_v6()
                .await
                .ok_or(anyhow!("Failed to get the public IPv6 address"))?,
        )
    } else {
        None
    };

    Ok(build_update_url(
        &config.service.domains_csv(),
        token.as_str(),
        ip,
        ipv6,
    ))
}

fn clear_ip_addresses(config: &Config) -> Result<minreq::Response> {
    let token = config
        .service
        .token
        .as_ref()
        .ok_or(anyhow!("No token configured"))?;

    let url = build_clear_url(&config.service.domains_csv(), token.as_str());

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
                    _ => {
                        return Err(anyhow!(
                            "Failed to clear IP addresses: DuckDNS responded with '{body}'"
                        ));
                    }
                }
                log::debug!("Clear sent. Response: {res:?}");
            }
            Err(e) => {
                return Err(anyhow!("Failed to clear IP addresses: {e}"));
            }
        }
    }

    log::debug!(
        "Sending update request for domains: {}",
        config.service.domains_csv()
    );
    match minreq::get(url).send() {
        Ok(res) => {
            let body = res.as_str()?;
            log::debug!("Update sent. Response: {body}");
            match body {
                "OK" => Ok(()),
                _ => Err(anyhow!("DuckDNS responded with '{body}'")),
            }
        }
        Err(e) => Err(anyhow!("Failed to update DuckDNS: {e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr, Ipv6Addr};

    #[test]
    fn build_update_url_ipv4_only() {
        let url = build_update_url("home", "tok123", Ipv4Addr::new(1, 2, 3, 4), None);
        assert_eq!(
            url,
            "https://www.duckdns.org/update?domains=home&token=tok123&ip=1.2.3.4"
        );
    }

    #[test]
    fn build_update_url_with_ipv6() {
        let url = build_update_url(
            "home",
            "tok123",
            Ipv4Addr::new(1, 2, 3, 4),
            Some(Ipv6Addr::LOCALHOST),
        );
        assert!(url.contains("&ipv6=::1"));
    }

    #[test]
    fn build_update_url_multiple_domains() {
        let url = build_update_url("a,b,c", "tok", Ipv4Addr::new(10, 0, 0, 1), None);
        assert!(url.contains("domains=a,b,c"));
    }

    #[test]
    fn build_clear_url_format() {
        let url = build_clear_url("home", "tok123");
        assert_eq!(
            url,
            "https://www.duckdns.org/update?domains=home&token=tok123&clear=true"
        );
    }
}
