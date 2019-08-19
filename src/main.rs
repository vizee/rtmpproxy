#![feature(async_await)]

#[macro_use]
extern crate lazy_static;

use tokio::net;
use tokio::prelude::*;

mod binorder;
mod resolve;
mod rtmp;

struct Config {
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
    let u = url::Url::parse(stream_url).expect("bad stream url");
    let host = u.host_str().expect("missing host");
    let port = u.port().unwrap_or(1935);
    let app_name = u.path().trim_matches('/');
    Ok(Config {
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

fn hijack_command_message(payload: &mut Vec<u8>) -> Result<bool, String> {
    use std::io::Cursor;
    use amf::error::DecodeError;
    use amf0::Value;
    use amf::amf0;

    let mut dec = amf0::Decoder::new(Cursor::new(&payload));
    let command = match dec.decode().map_err(|e| format!("{}", e))? {
        Value::String(s) => s,
        _ => return Err("command not a string".to_string()),
    };
    let trans_id = match dec.decode().map_err(|e| format!("{}", e))? {
        Value::Number(v) => v,
        _ => return Err("trans_id not a number".to_string()),
    };
    let mut args = Vec::new();
    loop {
        args.push(match dec.decode() {
            Ok(v) => v,
            Err(DecodeError::Io(_)) => break,
            Err(e) => return Err(format!("{}", e)),
        });
    }
    let mut done = false;
    let mut keep = false;
    match command.as_str() {
        "connect" => {
            if let Value::Object { class_name: _, entries } = &mut args[0] {
                for p in entries {
                    match p.key.as_str() {
                        "app" => { p.value = Value::String(CONFIG.app_name.clone()) }
                        "swfUrl" => { p.value = Value::String(CONFIG.play_url.clone()) }
                        "tcUrl" => { p.value = Value::String(CONFIG.play_url.clone()) }
                        _ => {}
                    }
                }
            } else {
                return Err("connect args not a object".to_string());
            }
        }
        "releaseStream" | "FCPublish" => {
            args[0] = Value::String(CONFIG.stream_name.clone());
        }
        "publish" => {
            args[0] = Value::String(CONFIG.stream_name.clone());
            done = true;
        }
        _ => keep = true,
    }
    if keep {
        return Ok(done);
    }
    let mut enc = amf0::Encoder::new(Vec::new());
    let _ = enc.encode(&Value::String(command));
    let _ = enc.encode(&Value::Number(trans_id));
    for arg in args {
        let _ = enc.encode(&arg);
    }
    *payload = enc.into_inner();
    Ok(done)
}

async fn hijack<R, W>(sc: &mut R, dc: &mut W) -> Result<(), String>
    where R: AsyncRead + Unpin,
        W: AsyncWrite + Unpin {
    let mut hijack_done = false;
    let mut max_chunk = 128usize;
    let mut last_header = rtmp::ChunkHeader::default();
    let mut nread = 0usize;
    let mut payload = Vec::new();
    while !hijack_done {
        let mut header = rtmp::read_header(sc).await.map_err(|e| format!("read header: {}", e))?;
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
                hijack_done = hijack_command_message(&mut payload)?;
            }
            _ => {}
        }
        rtmp::write_message(dc, max_chunk, &mut header, &payload).await
            .map_err(|e| format!("{}", e))?;
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
    if let Err(e) = rtmp::shadow_handshake(&mut sc, &mut dc).await {
        println!("handshake: {}", e);
        return;
    }
    let (mut sr, mut sw) = sc.split();
    let (mut dr, mut dw) = dc.split();
    tokio::spawn(async move {
        let _ = dr.copy(&mut sw).await;
    });
    if let Err(e) = hijack(&mut sr, &mut dw).await {
        println!("hijack: {}", e);
        return;
    }
    let _ = sr.copy(&mut dw).await;
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
