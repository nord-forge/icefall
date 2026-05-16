//! QA-270: runtime compatibility tests.
//!
//! Two layers:
//!
//! 1. **Pure logic** — `RuntimeQuirks` detection for every runtime/rootless
//!    combination. These always run.
//! 2. **Live runtime** — operations against whatever container runtime is
//!    actually present on the host. These **skip** (not fail) when no runtime
//!    socket is reachable, so the suite stays green in environments without
//!    Docker/Podman (e.g. most CI unit-test jobs).
//!
//! The full Docker / rootful-Podman / rootless-Podman matrix is exercised by
//! the CI integration job and the documented manual checklist; this module is
//! the part that lives in `cargo test`.

#[cfg(test)]
mod runtime_compat {
    use crate::config::ContainerRuntime;
    use crate::docker::quirks::{DnsBackend, RuntimeQuirks};
    use crate::docker::DockerClient;

    // --- Layer 1: RuntimeQuirks detection (always runs) ---

    #[test]
    fn quirks_matrix_docker() {
        let q = RuntimeQuirks::detect(ContainerRuntime::Docker, "/var/run/docker.sock", &[]);
        assert_eq!(q.runtime, ContainerRuntime::Docker);
        assert!(!q.rootless);
        assert_eq!(q.host_bind_ip, "0.0.0.0");
        assert!(q.supports_cgroup_limits);
        assert_eq!(q.min_unprivileged_port, 0);
        assert_eq!(q.dns_backend, DnsBackend::DockerBuiltIn);
    }

    #[test]
    fn quirks_matrix_rootful_podman() {
        let q = RuntimeQuirks::detect(ContainerRuntime::Podman, "/run/podman/podman.sock", &[]);
        assert_eq!(q.runtime, ContainerRuntime::Podman);
        assert!(!q.rootless);
        assert_eq!(q.host_bind_ip, "0.0.0.0");
        assert!(q.supports_cgroup_limits);
        assert_eq!(q.min_unprivileged_port, 0);
        assert_eq!(q.dns_backend, DnsBackend::Netavark);
    }

    #[test]
    fn quirks_matrix_rootless_podman() {
        let q = RuntimeQuirks::detect(
            ContainerRuntime::Podman,
            "/run/user/1000/podman/podman.sock",
            &["name=rootless".to_string()],
        );
        assert_eq!(q.runtime, ContainerRuntime::Podman);
        assert!(q.rootless);
        assert_eq!(q.host_bind_ip, "127.0.0.1");
        // Rootless cannot assume cgroup delegation.
        assert!(!q.supports_cgroup_limits);
        // Rootless cannot publish privileged ports.
        assert_eq!(q.min_unprivileged_port, 1024);
    }

    #[test]
    fn rootless_is_detected_by_socket_or_security_options() {
        // Socket-path signal alone is enough.
        let by_socket = RuntimeQuirks::detect(
            ContainerRuntime::Podman,
            "/run/user/1000/podman/podman.sock",
            &[],
        );
        assert!(by_socket.rootless);

        // security_options signal alone is enough.
        let by_secopt = RuntimeQuirks::detect(
            ContainerRuntime::Podman,
            "/run/podman/podman.sock",
            &["name=rootless".to_string()],
        );
        assert!(by_secopt.rootless);
    }

    // --- Layer 2: live runtime (skips when no runtime is present) ---

    /// Connect to whatever runtime is on the host, or return `None` to skip.
    async fn live_client() -> Option<DockerClient> {
        let socket = crate::config::defaults::detect_socket();
        match DockerClient::connect(&socket).await {
            Ok(client) => Some(client),
            Err(_) => None,
        }
    }

    #[tokio::test]
    async fn live_runtime_quirks_are_consistent() {
        let Some(client) = live_client().await else {
            eprintln!("skipping: no container runtime available");
            return;
        };
        let q = client.quirks();

        // host_bind_ip and min_unprivileged_port must agree with rootless-ness.
        if q.rootless {
            assert_eq!(q.host_bind_ip, "127.0.0.1");
            assert_eq!(q.min_unprivileged_port, 1024);
            assert!(!q.supports_cgroup_limits);
        } else {
            assert_eq!(q.host_bind_ip, "0.0.0.0");
            assert_eq!(q.min_unprivileged_port, 0);
        }

        // Only Podman is ever flagged rootless.
        if q.rootless {
            assert_eq!(q.runtime, ContainerRuntime::Podman);
        }
    }

    #[tokio::test]
    async fn live_network_create_and_remove() {
        let Some(client) = live_client().await else {
            eprintln!("skipping: no container runtime available");
            return;
        };
        let name = "icefall-qa270-net";
        let _ = client.remove_network(name).await;

        let created = client.create_network(name).await;
        assert!(created.is_ok(), "network create failed: {created:?}");

        let listed = client
            .list_networks()
            .await
            .unwrap_or_default()
            .into_iter()
            .any(|n| n.name == name);
        assert!(listed, "created network not found in list");

        let _ = client.remove_network(name).await;
    }

    #[tokio::test]
    async fn live_dns_check_does_not_panic() {
        let Some(client) = live_client().await else {
            eprintln!("skipping: no container runtime available");
            return;
        };
        // check_network_dns is best-effort and must never panic, on any runtime.
        client.check_network_dns().await;
    }
}
