#![feature(futures_api)]
#![feature(bind_by_move_pattern_guards)]
#![feature(maybe_uninit)]
#[macro_use]
extern crate lazy_static;

//use std::collections::HashMap;
//use std::str::Bytes;
//use std::thread;
//use std::thread::sleep;
//use std::time::Duration;
//
//use actix::prelude::*;
//use bytes::buf::BufMut;
//use bytes::BytesMut;
//use tokio::codec::{BytesCodec, FramedWrite};
//use tokio::fs::File;
//use tokio::io::{ReadHalf, WriteHalf};
//use tokio::prelude::*;
//use tokio::prelude::future::Future;
//use tokio::prelude::future::lazy;
//use tokio::prelude::stream::Stream;
//use tokio::sync::mpsc::*;
#[cfg(target_os = "linux")]
pub mod os;

//fn test() {
//    let mut w1=os::nix::open_tuntap_device("w1".to_string(),false).unwrap();
//    let mut w2=os::nix::open_tuntap_device("w2".to_string(),false).unwrap();
//    let mut w1_clone = w1.try_clone().unwrap();
//    let mut w2_clone = w2.try_clone().unwrap();
//    let (w1_read,_) = tokio::fs::File::from_std(w1).split();
//    let (_,w1_write) = tokio::fs::File::from_std(w1_clone).split();
//    let (w2_read,_) = tokio::fs::File::from_std(w2).split();
//    let (_,w2_write) = tokio::fs::File::from_std(w2_clone).split();
//    let task1 = tokio::io::copy(w1_read,w2_write).map_err(|e|{
//        dbg!(e);
//    }).map(|_|());
//    let task2 = tokio::io::copy(w2_read,w1_write).map_err(|e|{
//        dbg!(e);
//    }).map(|_|());
//    tokio::run(lazy(||{
//        tokio::spawn(task1);
//        tokio::spawn(task2);
//        Ok(())
//    }));
//}
#[cfg(test)]
#[cfg(target_os = "linux")]
mod tests {
    use core::borrow::BorrowMut;

    use bytes::buf::BufMut;
    use bytes::BytesMut;
    use futures::future::lazy;
    use futures::StreamExt;
    use futures::task::SpawnExt;

    use crate::os;
    use crate::os::nix::{open_tuntap_device, simple_demo};
    use crate::os::{TunTap};
    use futures::future::{Future,FutureExt};
    use std::time::SystemTime;
    use std::collections::VecDeque;

    #[test]
    fn main() {
        let context = os::linux::EpollContext::new().unwrap();
        let tap = os::TunTap::new("tap1".to_string(), true).unwrap();
        let tap2 = os::TunTap::new("tap2".to_string(), true).unwrap();
        let (tap_read,_) = tap.split_to_epoll_stream(context.clone());
        let (_, tap_writer) = tap2.split_to_epoll_stream(context.clone());
        let task = tap_read.take(10).forward(tap_writer);
        context.spawn_executor();
        futures::executor::block_on(task);
    }
}