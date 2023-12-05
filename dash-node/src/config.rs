use crate::crypto;

use std::collections::{HashMap, HashSet};
use std::env::current_exe;
use std::fs::{read_dir, read_to_string, File};
use std::io::Read;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{anyhow, Result};
use hotstuff_rs::types::{DalekKeypair, PublicKeyBytes};
use log::debug;
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub peer_listen_addr: SocketAddr,
    pub client_listen_addr: SocketAddr,
    #[serde(skip)]
    pub my_keypair: Option<DalekKeypair>,
    #[serde(skip)]
    pub peer_addresses: HashMap<PublicKeyBytes, SocketAddr>,
    #[serde(skip)]
    pub validators: HashSet<PublicKeyBytes>,
    #[serde(
        deserialize_with = "parse_milliseconds",
        serialize_with = "serialize_milliseconds",
        rename = "minimum_view_timeout_ms"
    )]
    pub minimum_view_timeout: Duration,
    pub sync_request_limit: u32,
    #[serde(
        deserialize_with = "parse_milliseconds",
        serialize_with = "serialize_milliseconds",
        rename = "sync_response_timeout_ms"
    )]
    pub sync_response_timeout: Duration,
}

impl Config {
    pub fn from_path<P: AsRef<Path>>(config_dir: P) -> Result<Self> {
        if !config_dir.as_ref().is_dir() {
            return Err(anyhow!("config dir not found, or not a directory"));
        }
        let config_path = config_dir.as_ref().join("config.yaml");
        let peers_dir = config_dir.as_ref().join("peers");
        let seckey_path = config_dir.as_ref().join("sec_key");
        let config_str = read_to_string(config_path).expect("Cannot read config.yaml!");
        let mut res = serde_yaml::from_str::<Config>(&config_str)?;
        let pem = read_to_string(seckey_path).expect("Cannot read sec_key file!");
        let keypair = crypto::keypair_from_pem(&pem)?;
        debug!(
            "my pubkey is {}",
            crypto::publickey_to_base64(keypair.public.to_bytes())
        );
        res.my_keypair = Some(keypair);
        res.load_peers(peers_dir);
        Ok(res)
    }

    pub fn new() -> Result<Self> {
        let current_exe = current_exe()?;
        let config_dir = current_exe.parent().unwrap().join("config");
        let res = Self::from_path(config_dir)?;
        Ok(res)
    }

    fn load_peers(&mut self, peers_dir: PathBuf) {
        let confs_entry: Result<Vec<_>, _> = read_dir(peers_dir)
            .expect("Cannot access peers directory!")
            .collect();
        self.peer_addresses = confs_entry
            .expect("Cannot access peers directory!")
            .into_iter()
            .filter_map(|entry| {
                if entry.path().is_file() {
                    let mut f = File::open(entry.path()).expect("Cannot open the peer config!");
                    let mut buf = String::new();
                    f.read_to_string(&mut buf)
                        .expect("Cannot read the peer config!");
                    let conf = serde_yaml::from_str::<PeerConfig>(&buf)
                        .expect("Parse the peer config failed!");
                    Some((conf.public_key, conf.host_addr))
                } else {
                    None
                }
            })
            .collect();
        self.validators = self.peer_addresses.keys().copied().collect();
    }
}

fn parse_milliseconds<'de, D>(d: D) -> Result<Duration, D::Error>
where
    D: Deserializer<'de>,
{
    let millisecs: u64 = Deserialize::deserialize(d)?;
    Ok(Duration::from_millis(millisecs))
}

#[derive(Clone, Deserialize, Serialize)]
pub struct PeerConfig {
    pub host_addr: SocketAddr,
    #[serde(deserialize_with = "parse_pubkey", serialize_with = "serialize_pubkey")]
    pub public_key: PublicKeyBytes,
}

fn parse_pubkey<'de, D>(d: D) -> Result<PublicKeyBytes, D::Error>
where
    D: Deserializer<'de>,
{
    let pubkey: String = Deserialize::deserialize(d)?;
    Ok(crypto::publickey_from_base64(&pubkey).expect("Parse public key from base64 failed!"))
}

fn serialize_pubkey<S>(key: &PublicKeyBytes, s: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    s.serialize_str(&crypto::publickey_to_base64(*key))
}

fn serialize_milliseconds<S>(duration: &Duration, s: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    s.serialize_u128(duration.as_millis())
}
