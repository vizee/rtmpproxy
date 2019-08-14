use std::pin::Pin;
use std::task::{Context, Poll};

use libc;
use tokio::net;
use tokio::prelude::*;

macro_rules! syscall {
    ($e: expr) => {{
        let r = unsafe { $e };
        if r < 0 {
            Err(unsafe { *libc::__errno_location() })
        } else {
            Ok(r)
        }
    }};
}

pub struct SpliceFuture {
    from: net::TcpStream,
    to: net::TcpStream,
    buffered: usize,
    pipe_size: usize,
    pfd_r: i32,
    pfd_w: i32,
}

impl SpliceFuture {
    pub fn try_new(from: net::TcpStream, to: net::TcpStream) -> Result<SpliceFuture, i32> {
        let mut pfd = [0i32; 2];
        syscall!(libc::pipe(pfd.as_mut_ptr()))?;
        syscall!(libc::fcntl(pfd[0], libc::F_GETPIPE_SZ))
            .map(|n| SpliceFuture {
                from,
                to,
                buffered: 0,
                pipe_size: n as usize,
                pfd_r: pfd[0],
                pfd_w: pfd[1],
            })
            .map_err(|e| {
                unsafe {
                    libc::close(pfd[0]);
                    libc::close(pfd[1]);
                };
                e
            })
    }
}

impl Future for SpliceFuture {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        unimplemented!()
    }
}
