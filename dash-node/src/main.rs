use dash_node::{app, config::Config, kv_store::KVStoreImpl, network::NetworkImpl};

use clap::{Arg, Command};
use hotstuff_rs::{
    pacemaker::DefaultPacemaker,
    replica::Replica,
    types::{AppStateUpdates, ValidatorSetUpdates},
};
use log::LevelFilter;
use simple_logger::SimpleLogger;

fn main() {
    SimpleLogger::new()
        .with_level(LevelFilter::Debug)
        .env()
        .init()
        .unwrap();
    let args = Command::new("dash-node")
        .about("Validator node for dash.")
        .version("0.1.0")
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
    let config = if let Some(path) = args.get_one::<String>("config") {
        Config::from_path(path).unwrap()
    } else {
        Config::new().unwrap()
    };

    let app = app::AppImpl::new();
    let mut initial_validators = ValidatorSetUpdates::new();

    config.validators.iter().for_each(|pubkey| {
        initial_validators.insert(pubkey.clone(), 1);
    });

    let kv_store = KVStoreImpl::default();

    Replica::initialize(kv_store.clone(), AppStateUpdates::new(), initial_validators);
    let keypair = config.my_keypair.unwrap();
    let network = NetworkImpl::new(
        config.peer_addresses,
        config.host_address,
        keypair.public.to_bytes(),
    );

    let pacemaker = DefaultPacemaker::new(
        config.minimum_view_timeout,
        config.sync_request_limit,
        config.sync_response_timeout,
    );
    let _replica = Replica::start(app, keypair, network, kv_store, pacemaker);
    loop {}
}
