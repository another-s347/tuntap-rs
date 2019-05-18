use std::ffi::{CString};
use std::os::raw::{c_char, c_short};

use libc;

const IFNAMSIZ: usize = libc::IFNAMSIZ;
const IFREQ_SIZE: usize = 40;
const TUNSETIFF: u64 = 0x400454ca;

#[repr(C)]
pub struct raw_ifreq {
    pub ifrn_name: [c_char; IFNAMSIZ],
    pub ifr_flags: c_short,
    pub padding: [c_char; IFREQ_SIZE - IFNAMSIZ - 2],
}

pub struct Ifreq {
    pub name: String,
    pub flags: c_short,
}

impl Ifreq {
    fn to_raw(&self) -> Option<raw_ifreq> {
        let c_str = CString::new(self.name.clone()).unwrap();
        let c_name = c_str.as_bytes();
        if c_name.len() > IFNAMSIZ {
            return None;
        }
        let mut raw = raw_ifreq {
            ifrn_name: [0; 16],
            ifr_flags: 0,
            padding: [0; IFREQ_SIZE - IFNAMSIZ - 2],
        };
        for i in 0..c_name.len() {
            raw.ifrn_name[i] = c_name[i] as i8;
        }
        raw.ifr_flags = self.flags;
        Some(raw)
    }

    pub fn has_flag(&self, flag: IreqFlag) -> bool {
        self.flags & (flag as i16) != 0
    }

    pub fn set_flag(mut self, flag: IreqFlag) -> Self {
        self.flags = self.flags | (flag as i16);
        self
    }
}

pub enum IreqFlag {
    IffTun = 0x0001,
    IffTap = 0x0002,
    IffNoPi = 0x1000,
}

pub fn open_tuntap_device(device: String, non_blocking: bool) -> Option<i32> {
    let tun_path = CString::new("/dev/net/tun").unwrap();
    let fd = unsafe {
        libc::open(tun_path.as_ptr(), libc::O_RDWR)
    };
    if fd < 0 {
        println!("open fail");
        return None;
    }
    let s = Ifreq {
        name: device,
        flags: 0,
    }.set_flag(IreqFlag::IffTap).set_flag(IreqFlag::IffNoPi).to_raw().unwrap();
    if unsafe {
        libc::ioctl(fd, TUNSETIFF, &s)
    } < 0 {
        println!("ioctl err");
        unsafe { libc::close(fd); }
        return None;
    }
    if non_blocking {
        if unsafe {
            libc::fcntl(fd, libc::F_SETFL, libc::O_NONBLOCK)
        } < 0 {
            println!("set non-blocking err");
            unsafe { libc::close(fd); }
            return None;
        }
    }
    Some(fd)
}