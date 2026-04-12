use std::time::{Duration, SystemTime};

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;
use tokio::{io::AsyncReadExt, net::windows::named_pipe::ClientOptions};

use super::{config, strings};

pub use config::Token;

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

        let mut buf = vec![0; super::consts::PIPE_BUFFER_SIZE];
        let n = client.read(&mut buf).await?;
        decode(&buf[..n])
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
    Status(Option<SystemTime>),
    Version(String),
}
