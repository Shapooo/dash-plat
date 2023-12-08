use crate::kv_store::KVStoreImpl;
use dash_common::{NewTransactionRequest, TransactionHash, TransactionReceipt, TransactionResult};
use dash_network::server::Server;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use borsh::{BorshDeserialize, BorshSerialize};
use bytes::Bytes;
use hotstuff_rs::{replica::Replica, types::PublicKeyBytes};
use log::{error, trace};
use tokio::{
    runtime::Runtime,
    sync::{
        mpsc::{channel, Receiver, Sender},
        Mutex,
    },
};

pub struct ClientActor();

impl ClientActor {
    pub fn spawn(
        pubkey: PublicKeyBytes,
        listen_addr: SocketAddr,
        block_sender: Sender<NewTransactionRequest>,
        replica: Arc<Replica<KVStoreImpl>>,
        rt: Arc<Runtime>,
    ) {
        let (sender, receiver) = channel(1000);
        let map: Arc<Mutex<HashMap<TransactionHash, PublicKeyBytes>>> = Default::default();
        CommitChecker::spawn(replica, sender, map.clone());
        thread::spawn(move || {
            rt.block_on(async {
                Actor::spawn(listen_addr, block_sender, receiver, pubkey, map);
                loop {
                    tokio::time::sleep(Duration::from_secs(u64::MAX)).await;
                }
            });
        });
    }
}

struct Actor {
    #[allow(unused)]
    listen_addr: SocketAddr,
    block_sender: Sender<NewTransactionRequest>,
    #[allow(unused)]
    net_sender: Sender<(SocketAddr, Bytes)>,
    net_receiver: Receiver<(SocketAddr, Bytes)>,
    requesters_addr_map: HashMap<PublicKeyBytes, SocketAddr>,
    block_requester_map: Arc<Mutex<HashMap<TransactionHash, PublicKeyBytes>>>,
    receiver: Receiver<(PublicKeyBytes, TransactionHash)>,
    pubkey: PublicKeyBytes,
}

impl Actor {
    fn spawn(
        listen_addr: SocketAddr,
        block_sender: Sender<NewTransactionRequest>,
        receiver: Receiver<(PublicKeyBytes, TransactionHash)>,
        pubkey: PublicKeyBytes,
        block_requester_map: Arc<Mutex<HashMap<TransactionHash, PublicKeyBytes>>>,
    ) {
        tokio::spawn(async move {
            let (net_sender, net_receiver) = Server::spawn(listen_addr);
            Self {
                listen_addr,
                block_sender,
                net_sender,
                net_receiver,
                requesters_addr_map: Default::default(),
                block_requester_map,
                receiver,
                pubkey,
            }
            .run()
            .await
        });
    }

    async fn run(&mut self) {
        loop {
            tokio::select! {
                Some((addr, msg_bytes)) = self.net_receiver.recv() => {
                    let mut msg_bytes = msg_bytes.as_ref();
                    if let Ok(request) = NewTransactionRequest::deserialize(&mut msg_bytes) {
                        self.requesters_addr_map.insert(request.requester, addr);
                        self.block_requester_map.lock().await.insert(request.hash, request.requester);
                        trace!("received request {:?}", request.hash);
                        self.block_sender.send(request).await.unwrap()
                    }
                }
                Some((pubkey, hash)) = self.receiver.recv() => {
                    trace!("commited {:?}", hash);
                    if let Some(addr) = self.requesters_addr_map.get(&pubkey) {
                        trace!("send recept to {}", addr);
                        self.net_sender
                            .send((
                                *addr,
                                TransactionReceipt {
                                    requester: pubkey,
                                    receiptor: self.pubkey,
                                    hash,
                                    result: TransactionResult::Commited,
                                }
                                .try_to_vec()
                                .unwrap()
                                .into(),
                            ))
                            .await
                            .unwrap();
                    } else {
                        error!("Unknown requester!");
                    }
                }
            }
        }
    }
}
struct CommitChecker {
    replica: Arc<Replica<KVStoreImpl>>,
    senders: Sender<(PublicKeyBytes, TransactionHash)>,
    block_sender_map: Arc<Mutex<HashMap<TransactionHash, PublicKeyBytes>>>,
}

impl CommitChecker {
    fn spawn(
        replica: Arc<Replica<KVStoreImpl>>,
        sender: Sender<(PublicKeyBytes, TransactionHash)>,
        block_sender_map: Arc<Mutex<HashMap<TransactionHash, PublicKeyBytes>>>,
    ) {
        thread::spawn(move || {
            let mut checker = Self {
                replica,
                senders: sender,
                block_sender_map,
            };
            loop {
                checker.run();
            }
        });
    }

    fn run(&mut self) {
        let mut receipted_height = 0;
        loop {
            let snapshot = self.replica.block_tree_camera().snapshot();
            trace!("receipted height {}", receipted_height);
            if let Some(hc_block) = snapshot.highest_committed_block() {
                let highest_commited_height = snapshot.block_height(&hc_block).unwrap();
                trace!("commited height {}", highest_commited_height);
                for height in receipted_height + 1..=highest_commited_height {
                    let block = snapshot.block_at_height(height).unwrap();
                    let hash = snapshot.block(&block).unwrap().data_hash;
                    let Some(pubkey) = self.block_sender_map.blocking_lock().remove(&hash) else {
                        error!("Unknown block {:?}!", block);
                        continue;
                    };
                    self.senders.blocking_send((pubkey, hash)).unwrap();
                }
                receipted_height = highest_commited_height;
            }
            thread::sleep(Duration::from_millis(500));
        }
    }
}
