use std::pin::Pin;
use std::task::{Context, Poll};

use tokio::net;
use tokio::prelude::*;

pub struct IOCopy {
    from: net::TcpStream,
    to: net::TcpStream,
    buf: Vec<u8>,
}

impl IOCopy {
    pub fn new(from: net::TcpStream, to: net::TcpStream, buffer_size: usize) -> Self {
        Self { from, to, buf: vec![0; buffer_size] }
    }
}

impl Future for IOCopy {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        unimplemented!()
    }
}
