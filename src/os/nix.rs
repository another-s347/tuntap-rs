use std::collections::HashMap;
use std::ffi::{c_void, CString};
use std::fs::File as StdFile;
use std::fs::OpenOptions;
use std::io::{Error, Read};
use std::marker::PhantomData;
use std::mem;
use std::mem::size_of;
use std::os::raw::{c_char, c_int, c_short};
use std::os::unix::io::FromRawFd;
use std::pin::Pin;
use std::sync::atomic::Ordering;
use std::sync::Mutex;
use std::thread;
use std::thread::sleep;
use std::thread::ThreadId;
use std::time::Duration;

use bytes::{BufMut, BytesMut};
use libc;
use nix;
use nix::sys::aio::{aio_suspend, AioCb};
use nix::sys::signal::{SigAction, sigaction, Signal, SigSet};
use nix::sys::signal;
use nix::sys::signal::SaFlags;
use nix::sys::signal::SigHandler;
use tokio;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::io::ReadHalf;
use tokio::prelude::*;
use tokio::prelude::Stream;

const IFNAMSIZ: usize = libc::IFNAMSIZ;
const ifreq_SIZE: usize = 40;
const TUNSETIFF: u64 = 0x400454ca;

#[repr(C)]
pub struct raw_ifreq {
    pub ifrn_name: [c_char; IFNAMSIZ],
    pub ifr_flags: c_short,
    pub padding: [c_char; ifreq_SIZE - IFNAMSIZ - 2],
}

pub struct ifreq {
    pub name: String,
    pub flags: c_short,
}

impl ifreq {
    fn to_raw(&self) -> Option<raw_ifreq> {
        let c_str = CString::new(self.name.clone()).unwrap();
        let c_name = c_str.as_bytes();
        if c_name.len() > IFNAMSIZ {
            return None;
        }
        let mut raw = raw_ifreq {
            ifrn_name: [0; 16],
            ifr_flags: 0,
            padding: [0; ifreq_SIZE - IFNAMSIZ - 2],
        };
        for i in 0..c_name.len() {
            raw.ifrn_name[i] = c_name[i] as i8;
        }
        raw.ifr_flags = self.flags;
        Some(raw)
    }

    pub fn has_flag(&self, flag: ireq_flag) -> bool {
        self.flags & (flag as i16) != 0
    }

    pub fn set_flag(mut self, flag: ireq_flag) -> Self {
        self.flags = self.flags | (flag as i16);
        self
    }
}

pub enum ireq_flag {
    IFF_TUN = 0x0001,
    IFF_TAP = 0x0002,
    IFF_NO_PI = 0x1000,
}

pub fn open_tuntap_device(device: String) -> Option<StdFile> {
    let tun_path = CString::new("/dev/net/tun").unwrap();
    let fd = unsafe {
        libc::open(tun_path.as_ptr(), libc::O_RDWR)
    };
    if fd < 0 {
        println!("open fail");
        return None;
    }
    let mut s = ifreq {
        name: device,
        flags: 0,
    }.set_flag(ireq_flag::IFF_TAP).set_flag(ireq_flag::IFF_NO_PI).to_raw().unwrap();
    let r = unsafe {
        libc::ioctl(fd, TUNSETIFF, &s)
    };
    if r < 0 {
        println!("ioctl err");
        unsafe { libc::close(fd); }
        return None;
    }
    Some(unsafe { StdFile::from_raw_fd(fd) })
}

pub fn simple_demo() {
    let tun_path = CString::new("/dev/net/tun").unwrap();
    let fd = unsafe {
        libc::open(tun_path.as_ptr(), libc::O_RDWR)
    };
    if fd < 0 {
        println!("open fail");
        return;
    }
    let mut s = ifreq {
        name: "w1".to_string(),
        flags: 0,
    }.set_flag(ireq_flag::IFF_TAP).set_flag(ireq_flag::IFF_NO_PI).to_raw().unwrap();
    let r = unsafe {
        libc::ioctl(fd, TUNSETIFF, &s)
    };
    if r < 0 {
        println!("ioctl err");
        unsafe { libc::close(fd); }
        return;
    }
    let mut stdfile: StdFile = unsafe {
        StdFile::from_raw_fd(fd)
    };
    let mut tokiofile = tokio::fs::File::from_std(stdfile);
    let reader = tokio::codec::FramedRead::new(tokiofile, tokio::codec::BytesCodec::new());
    //stdfile.write_all(&mut vec![1,2,3]);
    let task = reader.for_each(|p| {
        println!("{}", p.len());
        Ok(())
    }).map_err(|_| ());
    tokio::run(task)
}