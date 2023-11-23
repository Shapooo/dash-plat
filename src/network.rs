use std::collections::HashMap;
use std::io::Write;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::{
    mpsc::{channel, Receiver, Sender, TryRecvError},
    Arc, Mutex, RwLock, TryLockError,
};
use std::thread::spawn;

use borsh::{BorshDeserialize, BorshSerialize};
use hotstuff_rs::{
    messages::Message as HsMessage,
    networking,
    types::{PublicKeyBytes, ValidatorSet, ValidatorSetUpdates},
};
use log::{error, trace};

#[derive(Clone)]
#[allow(dead_code)]
pub struct NetworkImpl {
    validator_set: Arc<RwLock<ValidatorSet>>,
    peer_addresses: Arc<RwLock<HashMap<PublicKeyBytes, SocketAddr>>>,
    tx_sender: Sender<Message>,
    rx_receiver: Arc<Mutex<Receiver<Message>>>,
    host_addr: SocketAddr,
}

impl NetworkImpl {
    pub fn new(
        initial_peers: HashMap<PublicKeyBytes, SocketAddr>,
        host_addr: SocketAddr,
        public_key: PublicKeyBytes,
    ) -> Self {
        let peer_address = Arc::new(RwLock::new(initial_peers));
        let (tx_sender, tx_receiver) = channel::<Message>();
        let (rx_sender, rx_receiver) = channel::<Message>();

        // TODO: impl receiving thread
        spawn(move || {
            let listener = TcpListener::bind(host_addr).unwrap();
            loop {
                match listener.accept() {
                    Ok((mut socket, _addr)) => {
                        let msg = Message::deserialize_reader(&mut socket).unwrap();
                        rx_sender.send(msg).unwrap();
                    }
                    Err(e) => println!("couldn't get client: {e:?}"),
                }
            }
        });
        {
            // TODO: exception handle when connect
            let peer_address = peer_address.clone();
            spawn(move || {
                let mut connections: HashMap<PublicKeyBytes, TcpStream> = HashMap::new();
                loop {
                    let msg = tx_receiver.recv().unwrap();
                    trace!("send msg to {}", crate::crypto::publickey_to_base64(msg.0));
                    let mut stream = match connections.get(&msg.0) {
                        Some(stream) => stream,
                        None => {
                            let addr = peer_address.read().unwrap().get(&msg.0).unwrap().clone();
                            let stream = TcpStream::connect(addr).unwrap();
                            stream.set_write_timeout(None).unwrap();
                            connections.insert(msg.0.clone(), stream);
                            connections.get(&msg.0).unwrap()
                        }
                    };
                    let msg = Message(public_key, msg.1);
                    stream.write(&msg.try_to_vec().unwrap()).unwrap();
                }
            });
        }
        Self {
            validator_set: Arc::new(RwLock::new(ValidatorSet::new())),
            peer_addresses: peer_address,
            tx_sender,
            rx_receiver: Arc::new(Mutex::new(rx_receiver)),
            host_addr,
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

    fn broadcast(&mut self, message: HsMessage) {
        let validators: Vec<_> = self
            .validator_set
            .read()
            .unwrap()
            .validators()
            .copied()
            .collect();
        for peer in validators {
            self.send(peer, message.clone());
        }
    }

    fn send(&mut self, peer: PublicKeyBytes, message: HsMessage) {
        self.tx_sender.send(Message(peer, message)).unwrap();
    }

    fn recv(&mut self) -> Option<(PublicKeyBytes, HsMessage)> {
        let chan = match self.rx_receiver.try_lock() {
            Ok(chan) => chan,
            Err(TryLockError::WouldBlock) => return None,
            Err(e) => Err(e).unwrap(),
        };
        match chan.try_recv() {
            Ok(msg) => Some((msg.0, msg.1)),
            Err(TryRecvError::Empty) => None,
            Err(e) => Err(e).unwrap(),
        }
    }
}

// TODO: add Signature
#[derive(BorshDeserialize, BorshSerialize)]
struct Message(PublicKeyBytes, HsMessage);

#[cfg(test)]
mod network_test {
    use super::*;
    use hotstuff_rs::{
        messages::{Message as HsMessage, ProgressMessage, Proposal},
        types::{Block, QuorumCertificate},
    };
    use networking::Network;

    #[test]
    fn new_test() {
        NetworkImpl::new(
            Default::default(),
            "127.0.0.1:8082".parse().unwrap(),
            [0; 32],
        );
    }

    #[test]
    fn send_receive_test() {
        let mut peers: HashMap<[u8; 32], SocketAddr> = HashMap::new();
        let sender_addr = "127.0.0.1:8080".parse().unwrap();
        let receiver_addr = "127.0.0.1:8081".parse().unwrap();
        let sender_pubkey = [0; 32];
        let receiver_pubkey = [1; 32];
        peers.insert(sender_pubkey, sender_addr);
        peers.insert(receiver_pubkey, receiver_addr);
        let mut receiver = NetworkImpl::new(peers.clone(), receiver_addr, receiver_pubkey);
        let mut sender = NetworkImpl::new(peers, sender_addr, sender_pubkey);
        sender.send(
            receiver_pubkey,
            HsMessage::ProgressMessage(ProgressMessage::Proposal(Proposal {
                chain_id: 0,
                view: 0,
                block: Block::new(
                    0,
                    QuorumCertificate::genesis_qc(),
                    Default::default(),
                    Default::default(),
                ),
            })),
        );

        loop {
            if let Some((key, _msg)) = receiver.recv() {
                println!("received {:?}", key);
                break;
            }
        }
    }
}
