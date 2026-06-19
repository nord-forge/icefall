//! Provider-agnostic public/private repo probing for the app-create wizard.
//!
//! Pasting a repo URL should tell the user, without credentials, whether the
//! repo is reachable (public) or needs a connection (private). Each supported
//! host exposes an anonymous "get repo" API that answers 200 for public repos
//! and 404 for private-or-missing ones, so detection is: identify the provider
//! from the host, then probe its API.
//!
//! Hosted providers (github.com, gitlab.com, bitbucket.org) are matched by host.
//! An unknown host gets a best-effort GitLab heuristic (many self-hosted GitLab
//! instances live on custom domains) guarded against SSRF; if that doesn't look
//! like GitLab we report `Unknown` and the UI shows generic private-repo
//! guidance.

use std::net::IpAddr;

/// Which git host a URL points at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provider {
    GitHub,
    GitLab,
    Bitbucket,
    /// A host we'll heuristically treat as a (possibly self-hosted) GitLab.
    GitLabSelfHosted,
    /// Not a recognized git host.
    Unknown,
}

/// Outcome of a public/private probe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepoStatus {
    /// Reachable anonymously — no credentials needed to deploy.
    Public,
    /// 404 anonymously: private repo or a typo (the host won't say which).
    PrivateOrMissing,
    /// URL isn't a recognized git host / couldn't be parsed.
    NotRecognized,
    /// Probe failed (network, rate limit, unexpected status) — caller should
    /// fall back to generic guidance rather than asserting public/private.
    Unknown,
}

impl RepoStatus {
    /// The wire string the API/UI use.
    pub fn as_str(self) -> &'static str {
        match self {
            RepoStatus::Public => "public",
            RepoStatus::PrivateOrMissing => "private_or_missing",
            RepoStatus::NotRecognized => "not_github", // kept for UI back-compat
            RepoStatus::Unknown => "unknown",
        }
    }
}

/// Extract the host (lowercased, no port) from an https/ssh-ish repo URL.
fn host_of(url: &str) -> Option<String> {
    let u = url.trim();
    // Strip scheme.
    let rest = u
        .strip_prefix("https://")
        .or_else(|| u.strip_prefix("http://"))
        .or_else(|| u.strip_prefix("git@")) // git@host:owner/repo
        .unwrap_or(u);
    // For scp-style `git@host:owner/repo`, the host ends at ':'; for URLs it
    // ends at '/'. Take up to whichever comes first.
    let end = rest.find(['/', ':']).unwrap_or(rest.len());
    let host = &rest[..end];
    let host = host.split('@').next_back().unwrap_or(host); // strip any leftover user@
    if host.is_empty() {
        None
    } else {
        Some(host.to_lowercase())
    }
}

/// Classify a repo URL's provider by its host.
pub fn detect(url: &str) -> Provider {
    let Some(host) = host_of(url) else {
        return Provider::Unknown;
    };
    match host.as_str() {
        "github.com" | "www.github.com" => Provider::GitHub,
        "gitlab.com" | "www.gitlab.com" => Provider::GitLab,
        "bitbucket.org" | "www.bitbucket.org" => Provider::Bitbucket,
        _ => Provider::GitLabSelfHosted, // candidate for the heuristic probe
    }
}

/// Reject hosts that could be used to reach internal infrastructure (SSRF) when
/// we probe an arbitrary user-supplied host for the self-hosted GitLab case.
/// Only applies to the heuristic path; the known hosted providers are constants.
fn is_safe_public_host(host: &str) -> bool {
    // No localhost-ish names.
    if host == "localhost" || host.ends_with(".localhost") || host.ends_with(".local") {
        return false;
    }
    // A bare IP must be global (not private/loopback/link-local).
    if let Ok(ip) = host.parse::<IpAddr>() {
        return is_global_ip(ip);
    }
    // Require a dotted name (has a TLD); rejects single-label internal names.
    host.contains('.')
}

/// True for IPs that are routable on the public internet.
fn is_global_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            !(v4.is_private()
                || v4.is_loopback()
                || v4.is_link_local()
                || v4.is_broadcast()
                || v4.is_documentation()
                || v4.is_unspecified()
                // Carrier-grade NAT 100.64.0.0/10.
                || (v4.octets()[0] == 100 && (v4.octets()[1] & 0xc0) == 0x40))
        }
        IpAddr::V6(v6) => !(v6.is_loopback() || v6.is_unspecified() || v6.is_unique_local()),
    }
}

/// Probe a repo URL's public/private status across supported providers.
///
/// `http` is a shared reqwest client; `owner`/`repo` are pre-parsed from the URL
/// (use [`crate::github::owner_repo`]). Returns a [`RepoStatus`] plus the
/// resolved [`Provider`] for telemetry/UI.
pub async fn probe(
    http: &reqwest::Client,
    url: &str,
    owner: &str,
    repo: &str,
) -> (Provider, RepoStatus) {
    let provider = detect(url);
    let status = match provider {
        Provider::GitHub => probe_github(http, owner, repo).await,
        Provider::GitLab => probe_gitlab(http, "https://gitlab.com", owner, repo).await,
        Provider::Bitbucket => probe_bitbucket(http, owner, repo).await,
        Provider::GitLabSelfHosted => {
            // Heuristic: only probe https + a safe public host.
            match host_of(url) {
                Some(host) if is_safe_public_host(&host) => {
                    probe_gitlab(http, &format!("https://{host}"), owner, repo).await
                }
                _ => RepoStatus::Unknown,
            }
        }
        Provider::Unknown => RepoStatus::NotRecognized,
    };
    (provider, status)
}

/// Map a probe HTTP result to a status: 200 → public, 404 → private/missing,
/// anything else (incl. network error) → unknown.
async fn status_from(resp: Result<reqwest::Response, reqwest::Error>, ctx: &str) -> RepoStatus {
    match resp {
        Ok(r) => match r.status().as_u16() {
            200 => RepoStatus::Public,
            404 => RepoStatus::PrivateOrMissing,
            other => {
                tracing::warn!("{ctx} probe got unexpected status {other}");
                RepoStatus::Unknown
            }
        },
        Err(e) => {
            tracing::warn!("{ctx} probe failed: {e}");
            RepoStatus::Unknown
        }
    }
}

async fn probe_github(http: &reqwest::Client, owner: &str, repo: &str) -> RepoStatus {
    let url = format!("https://api.github.com/repos/{}/{}", enc(owner), enc(repo));
    let resp = http
        .get(&url)
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "Icefall-PaaS")
        .send()
        .await;
    status_from(resp, "github").await
}

async fn probe_gitlab(http: &reqwest::Client, base: &str, owner: &str, repo: &str) -> RepoStatus {
    // GitLab identifies a project by URL-encoded "owner/repo".
    let project = format!("{}/{}", owner, repo);
    let url = format!("{base}/api/v4/projects/{}", enc(&project));
    let resp = http
        .get(&url)
        .header("User-Agent", "Icefall-PaaS")
        .send()
        .await;
    status_from(resp, "gitlab").await
}

async fn probe_bitbucket(http: &reqwest::Client, owner: &str, repo: &str) -> RepoStatus {
    let url = format!(
        "https://api.bitbucket.org/2.0/repositories/{}/{}",
        enc(owner),
        enc(repo)
    );
    let resp = http
        .get(&url)
        .header("User-Agent", "Icefall-PaaS")
        .send()
        .await;
    status_from(resp, "bitbucket").await
}

/// Percent-encode a single path segment.
fn enc(segment: &str) -> String {
    const ENCODE: &percent_encoding::AsciiSet = &percent_encoding::NON_ALPHANUMERIC
        .remove(b'-')
        .remove(b'_')
        .remove(b'.')
        .remove(b'~');
    percent_encoding::utf8_percent_encode(segment, ENCODE).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_hosted_providers() {
        assert_eq!(detect("https://github.com/a/b"), Provider::GitHub);
        assert_eq!(detect("https://gitlab.com/a/b"), Provider::GitLab);
        assert_eq!(detect("https://bitbucket.org/a/b"), Provider::Bitbucket);
        assert_eq!(detect("git@github.com:a/b.git"), Provider::GitHub);
    }

    #[test]
    fn unknown_host_is_selfhosted_candidate() {
        assert_eq!(
            detect("https://git.company.com/team/app"),
            Provider::GitLabSelfHosted
        );
    }

    #[test]
    fn host_parsing() {
        assert_eq!(
            host_of("https://github.com/a/b").as_deref(),
            Some("github.com")
        );
        assert_eq!(
            host_of("git@gitlab.com:a/b.git").as_deref(),
            Some("gitlab.com")
        );
        assert_eq!(
            host_of("https://git.co:8443/a/b").as_deref(),
            Some("git.co")
        );
    }

    #[test]
    fn ssrf_guard_rejects_internal_hosts() {
        assert!(!is_safe_public_host("localhost"));
        assert!(!is_safe_public_host("gitlab.local"));
        assert!(!is_safe_public_host("internal")); // single label
        assert!(!is_safe_public_host("127.0.0.1"));
        assert!(!is_safe_public_host("10.0.0.5"));
        assert!(!is_safe_public_host("192.168.1.1"));
        assert!(!is_safe_public_host("169.254.169.254")); // cloud metadata
        assert!(!is_safe_public_host("100.64.0.1")); // CGNAT
        assert!(is_safe_public_host("git.company.com"));
        assert!(is_safe_public_host("gitlab.example.org"));
    }
}
