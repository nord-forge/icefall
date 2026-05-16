//! Runtime quirk detection.
//!
//! Icefall talks to either Docker or Podman through the same `bollard` Unix
//! socket API. They are *mostly* compatible, but a handful of behaviors differ
//! — especially under rootless Podman. `RuntimeQuirks` captures those
//! differences as data, resolved once when the client connects, so the rest of
//! the code branches on a value instead of hardcoding Docker assumptions.

use crate::config::ContainerRuntime;

/// Which DNS backend the runtime uses for container name resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum DnsBackend {
    /// Docker's built-in DNS on user-defined networks.
    DockerBuiltIn,
    /// Podman's netavark + aardvark-dns stack.
    Netavark,
    /// Unknown or legacy — name resolution between containers is not assured.
    Unknown,
}

/// Behavioral differences of the active container runtime.
#[derive(Debug, Clone, serde::Serialize)]
pub struct RuntimeQuirks {
    pub runtime: ContainerRuntime,
    /// True for rootless Podman (daemon running as a non-root user).
    pub rootless: bool,
    /// Host IP to bind published container ports to. Docker and rootful Podman
    /// can bind `0.0.0.0`; rootless Podman should use loopback — Caddy runs on
    /// the same host and proxies to it.
    pub host_bind_ip: String,
    /// Whether `cpu_shares` / `memory` limits are actually enforced. Rootless
    /// Podman ignores cgroup limits unless cgroups v2 + delegation is set up.
    pub supports_cgroup_limits: bool,
    /// DNS backend, which determines whether inter-container hostname
    /// resolution can be relied on.
    pub dns_backend: DnsBackend,
    /// Lowest port number the runtime can publish on the host. 0 for Docker /
    /// rootful Podman; 1024 for rootless Podman (cannot bind privileged ports).
    pub min_unprivileged_port: u16,
}

impl RuntimeQuirks {
    /// Quirks for a plain rootful Docker daemon — the baseline assumption.
    pub fn docker_default() -> Self {
        Self {
            runtime: ContainerRuntime::Docker,
            rootless: false,
            host_bind_ip: "0.0.0.0".to_string(),
            supports_cgroup_limits: true,
            dns_backend: DnsBackend::DockerBuiltIn,
            min_unprivileged_port: 0,
        }
    }

    /// Resolve quirks from the socket path and the runtime's `info` response.
    ///
    /// `socket_path` is a strong signal for rootless (the socket then lives
    /// under a per-user runtime directory). `security_options` from `docker
    /// info` / `podman info` carries an explicit `name=rootless` entry on
    /// rootless Podman, which confirms it.
    pub fn detect(
        runtime: ContainerRuntime,
        socket_path: &str,
        security_options: &[String],
    ) -> Self {
        if runtime == ContainerRuntime::Docker {
            return Self::docker_default();
        }

        let rootless = is_rootless_socket(socket_path)
            || security_options.iter().any(|opt| opt.contains("rootless"));

        Self {
            runtime: ContainerRuntime::Podman,
            rootless,
            // Rootless Podman cannot reliably publish on 0.0.0.0; loopback is
            // sufficient because Caddy is co-located and proxies to it.
            host_bind_ip: if rootless {
                "127.0.0.1".to_string()
            } else {
                "0.0.0.0".to_string()
            },
            // Rootful Podman honors cgroup limits; rootless only does so with
            // cgroups v2 delegation, which we cannot assume.
            supports_cgroup_limits: !rootless,
            // Modern Podman (>= 4, which the installer requires) uses netavark.
            dns_backend: DnsBackend::Netavark,
            min_unprivileged_port: if rootless { 1024 } else { 0 },
        }
    }
}

/// True if the socket path indicates a rootless (per-user) runtime.
fn is_rootless_socket(socket_path: &str) -> bool {
    socket_path.contains("/run/user/")
        || std::env::var("XDG_RUNTIME_DIR")
            .ok()
            .filter(|d| !d.is_empty())
            .is_some_and(|dir| socket_path.starts_with(&dir))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn docker_default_is_rootful_baseline() {
        let q = RuntimeQuirks::docker_default();
        assert_eq!(q.runtime, ContainerRuntime::Docker);
        assert!(!q.rootless);
        assert_eq!(q.host_bind_ip, "0.0.0.0");
        assert!(q.supports_cgroup_limits);
        assert_eq!(q.min_unprivileged_port, 0);
    }

    #[test]
    fn docker_detect_ignores_socket_and_security_options() {
        // Docker is never rootless from Icefall's perspective.
        let q = RuntimeQuirks::detect(
            ContainerRuntime::Docker,
            "/run/user/1000/docker.sock",
            &["name=rootless".to_string()],
        );
        assert!(!q.rootless);
        assert_eq!(q.runtime, ContainerRuntime::Docker);
    }

    #[test]
    fn rootful_podman_socket_is_not_rootless() {
        let q = RuntimeQuirks::detect(ContainerRuntime::Podman, "/run/podman/podman.sock", &[]);
        assert!(!q.rootless);
        assert_eq!(q.host_bind_ip, "0.0.0.0");
        assert!(q.supports_cgroup_limits);
        assert_eq!(q.min_unprivileged_port, 0);
        assert_eq!(q.dns_backend, DnsBackend::Netavark);
    }

    #[test]
    fn rootless_podman_detected_from_socket_path() {
        let q = RuntimeQuirks::detect(
            ContainerRuntime::Podman,
            "/run/user/1000/podman/podman.sock",
            &[],
        );
        assert!(q.rootless);
        assert_eq!(q.host_bind_ip, "127.0.0.1");
        assert!(!q.supports_cgroup_limits);
        assert_eq!(q.min_unprivileged_port, 1024);
    }

    #[test]
    fn rootless_podman_detected_from_security_options() {
        let q = RuntimeQuirks::detect(
            ContainerRuntime::Podman,
            "/run/podman/podman.sock",
            &["name=rootless".to_string(), "name=seccomp".to_string()],
        );
        assert!(q.rootless);
        assert_eq!(q.host_bind_ip, "127.0.0.1");
    }
}
