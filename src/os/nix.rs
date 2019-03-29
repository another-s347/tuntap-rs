use libc;
use std::ffi::{CString, c_void};
use std::os::raw::{c_short, c_char, c_int};
use std::mem::size_of;
use futures::stream::Stream;
use futures::task::Waker;
use futures::Poll;
use bytes::{BytesMut, BufMut};
use std::pin::Pin;
use futures::executor::ThreadPool;
use std::fs::OpenOptions;
use futures::StreamExt;
use futures::future::ready;
use std::io::Error;

const IFNAMSIZ:usize = libc::IFNAMSIZ;
const ifreq_SIZE:usize = 40;
const TUNSETIFF:u64 = 0x400454ca;

#[repr(C)]
pub struct raw_ifreq {
    pub ifrn_name: [c_char;IFNAMSIZ],
    pub ifr_flags: c_short,
    pub padding:[c_char;ifreq_SIZE-IFNAMSIZ-2]
}

pub struct ifreq {
    pub name:String,
    pub flags:c_short
}

impl ifreq {
    fn to_raw(&self)->Option<raw_ifreq> {
        let c_str = CString::new(self.name.clone()).unwrap();
        let c_name = c_str.as_bytes();
        if c_name.len() > IFNAMSIZ {
            return None;
        }
        let mut raw = raw_ifreq {
            ifrn_name:[0;16],
            ifr_flags:0,
            padding:[0;ifreq_SIZE-IFNAMSIZ-2]
        };
        for i in 0..c_name.len() {
            raw.ifrn_name[i]=c_name[i] as i8;
        }
        raw.ifr_flags = self.flags;
        Some(raw)
    }

    pub fn has_flag(&self,flag:ireq_flag)->bool {
        self.flags & (flag as i16) != 0
    }

    pub fn set_flag(mut self,flag:ireq_flag)->Self {
        self.flags = self.flags | (flag as i16);
        self
    }
}

pub enum ireq_flag {
    IFF_TUN=0x0001,
    IFF_TAP=0x0002,
    IFF_NO_PI=0x1000
}

pub struct TunTapBiStream {
    pub fd: c_int,
    pub buffer: [u8;1024],
    pub counter :u32
}

impl Stream for TunTapBiStream {
    type Item = BytesMut;

    fn poll_next(mut self: Pin<&mut Self>, waker: &Waker) -> Poll<Option<Self::Item>> {
        let nread = unsafe {
            libc::read(self.fd,self.buffer.as_mut_ptr() as *mut c_void,1024)
        };
        if nread < 0 {
            if Error::last_os_error().raw_os_error().unwrap()==libc::EAGAIN {
                waker.wake();
                return Poll::Pending;
            }
            else {
                return Poll::Ready(None);
            }
        }
        let mut r=BytesMut::with_capacity(nread as usize);
        r.put_slice(&self.buffer[0..nread as usize]);
        return Poll::Ready(Some(r));
    }
}

pub fn open_tuntap_device(device:String) -> Option<TunTapBiStream> {
    let s = libc::aiocb {
        
    };
    let tun_path = CString::new("/dev/net/tun").unwrap();
    let fd = unsafe {
        libc::open(tun_path.as_ptr(),libc::O_RDWR)
    };
    if fd < 0 {
        println!("open fail");
        return None;
    }
    let mut s = ifreq {
        name: device,
        flags: 0
    }.set_flag(ireq_flag::IFF_TAP).set_flag(ireq_flag::IFF_NO_PI).to_raw().unwrap();
    let r = unsafe {
        libc::ioctl(fd,TUNSETIFF, &s)
    };
    if r < 0 {
        println!("ioctl err");
        unsafe { libc::close(fd); }
        return None;
    }
//    unsafe {
//        let flags = libc::fcntl(fd,libc::F_GETFL,0);
//        libc::fcntl(fd,libc::F_SETFL,flags | libc::O_NONBLOCK);
//    }
    Some(TunTapBiStream {
        fd,
        buffer:[0u8;1024],
        counter:0
    })
}

fn simple_demo() {
    let tun_path = CString::new("/dev/net/tun").unwrap();
    let fd = unsafe {
        libc::open(tun_path.as_ptr(),libc::O_RDWR)
    };
    if fd < 0 {
        println!("open fail");
        return;
    }
    let mut s = ifreq {
        name: "w1".to_string(),
        flags: 0
    }.set_flag(ireq_flag::IFF_TAP).set_flag(ireq_flag::IFF_NO_PI).to_raw().unwrap();
    let r = unsafe {
        libc::ioctl(fd,TUNSETIFF, &s)
    };
    if r < 0 {
        println!("ioctl err");
        unsafe { libc::close(fd); }
        return;
    }
    let mut buffer=[0u8;1024];
    loop {
        unsafe {
            let nread = libc::read(fd,buffer.as_mut_ptr() as *mut c_void,1024);
            if nread < 0 {
                libc::close(fd);
                break;
            }
            println!("read {} bytes from device",nread);
        }
    }
}

pub fn stream_demo() {
    let stream = open_tuntap_device("w1".to_string()).unwrap();
    let app = stream.for_each(|data|{
        println!("read {} bytes from device",data.len());
        ready(())
    });
    ThreadPool::new().unwrap().run(app);
}
