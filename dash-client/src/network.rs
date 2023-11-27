use crate::message::{NewTransactionRequest, TransactionReceipt};

use std::io::Write;
use std::net::{SocketAddr, TcpStream};
use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};
use std::thread::spawn;

use anyhow::{anyhow, Result};
use borsh::{BorshDeserialize, BorshSerialize};

pub struct Network {
    // peers: Vec<SocketAddr>,
    tx_sender: Sender<NewTransactionRequest>,
    rx_receiver: Receiver<TransactionReceipt>,
}

impl Network {
    pub fn new(peers: Vec<SocketAddr>) -> Result<Self> {
        let (tx_sender, rx_receiver) =
            spawn_main_worker_thread(peers.iter().copied().map(|peer| (peer, None)).collect())?;
        Ok(Self {
            // peers,
            tx_sender,
            rx_receiver,
        })
    }

    pub fn send_transaction(&self, transaction: NewTransactionRequest) -> Result<()> {
        Ok(self.tx_sender.send(transaction)?)
    }

    pub fn receive_transaction_receipt(&self) -> Result<Option<TransactionReceipt>> {
        match self.rx_receiver.try_recv() {
            Ok(receipt) => Ok(Some(receipt)),
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => Err(anyhow!("network disconnected")),
        }
    }
}

fn spawn_main_worker_thread(
    mut peers: Vec<(SocketAddr, Option<Sender<NewTransactionRequest>>)>,
) -> Result<(Sender<NewTransactionRequest>, Receiver<TransactionReceipt>)> {
    let (tx_sender, tx_receiver) = channel();
    let (rx_sender, rx_receiver) = channel();

    spawn(move || {
        // let (receipt_sender, receipt_receiver) = channel();
        loop {
            let transaction: NewTransactionRequest = tx_receiver.recv().unwrap();
            for (peer, sender) in peers.iter_mut() {
                if let Some(sender) = sender {
                    sender.send(transaction.clone()).unwrap();
                } else {
                    let new_sender = spawn_worker_thread(*peer, rx_sender.clone()).unwrap();
                    new_sender.send(transaction.clone()).unwrap();
                    *sender = Some(new_sender);
                }
            }
        }
    });
    Ok((tx_sender, rx_receiver))
}

fn spawn_worker_thread(
    addr: SocketAddr,
    receipt_sender: Sender<TransactionReceipt>,
) -> Result<Sender<NewTransactionRequest>> {
    let mut stream = TcpStream::connect(addr)?;
    let (tx_sender, tx_receiver) = channel::<NewTransactionRequest>();
    spawn(move || loop {
        match tx_receiver.try_recv() {
            Ok(transaction) => {
                stream
                    .write_all(&transaction.try_to_vec().unwrap())
                    .unwrap();
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => panic!("main thread disconnected"),
        }
        let receipt = TransactionReceipt::deserialize_reader(&mut stream).unwrap();
        receipt_sender.send(receipt).unwrap();
    });
    Ok(tx_sender)
}
