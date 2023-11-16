use std::collections::HashMap;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::{
    mpsc::{channel, Receiver, Sender, TryRecvError},
    Arc, Mutex, RwLock, TryLockError,
};
use std::thread::spawn;

use hotstuff_rs::{
    messages::Message as HsMessage,
    networking,
    types::{PublicKeyBytes, ValidatorSet, ValidatorSetUpdates},
};

#[derive(Clone)]
pub struct NetworkImpl {
    validator_set: Arc<RwLock<ValidatorSet>>,
    peer_address: Arc<RwLock<HashMap<PublicKeyBytes, SocketAddr>>>,
    tx_sender: Sender<Message>,
    rx_receiver: Arc<Mutex<Receiver<Message>>>,
    host_addr: SocketAddr,
}

impl NetworkImpl {
    pub fn new(initial_peers: HashMap<PublicKeyBytes, SocketAddr>, host_addr: SocketAddr) -> Self {
        let peer_address = Arc::new(RwLock::new(initial_peers));
        let (tx_sender, tx_receiver) = channel::<Message>();
        let (rx_sender, rx_receiver) = channel::<Message>();

        // TODO: impl receiving thread
        spawn(move || {
            let listener = TcpListener::bind(host_addr).unwrap();
            loop {
                match listener.accept() {
                    Ok((_socket, addr)) => println!("new client: {addr:?}"),
                    Err(e) => println!("couldn't get client: {e:?}"),
                }
            }
        });
        {
            // TODO: send msg
            // TODO: exception handle when connect
            let peer_address = peer_address.clone();
            spawn(move || {
                let mut connection_map: HashMap<PublicKeyBytes, TcpStream> = HashMap::new();
                loop {
                    let msg = tx_receiver.recv().unwrap();
                    let stream = match connection_map.get(&msg.0) {
                        Some(stream) => stream,
                        None => {
                            let addr = peer_address.read().unwrap().get(&msg.0).unwrap().clone();
                            let stream = TcpStream::connect(addr).unwrap();
                            connection_map.insert(msg.0.clone(), stream);
                            connection_map.get(&msg.0).unwrap()
                        }
                    };
                }
            });
        }
        Self {
            validator_set: Arc::new(RwLock::new(ValidatorSet::new())),
            peer_address,
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
struct Message(PublicKeyBytes, HsMessage);
