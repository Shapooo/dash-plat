use dash_plat::{app, config::Config, kv_store::KVStoreImpl, network::NetworkImpl};

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
    let config = Config::new().unwrap();
    let app = app::AppImpl::new();
    let mut initial_validators = ValidatorSetUpdates::new();
    config.validators.iter().for_each(|pubkey| {
        initial_validators.insert(pubkey.clone(), 0);
    });

    let kv_store = KVStoreImpl::default();

    Replica::initialize(kv_store.clone(), AppStateUpdates::new(), initial_validators);
    let network = NetworkImpl::new(config.peer_addresses, config.host_address);

    let pacemaker = DefaultPacemaker::new(
        config.minimum_view_timeout,
        config.sync_request_limit,
        config.sync_response_timeout,
    );
    let _replica = Replica::start(app, config.my_keypair, network, kv_store, pacemaker);
    loop {}
}
