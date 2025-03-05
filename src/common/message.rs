use std::time::Duration;

use anyhow::{Result, anyhow};
use bincode::{deserialize, serialize};
use serde::{Deserialize as _Deserialize, Serialize as _Serialize};
use tokio::{io::AsyncReadExt, net::windows::named_pipe::ClientOptions};
use tokio::io::AsyncWriteExt;

use super::{config, strings};


pub trait Serialize {
	fn serialize(&self) -> Result<Vec<u8>>;
}

pub trait Deserialize {
	fn deserialize(bytes: &[u8]) -> Result<Self>
		where Self: Sized;
}

#[derive(_Serialize, _Deserialize, Debug)]
pub enum Request {
	Interval(Duration),
	Token(String),
	AddDomain(String),
	RemoveDomain(String),
	Ipv6(bool),
	ForceUpdate,
	DebugLevel(String),
	GetConfig,
	GetStatus,
}

impl Request {
	pub async fn send(self) -> Result<Response> {
		// Try to connect to the named pipe
		let mut client = ClientOptions::new()
			.open(strings::PIPE_NAME)
			.map_err(|_| anyhow!("Failed to communicate with the service. Verify it is running."))?;

		let service_request = ServiceRequest::new(self);
		let encode = serialize(&service_request)?;
		client.write_all(&encode).await?;

		let mut buf = vec![0; std::mem::size_of::<Response>()];
		client.read(&mut buf).await?;
		deserialize(&buf).map_err(|e| anyhow!("{e}"))
	}
}

#[derive(_Serialize, _Deserialize, Debug)]
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

	pub fn is_compatiable(&self) -> bool {
		self.version == strings::VERSION
	}

	pub fn version(&self) -> &str {
		&self.version
	}

	pub fn request(&self) -> &Request {
		&self.request
	}
}

impl Deserialize for ServiceRequest {
	fn deserialize(bytes: &[u8]) -> Result<Self> {
		deserialize::<Self>(bytes).map_err(|e| anyhow!("{e}"))
	}
}

impl Serialize for ServiceRequest {
	fn serialize(&self) -> Result<Vec<u8>> {
		serialize(self).map_err(|e| anyhow!("{e}"))
	}
}

#[derive(_Serialize, _Deserialize, Debug)]
pub enum Response {
	Ok,
	Err(String),
	Config(config::ServiceConfig),
	Status(bool),
}

// TODO: implement with macro?
impl Serialize for Response {
	fn serialize(&self) -> Result<Vec<u8>> {
		serialize(self).map_err(|e| anyhow!("{e}"))
	}
}