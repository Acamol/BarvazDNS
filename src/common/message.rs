use std::time::Duration;

use anyhow::{Result, anyhow};
use bincode::{deserialize, serialize};
use serde::{Deserialize, Serialize};


#[derive(Serialize, Deserialize, Debug)]
pub enum Message {
	Interval(Duration),
	Token(String),
	AddDomain(String),
	RemoveDomain(String),
}

impl Message {
	pub fn serialize(&self) -> Result<Vec<u8>> {
		serialize(&self).map_err(|e| anyhow!("{e}"))
	}

	pub fn deserialize(bytes: &Vec<u8>) -> Result<Self> {
		deserialize::<Message>(bytes).map_err(|e| anyhow!("{e}"))
	}
}
