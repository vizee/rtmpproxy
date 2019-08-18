#![feature(async_await)]

#[macro_use]
extern crate lazy_static;

use tokio::net;
use tokio::prelude::*;

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
    let data = ::std::fs::read(fname)
        .map_err(|e| format!("read config file failed: {}", e))?;
    let conf: toml::value::Table = toml::from_slice(&data)
        .map_err(|e| format!("bad config: {}", e))?;
    let stream_url = conf.get("stream")
        .and_then(|v| v.as_str())
        .ok_or("stream url undefiend".to_string())?;
    let listen = conf.get("listen")
        .and_then(|v| v.as_str())
        .unwrap_or(":1935");
    let debug = conf.get("debug")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let u = url::Url::parse(stream_url)
        .expect("bad stream url");
    let host = u.host_str()
        .expect("missing host");
    let port = u.port()
        .unwrap_or(1935);
    let app_name = u.path().trim_matches('/');
    Ok(Config {
        debug,
        listen: listen.to_string(),
        server: format!("{}:{}", host, port),
        app_name: app_name.to_string(),
        play_url: format!("rtmp://{}/{}", host, app_name),
        stream_name: u.query()
            .map(|v| format!("?{}", v))
            .unwrap_or("".to_string()),
    })
}

struct Conn {
    s: net::TcpStream,
}

impl Conn {
    async fn hijack(&mut self) -> Result<net::TcpStream, String> {
        let sa = RESOLVER.resolve(&CONFIG.server).await?;
        println!("connect: {:?}", sa);
        let mut conn = net::TcpStream::connect(&sa).await.map_err(|e| format!("{}", e))?;
        Ok(conn)
    }

    async fn ioloop(&mut self) {
        if let Err(e) = self.hijack().await {
            println!("hijack: {}", e);
            return;
        }
        unimplemented!()
    }
}

#[tokio::main]
async fn main() {
    let la = CONFIG.listen.parse().expect("bad listen address");
    let mut ln = net::TcpListener::bind(&la).expect("bind address");
    loop {
        match ln.accept().await {
            Ok((mut s, _)) => {
                tokio::spawn(async move {
                    let mut conn = Conn { s };
                    conn.ioloop().await;
                });
            }
            Err(e) => println!("accept: {}", e),
        };
    }
}
