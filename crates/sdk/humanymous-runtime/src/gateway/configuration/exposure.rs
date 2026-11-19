//! Listener exposure rules of owner decision B01 #1 (issue #75): a public
//! edge listener, a separate admin TCP listener and the Core role, each by
//! explicit opt-in only. Shared by the Gate configuration, the Core role and
//! their accept loops so one rule decides every listener.
//!
//! - The default is loopback only: the listen address must be a loopback IP
//!   and only loopback peers are served.
//! - A non-loopback listen needs the explicit `public_listen` key and a TLS
//!   listener whose certificate is not the development self-signed one. There
//!   is no public plaintext listener (Go `cmd/gate` and `cmd/server` always
//!   served the public port over TLS).
//! - `public_listen` with a loopback address is refused: the key must mean
//!   what it says, so readiness never reports a public edge that is not one.
//! - An admin listener is always loopback only; no key widens it (Go
//!   defaulted `-admin-addr` to loopback and only warned on a wider bind).
use crate::gateway::tls::listener::ListenerTls;
use std::net::SocketAddr;

/// Whether `address` may be bound for a serving listener with this exposure.
pub fn listen_permitted(address: SocketAddr, public: bool, tls: Option<&ListenerTls>) -> bool {
    if public {
        !address.ip().is_loopback() && tls.is_some_and(|tls| !tls.dev_self_signed)
    } else {
        address.ip().is_loopback()
    }
}

/// Whether an accepted peer is served. Only an explicitly public listener
/// serves non-loopback peers; a loopback-only listener drops them.
pub fn peer_admitted(peer: SocketAddr, public: bool) -> bool {
    public || peer.ip().is_loopback()
}

/// Whether a bound listener's local address matches its exposure. Accept
/// loops check this before serving, so a listener bound outside the
/// validated configuration is refused instead of widened.
pub fn bound_permitted(local: SocketAddr, public: bool) -> bool {
    public || local.ip().is_loopback()
}

/// An admin listener address: loopback only, never widened.
pub fn admin_listen_permitted(address: SocketAddr) -> bool {
    address.ip().is_loopback()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tls(dev: bool) -> ListenerTls {
        ListenerTls {
            certificate: Some("chain.pem".into()),
            key: Some("key.pem".into()),
            dev_self_signed: dev,
            ..ListenerTls::default()
        }
    }

    #[test]
    fn public_listen_needs_the_key_and_a_real_tls_source() {
        let public: SocketAddr = "0.0.0.0:8444".parse().unwrap();
        let interface: SocketAddr = "192.0.2.10:8444".parse().unwrap();
        let loopback: SocketAddr = "127.0.0.1:8444".parse().unwrap();
        let provisioned = tls(false);
        // Default: loopback only, with or without TLS.
        assert!(listen_permitted(loopback, false, None));
        assert!(listen_permitted(loopback, false, Some(&provisioned)));
        assert!(!listen_permitted(public, false, None));
        assert!(!listen_permitted(public, false, Some(&provisioned)));
        assert!(!listen_permitted(interface, false, Some(&provisioned)));
        // Explicit opt-in: TLS required, no plaintext, no dev certificate.
        assert!(!listen_permitted(public, true, None));
        assert!(!listen_permitted(public, true, Some(&tls(true))));
        assert!(listen_permitted(public, true, Some(&provisioned)));
        assert!(listen_permitted(interface, true, Some(&provisioned)));
        assert!(listen_permitted(
            "[::]:8444".parse().unwrap(),
            true,
            Some(&provisioned)
        ));
        // The key never applies to a loopback address.
        assert!(!listen_permitted(loopback, true, Some(&provisioned)));
    }

    #[test]
    fn non_loopback_peers_reach_only_a_public_listener() {
        let remote: SocketAddr = "198.51.100.7:50000".parse().unwrap();
        let local: SocketAddr = "127.0.0.1:50000".parse().unwrap();
        assert!(peer_admitted(local, false));
        assert!(!peer_admitted(remote, false));
        assert!(peer_admitted(remote, true));
        assert!(peer_admitted(local, true));
        assert!(bound_permitted("127.0.0.1:1".parse().unwrap(), false));
        assert!(!bound_permitted("0.0.0.0:1".parse().unwrap(), false));
        assert!(bound_permitted("0.0.0.0:1".parse().unwrap(), true));
    }

    #[test]
    fn admin_listen_is_loopback_only() {
        assert!(admin_listen_permitted("127.0.0.1:8445".parse().unwrap()));
        assert!(admin_listen_permitted("[::1]:8445".parse().unwrap()));
        assert!(!admin_listen_permitted("0.0.0.0:8445".parse().unwrap()));
        assert!(!admin_listen_permitted("192.0.2.10:8445".parse().unwrap()));
    }
}
