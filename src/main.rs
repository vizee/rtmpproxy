#![feature(async_await)]

#[macro_use]
extern crate lazy_static;

use std::convert::TryInto;

use tokio::net;
use tokio::prelude::*;

mod binorder;
mod resolve;
mod rtmp;

struct Config {
    debug: bool,
    listen: String,
    server: String,
    play_url: String,
    app_name: String,
    stream_name: String,
}

lazy_static! {
    static ref CONFIG: Config = load_config("rtmpproxy.conf").expect("load config");
    static ref RESOLVER: resolve::Resolver = resolve::Resolver::new();
}

fn load_config(fname: &str) -> Result<Config, String> {
    let data = ::std::fs::read(fname).map_err(|e| format!("read config file failed: {}", e))?;
    let conf: toml::value::Table =
        toml::from_slice(&data).map_err(|e| format!("bad config: {}", e))?;
    let stream_url = conf
        .get("stream")
        .and_then(|v| v.as_str())
        .ok_or("stream url undefiend".to_string())?;
    let listen = conf
        .get("listen")
        .and_then(|v| v.as_str())
        .unwrap_or(":1935");
    let debug = conf.get("debug").and_then(|v| v.as_bool()).unwrap_or(false);
    let u = url::Url::parse(stream_url).expect("bad stream url");
    let host = u.host_str().expect("missing host");
    let port = u.port().unwrap_or(1935);
    let app_name = u.path().trim_matches('/');
    Ok(Config {
        debug,
        listen: listen.to_string(),
        server: format!("{}:{}", host, port),
        app_name: app_name.to_string(),
        play_url: format!("rtmp://{}/{}", host, app_name),
        stream_name: u
            .query()
            .map(|v| format!("?{}", v))
            .unwrap_or("".to_string()),
    })
}

fn hijack_command(header: &rtmp::ChunkHeader, payload: &mut Vec<u8>) -> Result<bool, String> {
    unimplemented!()
}

async fn hijack(sc: &mut net::TcpStream, dc: &mut net::TcpStream) -> Result<(), String> {
    let mut hijack_done = false;
    let mut max_chunk = 128usize;
    let mut last_header = rtmp::ChunkHeader::default();
    let mut nread = 0usize;
    let mut payload = Vec::new();
    while !hijack_done {
        let mut header = rtmp::read_header(sc).await?;
        if nread != 0 || header.cs_id != last_header.cs_id {
            return Err("unsupport multi-chunkstream at a time".to_string());
        }
        match header.format {
            1 => {
                header.stream_id = last_header.stream_id;
            }
            2 => {
                header.length = last_header.length;
                header.type_id = last_header.type_id;
                header.stream_id = last_header.stream_id;
            }
            3 => {
                header.timestamp = last_header.timestamp;
                header.length = last_header.length;
                header.type_id = last_header.type_id;
                header.stream_id = last_header.stream_id;
            }
            _ => {}
        }
        last_header = header.clone();
        if payload.len() != header.length as usize {
            payload.resize(header.length as usize, 0);
        }
        let n = max_chunk.min(payload.len() - nread);
        sc.read_exact(&mut payload[nread..nread + n])
            .await
            .map_err(|e| format!("{}", e))?;
        nread += n;
        if nread < payload.len() {
            continue;
        }
        match header.type_id {
            1 => {
                max_chunk = binorder::to_be_u32(&payload)
                    .map_err(|e| format!("bad payload: {}", e))?
                    as usize;
            }
            20 => {
                hijack_done = hijack_command(&header, &mut payload)?;
            }
            _ => {}
        }
        rtmp::write_message(dc, max_chunk, &header, &payload).await?;
        nread = 0;
    }
    Ok(())
}

async fn connect_server() -> Result<net::TcpStream, String> {
    let sa = RESOLVER.resolve(&CONFIG.server).await?;
    net::TcpStream::connect(&sa)
        .await
        .map_err(|e| format!("connect: {}", e))
}

async fn proxy_conn(mut sc: net::TcpStream) {
    let mut dc = match connect_server().await {
        Ok(c) => c,
        Err(e) => {
            println!("connect_server: {}", e);
            return;
        }
    };
    // TODO copy dc to sc
    if let Err(e) = rtmp::shadow_handshake(&mut sc, &mut dc).await {
        println!("handshake: {}", e);
        return;
    }
    if let Err(e) = hijack(&mut sc, &mut dc).await {
        println!("hijack: {}", e);
        return;
    }
    unimplemented!()
}

#[tokio::main]
async fn main() {
    let la = CONFIG.listen.parse().expect("bad listen address");
    let mut ln = net::TcpListener::bind(&la).expect("bind address");
    loop {
        match ln.accept().await {
            Ok((s, _)) => {
                tokio::spawn(async move {
                    proxy_conn(s).await;
                });
            }
            Err(e) => println!("accept: {}", e),
        };
    }
}
