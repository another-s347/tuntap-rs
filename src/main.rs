#![feature(futures_api)]
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

use crate::os::nix::{simple_demo, open_tuntap_device};
use futures::StreamExt;
use crate::os::linux::try_epoll;

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

fn main() {
    let context=os::linux::EpollContext::new().unwrap();
    let tap = open_tuntap_device("tap1".to_string(),true).unwrap();
    let tap_stream = os::linux::FdStream {
        context:context.clone(),
        fd: tap,
        waker: None,
        buf: vec![0u8;024]
    };
    context.spawn_executor();
    futures::executor::block_on(tap_stream.for_each(|_|{
        println!("execute!");
        futures::future::ready(())
    }));
}