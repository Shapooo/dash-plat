use std::collections::{HashMap, VecDeque};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex, RwLock, TryLockError};
use std::thread;
use std::time::Duration;

use anyhow::Result;
use borsh::{BorshDeserialize, BorshSerialize};
use bytes::Bytes;
use futures::{SinkExt, StreamExt};
use hotstuff_rs::{
    messages::Message as HsMessage,
    networking,
    types::{PublicKeyBytes, ValidatorSet, ValidatorSetUpdates},
};
use log::{error, trace, warn};
use tokio::{
    net::{TcpListener, TcpStream},
    runtime::Builder,
    sync::mpsc::{channel, error::TryRecvError, Receiver, Sender},
    time,
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
        let peer_addresses = Arc::new(initial_peers);

        let (tx_sender, rx_receiver) =
            NetworkDispatcher::spawn(host_addr, peer_addresses.clone(), public_key);

        Self {
            validator_set: Arc::new(RwLock::new(ValidatorSet::new())),
            peer_addresses,
            tx_sender,
            rx_receiver: Arc::new(Mutex::new(rx_receiver)),
            host_addr,
        }
    }
}

#[allow(dead_code)]
struct NetworkDispatcher {
    host_addr: SocketAddr,
    rx_sender: Sender<Message>,
    tx_receiver: Receiver<Message>,
    peer_addresses: Arc<HashMap<PublicKeyBytes, SocketAddr>>,
    public_key: PublicKeyBytes,
}

impl NetworkDispatcher {
    fn spawn(
        host_addr: SocketAddr,
        peer_addresses: Arc<HashMap<PublicKeyBytes, SocketAddr>>,
        public_key: PublicKeyBytes,
    ) -> (Sender<Message>, Receiver<Message>) {
        let (tx_sender, tx_receiver) = channel::<Message>(1000);
        let (rx_sender, rx_receiver) = channel::<Message>(1000);
        thread::spawn(move || {
            let rt = Builder::new_current_thread().enable_all().build().unwrap();
            rt.block_on(async move {
                Self {
                    host_addr,
                    rx_sender,
                    tx_receiver,
                    peer_addresses,
                    public_key,
                }
                .run()
                .await
            });
        });
        (tx_sender, rx_receiver)
    }

    async fn run(&mut self) {
        // spawn listening thread
        let host_addr = self.host_addr;
        let rx_sender = self.rx_sender.clone();
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
        let mut senders: HashMap<PublicKeyBytes, _> = self
            .peer_addresses
            .iter()
            .map(|(key, addr)| (*key, SendingWorker::spawn(*addr)))
            .collect();
        while let Some(msg) = self.tx_receiver.recv().await {
            trace!("send msg to {}", crate::crypto::publickey_to_base64(msg.0));
            let sender = senders.entry(msg.0).or_insert_with(|| {
                let addr = *self.peer_addresses.get(&msg.0).unwrap();
                SendingWorker::spawn(addr)
            });
            let msg = Message(self.public_key, msg.1);
            sender.send(msg).await.unwrap();
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

struct SendingWorker {
    remote_addr: SocketAddr,
    receiver: Receiver<Message>,
    buffer: VecDeque<Message>,
}

impl SendingWorker {
    fn spawn(remote_addr: SocketAddr) -> Sender<Message> {
        let (sender, receiver) = channel(1000);
        tokio::spawn(async move {
            Self {
                remote_addr,
                receiver,
                buffer: Default::default(),
            }
            .run()
            .await
        });
        sender
    }

    async fn run(&mut self) {
        let mut delay = 200;
        let mut retry = 0;
        loop {
            match TcpStream::connect(self.remote_addr).await {
                Ok(stream) => {
                    trace!("Outgoing connection established with {}", self.remote_addr);

                    // Reset the delay.
                    delay = 200;
                    retry = 0;

                    // Try to transmit all messages in the buffer and keep transmitting incoming messages.
                    // The following function only returns if there is an error.
                    if let Err(e) = self.keep_alive(stream).await {
                        warn!("{}", e);
                    }
                }
                Err(e) => {
                    warn!(
                        "connect to {}, retry {} times, reason {}",
                        self.remote_addr, retry, e
                    );
                    let timer = time::sleep(Duration::from_millis(delay));
                    tokio::pin!(timer);

                    'waiter: loop {
                        tokio::select! {
                            // Wait an increasing delay before attempting to reconnect.
                            () = &mut timer => {
                                delay = std::cmp::min(2*delay, 60_000);
                                retry +=1;
                                break 'waiter;
                            },

                            // Drain the channel into the buffer to not saturate the channel and block the caller task.
                            // The caller is responsible to cleanup the buffer through the cancel handlers.
                            Some(msg) = self.receiver.recv() => {
                                self.buffer.push_back(msg);
                                if self.buffer.len() > 1000 {
                                    warn!("400 msg droped");
                                    self.buffer.drain(0..400);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    async fn keep_alive(&mut self, stream: TcpStream) -> Result<()> {
        let mut writer = Framed::new(stream, LengthDelimitedCodec::new());
        loop {
            while let Some(msg) = self.buffer.pop_front() {
                let msg_bytes: Bytes = msg.try_to_vec()?.into();
                trace!("send msg to {} in keep_alive", self.remote_addr);
                writer.send(msg_bytes).await?;
            }
            while let Some(msg) = self.receiver.recv().await {
                let msg_bytes: Bytes = msg.try_to_vec()?.into();
                trace!("send msg to {} in keep_alive", self.remote_addr);
                writer.send(msg_bytes).await?;
            }
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
