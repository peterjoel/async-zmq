use std::io::Error;
use std::pin::Pin;
use std::task::{Context, Poll};

use zmq::SocketType;

use crate::Sink;
use crate::socket::{AsRaw, MessageBuf, Sender, ZmqSocket};

pub fn publish(endpoint: &str) -> Result<Publish, zmq::Error> {
    let socket = zmq::Context::new().socket(SocketType::PUB)?;

    socket.bind(endpoint)?;

    Ok(Publish::from(socket))
}

pub struct Publish(Sender);

impl AsRaw for Publish {
    fn as_raw_socket(&self) -> &zmq::Socket {
        &self.0.socket.get_ref().0
    }
}

impl<T: Into<MessageBuf>> Sink<T> for Publish {
    type Error = Error;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Sink::<T>::poll_ready(Pin::new(&mut self.get_mut().0), cx)
    }

    fn start_send(self: Pin<&mut Self>, item: T) -> Result<(), Self::Error> {
        Pin::new(&mut self.get_mut().0).start_send(item)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Error>> {
        Sink::<T>::poll_flush(Pin::new(&mut self.get_mut().0), cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), Error>> {
        Sink::<T>::poll_close(Pin::new(&mut self.get_mut().0), cx)
    }
}

impl From<zmq::Socket> for Publish {
    fn from(socket: zmq::Socket) -> Self {
        Self(Sender {
            socket: ZmqSocket::from(socket),
            buffer: MessageBuf::default(),
        })
    }
}
