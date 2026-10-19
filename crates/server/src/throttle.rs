//! In-process request throttles and per-device bookkeeping throttles.
//!
//! Counters are per server process and fixed-window. They bound anonymous
//! pressure on shared resources; PostgreSQL remains the authority for limits
//! that must hold across replicas.
use axum::{
    extract::{ConnectInfo, FromRequestParts},
    http::{HeaderMap, request::Parts},
};
use std::{
    collections::HashMap,
    convert::Infallible,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

const WINDOW: Duration = Duration::from_secs(60);
const MAX_KEYS: usize = 4096;

pub type Attempts = Arc<Mutex<HashMap<String, (Instant, u32)>>>;
pub type Seen<K> = Arc<Mutex<HashMap<K, Instant>>>;

/// Count one attempt for `key` and report whether it is within `limit` per
/// minute. A full table refuses new keys rather than evicting live ones.
pub fn allow(attempts: &Attempts, key: &str, limit: u32) -> bool {
    let mut attempts = attempts.lock().unwrap_or_else(|e| e.into_inner());
    let now = Instant::now();
    attempts.retain(|_, (at, _)| now.duration_since(*at) < WINDOW);
    if attempts.len() >= MAX_KEYS && !attempts.contains_key(key) {
        return false;
    }
    let (_, count) = attempts.entry(key.to_owned()).or_insert((now, 0));
    *count += 1;
    *count <= limit
}

/// True when `key` was not recorded within `interval`, recording it now. A
/// caller that fails afterwards should `forget` the key so the work is retried.
pub fn due<K: std::hash::Hash + Eq>(seen: &Seen<K>, key: K, interval: Duration) -> bool {
    let mut seen = seen.lock().unwrap_or_else(|e| e.into_inner());
    let now = Instant::now();
    if seen.len() >= MAX_KEYS {
        seen.retain(|_, at| now.duration_since(*at) < interval);
    }
    match seen.get(&key) {
        Some(at) if now.duration_since(*at) < interval => false,
        _ => {
            seen.insert(key, now);
            true
        }
    }
}

pub fn forget<K: std::hash::Hash + Eq>(seen: &Seen<K>, key: &K) {
    seen.lock().unwrap_or_else(|e| e.into_inner()).remove(key);
}

/// The caller's network address. Behind the bundled reverse proxy the socket
/// peer is a private address, so only then is the proxy-appended last
/// `X-Forwarded-For` entry believed; a public peer is never taken at its word.
/// Without connection info (in-process requests) every caller shares one key.
pub struct Peer(pub IpAddr);

fn private(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => ip.is_private() || ip.is_loopback() || ip.is_link_local(),
        IpAddr::V6(ip) => {
            ip.is_loopback() || (ip.segments()[0] & 0xfe00) == 0xfc00 || ip.is_unicast_link_local()
        }
    }
}

fn address(peer: Option<IpAddr>, headers: &HeaderMap) -> IpAddr {
    let Some(peer) = peer else {
        return IpAddr::V4(Ipv4Addr::UNSPECIFIED);
    };
    if private(peer)
        && let Some(forwarded) = headers
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.rsplit(',').next())
            .and_then(|v| v.trim().parse::<IpAddr>().ok())
    {
        return forwarded;
    }
    peer
}

impl<S: Send + Sync> FromRequestParts<S> for Peer {
    type Rejection = Infallible;
    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Infallible> {
        let peer = parts
            .extensions
            .get::<ConnectInfo<SocketAddr>>()
            .map(|c| c.0.ip());
        Ok(Self(address(peer, &parts.headers)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_count_and_bound_keys() {
        let attempts = Attempts::default();
        for _ in 0..3 {
            assert!(allow(&attempts, "a", 3));
        }
        assert!(!allow(&attempts, "a", 3));
        assert!(allow(&attempts, "b", 3));
        for i in 0..MAX_KEYS {
            allow(&attempts, &format!("k{i}"), 1);
        }
        assert!(!allow(&attempts, "new", 1));
    }

    #[test]
    fn due_fires_once_per_interval_and_after_forget() {
        let seen: Seen<u8> = Seen::default();
        assert!(due(&seen, 1, Duration::from_secs(60)));
        assert!(!due(&seen, 1, Duration::from_secs(60)));
        assert!(due(&seen, 2, Duration::from_secs(60)));
        forget(&seen, &1);
        assert!(due(&seen, 1, Duration::from_secs(60)));
        assert!(due(&seen, 3, Duration::ZERO));
        assert!(due(&seen, 3, Duration::ZERO));
    }

    #[test]
    fn forwarded_address_is_believed_only_from_a_private_peer() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "9.9.9.9, 203.0.113.5".parse().unwrap());
        let proxy: IpAddr = "172.18.0.2".parse().unwrap();
        let public: IpAddr = "198.51.100.7".parse().unwrap();
        assert_eq!(
            address(Some(proxy), &headers),
            "203.0.113.5".parse::<IpAddr>().unwrap()
        );
        assert_eq!(address(Some(public), &headers), public);
        assert_eq!(address(Some(proxy), &HeaderMap::new()), proxy);
        assert_eq!(address(None, &headers), IpAddr::V4(Ipv4Addr::UNSPECIFIED));
    }
}
