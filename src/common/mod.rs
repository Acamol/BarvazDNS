use std::time::Duration;

use anyhow::{Result, anyhow};
use bincode::{deserialize, serialize};
use serde::{Deserialize, Serialize};

pub mod strings;

#[derive(Serialize, Deserialize, Debug)]
pub enum message {
	Interval(Duration),
}

impl message {
	pub fn new(interval: Duration) -> Self {
		message::Interval(interval)
	}

	pub fn serialize(&self) -> Result<Vec<u8>> {
		serialize(&self).map_err(|e| anyhow!("{e}"))
	}

	pub fn deserialize(bytes: &Vec<u8>) -> Result<Self> {
		deserialize::<message>(bytes).map_err(|e| anyhow!("{e}"))
	}
}