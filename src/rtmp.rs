use tokio::net;
use tokio::prelude::*;

pub async fn shadow_handshake(sc: &mut net::TcpStream, dc: &mut net::TcpStream) -> Result<(), String> {
    unimplemented!()
}
