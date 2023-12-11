use crate::common::{Channel, Reader, Writer};

use std::collections::{HashMap, hash_map::Entry};
use std::net::SocketAddr;

use bytes::Bytes;
use futures::{SinkExt, StreamExt};
use log::{error, trace, warn};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::mpsc::{channel, Receiver, Sender},
};
use tokio_util::codec::{Framed, LengthDelimitedCodec};

pub struct Server {
    host_addr: SocketAddr,
    sender: Sender<(SocketAddr, Bytes)>,
    receiver: Receiver<(SocketAddr, Bytes)>,
    connections: HashMap<SocketAddr, Sender<(SocketAddr, Bytes)>>,
}

impl Server {
    pub fn spawn(host_addr: SocketAddr) -> Channel {
        let (sender, ret_receiver) = channel(1000);
        let (ret_sender, receiver) = channel(1000);
        tokio::spawn(async move {
            Self {
                host_addr,
                sender,
                receiver,
                connections: Default::default(),
            }
            .run()
            .await;
        });
        (ret_sender, ret_receiver)
    }

    async fn run(&mut self) {
        let listener = TcpListener::bind(self.host_addr)
            .await
            .expect("Failed to bind TCP port!");
        loop {
            tokio::select! {
                connection = listener.accept() => {
                    match connection {
                        Ok((socket, addr)) => {
                            trace!("accept connection from {}", addr);
                            let (sender, receiver) = channel(1000);
                            self.connections.insert(addr, sender);
                            Connection::spawn(addr, socket, self.sender.clone(), receiver);
                        }
                        Err(e) => error!("couldn't get client: {e:?}"),
                    }
                }
                Some((addr, msg)) = self.receiver.recv() => {
                    match  self.connections.entry(addr) {
                        Entry::Occupied(mut entry) => {
                            trace!("sending msg to {}", addr);
                            if let Err(e) = entry.get_mut().send((addr, msg)).await {
                                warn!("Disconnectted from {}: {}", addr, e);
                                entry.remove();
                            }
                        }
                        Entry::Vacant(_) => warn!("No connection from {}", addr),
                    }
                }
            }
        }
    }
}

struct Connection {
    sender: Sender<(SocketAddr, Bytes)>,
    receiver: Receiver<(SocketAddr, Bytes)>,
    remote_addr: SocketAddr,
    reader: Reader,
    writer: Writer,
}

impl Connection {
    fn spawn(
        remote_addr: SocketAddr,
        socket: TcpStream,
        sender: Sender<(SocketAddr, Bytes)>,
        receiver: Receiver<(SocketAddr, Bytes)>,
    ) {
        let (writer, reader) = Framed::new(socket, LengthDelimitedCodec::new()).split();
        tokio::spawn(async move {
            Self {
                sender,
                receiver,
                remote_addr,
                reader,
                writer,
            }
            .run()
            .await
        });
    }

    async fn run(&mut self) {
        loop {
            tokio::select! {
                Some(framed_data) = self.reader.next() => {
                    match framed_data {
                        Ok(data) => {
                            trace!("received msg from: {}", self.remote_addr,);
                            self.sender
                                .send((self.remote_addr, data.freeze()))
                                .await
                                .unwrap()
                        }
                        Err(e) => error!("{}", e),
                    };
                },
                Some((addr, data)) = self.receiver.recv() => {
                    trace!("sending msg to {}", addr);
                    if let Err(e) = self.writer.send(data).await {
                        warn!("Disconnectted from {}: {}", self.remote_addr, e);
                        return ;
                    }
                }
            }
        }
    }
}
