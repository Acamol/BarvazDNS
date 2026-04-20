use std::time::{Duration, SystemTime};

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;
use tokio::{io::AsyncReadExt, net::windows::named_pipe::ClientOptions};

use super::{config, strings};

pub use config::Token;

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct UpdateStatus {
    /// Timestamp + domains of the last successful update, if any.
    pub last_success: Option<(SystemTime, Vec<String>)>,
}

pub fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    Ok(bincode::serialize(value)?)
}

pub fn decode<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    Ok(bincode::deserialize(bytes)?)
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Request {
    Interval(Duration),
    Token(Token),
    AddDomain(String),
    RemoveDomain(String),
    Ipv6(bool),
    ForceUpdate,
    DebugLevel(String),
    GetConfig,
    GetStatus,
    Version,
}

impl Request {
    pub async fn send(self) -> Result<Response> {
        // Try to connect to the named pipe
        let mut client = ClientOptions::new().open(strings::PIPE_NAME).map_err(|_| {
            anyhow!("Failed to communicate with the service. Verify it is running.")
        })?;

        let service_request = ServiceRequest::new(self);
        let encoded = encode(&service_request)?;
        client.write_all(&encoded).await?;

        let mut buf = Vec::new();
        client.read_to_end(&mut buf).await?;
        decode(&buf)
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ServiceRequest {
    version: String,
    request: Request,
}

impl ServiceRequest {
    fn new(request: Request) -> Self {
        ServiceRequest {
            version: strings::VERSION.to_string(),
            request,
        }
    }

    pub fn is_compatible(&self) -> bool {
        if let Request::Version = self.request {
            // Always allow Version requests through, even when versions differ,
            // since the purpose of this request is to query a potentially-mismatched service.
            return true;
        }
        self.version == strings::VERSION
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn request(&self) -> &Request {
        &self.request
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Response {
    Ok,
    Err(String),
    Config(config::ServiceConfig),
    Status(UpdateStatus),
    Version(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roundtrip_request(request: Request) {
        let encoded = encode(&request).expect("encode failed");
        let decoded: Request = decode(&encoded).expect("decode failed");
        // Verify by re-encoding — same bytes means same value
        let re_encoded = encode(&decoded).expect("re-encode failed");
        assert_eq!(encoded, re_encoded);
    }

    #[test]
    fn encode_decode_request_interval() {
        roundtrip_request(Request::Interval(Duration::from_secs(300)));
    }

    #[test]
    fn encode_decode_request_token() {
        roundtrip_request(Request::Token(Token::new("test-token".to_string())));
    }

    #[test]
    fn encode_decode_request_add_domain() {
        roundtrip_request(Request::AddDomain("example".to_string()));
    }

    #[test]
    fn encode_decode_request_remove_domain() {
        roundtrip_request(Request::RemoveDomain("example".to_string()));
    }

    #[test]
    fn encode_decode_request_ipv6() {
        roundtrip_request(Request::Ipv6(true));
        roundtrip_request(Request::Ipv6(false));
    }

    #[test]
    fn encode_decode_request_force_update() {
        roundtrip_request(Request::ForceUpdate);
    }

    #[test]
    fn encode_decode_request_version() {
        roundtrip_request(Request::Version);
    }

    #[test]
    fn encode_decode_response_ok() {
        let encoded = encode(&Response::Ok).unwrap();
        let decoded: Response = decode(&encoded).unwrap();
        assert!(matches!(decoded, Response::Ok));
    }

    #[test]
    fn encode_decode_response_err() {
        let encoded = encode(&Response::Err("something failed".to_string())).unwrap();
        let decoded: Response = decode(&encoded).unwrap();
        assert!(matches!(decoded, Response::Err(msg) if msg == "something failed"));
    }

    #[test]
    fn encode_decode_response_version() {
        let encoded = encode(&Response::Version("1.0.0".to_string())).unwrap();
        let decoded: Response = decode(&encoded).unwrap();
        assert!(matches!(decoded, Response::Version(v) if v == "1.0.0"));
    }

    #[test]
    fn decode_corrupted_data_fails() {
        let result: Result<Request> = decode(&[0xFF, 0xFF, 0xFF]);
        assert!(result.is_err());
    }

    #[test]
    fn is_compatible_version_request_always_passes() {
        let mut req = ServiceRequest::new(Request::Version);
        req.version = "0.0.0-mismatch".to_string();
        // Version requests bypass the version check, even with a mismatched version
        assert!(req.is_compatible());
    }

    #[test]
    fn is_compatible_mismatched_version_fails() {
        let mut req = ServiceRequest::new(Request::ForceUpdate);
        req.version = "0.0.0-mismatch".to_string();
        assert!(!req.is_compatible());
    }
}
