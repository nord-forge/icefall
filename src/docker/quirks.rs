//! Runtime quirk detection. Docker and Podman share the `bollard` socket API
//! but differ in behavior (especially rootless Podman); `RuntimeQuirks` captures
//! those differences as data resolved once at connect time.

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

    /// Resolve quirks from the socket path (a per-user runtime dir signals
    /// rootless) and `security_options` (a `name=rootless` entry confirms it).
    /// Probes the host for cgroups-v2 delegation so rootless limit enforcement
    /// is detected rather than assumed off.
    pub fn detect(
        runtime: ContainerRuntime,
        socket_path: &str,
        security_options: &[String],
    ) -> Self {
        Self::detect_with(
            runtime,
            socket_path,
            security_options,
            cgroup_v2_delegation_available(),
        )
    }

    /// Like [`detect`], but with cgroup delegation passed in (testable seam).
    ///
    /// [`detect`]: RuntimeQuirks::detect
    pub fn detect_with(
        runtime: ContainerRuntime,
        socket_path: &str,
        security_options: &[String],
        cgroup_delegation: bool,
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
            // Rootful Podman always honors cgroup limits. Rootless does too, but
            // ONLY when the host has cgroups-v2 with memory/cpu controllers
            // delegated to the user — which the installer now sets up. We probe
            // for it rather than blanket-assume rootless can't enforce limits.
            supports_cgroup_limits: !rootless || cgroup_delegation,
            // Modern Podman (>= 4, which the installer requires) uses netavark.
            dns_backend: DnsBackend::Netavark,
            min_unprivileged_port: if rootless { 1024 } else { 0 },
        }
    }
}

/// Whether the host has cgroups v2 with the `memory` and `cpu` controllers
/// available for delegation to a rootless user. On a unified-v2 host with
/// systemd `Delegate=` set up (what the installer configures), the user's
/// cgroup exposes these controllers; without it, rootless resource limits are
/// silently ignored by the kernel.
///
/// Best-effort filesystem probe — any error is treated as "no delegation" so we
/// degrade to the conservative warning rather than over-promise enforcement.
fn cgroup_v2_delegation_available() -> bool {
    // cgroups v2 is a single unified hierarchy at /sys/fs/cgroup with a
    // top-level `cgroup.controllers` file. cgroups v1 has no such file.
    let root_controllers = match std::fs::read_to_string("/sys/fs/cgroup/cgroup.controllers") {
        Ok(c) => c,
        Err(_) => return false, // not unified v2 (or unreadable) → no delegation
    };
    if !(root_controllers.contains("memory") && root_controllers.contains("cpu")) {
        return false;
    }

    // The controllers must also be delegated down to the user's slice. Check the
    // running process's own cgroup subtree for the same controllers; if systemd
    // delegated them, they appear in our subtree_control / controllers file.
    for path in [
        "/sys/fs/cgroup/cgroup.subtree_control",
        // Per-user delegated slice (systemd user manager).
        "/sys/fs/cgroup/user.slice/cgroup.controllers",
    ] {
        if let Ok(c) = std::fs::read_to_string(path) {
            if c.contains("memory") && c.contains("cpu") {
                return true;
            }
        }
    }
    false
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
        // Rootful honors limits regardless of delegation.
        let q = RuntimeQuirks::detect_with(
            ContainerRuntime::Podman,
            "/run/podman/podman.sock",
            &[],
            false,
        );
        assert!(!q.rootless);
        assert_eq!(q.host_bind_ip, "0.0.0.0");
        assert!(q.supports_cgroup_limits);
        assert_eq!(q.min_unprivileged_port, 0);
        assert_eq!(q.dns_backend, DnsBackend::Netavark);
    }

    #[test]
    fn rootless_podman_without_delegation_cannot_enforce_limits() {
        let q = RuntimeQuirks::detect_with(
            ContainerRuntime::Podman,
            "/run/user/1000/podman/podman.sock",
            &[],
            false,
        );
        assert!(q.rootless);
        assert_eq!(q.host_bind_ip, "127.0.0.1");
        assert!(!q.supports_cgroup_limits);
        assert_eq!(q.min_unprivileged_port, 1024);
    }

    #[test]
    fn rootless_podman_with_delegation_enforces_limits() {
        // The installer sets up cgroups-v2 delegation; then rootless limits work.
        let q = RuntimeQuirks::detect_with(
            ContainerRuntime::Podman,
            "/run/user/1000/podman/podman.sock",
            &[],
            true,
        );
        assert!(q.rootless);
        assert!(q.supports_cgroup_limits);
        // Loopback bind + unprivileged-port floor still apply for rootless.
        assert_eq!(q.host_bind_ip, "127.0.0.1");
        assert_eq!(q.min_unprivileged_port, 1024);
    }

    #[test]
    fn rootless_podman_detected_from_security_options() {
        let q = RuntimeQuirks::detect_with(
            ContainerRuntime::Podman,
            "/run/podman/podman.sock",
            &["name=rootless".to_string(), "name=seccomp".to_string()],
            false,
        );
        assert!(q.rootless);
        assert_eq!(q.host_bind_ip, "127.0.0.1");
    }
}
