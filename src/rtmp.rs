use std::io;

use tokio::net;
use tokio::prelude::*;

use crate::binorder;

#[derive(Clone, Debug, Default)]
pub struct ChunkHeader {
    pub format: u32,
    pub cs_id: u32,
    pub timestamp: u32,
    pub length: u32,
    pub type_id: u32,
    pub stream_id: u32,
}

impl ChunkHeader {
    pub fn as_bytes(&self) -> Vec<u8> {
        let mut n = 1usize;
        let mut cs_id = self.cs_id;
        if cs_id >= 256 + 64 {
            cs_id = 1;
            n += 2;
        } else if cs_id >= 64 {
            cs_id = 0;
            n += 1;
        }
        match self.format {
            0 => n += 11,
            1 => n += 7,
            2 => n += 3,
            _ => {}
        };
        if self.timestamp >= 0xffffff {
            n += 4;
        }
        let mut data = Vec::with_capacity(n);
        data.push(((self.format << 6) | cs_id) as u8);
        if self.cs_id == 0 {
            data.push((self.cs_id - 64) as u8);
        } else if self.cs_id == 1 {
            data.extend(((self.cs_id - 64) as u16).to_be_bytes().iter());
        }
        let mut ts = self.timestamp;
        let mut exts = 0u32;
        if ts >= 0xffffff {
            exts = ts;
            ts = 0xffffff
        }
        match self.format {
            0 | 1 => {
                data.extend(ts.to_be_bytes().iter().skip(1));
                data.extend(self.length.to_be_bytes().iter().skip(1));
                data.push(self.type_id as u8);
                if self.format == 0 {
                    data.extend(self.stream_id.to_le_bytes().iter());
                }
            }
            2 => {
                data.extend(ts.to_be_bytes().iter().skip(1));
            }
            _ => {}
        }
        if ts == 0xffffff && self.format != 3 {
            data.extend(exts.to_be_bytes().iter());
        }
        data
    }
}

async fn copy_sized(s: &mut net::TcpStream, d: &mut net::TcpStream, n: usize) -> io::Result<()> {
    let mut msg = vec![0u8; n];
    s.read_exact(&mut msg).await?;
    d.write_all(&mut msg).await?;
    d.read_exact(&mut msg).await?;
    s.write_all(&mut msg).await?;
    Ok(())
}

pub async fn shadow_handshake(sc: &mut net::TcpStream, dc: &mut net::TcpStream) -> io::Result<()> {
    copy_sized(sc, dc, 1 + 1536).await?;
    copy_sized(sc, dc, 1536).await?;
    Ok(())
}

pub async fn read_header<R>(c: &mut R) -> io::Result<ChunkHeader>
    where R: AsyncRead + Unpin {
    let mut fixed = [0u8; 18];
    c.read_exact(&mut fixed[..1]).await?;
    let format = (fixed[0] >> 6) as u32;
    let cs_id = (fixed[0] & 0x3f) as u32;
    let mut n = 0;
    if cs_id <= 1 {
        n += cs_id + 1;
    }
    match format {
        0 => n += 11,
        1 => n += 7,
        2 => n += 3,
        _ => {}
    }
    let mut header = ChunkHeader {
        format,
        cs_id,
        timestamp: 0,
        length: 0,
        type_id: 0,
        stream_id: 0,
    };
    if n > 0 {
        c.read_exact(&mut fixed[..n as usize]).await?;
        let mut p = 0usize;
        match cs_id {
            0 => {
                header.cs_id = fixed[0] as u32 + 64;
                p = 1;
            }
            1 => {
                header.cs_id = binorder::to_be_u16(&fixed[..2]).unwrap() as u32 + 64;
                p = 2;
            }
            _ => {}
        }
        let hbuf = &fixed[p..];
        match format {
            0 => {
                header.timestamp = binorder::to_be_u32(&hbuf[..4]).unwrap() >> 8;
                header.length = binorder::to_be_u32(&hbuf[3..7]).unwrap() >> 8;
                header.type_id = hbuf[6] as u32;
                header.stream_id = binorder::to_le_u32(&hbuf[7..11]).unwrap();
            }
            1 => {
                header.timestamp = binorder::to_be_u32(&hbuf[..4]).unwrap() >> 8;
                header.length = binorder::to_be_u32(&hbuf[3..7]).unwrap() >> 8;
                header.type_id = hbuf[6] as u32;
            }
            2 => {
                header.timestamp = binorder::to_be_u32(&hbuf[..4]).unwrap() >> 8;
            }
            _ => {}
        }
        if header.timestamp == 0xffffff {
            c.read_exact(&mut fixed[..4]).await?;
            header.timestamp = binorder::to_be_u32(&mut fixed[..4]).unwrap();
        }
    }
    Ok(header)
}

pub async fn write_message<W>(c: &mut W, chunk_size: usize, header: &mut ChunkHeader, payload: &[u8]) -> io::Result<()>
    where W: AsyncWrite + Unpin {
    let mut nwrote = 0usize;
    while nwrote < payload.len() {
        if nwrote == 0 {
            header.format = 0;
            header.length = payload.len() as u32;
        } else {
            header.format = 3;
        }
        let data = header.as_bytes();
        c.write_all(&data).await?;
        let n = chunk_size.min(payload.len() - nwrote);
        c.write_all(&payload[nwrote..nwrote + n]).await?;
        nwrote += n;
    }
    Ok(())
}
