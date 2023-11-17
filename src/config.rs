use std::collections::{HashMap, HashSet};
use std::env::current_exe;
use std::net::SocketAddr;
use std::path::Path;
use std::time::Duration;

use hotstuff_rs::types::PublicKeyBytes;

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Config {
    pub host_address: SocketAddr,
    pub peer_addresses: HashMap<PublicKeyBytes, SocketAddr>,
    pub validators: HashSet<PublicKeyBytes>,
    pub minimum_view_timeout: Duration,
    pub sync_request_limit: u32,
    pub sync_response_timeout: Duration,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            host_address: "127.0.0.1:8080".parse::<SocketAddr>().unwrap(),
            peer_addresses: Default::default(),
            validators: Default::default(),
            minimum_view_timeout: Default::default(),
            sync_request_limit: Default::default(),
            sync_response_timeout: Default::default(),
        }
    }
}

impl Config {
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self> {
        if !path.as_ref().is_file() {
            return Err(anyhow!("config file not found"));
        }
        let config_str = std::fs::read_to_string(path).unwrap();
        Ok(serde_yaml::from_str::<Config>(&config_str)?)
    }

    pub fn new() -> Result<Self> {
        let current_exe = current_exe()?;
        let config_path = current_exe
            .parent()
            .unwrap()
            .join("config")
            .join("config.yaml");
        Self::from_path(config_path)
    }
}
