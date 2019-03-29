#![feature(futures_api)]

pub mod os;

fn main() {
    os::nix::stream_demo();
}
