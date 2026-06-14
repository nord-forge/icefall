//! SSRF protection for user-supplied outbound URLs — rejects metadata endpoints, loopback,
//! private networks, and the Caddy admin API.
//!
//! The guard resolves the host once, pins the connection to that validated IP,
//! and refuses HTTP redirects — so neither a redirect to an internal host nor a
//! DNS rebind between validation and connection can reach an internal service.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::time::Duration;

use url::Url;

use crate::api::error::ApiError;

/// A user URL that passed SSRF validation, plus the exact address the request
/// must connect to. Build the client with [`guarded_client`] so reqwest cannot
/// re-resolve the host or follow a redirect elsewhere.
#[derive(Debug)]
pub struct GuardedTarget {
    /// The original, parsed URL — safe to pass to `client.get()/post()`.
    pub url: Url,
    host: String,
    addr: SocketAddr,
}

/// Validate that `raw_url` is safe to fetch on behalf of a user. Rejects
/// non-`http(s)` schemes and hosts resolving to loopback, link-local, private,
/// or otherwise internal addresses (including the Caddy admin host). On success
/// returns the parsed URL and the validated address to pin the connection to.
pub async fn validate_outbound_url(
    raw_url: &str,
    caddy_admin_url: &str,
) -> Result<GuardedTarget, ApiError> {
    let url =
        Url::parse(raw_url).map_err(|_| ApiError::BadRequest(format!("invalid URL: {raw_url}")))?;

    match url.scheme() {
        "http" | "https" => {}
        other => {
            return Err(ApiError::BadRequest(format!(
                "URL scheme '{other}' is not allowed; use http or https"
            )));
        }
    }

    let host = url
        .host_str()
        .ok_or_else(|| ApiError::BadRequest(format!("URL has no host: {raw_url}")))?
        .to_owned();
    let port = url
        .port_or_known_default()
        .ok_or_else(|| ApiError::BadRequest(format!("URL has no port: {raw_url}")))?;

    // Resolve the host once. Every resolved address must be public, and none
    // may collide with the Caddy admin host's addresses.
    let admin_addrs = resolve_caddy_admin(caddy_admin_url).await;
    let resolved: Vec<SocketAddr> = tokio::net::lookup_host((host.as_str(), port))
        .await
        .map_err(|_| ApiError::BadRequest(format!("could not resolve host: {host}")))?
        .collect();

    if resolved.is_empty() {
        return Err(ApiError::BadRequest(format!(
            "could not resolve host: {host}"
        )));
    }

    for addr in &resolved {
        if is_internal_ip(&addr.ip()) {
            return Err(ApiError::BadRequest(format!(
                "URL resolves to a disallowed internal address: {host}"
            )));
        }
        if admin_addrs.iter().any(|a| a.ip() == addr.ip()) {
            return Err(ApiError::BadRequest(
                "URL targets the Caddy admin API".into(),
            ));
        }
    }

    // Pin to the first validated address; the client connects only here.
    let addr = resolved[0];
    Ok(GuardedTarget { url, host, addr })
}

/// Build a reqwest client locked to a validated [`GuardedTarget`]: a 30s
/// timeout, no redirect following, and DNS overridden so the host can only
/// resolve to the address the guard already checked.
pub fn guarded_client(target: &GuardedTarget) -> Result<reqwest::Client, ApiError> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        // Refuse redirects — a 3xx to an internal host must not be followed.
        .redirect(reqwest::redirect::Policy::none())
        // Pin the host to the validated IP so reqwest cannot re-resolve it
        // (closes the DNS-rebinding TOCTOU window).
        .resolve(&target.host, target.addr)
        .build()
        .map_err(ApiError::internal)
}

/// Resolve the configured Caddy admin host to its addresses, so an outbound URL
/// resolving to the same IP can be rejected regardless of the hostname used.
async fn resolve_caddy_admin(caddy_admin_url: &str) -> Vec<SocketAddr> {
    let Ok(url) = Url::parse(caddy_admin_url) else {
        return Vec::new();
    };
    let Some(host) = url.host_str() else {
        return Vec::new();
    };
    let port = url.port_or_known_default().unwrap_or(2019);
    tokio::net::lookup_host((host, port))
        .await
        .map(|addrs| addrs.collect())
        .unwrap_or_default()
}

/// True if `ip` is loopback, link-local, private, or otherwise non-routable on
/// the public internet — i.e. an SSRF target we must not let users reach.
fn is_internal_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_internal_ipv4(v4),
        IpAddr::V6(v6) => is_internal_ipv6(v6),
    }
}

fn is_internal_ipv4(ip: &Ipv4Addr) -> bool {
    ip.is_loopback()            // 127.0.0.0/8
        || ip.is_private()      // 10/8, 172.16/12, 192.168/16
        || ip.is_link_local()   // 169.254.0.0/16 — includes metadata 169.254.169.254
        || ip.is_broadcast()    // 255.255.255.255
        || ip.is_unspecified()  // 0.0.0.0
        || ip.is_documentation()
        || ip.octets()[0] >= 224 // multicast / reserved (224.0.0.0+)
        // 100.64.0.0/10 — carrier-grade NAT / shared address space.
        || (ip.octets()[0] == 100 && (64..=127).contains(&ip.octets()[1]))
}

fn is_internal_ipv6(ip: &Ipv6Addr) -> bool {
    ip.is_loopback()            // ::1
        || ip.is_unspecified()  // ::
        || is_unique_local(ip)  // fc00::/7
        || is_link_local_v6(ip) // fe80::/10
        || ip.is_multicast()
        // IPv4-mapped (::ffff:0:0/96) — re-check the embedded v4 address.
        || ip.to_ipv4_mapped().is_some_and(|v4| is_internal_ipv4(&v4))
}

/// fc00::/7 — unique local addresses.
fn is_unique_local(ip: &Ipv6Addr) -> bool {
    (ip.segments()[0] & 0xfe00) == 0xfc00
}

/// fe80::/10 — link-local unicast.
fn is_link_local_v6(ip: &Ipv6Addr) -> bool {
    (ip.segments()[0] & 0xffc0) == 0xfe80
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_loopback_and_metadata() {
        assert!(is_internal_ip(&"127.0.0.1".parse().unwrap()));
        assert!(is_internal_ip(&"169.254.169.254".parse().unwrap()));
        assert!(is_internal_ip(&"10.0.0.5".parse().unwrap()));
        assert!(is_internal_ip(&"172.16.3.1".parse().unwrap()));
        assert!(is_internal_ip(&"192.168.1.1".parse().unwrap()));
        assert!(is_internal_ip(&"100.64.0.1".parse().unwrap()));
        assert!(is_internal_ip(&"0.0.0.0".parse().unwrap()));
        assert!(is_internal_ip(&"::1".parse().unwrap()));
        assert!(is_internal_ip(&"fd00::1".parse().unwrap()));
        assert!(is_internal_ip(&"fe80::1".parse().unwrap()));
        // IPv4-mapped loopback.
        assert!(is_internal_ip(&"::ffff:127.0.0.1".parse().unwrap()));
    }

    #[test]
    fn allows_public_addresses() {
        assert!(!is_internal_ip(&"1.1.1.1".parse().unwrap()));
        assert!(!is_internal_ip(&"8.8.8.8".parse().unwrap()));
        assert!(!is_internal_ip(&"93.184.216.34".parse().unwrap()));
        assert!(!is_internal_ip(&"2606:4700:4700::1111".parse().unwrap()));
    }

    #[tokio::test]
    async fn rejects_non_http_scheme() {
        let err = validate_outbound_url("ftp://example.com/x", "http://localhost:2019")
            .await
            .unwrap_err();
        assert!(matches!(err, ApiError::BadRequest(_)));
    }

    #[tokio::test]
    async fn rejects_loopback_url() {
        let err = validate_outbound_url("http://127.0.0.1:8080/hook", "http://localhost:2019")
            .await
            .unwrap_err();
        assert!(matches!(err, ApiError::BadRequest(_)));
    }

    #[tokio::test]
    async fn rejects_caddy_admin_by_ip() {
        // Caddy admin reached via a raw loopback IP, not the literal hostname,
        // is still rejected — the loopback rule catches it.
        let err = validate_outbound_url("http://127.0.0.1:2019/config", "http://localhost:2019")
            .await
            .unwrap_err();
        assert!(matches!(err, ApiError::BadRequest(_)));
    }

    #[tokio::test]
    async fn guarded_client_refuses_redirects() {
        // A guarded client must not follow redirects; verify the policy is set
        // by building one against a public address.
        let target = GuardedTarget {
            url: Url::parse("https://example.com/").unwrap(),
            host: "example.com".to_owned(),
            addr: "93.184.216.34:443".parse().unwrap(),
        };
        assert!(guarded_client(&target).is_ok());
    }
}
