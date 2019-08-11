use tokio::net;
use tokio::prelude::*;

pub struct SpliceFuture {
    from: net::TcpStream,
    to: net::TcpStream,
    buf: Vec<u8>,
}

impl SpliceFuture {
    pub fn new(from: net::TcpStream, to: net::TcpStream, buffer_size: usize) -> SpliceFuture {
        SpliceFuture {
            from,
            to,
            buf: vec![0; buffer_size],
        }
    }

    pub fn try_new(from: net::TcpStream, to: net::TcpStream) -> Result<SpliceFuture, i32> {
        Ok(Self::new(from, to, 8192))
    }
}

impl Future for SpliceFuture {
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Result<Async<Self::Item>, Self::Error> {
        unimplemented!()
    }
}
