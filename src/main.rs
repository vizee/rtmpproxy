#![feature(async_await)]
#![feature(const_string_new)]

use std::fs;
use std::sync;

use std::task::{Context, Poll};
use tokio::net;
use tokio::prelude::*;

mod iocopy;
mod rtmp;

#[derive(Debug)]
struct Config {
    debug: bool,
    listen: String,
    server: String,
    play_url: String,
    app_name: String,
    stream_name: String,
}

fn load_config(fname: &str) -> Result<Config, String> {
    let data = fs::read(fname).map_err(|e| format!("read config file failed: {}", e))?;
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
        debug: debug,
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

fn get_confg() -> &'static Config {
    static mut CONFIG: Config = Config {
        debug: false,
        listen: String::new(),
        server: String::new(),
        play_url: String::new(),
        app_name: String::new(),
        stream_name: String::new(),
    };
    static INIT: sync::Once = sync::Once::new();

    INIT.call_once(|| unsafe {
        CONFIG = load_config("rtmpproxy.conf").expect("load config");
    });

    unsafe { &CONFIG }
}

struct Conn {
    s: net::TcpStream,
}

impl Conn {
    async fn ioloop(&mut self) {}
}

#[tokio::main]
async fn main() {
    let la = get_confg().listen.parse().expect("bad listen address");
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
