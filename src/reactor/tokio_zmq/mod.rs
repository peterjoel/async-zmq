//! Socket type registered in tokio reactor

use crate::{
    reactor::{evented, AsRawSocket},
    socket::MessageBuf,
};

use mio::Ready;
use std::task::{Context, Poll};
use tokio::io::PollEvented;
use zmq::Error;

/// Poll Evented Socket
pub(crate) type ZmqSocketEvented = PollEvented<evented::ZmqSocket>;

/// ZMQ Socket
pub(crate) struct ZmqSocket {
    evented: ZmqSocketEvented,
}

impl ZmqSocket {
    fn poll_event(&self, event: zmq::PollEvents) -> Poll<Result<(), Error>> {
        match self.as_socket().poll(event, 100) {
            Ok(_) => {}
            Err(e) => return Poll::Ready(Err(e)),
        };

        if self.as_socket().get_events()?.contains(event) {
            Poll::Ready(Ok(()))
        } else {
            Poll::Pending
        }
    }

    pub(crate) fn send(
        &self,
        cx: &mut Context<'_>,
        buffer: &mut MessageBuf,
    ) -> Poll<Result<(), Error>> {
        futures::ready!(self.evented.poll_write_ready(cx));
        futures::ready!(self.poll_event(zmq::POLLOUT))?;

        while let Some(msg) = buffer.pop_front() {
            let mut flags = zmq::DONTWAIT;
            if !buffer.is_empty() {
                flags |= zmq::SNDMORE;
            }

            match self.as_socket().send(msg, flags) {
                Ok(_) => {}
                Err(zmq::Error::EAGAIN) => return Poll::Pending,
                Err(e) => return Poll::Ready(Err(e.into())),
            }
        }

        Poll::Ready(Ok(()))
    }

    pub(crate) fn recv(&self, cx: &mut Context<'_>) -> Poll<Result<MessageBuf, Error>> {
        let e_ready = Ready::readable();

        match self.evented.poll_read_ready(cx, e_ready) {
            Poll::Ready(_) => {}
            Poll::Pending => return Poll::Pending,
        };

        match self.poll_event(zmq::POLLIN) {
            Poll::Ready(_) => {}
            Poll::Pending => {
                self.evented.clear_read_ready(cx, e_ready).unwrap();
                return Poll::Pending;
            }
        }

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

impl AsRawSocket for ZmqSocket {
    fn as_socket(&self) -> &zmq::Socket {
        &self.evented.get_ref().0
    }
}

impl From<zmq::Socket> for ZmqSocket {
    fn from(socket: zmq::Socket) -> Self {
        Self {
            evented: ZmqSocketEvented::new(evented::ZmqSocket(socket)).unwrap(),
        }
    }
}
