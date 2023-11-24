use dash_common::BlockMsg;

use std::net::{SocketAddr, TcpListener};
use std::sync::mpsc::Sender;
use std::thread::spawn;

use anyhow::Result;
use borsh::BorshDeserialize;

pub struct ClientHandler {
    block_tx: Sender<Vec<u8>>,
    listen_addr: SocketAddr,
}

impl ClientHandler {
    pub fn new(listen_addr: SocketAddr, block_tx: Sender<Vec<u8>>) -> Self {
        Self {
            block_tx,
            listen_addr,
        }
    }

    pub fn run(&self) -> Result<()> {
        let listen_addr = self.listen_addr;
        let block_tx = self.block_tx.clone();
        spawn(move || {
            let listener = TcpListener::bind(listen_addr).unwrap();
            loop {
                match listener.accept() {
                    Ok((mut socket, _addr)) => {
                        let block_tx = block_tx.clone();
                        spawn(move || loop {
                            let block = BlockMsg::deserialize_reader(&mut socket).unwrap();
                            block_tx.send(block.data).unwrap();
                        });
                    }
                    Err(e) => panic!("{}", e),
                }
            }
        });
        Ok(())
    }
}
