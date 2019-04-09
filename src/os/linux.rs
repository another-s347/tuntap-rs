use libc;
use nix::sys::epoll::{epoll_create, epoll_wait, EpollEvent};
use nix::sys::epoll::EpollOp::EpollCtlAdd;
use nix::unistd::{read, write};
use futures::stream::Stream;
use futures::task::Waker;
use futures::{Poll, Future};
use std::thread::{JoinHandle,self};
use std::os::unix::io::RawFd;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::{Mutex, Arc, RwLock};
use crate::os::nix::open_tuntap_device;
use bytes::{BytesMut, Bytes};
use nix::sys::epoll::EpollFlags;
use core::borrow::BorrowMut;
use futures::prelude::sink::Sink;
use bytes::buf::BufMut;

lazy_static! {
    static ref EPOLL_GLOBAL:EpollContext = EpollContext::new().unwrap();
}

pub struct FdReadStream {
    pub context:EpollContext,
    pub fd:RawFd,
    pub waker:Option<Waker>,
    pub buf:BytesMut
}

impl Stream for FdReadStream {
    type Item = ();

    fn poll_next(mut self: Pin<&mut Self>, waker: &Waker) -> Poll<Option<Self::Item>> {
        if self.waker.is_none() {
            self.waker=Some(waker.clone());
            let fd = self.fd.clone();
            self.context.add_fd(fd,waker.clone(),nix::sys::epoll::EpollFlags::EPOLLIN);
            return Poll::Pending;
        }
        let this_waker = (&self).waker.clone().unwrap();
        match read(self.fd, self.buf.borrow_mut()) {
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

pub struct FdWriteFuture {
    pub context:EpollContext,
    pub fd:RawFd,
    pub waker:Option<Waker>,
    pub buf:BytesMut
}

impl Sink for FdWriteFuture {
    type SinkItem = Bytes;
    type SinkError = ();

    fn poll_ready(self:Pin<&mut Self>, waker: &Waker) -> Poll<Result<(), Self::SinkError>> {
        Poll::Ready(Ok(()))
    }

    fn start_send(mut self:Pin<&mut Self>, item: Self::SinkItem) -> Result<(), Self::SinkError> {
        if item.len() == 0 {
            return Ok(())
        }
        self.buf.reserve(item.len());
        self.buf.put(item);
        Ok(())
    }

    fn poll_flush(mut self:Pin<&mut Self>, waker: &Waker) -> Poll<Result<(), Self::SinkError>> {
        if self.waker.is_none() {
            self.waker=Some(waker.clone());
            let fd = self.fd.clone();
            self.context.add_fd(fd,waker.clone(),nix::sys::epoll::EpollFlags::EPOLLOUT|nix::sys::epoll::EpollFlags::EPOLLONESHOT);
        }
        if self.buf.len() == 0 {
            println!("flushed");
            return Poll::Ready(Ok(()));
            }
        let this_waker = (&self).waker.clone().unwrap();
        match write(self.fd, self.buf.as_ref()) {
            Ok(size)=>{
                println!("write == {}",size);
                self.buf.advance(size);
                if self.buf.len() == 0 {
                    return Poll::Ready(Ok(()))
                }
                else {
                    unimplemented!("what to do with waker?");
                    // what to do with waker?
                    return Poll::Pending;
                }
            }
            Err(nix::Error::Sys(EAGAIN))=>{
                waker.will_wake(&this_waker);
//                println!("eagain");
                return Poll::Pending;
            }
            e=>{
                dbg!(e);
                Poll::Ready(Err(()))
            }
            }
        }

    fn poll_close(self:Pin<&mut Self>, waker: &Waker) -> Poll<Result<(), Self::SinkError>> {
        self.poll_flush(waker)
    }
}

#[derive(Clone)]
pub struct EpollContext {
    pub epoll_fd:RawFd,
    pub wakers:Arc<RwLock<HashMap<u64,Waker>>>,
    pub counter: Arc<RwLock<u64>>
}

impl EpollContext {
    pub fn new()->Option<Self> {
        epoll_create().map(|fd|{
            EpollContext {
                epoll_fd:fd,
                wakers:Arc::new(RwLock::new(HashMap::new())),
                counter:Arc::new(RwLock::new(1))
            }
        }).ok()
    }

    pub fn spawn_executor(&self)->JoinHandle<()> {
        let epoll_fd = self.epoll_fd;
        let wakers = self.wakers.clone();
        thread::spawn(move||{
            let mut events: Vec<EpollEvent> = Vec::new();
            for _ in 0..10 {
                events.push(EpollEvent::empty());
            }
            loop {
                let c=epoll_wait(epoll_fd,&mut events,-1).unwrap();
                wakers.read().map(|w|{
                    for i in &events {
                        let fd = i.data();
                        if let Some(waker)=w.get(&fd) {
                            waker.wake();
                        }
                    }
                });
            }
        })
    }

    pub fn add_fd(&mut self,fd:RawFd,waker:Waker,flags:EpollFlags) -> Result<(),()> {
        let id = {
            let mut counter: RwLockWriteGuard<u64> = self.counter.write().unwrap();
            let id:u64 = (*counter).clone();
            *counter+=1;
            let mut wakers = self.wakers.write().unwrap();
            wakers.insert(id,waker);
            id
        };
        let mut epoll_event = nix::sys::epoll::EpollEvent::new(flags, id);
        nix::sys::epoll::epoll_ctl(self.epoll_fd, EpollCtlAdd, fd, Some(&mut epoll_event)).map_err(|e|{
            dbg!(e);
        })
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