#![feature(const_string_new)]

use std::fs;
use std::io;

use tokio::net;
use tokio::prelude::*;

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
    let listen = conf.get("listen").and_then(|v| v.as_str())
        .unwrap_or(":1935");
    let stream_url = conf.get("stream").and_then(|v| v.as_str())
        .unwrap_or("rtmp://hostname/app/?args");
    let debug = conf.get("debug").and_then(|v| v.as_bool())
        .unwrap_or(false);
    let u = url::Url::parse(stream_url).expect("bad url");
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

impl Stream for Conn {
    type Item = Vec<u8>;
    type Error = io::Error;

    fn poll(&mut self) -> Result<Async<Option<Self::Item>>, Self::Error> {
        unimplemented!()
    }
}

fn main() {
    load_config("./rtmpproxy.conf");
    let la = CONFIG.listen.parse().expect("bad listen address");
    let listener = net::TcpListener::bind(&la).expect("bind address");
    let listening = listener.incoming()
        .map_err(|e| eprintln!("accept failed: {}", e))
        .for_each(|s| {
            let conn = Conn { s };
            tokio::spawn(conn
                .for_each(|v| {
                    Ok(())
                })
                .map_err(|e | eprintln!("connection: {:?}", e))
                .into_future());
            Ok(())
        });
    tokio::runtime::current_thread::run(listening);
}
