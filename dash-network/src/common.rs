use std::net::SocketAddr;

use bytes::Bytes;
use futures::stream::{SplitSink, SplitStream};
use tokio::{
    net::TcpStream,
    sync::mpsc::{Receiver, Sender},
};
use tokio_util::codec::{Framed, LengthDelimitedCodec};

/// Convenient alias for the writer end of the TCP channel.
pub type Writer = SplitSink<Framed<TcpStream, LengthDelimitedCodec>, Bytes>;
pub type Reader = SplitStream<Framed<TcpStream, LengthDelimitedCodec>>;
pub type Channel = (Sender<(SocketAddr, Bytes)>, Receiver<(SocketAddr, Bytes)>);
