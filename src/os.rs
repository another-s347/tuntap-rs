use std::fs::{File as StdFile};
use std::os::raw::c_int;
use std::os::unix::io::FromRawFd;
use std::sync::Arc;
use mio::{ PollOpt, Token, Ready};
use mio::unix::EventedFd;
use ::nix::unistd::{read, write};
use ::nix::unistd::close;
use std::io::Error;
use tokio::reactor::PollEvented2;

pub mod nix;

#[cfg(target_os = "linux")]
pub struct TunTap {
    inner: Arc<Inner>
}

#[cfg(target_os = "linux")]
impl TunTap {
    pub fn new_raw(device: String, non_blocking: bool) -> Option<i32> {
        nix::open_tuntap_device(device, non_blocking)
    }

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

    pub fn into_tokio(self) -> PollEvented2<Self> {
        tokio::reactor::PollEvented2::new(self)
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
        close(self.0).unwrap();
    }
}

impl std::io::Read for TunTap {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, std::io::Error> {
        match read(self.inner.0, buf) {
            Ok(size) => {
                Ok(size)
            }
            Err(::nix::Error::Sys(EAGAIN)) => {
                Err(std::io::Error::new(std::io::ErrorKind::WouldBlock,""))
            }
            Err(other)=>{
                Err(std::io::Error::new(std::io::ErrorKind::Other,other.to_string()))
            }
        }
    }
}

impl std::io::Write for TunTap {
    fn write(&mut self, buf: &[u8]) -> Result<usize, std::io::Error> {
        match write(self.inner.0, buf) {
            Ok(size) => {
                Ok(size)
            }
            Err(::nix::Error::Sys(EAGAIN)) => {
                Err(std::io::Error::new(std::io::ErrorKind::WouldBlock,""))
            }
            Err(other)=>{
                Err(std::io::Error::new(std::io::ErrorKind::Other,other.to_string()))
            }
        }
    }

    fn flush(&mut self) -> Result<(), std::io::Error> {
        unimplemented!()
    }
}

impl mio::event::Evented for TunTap {
    fn register(&self, poll: &mio::Poll, token: Token, interest: Ready, opts: PollOpt) -> Result<(), Error> {
        EventedFd(&self.inner.0).register(poll,token,interest,opts)
    }

    fn reregister(&self, poll: &mio::Poll, token: Token, interest: Ready, opts: PollOpt) -> Result<(), Error> {
        EventedFd(&self.inner.0).reregister(poll,token,interest,opts)
    }

    fn deregister(&self, poll: &mio::Poll) -> Result<(), Error> {
        EventedFd(&self.inner.0).deregister(poll)
    }
}