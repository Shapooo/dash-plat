use dash_node::{
    app,
    client_actor::ClientActor,
    config::Config,
    kv_store::KVStoreImpl,
    network::{NetConfig, NetworkImpl},
};

use std::sync::Arc;

use anyhow::Result;
use clap::Arg;
use hotstuff_rs::{
    pacemaker::DefaultPacemaker,
    replica::Replica,
    types::{AppStateUpdates, ValidatorSetUpdates},
};
use log::LevelFilter;
use simple_logger::SimpleLogger;
use tokio::{runtime::Builder, sync::mpsc::channel};

fn main() -> Result<()> {
    init_logger()?;
    let config = init_config()?;

    let kv_store = KVStoreImpl::default();
    let rt = Arc::new(Builder::new_multi_thread().enable_all().build().unwrap());
    let (block_sender, block_receiver) = channel(1000);
    let app = app::AppImpl::new(block_receiver);
    let mut initial_validators = ValidatorSetUpdates::new();

    config.validators.iter().for_each(|pubkey| {
        initial_validators.insert(*pubkey, 1);
    });

    Replica::initialize(kv_store.clone(), AppStateUpdates::new(), initial_validators);
    let keypair = config
        .my_keypair
        .expect("FATAL: my keypair not initialized!");
    let public_key = keypair.public.to_bytes();
    let net_config = NetConfig {
        listen_addr: config.peer_listen_addr,
        public_key,
        initial_peers: config.peer_addresses,
    };
    let network = NetworkImpl::new(net_config, rt.clone());

    let pacemaker = DefaultPacemaker::new(
        config.minimum_view_timeout,
        config.sync_request_limit,
        config.sync_response_timeout,
    );
    let _replica = Replica::start(app, keypair, network, kv_store, pacemaker);
    ClientActor::spawn(
        public_key,
        config.client_listen_addr,
        block_sender,
        Arc::new(_replica),
        rt,
    );
    loop {
        std::thread::sleep(std::time::Duration::from_secs(u64::MAX));
    }
}

fn init_logger() -> Result<()> {
    SimpleLogger::new()
        .with_level(LevelFilter::Debug)
        .with_module_level("hotstuff_rs", LevelFilter::Warn)
        .env()
        .init()?;
    Ok(())
}

fn init_config() -> Result<Config> {
    let args = clap::command!()
        .arg(
            Arg::new("config")
                .short('c')
                .long("config_dir")
                .action(clap::ArgAction::Set)
                .help(
                    "set config directory, defaults to `config/' \
                     in the same directory as dash-node binary",
                ),
        )
        .get_matches();
    if let Some(path) = args.get_one::<String>("config") {
        Config::from_path(path)
    } else {
        Config::new()
    }
}
