//! Socket type registered in async-std reactor

mod watcher;
pub(crate) use watcher::Watcher;

use crate::{
    reactor::{evented, AsRawSocket},
    socket::MessageBuf,
};

use futures::ready;
use std::task::{Context, Poll};
use zmq::Error;

pub(crate) type ZmqSocket = Watcher<evented::ZmqSocket>;

impl ZmqSocket {
    fn poll_event(&self, event: zmq::PollEvents) -> Poll<Result<(), Error>> {
        if self.as_socket().get_events()?.contains(event) {
            Poll::Ready(Ok(()))
        } else {
            Poll::Ready(Err(Error::EAGAIN))
        }
    }

    pub(crate) fn send(
        &self,
        cx: &mut Context<'_>,
        buffer: &mut MessageBuf,
    ) -> Poll<Result<(), Error>> {
        ready!(self.poll_write_ready(cx));
        ready!(self.poll_event(zmq::POLLOUT))?;

        while let Some(msg) = buffer.pop_front() {
            let mut flags = zmq::DONTWAIT;
            if !buffer.is_empty() {
                flags |= zmq::SNDMORE;
            }

            match self.as_socket().send(msg, flags) {
                Ok(_) => {}
                Err(Error::EAGAIN) => return Poll::Pending,
                Err(e) => return Poll::Ready(Err(e.into())),
            }
        }

        Poll::Ready(Ok(()))
    }

    pub(crate) fn recv(&self, cx: &mut Context<'_>) -> Poll<Result<MessageBuf, Error>> {
        ready!(self.poll_read_ready(cx));
        ready!(self.poll_event(zmq::POLLIN))?;

        let mut buffer = MessageBuf::default();
        let mut more = true;

        while more {
            let mut msg = zmq::Message::new();
            match self.as_socket().recv(&mut msg, zmq::DONTWAIT) {
                Ok(_) => {
                    more = msg.get_more();
                    buffer.0.push_back(msg);
                }
                Err(e) => return Poll::Ready(Err(e.into())),
            }
        }

        Poll::Ready(Ok(buffer))
    }
}

impl From<zmq::Socket> for ZmqSocket {
    fn from(socket: zmq::Socket) -> Self {
        Watcher::new(evented::ZmqSocket(socket))
    }
}

impl AsRawSocket for ZmqSocket {
    fn as_socket(&self) -> &zmq::Socket {
        &self.get_ref().0
    }
}
