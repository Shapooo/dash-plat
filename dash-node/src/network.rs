use dash_common::crypto::publickey_to_base64;
use dash_network::{client, server};

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex, RwLock, TryLockError};
use std::thread;
use std::time::Duration;

use borsh::{BorshDeserialize, BorshSerialize};
use bytes::Bytes;
use hotstuff_rs::{
    messages::Message as InnerMessage,
    networking,
    types::{PublicKeyBytes, ValidatorSet, ValidatorSetUpdates},
};
use log::warn;
use tokio::{
    runtime::Runtime,
    sync::mpsc::{channel, error::TryRecvError, Receiver, Sender},
};

pub struct NetConfig {
    pub initial_peers: HashMap<PublicKeyBytes, SocketAddr>,
    pub public_key: PublicKeyBytes,
    pub listen_addr: SocketAddr,
}

#[derive(Clone)]
#[allow(dead_code)]
pub struct NetworkImpl {
    validator_set: Arc<RwLock<ValidatorSet>>,
    peer_addresses: Arc<HashMap<PublicKeyBytes, SocketAddr>>,
    address_peers: Arc<HashMap<SocketAddr, PublicKeyBytes>>,
    my_publickey: PublicKeyBytes,
    tx_sender: Sender<(PublicKeyBytes, Bytes)>,
    rx_receiver: Arc<Mutex<Receiver<(PublicKeyBytes, Bytes)>>>,
    listen_addr: SocketAddr,
}

impl NetworkImpl {
    pub fn new(config: NetConfig, rt: Arc<Runtime>) -> Self {
        let address_peers = Arc::new(
            config
                .initial_peers
                .iter()
                .map(|(key, addr)| (*addr, *key))
                .collect::<HashMap<_, _>>(),
        );
        let peer_addresses = Arc::new(config.initial_peers);

        let (tx_sender, tx_receiver) = channel(1000);
        let (rx_sender, rx_receiver) = channel(1000);

        let network = Self {
            validator_set: Arc::new(RwLock::new(ValidatorSet::new())),
            peer_addresses: peer_addresses.clone(),
            address_peers: address_peers.clone(),
            my_publickey: config.public_key,
            tx_sender,
            rx_receiver: Arc::new(Mutex::new(rx_receiver)),
            listen_addr: config.listen_addr,
        };

        thread::spawn(move || {
            rt.block_on(async {
                dispatching(
                    config.listen_addr,
                    tx_receiver,
                    rx_sender,
                    peer_addresses,
                    // address_peers,
                )
                .await;
            });
        });

        network
    }
}

async fn dispatching(
    listening_addr: SocketAddr,
    mut tx_receiver: Receiver<(PublicKeyBytes, Bytes)>,
    rx_sender: Sender<(PublicKeyBytes, Bytes)>,
    peer_addresses: Arc<HashMap<PublicKeyBytes, SocketAddr>>,
    // address_peers: Arc<HashMap<SocketAddr, PublicKeyBytes>>,
) {
    tokio::spawn(async move {
        let (sender, _receiver) = client::Client::spawn();
        while let Some((key, msg)) = tx_receiver.recv().await {
            if let Some(addr) = peer_addresses.get(&key) {
                sender.send((*addr, msg)).await.unwrap();
            } else {
                warn!("Cannot find addr of {}", publickey_to_base64(key));
            }
        }
    });
    tokio::spawn(async move {
        let (_sender, mut receiver) = server::Server::spawn(listening_addr);
        while let Some((_addr, msg)) = receiver.recv().await {
            rx_sender.send((Default::default(), msg)).await.unwrap();
        }
    });
    loop {
        tokio::time::sleep(Duration::from_secs(u64::MAX)).await;
    }
}

impl networking::Network for NetworkImpl {
    fn init_validator_set(&mut self, validator_set: ValidatorSet) {
        self.validator_set = Arc::new(RwLock::new(validator_set));
    }

    fn update_validator_set(&mut self, updates: ValidatorSetUpdates) {
        self.validator_set.write().unwrap().apply_updates(&updates);
    }

    fn broadcast(&mut self, message: InnerMessage) {
        let validators: Vec<_> = self
            .validator_set
            .read()
            .unwrap()
            .validators()
            .copied()
            .collect();
        for peer in validators {
            networking::Network::send(self, peer, message.clone());
        }
    }

    fn send(&mut self, peer: PublicKeyBytes, message: InnerMessage) {
        let msg = Message {
            from: self.my_publickey,
            to: peer,
            data: message,
        }
        .try_to_vec()
        .unwrap()
        .into();
        self.tx_sender.blocking_send((peer, msg)).unwrap();
    }

    fn recv(&mut self) -> Option<(PublicKeyBytes, InnerMessage)> {
        let mut chan = match self.rx_receiver.try_lock() {
            Ok(chan) => chan,
            Err(TryLockError::WouldBlock) => return None,
            Err(e) => panic!("{:?}", e),
        };
        match chan.try_recv() {
            Ok((_, data)) => {
                let Message { from, to, data } = Message::deserialize(&mut data.as_ref()).unwrap();
                if to != self.my_publickey {
                    warn!("Not my message, droped!");
                }
                Some((from, data))
            }
            Err(TryRecvError::Empty) => None,
            Err(e) => panic!("{:?}", e),
        }
    }
}

// TODO: add Signature
#[derive(BorshDeserialize, BorshSerialize)]
struct Message {
    from: PublicKeyBytes,
    to: PublicKeyBytes,
    data: InnerMessage,
}
