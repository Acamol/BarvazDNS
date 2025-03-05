use std::time::Duration;

use anyhow::{Result, anyhow};
use bincode::{deserialize, serialize};
use serde::{Deserialize as _Deserialize, Serialize as _Serialize};

use super::config;


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
}

impl Serialize for Request {
	fn serialize(&self) -> Result<Vec<u8>> {
		serialize(&self).map_err(|e| anyhow!("{e}"))
	}
}

impl Deserialize for Request {
	fn deserialize(bytes: &[u8]) -> Result<Self> {
		deserialize::<Self>(bytes).map_err(|e| anyhow!("{e}"))
	}
}

// TODO: implement with macro? request and response have the same implementation
#[derive(_Serialize, _Deserialize, Debug)]
pub enum Response {
	Nothing,
	Config(config::ServiceConfig),
}

impl Serialize for Response {
	fn serialize(&self) -> Result<Vec<u8>> {
		serialize(&self).map_err(|e| anyhow!("{e}"))
	}
}

impl Deserialize for Response {
	fn deserialize(bytes: &[u8]) -> Result<Self> {
		deserialize::<Self>(bytes).map_err(|e| anyhow!("{e}"))
	}
}