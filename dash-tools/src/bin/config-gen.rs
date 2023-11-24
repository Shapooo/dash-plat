use dash_node::{
    config::{Config, PeerConfig},
    crypto,
};

use std::io::Write;
use std::path::PathBuf;
use std::{fs::OpenOptions, time::Duration};

use anyhow::{anyhow, Result};
use clap::Parser;
use serde_yaml;

#[derive(Debug, Parser)]
#[command(
    name = "config-gen",
    version = "0.1.0",
    author = "Shapooo",
    about = "Generate config/keypair files"
)]
struct Cli {
    /// Number of cnofig/keypairs to generate
    #[arg(short, long, default_value = "4")]
    pub count: u16,
    /// Generate keypair only, default is false
    #[arg(long, default_value = "false")]
    pub keypair: bool,
    /// Output path
    #[arg(short, long, default_value = "./")]
    pub output_path: PathBuf,
    /// Start port
    #[arg(short, long, default_value = "3000")]
    pub start_port: u16,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    if !cli.output_path.is_dir() {
        return Err(anyhow!("output path is not a directory"));
    }
    if cli.keypair {
        (0..cli.count)
            .map(|n| gen_keypair_file(cli.output_path.join(n.to_string())))
            .collect::<Result<Vec<_>>>()?;
    } else {
        if cli.count as u32 + cli.start_port as u32 > u16::MAX as u32 {
            return Err(anyhow!("port overflow"));
        }
        (0..cli.count)
            .map(|n| gen_config_file(cli.output_path.join(n.to_string()), cli.start_port + n))
            .collect::<Result<Vec<_>>>()?;
    }
    Ok(())
}

fn gen_config_file(mut path: PathBuf, port: u16) -> Result<()> {
    let keypair = crypto::generate_keypair();
    let pubkey_bytes = keypair.public.to_bytes();
    let pem = crypto::keypair_to_pem(keypair);
    let name = path.file_name().unwrap().to_str().unwrap().to_string();

    path.set_file_name(name.clone() + ".sec");
    let mut seckey_file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&path)
        .unwrap();
    seckey_file.write_all(pem.as_bytes()).unwrap();

    path.set_file_name(name.clone() + ".peerconfig.yaml");
    let peer_config = PeerConfig {
        host_addr: ("127.0.0.1:".to_string() + &port.to_string()).parse()?,
        public_key: pubkey_bytes,
    };
    let peer_config_str = serde_yaml::to_string(&peer_config)?;
    let mut peer_config_file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&path)
        .unwrap();
    peer_config_file
        .write_all(peer_config_str.as_bytes())
        .unwrap();

    path.set_file_name(name + ".config.yaml");
    let config = Config {
        validators: Default::default(),
        peer_addresses: Default::default(),
        my_keypair: None,
        host_address: ("127.0.0.1:".to_string() + &port.to_string()).parse()?,
        minimum_view_timeout: Duration::from_millis(500),
        sync_request_limit: 100,
        sync_response_timeout: Duration::from_millis(5000),
    };
    let config_str = serde_yaml::to_string(&config)?;
    let mut config_file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&path)
        .unwrap();
    config_file.write_all(config_str.as_bytes()).unwrap();

    Ok(())
}

fn gen_keypair_file(mut path: PathBuf) -> Result<()> {
    let keypair = crypto::generate_keypair();
    let pubkey_bytes = keypair.public.to_bytes();
    let pem = crypto::keypair_to_pem(keypair);
    let pk_b64 = crypto::publickey_to_base64(pubkey_bytes);
    let name = path.file_name().unwrap().to_str().unwrap().to_string();

    path.set_file_name(name.clone() + ".sec");
    let mut keypair_file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&path)
        .unwrap();
    keypair_file.write_all(pem.as_bytes()).unwrap();

    path.set_file_name(name + ".pub");
    let mut pubkey_file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(path)
        .unwrap();
    pubkey_file.write_all(pk_b64.as_bytes()).unwrap();

    Ok(())
}
