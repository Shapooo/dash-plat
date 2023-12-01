use crate::{receiver::NetReceiver, sender::NetSender};

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
    pub listening_addr: SocketAddr,
}

#[derive(Clone)]
#[allow(dead_code)]
pub struct NetworkImpl {
    validator_set: Arc<RwLock<ValidatorSet>>,
    peer_addresses: Arc<HashMap<PublicKeyBytes, SocketAddr>>,
    address_peers: Arc<HashMap<SocketAddr, PublicKeyBytes>>,
    my_publickey: PublicKeyBytes,
    tx_sender: Sender<(SocketAddr, Bytes)>,
    rx_receiver: Arc<Mutex<Receiver<(SocketAddr, Bytes)>>>,
    listening_addr: SocketAddr,
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

        thread::spawn(move || {
            tokio::spawn(async {
                NetSender::spawn(tx_receiver);
                NetReceiver::spawn(config.listening_addr, rx_sender);
                loop {
                    tokio::time::sleep(Duration::from_secs(u64::MAX)).await;
                }
            });
        });

        Self {
            validator_set: Arc::new(RwLock::new(ValidatorSet::new())),
            peer_addresses,
            address_peers,
            my_publickey: config.public_key,
            tx_sender,
            rx_receiver: Arc::new(Mutex::new(rx_receiver)),
            listening_addr: config.listening_addr,
        }
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
        let peer_addr = match self.peer_addresses.get(&peer) {
            Some(addr) => *addr,
            None => {
                warn!("Unknown peer public key, message droped!");
                return;
            }
        };
        self.tx_sender.blocking_send((peer_addr, msg)).unwrap();
    }

    fn recv(&mut self) -> Option<(PublicKeyBytes, InnerMessage)> {
        let mut chan = match self.rx_receiver.try_lock() {
            Ok(chan) => chan,
            Err(TryLockError::WouldBlock) => return None,
            Err(e) => panic!("{:?}", e),
        };
        match chan.try_recv() {
            Ok((_remote_addr, data)) => {
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
