//! SSRF protection for user-supplied outbound URLs — rejects metadata endpoints, loopback,
//! private networks, and the Caddy admin API. Known limitation: a TOCTOU DNS-rebinding gap remains.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use url::Url;

use crate::api::error::ApiError;

/// Validate that `raw_url` is safe to fetch on behalf of a user. Rejects non-`http(s)`
/// schemes and hosts resolving to loopback, link-local, private, or internal addresses.
pub async fn validate_outbound_url(raw_url: &str, caddy_admin_url: &str) -> Result<(), ApiError> {
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
        .ok_or_else(|| ApiError::BadRequest(format!("URL has no host: {raw_url}")))?;

    // Reject the Caddy admin host by name before DNS resolution — it is an
    // internal control plane regardless of how its host resolves.
    if let Some(admin_host) = Url::parse(caddy_admin_url)
        .ok()
        .and_then(|u| u.host_str().map(str::to_owned))
    {
        if host.eq_ignore_ascii_case(&admin_host) {
            return Err(ApiError::BadRequest(
                "URL targets the Caddy admin API".into(),
            ));
        }
    }

    // Resolve the host and reject if any resolved address is internal. The
    // port is irrelevant to the address check; 0 is a valid placeholder.
    let addrs = tokio::net::lookup_host((host, 0))
        .await
        .map_err(|_| ApiError::BadRequest(format!("could not resolve host: {host}")))?;

    let mut resolved_any = false;
    for addr in addrs {
        resolved_any = true;
        if is_internal_ip(&addr.ip()) {
            return Err(ApiError::BadRequest(format!(
                "URL resolves to a disallowed internal address: {host}"
            )));
        }
    }

    if !resolved_any {
        return Err(ApiError::BadRequest(format!(
            "could not resolve host: {host}"
        )));
    }

    Ok(())
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
    async fn rejects_caddy_admin_host() {
        let err = validate_outbound_url("http://localhost:2019/config", "http://localhost:2019")
            .await
            .unwrap_err();
        assert!(matches!(err, ApiError::BadRequest(_)));
    }
}
