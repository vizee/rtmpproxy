use std::net::SocketAddr;
use std::time::Duration;

use tokio_threadpool::{ThreadPool, Builder};
use tokio::sync::oneshot::{self, Sender};

struct ResolveTask {
    tx: Sender<Result<SocketAddr, String>>,
    addr: String,
}

pub struct Resolver {
    pool: ThreadPool,
}

impl Resolver {
    pub fn new() -> Self {
        Resolver {
            pool: Builder::new()
                .pool_size(1)
                .keep_alive(Some(Duration::from_secs(300)))
                .build(),
        }
    }

    pub async fn resolve(&self, addr: &str) -> Result<SocketAddr, String> {
        let (tx, rx) = oneshot::channel();
        let task = ResolveTask {
            tx,
            addr: addr.into(),
        };
        self.pool.spawn(async move {
            use std::net::ToSocketAddrs;
            let _ = task.tx.send(task.addr.to_socket_addrs()
                .map_err(|e| format!("ToSocketAddrs: {}", e))
                .and_then(|mut v| v.next()
                    .ok_or("nothing resolved".to_string())));
        });
        let r = rx.await.unwrap_or_else(|e| Err(format!("rx: {}", e)));
        r
    }
}
