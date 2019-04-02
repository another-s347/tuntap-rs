use libc;
use nix::sys::epoll::{epoll_create, epoll_wait, EpollEvent};
use nix::sys::epoll::EpollOp::EpollCtlAdd;
use nix::unistd::read;
use futures::stream::Stream;
use futures::task::Waker;
use futures::Poll;
use std::thread::{JoinHandle,self};
use std::os::unix::io::RawFd;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::{Mutex, Arc};
use crate::os::nix::open_tuntap_device;

lazy_static! {
    static ref EPOLL_GLOBAL:EpollContext = EpollContext::new().unwrap();
}

pub struct FdStream {
    pub context:EpollContext,
    pub fd:RawFd,
    pub waker:Option<Waker>,
    pub buf:Vec<u8>
}

impl Stream for FdStream {
    type Item = ();

    fn poll_next(mut self: Pin<&mut Self>, waker: &Waker) -> Poll<Option<Self::Item>> {
        if self.waker.is_none() {
            self.waker=Some(waker.clone());
            // set waker
            let fd = self.fd.clone();
            self.context.add_fd(fd,waker.clone());
            return Poll::Pending;
        }
        let this_waker = (&self).waker.clone().unwrap();
        match read(self.fd,&mut self.buf) {
            Ok(size)=>{
                println!("read size == {}",size);
                Poll::Ready(Some(()))
            }
            Err(nix::Error::Sys(EAGAIN))=>{
                waker.will_wake(&this_waker);
                Poll::Pending
            }
            _=>{
                Poll::Ready(None)
            }
        }
    }
}

#[derive(Clone)]
pub struct EpollContext {
    pub epoll_fd:RawFd,
    pub wakers:Arc<Mutex<HashMap<RawFd,Waker>>>
}

impl EpollContext {
    pub fn new()->Option<Self> {
        epoll_create().map(|fd|{
            EpollContext {
                epoll_fd:fd,
                wakers:Arc::new(Mutex::new(HashMap::new()))
            }
        }).ok()
    }

    pub fn spawn_executor(&self)->JoinHandle<()> {
        let epoll_fd = self.epoll_fd;
        let wakers = self.wakers.clone();
        thread::spawn(move||{
            let mut events: Vec<EpollEvent> = Vec::new();
            for _ in 0..2 {
                events.push(EpollEvent::empty());
            }
            loop {
                println!("wait");
                let c=epoll_wait(epoll_fd,&mut events,300000).unwrap();
                println!("size {}",c);
                wakers.lock().map(|w|{
                    for i in &events {
                        let fd = i.data() as i32;
                        if let Some(waker)=w.get(&fd) {
                            waker.wake();
                        }
                        else{
                            println!("??? == {}",fd);
                        }
                    }
                });
            }
        })
    }

    pub fn add_fd(&mut self,fd:RawFd,waker:Waker) {
        self.wakers.lock().unwrap().insert(fd,waker);
        let mut epoll_event = nix::sys::epoll::EpollEvent::new(nix::sys::epoll::EpollFlags::EPOLLIN, fd as u64);
        nix::sys::epoll::epoll_ctl(self.epoll_fd, EpollCtlAdd, fd, Some(&mut epoll_event)).unwrap();
        println!("added");
    }
}

pub fn try_epoll() {
    let epoll_fd = epoll_create().unwrap();
    //let tap=0;
    let tap = open_tuntap_device("tap1".to_string(),true).unwrap();
    let mut epoll_event=nix::sys::epoll::EpollEvent::new(nix::sys::epoll::EpollFlags::EPOLLIN,tap as u64);
    nix::sys::epoll::epoll_ctl(epoll_fd,EpollCtlAdd,tap,Some(&mut epoll_event));
    let mut events:Vec<EpollEvent> = Vec::new();
    for _ in 0..1 {
        events.push(EpollEvent::empty());
    }
    let mut buffer = vec![0u8;1024];
    loop {
        let c=epoll_wait(epoll_fd,&mut events,300000).unwrap();
        println!("c == {}",c);
        for i in &events {
            let size=read(i.data() as i32,buffer.as_mut()).unwrap();
            println!("read size = {}",size);
        }
    }
}