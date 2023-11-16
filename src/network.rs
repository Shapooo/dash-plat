use std::collections::HashMap;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::{
    mpsc::{channel, Receiver, Sender},
    Arc, Mutex, RwLock,
};
use std::thread::spawn;

use hotstuff_rs::{
    messages::Message,
    networking,
    types::{PublicKeyBytes, ValidatorSet, ValidatorSetUpdates},
};

#[derive(Clone)]
pub struct NetworkImpl {
    validator_set: Arc<RwLock<ValidatorSet>>,
    address_map: Arc<RwLock<HashMap<PublicKeyBytes, SocketAddr>>>,
    tx_sender: Sender<Message>,
    rx_receiver: Arc<Mutex<Receiver<Message>>>,
}

impl NetworkImpl {
    pub fn new() -> Self {
        let (tx_sender, tx_receiver) = channel::<Message>();
        let (rx_sender, rx_receiver) = channel::<Message>();
        spawn(move || {
            let mut connection_map: HashMap<PublicKeyBytes, TcpStream> = HashMap::new();
            let listener = TcpListener::bind("127.0.0.1:8080").unwrap();
            loop {
                match listener.accept() {
                    Ok((_socket, addr)) => println!("new client: {addr:?}"),
                    Err(e) => println!("couldn't get client: {e:?}"),
                }
            }
        });
        spawn(move || {
            let mut connection_map: HashMap<PublicKeyBytes, TcpStream> = HashMap::new();
            let listener = TcpListener::bind("127.0.0.1:8080").unwrap();
            loop {
                let msg = tx_receiver.recv().unwrap();
                // TODO
            }
        });
        Self {
            validator_set: Arc::new(RwLock::new(ValidatorSet::new())),
            address_map: Arc::new(RwLock::new(HashMap::new())),
            tx_sender,
            rx_receiver: Arc::new(Mutex::new(rx_receiver)),
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

    fn broadcast(&mut self, message: Message) {
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

    fn send(&mut self, peer: PublicKeyBytes, message: Message) {
        self.tx_sender.send(message).unwrap();
    }

    fn recv(&mut self) -> Option<(PublicKeyBytes, Message)> {
        todo!()
    }
}
