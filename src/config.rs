use crate::crypto;

use std::collections::{HashMap, HashSet};
use std::env::current_exe;
use std::net::SocketAddr;
use std::path::Path;
use std::time::Duration;

use anyhow::{anyhow, Result};
use hotstuff_rs::types::PublicKeyBytes;
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Config {
    pub host_address: SocketAddr,
    #[serde(deserialize_with = "parse_peer_addresses")]
    pub peer_addresses: HashMap<PublicKeyBytes, SocketAddr>,
    #[serde(deserialize_with = "parse_validators")]
    pub validators: HashSet<PublicKeyBytes>,
    #[serde(deserialize_with = "parse_seconds")]
    pub minimum_view_timeout: Duration,
    pub sync_request_limit: u32,
    #[serde(deserialize_with = "parse_seconds")]
    pub sync_response_timeout: Duration,
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

fn parse_seconds<'de, D>(d: D) -> Result<Duration, D::Error>
where
    D: Deserializer<'de>,
{
    let secs: u64 = Deserialize::deserialize(d)?;
    Ok(Duration::from_secs(secs))
}

fn parse_peer_addresses<'de, D>(d: D) -> Result<HashMap<PublicKeyBytes, SocketAddr>, D::Error>
where
    D: Deserializer<'de>,
{
    let pa_map: HashMap<String, SocketAddr> = Deserialize::deserialize(d)?;
    let res: Result<HashMap<_, _>> = pa_map
        .into_iter()
        .map(|(ps, addr)| crypto::publickey_from_base64(&ps).map(|ps| (ps, addr)))
        .collect();
    Ok(res.unwrap())
}

fn parse_validators<'de, D>(d: D) -> Result<HashSet<PublicKeyBytes>, D::Error>
where
    D: Deserializer<'de>,
{
    let validators: Vec<String> = Deserialize::deserialize(d)?;
    let res: Result<HashSet<PublicKeyBytes>> = validators
        .into_iter()
        .map(|s| crypto::publickey_from_base64(&s))
        .collect();
    Ok(res.unwrap())
}
