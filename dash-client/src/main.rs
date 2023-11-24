#![allow(dead_code, unused_imports, unused_variables)]
use dash_common::BlockMsg;

use std::env::current_exe;
use std::fs::File;
use std::io::Read;
use std::net::SocketAddr;

use anyhow::Result;
use serde::Deserialize;

fn main() -> Result<()> {
    let config = get_config()?;
    Ok(())
}

fn get_config() -> Result<Config> {
    let config_path = current_exe()?.parent().unwrap().join("config.yaml");
    let mut config_file = File::open(config_path)?;
    let mut config_str = String::new();
    config_file.read_to_string(&mut config_str)?;
    let config: Config = serde_yaml::from_str(&config_str)?;
    Ok(config)
}

#[derive(Clone, Debug, Deserialize)]
struct Config {
    peers: Vec<SocketAddr>,
    blocks_per_min: u64,
}

struct BlockGen {}

struct Network {}
