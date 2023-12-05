use dash_common::NewTransactionRequest;
use dash_network::server::Server;

// use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use borsh::BorshDeserialize;
use bytes::Bytes;
use tokio::{
    runtime::Runtime,
    sync::mpsc::{channel, Receiver, Sender},
};

pub struct ClientActor();

impl ClientActor {
    pub fn spawn(listen_addr: SocketAddr, rt: Arc<Runtime>) -> Receiver<Vec<u8>> {
        let (block_sender, block_receiver) = channel(1000);
        thread::spawn(move || {
            rt.block_on(async {
                Actor::spawn(listen_addr, block_sender);
                loop {
                    tokio::time::sleep(Duration::from_secs(u64::MAX)).await;
                }
            });
        });
        block_receiver
    }
}

struct Actor {
    #[allow(unused)]
    listen_addr: SocketAddr,
    block_sender: Sender<Vec<u8>>,
    #[allow(unused)]
    net_sender: Sender<(SocketAddr, Bytes)>,
    net_receiver: Receiver<(SocketAddr, Bytes)>,
    // id_map: HashMap<u64, SocketAddr>,
    // clients: HashMap<SocketAddr, Sender<Vec<u8>>>,
}

impl Actor {
    fn spawn(listen_addr: SocketAddr, block_sender: Sender<Vec<u8>>) {
        tokio::spawn(async move {
            let (net_sender, net_receiver) = Server::spawn(listen_addr);
            Self {
                listen_addr,
                block_sender,
                net_sender,
                net_receiver,
                // clients: Default::default(),
                // id_map: Default::default(),
            }
            .run()
            .await
        });
    }

    async fn run(&mut self) {
        while let Some((_addr, msg_bytes)) = self.net_receiver.recv().await {
            let mut msg_bytes = msg_bytes.as_ref();
            if let Ok(NewTransactionRequest { id, data }) =
                NewTransactionRequest::deserialize(&mut msg_bytes)
            {
                let _ = id;
                self.block_sender.send(data).await.unwrap()
            }
        }
    }
}
