#[cfg(target_os = "linux")]
pub mod os;

#[cfg(test)]
#[cfg(target_os = "linux")]
mod tests {
    use tokio::prelude::*;

    use crate::os;

    #[test]
    fn test_fut() {
        let tap1 = os::TunTap::new("tap1".to_string(), true).unwrap().into_tokio();
        let (r1, w1) = tap1.split();
        let tap2 = os::TunTap::new("tap2".to_string(), true).unwrap().into_tokio();
        let (r2, w2) = tap2.split();
        tokio::run(futures::lazy(|| {
            tokio::spawn(tokio::io::copy(r1, w2).map_err(|x| {
                dbg!(x);
            }).map(|_| ()));
            tokio::spawn(tokio::io::copy(r2, w1).map_err(|x| {
                dbg!(x);
            }).map(|_| ()));
            Ok(())
        }));
    }
}