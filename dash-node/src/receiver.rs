use std::net::SocketAddr;

use bytes::Bytes;
use futures::StreamExt;
use log::{error, trace};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::mpsc::Sender,
};
use tokio_util::codec::{Framed, LengthDelimitedCodec};

pub struct NetReceiver {
    host_addr: SocketAddr,
    sender: Sender<(SocketAddr, Bytes)>,
}

impl NetReceiver {
    pub fn spawn(host_addr: SocketAddr, sender: Sender<(SocketAddr, Bytes)>) {
        tokio::spawn(async move {
            Self { host_addr, sender }.run().await;
        });
    }

    async fn run(&mut self) {
        let listener = TcpListener::bind(self.host_addr)
            .await
            .expect("Failed to bind TCP port!");
        loop {
            match listener.accept().await {
                Ok((socket, addr)) => {
                    trace!("accept connection from {}", addr);
                    Connection::spawn(addr, socket, self.sender.clone());
                }
                Err(e) => error!("couldn't get client: {e:?}"),
            }
        }
    }
}

struct Connection {
    sender: Sender<(SocketAddr, Bytes)>,
    remote_addr: SocketAddr,
    framed_reader: Framed<TcpStream, LengthDelimitedCodec>,
}

impl Connection {
    fn spawn(remote_addr: SocketAddr, socket: TcpStream, sender: Sender<(SocketAddr, Bytes)>) {
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
        while let Some(framed_data) = self.framed_reader.next().await {
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
        }
    }
}
