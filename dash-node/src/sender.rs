use std::collections::{HashMap, VecDeque};
use std::net::SocketAddr;
use std::time::Duration;

use anyhow::Result;
use bytes::Bytes;
use futures::SinkExt;
use log::{trace, warn};
use tokio::{
    net::TcpStream,
    sync::mpsc::{channel, Receiver, Sender},
    time,
};
use tokio_util::codec::{Framed, LengthDelimitedCodec};

pub struct NetSender {
    receiver: Receiver<(SocketAddr, Bytes)>,
    sender_workers: HashMap<SocketAddr, Sender<Bytes>>,
}

impl NetSender {
    pub fn spawn(receiver: Receiver<(SocketAddr, Bytes)>) {
        tokio::spawn(async move {
            Self {
                receiver,
                sender_workers: Default::default(),
            }
            .run()
            .await;
        });
    }

    async fn run(&mut self) {
        while let Some((dest_addr, data)) = self.receiver.recv().await {
            let sender = self
                .sender_workers
                .entry(dest_addr)
                .or_insert_with(|| Connection::spawn(dest_addr));
            sender.send(data).await.unwrap();
        }
    }
}

struct Connection {
    remote_addr: SocketAddr,
    receiver: Receiver<Bytes>,
    buffer: VecDeque<Bytes>,
}

impl Connection {
    fn spawn(remote_addr: SocketAddr) -> Sender<Bytes> {
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
                            Some(request) = self.receiver.recv() => {
                                self.buffer.push_back(request);
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
            while let Some(data) = self.buffer.pop_front() {
                trace!("send msg to {} in keep_alive", self.remote_addr);
                writer.send(data).await?;
            }
            while let Some(data) = self.receiver.recv().await {
                trace!("send msg to {} in keep_alive", self.remote_addr);
                writer.send(data).await?;
            }
        }
    }
}
