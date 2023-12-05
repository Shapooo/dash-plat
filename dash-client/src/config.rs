use std::env::current_exe;
use std::net::SocketAddr;
use std::path::Path;

use anyhow::{anyhow, Ok, Result};
use serde::{Deserialize, Serialize};
use tokio::fs::read_to_string;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub node_addrs: Vec<SocketAddr>,
}

impl Config {
    pub async fn new() -> Result<Self> {
        let current_exe = current_exe()?;
        let config_path = current_exe.parent().unwrap().join("client_config.yaml");
        let res = Self::from_path(config_path).await?;
        Ok(res)
    }

    pub async fn from_path<P: AsRef<Path>>(path: P) -> Result<Self> {
        if !path.as_ref().is_file() {
            return Err(anyhow!("config file not found, or not a file"));
        }

        let config_str = read_to_string(path)
            .await
            .expect("Cannot read config.yaml!");
        let config = serde_yaml::from_str::<Config>(&config_str)?;
        Ok(config)
    }
}
