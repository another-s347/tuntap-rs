use std::collections::vec_deque;
use std::collections::VecDeque;
use std::fs::{File as StdFile, read};
use std::marker::Unpin;
use std::os::raw::c_int;
use std::os::unix::io::FromRawFd;
use std::pin::Pin;
use std::time::{Duration, Instant};
use futures::future::Future;
use bytes::BytesMut;
use futures::{Poll, Stream};
use futures::task::Waker;
use tokio::fs::File as TokioFile;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::io::{ReadHalf, WriteHalf};

use ::nix::unistd::close;

use crate::os::linux::{EpollContext, FdReadStream, FdWriteFuture};
use std::sync::Arc;

pub mod nix;
pub mod linux;

#[cfg(target_os = "linux")]
pub struct TunTap {
    inner: Arc<Inner>
}

#[cfg(target_os = "linux")]
impl TunTap {
    pub fn new(device: String, non_blocking: bool) -> Option<TunTap> {
        nix::open_tuntap_device(device, non_blocking).map(|fd| {
            TunTap {
                inner: Arc::new(Inner(fd))
            }
        })
    }

    pub fn into_std(self) -> StdFile {
        unsafe {
            StdFile::from_raw_fd(self.inner.0)
        }
    }

    pub fn split_to_tokio_stream(&self) -> (ReadHalf<TokioFile>, WriteHalf<TokioFile>) {
        let f1 = unsafe {
            StdFile::from_raw_fd((self.as_ref()).clone())
        };
        let f2 = unsafe {
            StdFile::from_raw_fd((self.as_ref()).clone())
        };
        let (read, _) = tokio::fs::File::from_std(f1).split();
        let (_, write) = tokio::fs::File::from_std(f2).split();
        (read, write)
    }

    pub fn split_to_epoll_stream(&self, epollContext: EpollContext) -> (FdReadStream, FdWriteFuture) {
        let read_buffer_size: usize = 1024;
        let mut buffer = BytesMut::with_capacity(read_buffer_size);
        buffer.resize(read_buffer_size, 0);
        let read = linux::FdReadStream {
            context: epollContext.clone(),
            fd: (self.as_ref()).clone(),
            waker: None,
            buf: buffer,
            size: 0,
        };
        let write = linux::FdWriteFuture {
            context: epollContext,
            fd: (self.as_ref()).clone(),
            waker: None,
            buf: BytesMut::new(),
        };
        (read, write)
    }
}

impl AsRef<c_int> for TunTap {
    fn as_ref(&self) -> &i32 {
        &self.inner.as_ref().0
    }
}

struct Inner(pub c_int);

impl Drop for Inner {
    fn drop(&mut self) {
        unsafe {
            close(self.0)
        };
    }
}