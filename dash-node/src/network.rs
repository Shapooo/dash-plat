use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex, RwLock, TryLockError};
use std::thread::spawn;

use borsh::{BorshDeserialize, BorshSerialize};
use bytes::Bytes;
use futures::{SinkExt, StreamExt};
use hotstuff_rs::{
    messages::Message as HsMessage,
    networking,
    types::{PublicKeyBytes, ValidatorSet, ValidatorSetUpdates},
};
use log::{error, trace};
use tokio::{
    net::{TcpListener, TcpStream},
    runtime::Builder,
    sync::mpsc::{channel, error::TryRecvError, Receiver, Sender},
};
use tokio_util::codec::{Framed, LengthDelimitedCodec};

#[derive(Clone)]
#[allow(dead_code)]
pub struct NetworkImpl {
    validator_set: Arc<RwLock<ValidatorSet>>,
    peer_addresses: Arc<HashMap<PublicKeyBytes, SocketAddr>>,
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
        let peer_address = Arc::new(initial_peers);
        let (tx_sender, tx_receiver) = channel::<Message>(1000);
        let (rx_sender, rx_receiver) = channel::<Message>(1000);

        let res = Self {
            validator_set: Arc::new(RwLock::new(ValidatorSet::new())),
            peer_addresses: peer_address.clone(),
            tx_sender,
            rx_receiver: Arc::new(Mutex::new(rx_receiver)),
            host_addr,
        };

        spawn(move || {
            let rt = Builder::new_current_thread().enable_all().build().unwrap();
            rt.block_on(async move {
                spawn_working_threads(host_addr, rx_sender, tx_receiver, peer_address, public_key)
                    .await;
            });
        });
        res
    }
}

async fn spawn_working_threads(
    host_addr: SocketAddr,
    rx_sender: Sender<Message>,
    mut tx_receiver: Receiver<Message>,
    peer_addresses: Arc<HashMap<PublicKeyBytes, SocketAddr>>,
    public_key: PublicKeyBytes,
) {
    // spawn listening thread
    tokio::spawn(async move {
        let listener = TcpListener::bind(host_addr)
            .await
            .expect("Failed to bind TCP port!");
        loop {
            match listener.accept().await {
                Ok((socket, addr)) => {
                    trace!("accept connection from {}", addr);
                    ReceivingWorker::spawn(addr, socket, rx_sender.clone());
                }
                Err(e) => error!("couldn't get client: {e:?}"),
            }
        }
    });

    // looping sending msg
    let mut connections: HashMap<PublicKeyBytes, TcpStream> = HashMap::new();
    while let Some(msg) = tx_receiver.recv().await {
        trace!("send msg to {}", crate::crypto::publickey_to_base64(msg.0));
        let stream = match connections.get_mut(&msg.0) {
            Some(stream) => Some(stream),
            None => {
                let addr = *peer_addresses.get(&msg.0).unwrap();
                TcpStream::connect(addr).await.ok().and_then(|stream| {
                    connections.insert(msg.0, stream);
                    connections.get_mut(&msg.0)
                })
            }
        };
        if let Some(stream) = stream {
            let mut writer = Framed::new(stream, LengthDelimitedCodec::new());
            let msg = Message(public_key, msg.1);
            let msg_bytes: Bytes = msg.try_to_vec().unwrap().into();
            writer.send(msg_bytes).await.unwrap();
        }
    }
}

struct ReceivingWorker {
    sender: Sender<Message>,
    remote_addr: SocketAddr,
    framed_reader: Framed<TcpStream, LengthDelimitedCodec>,
}

impl ReceivingWorker {
    fn spawn(remote_addr: SocketAddr, socket: TcpStream, sender: Sender<Message>) {
        let reader = Framed::new(socket, LengthDelimitedCodec::new());
        tokio::spawn(async move {
            Self {
                sender,
                remote_addr,
                framed_reader: reader,
            }
            .run()
            .await
        });
    }

    async fn run(&mut self) {
        while let Some(raw_msg) = self.framed_reader.next().await {
            match raw_msg {
                Ok(msg_bytes) => match Message::deserialize(&mut msg_bytes.as_ref()) {
                    Ok(msg) => {
                        trace!(
                            "received msg from: {} {}",
                            self.remote_addr,
                            crate::crypto::publickey_to_base64(msg.0)
                        );
                        self.sender.send(msg).await.unwrap()
                    }
                    Err(e) => error!("{}", e),
                },
                Err(e) => error!("{}", e),
            };
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
        self.tx_sender
            .blocking_send(Message(peer, message))
            .unwrap();
    }

    fn recv(&mut self) -> Option<(PublicKeyBytes, HsMessage)> {
        let mut chan = match self.rx_receiver.try_lock() {
            Ok(chan) => chan,
            Err(TryLockError::WouldBlock) => return None,
            Err(e) => panic!("{:?}", e),
        };
        match chan.try_recv() {
            Ok(msg) => Some((msg.0, msg.1)),
            Err(TryRecvError::Empty) => None,
            Err(e) => panic!("{:?}", e),
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
