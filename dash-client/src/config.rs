use dash_common::crypto::keypair_from_pem;

use std::env::current_exe;
use std::net::SocketAddr;
use std::path::Path;

use anyhow::{anyhow, Ok, Result};
use hotstuff_rs::types::DalekKeypair;
use serde::{Deserialize, Serialize};
use tokio::fs::read_to_string;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub node_addrs: Vec<SocketAddr>,
    #[serde(skip)]
    pub keypair: Option<DalekKeypair>,
}

impl Config {
    pub async fn new() -> Result<Self> {
        let config_dir = current_exe()?.parent().unwrap().join("config");
        // let config_path = current_exe.parent().unwrap().join("client_config.yaml");
        let res = Self::from_path(config_dir).await?;
        Ok(res)
    }

    pub async fn from_path<P: AsRef<Path>>(path: P) -> Result<Self> {
        if !path.as_ref().is_dir() {
            return Err(anyhow!("config file not found, or not a directory"));
        }

        let config_path = path.as_ref().join("config.yaml");
        let config_str = read_to_string(config_path)
            .await
            .expect("Cannot read config.yaml!");
        let mut config = serde_yaml::from_str::<Config>(&config_str)?;

        let keypair_path = path.as_ref().join("sec_key");
        let keypair_str = read_to_string(keypair_path)
            .await
            .expect("Cannot read sec_key!");
        config.keypair = Some(keypair_from_pem(&keypair_str)?);

        Ok(config)
    }
}
