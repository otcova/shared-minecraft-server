use std::{
    sync::{Arc, Condvar, Mutex, RwLock},
    thread,
};

use crate::fetch::fetch_str;

#[derive(Clone)]
enum PubIpCache {
    /// The public ip is not on the cache, it needs to be fetched.
    Empty,
    /// The public ip is being currently fetched.
    Fetching(Arc<(Mutex<bool>, Condvar)>),
    /// Could not found public ip.
    Error,
    Ip(String),
}

static PUB_IP_CACHE: RwLock<PubIpCache> = RwLock::new(PubIpCache::Empty);

/// Loads public ip asyncronous
pub fn fetch() {
    let mut pub_ip_lock = PUB_IP_CACHE.write().unwrap();
    match &*pub_ip_lock {
        PubIpCache::Empty => {
            let fetch_ended_flag = Arc::new((Mutex::new(false), Condvar::new()));
            *pub_ip_lock = PubIpCache::Fetching(fetch_ended_flag.clone());

            thread::spawn(move || {
                let ip = [
                    "https://api.my-ip.io/ip",
                    "https://ifconfig.me/ip",
                    "https://api.ipify.org",
                ]
                .iter()
                .find_map(|url| fetch_str(url));

                if let Some(ip) = ip {
                    *PUB_IP_CACHE.write().unwrap() = PubIpCache::Ip(ip);
                } else {
                    *PUB_IP_CACHE.write().unwrap() = PubIpCache::Error;
                }

                let (ended_flag, condvar) = &*fetch_ended_flag;
                *ended_flag.lock().unwrap() = true;
                condvar.notify_all();
            });
        }
        _ => {}
    }
}

/// It will block the thread if fetch hasn't finished.
pub fn get() -> Option<String> {
    let pub_ip_lock = PUB_IP_CACHE.read().expect("Could not read public ip cache");
    match &*pub_ip_lock {
        PubIpCache::Empty => {
            drop(pub_ip_lock);
            fetch();
            get()
        }
        PubIpCache::Error => None,
        PubIpCache::Ip(ip) => Some(ip.clone()),
        PubIpCache::Fetching(fetch_ended_flag) => {
            let (lock, condvar) = &*fetch_ended_flag.clone();
            drop(pub_ip_lock);

            let mut fetch_ended = lock.lock().unwrap();
            while !*fetch_ended {
                fetch_ended = condvar.wait(fetch_ended).unwrap();
            }
            get()
        }
    }
}