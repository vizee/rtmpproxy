#![feature(async_await)]
#![feature(const_string_new)]

use std::fs;

use tokio::net;
use tokio::prelude::*;
use std::task::{Context, Poll};

mod splice;

struct Config {
    debug: bool,
    listen: String,
    server: String,
    play_url: String,
    app_name: String,
    stream_name: String,
}

static mut _CONFIG: Config = Config {
    debug: false,
    listen: String::new(),
    server: String::new(),
    play_url: String::new(),
    app_name: String::new(),
    stream_name: String::new(),
};

static CONFIG: &Config = unsafe { &_CONFIG };

fn load_config(fname: &str) {
    let data = fs::read(fname).expect("read config file failed");
    let conf: toml::value::Table = toml::from_slice(&data).expect("not a table");
    let stream_url = conf.get("stream").and_then(|v| v.as_str())
        .expect("stream url undefined");
    let listen = conf.get("listen").and_then(|v| v.as_str())
        .unwrap_or(":1935");
    let debug = conf.get("debug").and_then(|v| v.as_bool())
        .unwrap_or(false);
    let u = url::Url::parse(stream_url).expect("bad stream url");
    let host = u.host_str().expect("missing host");
    let port = u.port().unwrap_or(1935);
    unsafe {
        _CONFIG.debug = debug;
        _CONFIG.listen = listen.to_string();
        _CONFIG.server = format!("{}:{}", host, port);
        _CONFIG.app_name = u.path().trim_matches('/').to_string();
        _CONFIG.play_url = format!("rtmp://{}/{}", host, _CONFIG.play_url);
        _CONFIG.stream_name = format!("?{}", u.query().unwrap_or(""));
    }
}

struct Conn {
    s: net::TcpStream,
}

impl Conn {
    async fn ioloop(&mut self) {
    }
}

#[tokio::main]
async fn main() {
    load_config("./rtmpproxy.conf");
    let la = CONFIG.listen.parse().expect("bad listen address");
    let mut ln = net::TcpListener::bind(&la).expect("bind address");
    loop {
        match ln.accept().await {
            Ok((mut s, _)) => {
                tokio::spawn(async move {
                    let mut conn = Conn { s };
                    conn.ioloop().await
                });
            }
            Err(e) => println!("accept: {}", e),
        };
    }
}
