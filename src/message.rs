use std::{future::Future, pin::Pin, task::Poll};

use crate::nics::{Nic, NicId};

pub struct IncomingMsg {
    from: NicId,
    data: Pin<Vec<u8>>,
}

impl IncomingMsg {}

pub struct RecvMessage<'a> {
    mailbox: &'a mut Mailbox,
}

impl Future for RecvMessage<'_> {
    type Output = IncomingMsg;

    fn poll(mut self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        if self.mailbox.incoming.len() > 0 {
            Poll::Ready(self.mailbox.incoming.pop().unwrap())
        } else {
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}

pub struct OutgoingMsg {
    to: NicId,
    data: Pin<Vec<u8>>,
}

pub struct Mailbox {
    incoming: Vec<IncomingMsg>,
}

impl Mailbox {
    async fn send(out: &Nic) {

    }

    async fn recv() {

    }
}