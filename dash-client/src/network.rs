use dash_common::{NewTransactionRequest, TransactionReceipt};
use dash_network::client::Client;

use std::net::SocketAddr;

use anyhow::Result;
use borsh::{BorshDeserialize, BorshSerialize};
use bytes::Bytes;
use tokio::sync::mpsc::{channel, error::TryRecvError, Receiver, Sender};

pub struct Network {
    // peers: Vec<SocketAddr>,
    tx_sender: Sender<NewTransactionRequest>,
    rx_receiver: Receiver<TransactionReceipt>,
}

impl Network {
    pub fn new(peers: Vec<SocketAddr>) -> Result<Self> {
        let (tx_sender, rx_receiver) = spawn_main_worker_thread(peers)?;
        Ok(Self {
            // peers,
            tx_sender,
            rx_receiver,
        })
    }

    pub async fn send_transaction(&self, transaction: NewTransactionRequest) -> Result<()> {
        Ok(self.tx_sender.send(transaction).await?)
    }

    pub async fn receive_transaction_receipt(&mut self) -> Result<Option<TransactionReceipt>> {
        match self.rx_receiver.try_recv() {
            Ok(a) => Ok(Some(a)),
            Err(TryRecvError::Empty) => Ok(None),
            Err(e) => panic!("{}", e),
        }
    }
}

fn spawn_main_worker_thread(
    peers: Vec<SocketAddr>,
) -> Result<(Sender<NewTransactionRequest>, Receiver<TransactionReceipt>)> {
    let (tx_sender, mut tx_receiver) = channel::<NewTransactionRequest>(1000);
    let (rx_sender, rx_receiver) = channel(1000);

    tokio::spawn(async move {
        let (sender, mut receiver) = Client::spawn();
        loop {
            tokio::select! {
                Some(request) = tx_receiver.recv() => {
                    for peer in peers.iter() {
                        let data = Bytes::from(request.try_to_vec().unwrap());
                        sender.send((*peer, data)).await.unwrap();
                    }
                }
                Some((_addr, msg_bytes)) = receiver.recv() => {
                    let trans =
                        TransactionReceipt::deserialize_reader(&mut msg_bytes.as_ref()).unwrap();
                    rx_sender.send(trans).await.unwrap();
                }
            }
        }
    });
    Ok((tx_sender, rx_receiver))
}
