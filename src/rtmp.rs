use tokio::net;
use tokio::prelude::*;

#[derive(Clone, Debug, Default)]
pub struct ChunkHeader {
    pub format: u32,
    pub cs_id: u32,
    pub timestamp: u32,
    pub length: u32,
    pub type_id: u32,
    pub stream_id: u32,
}

pub async fn shadow_handshake(sc: &mut net::TcpStream, dc: &mut net::TcpStream) -> Result<(), String> {
    unimplemented!()
}

pub async fn read_header<R>(c: &R) -> Result<ChunkHeader, String>
    where R: AsyncRead + Unpin {
    unimplemented!()
}

pub async fn write_message<W>(c: &W, max_chunk: usize, header: &ChunkHeader, payload: &[u8]) -> Result<(), String>
    where W: AsyncWrite + Unpin {
    unimplemented!()
}
