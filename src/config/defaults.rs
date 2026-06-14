use std::path::PathBuf;

pub fn listen_addr() -> String {
    "0.0.0.0".to_string()
}

pub fn listen_port() -> u16 {
    3000
}

pub fn data_dir() -> PathBuf {
    PathBuf::from("/var/lib/icefall")
}

pub fn sqlite_path() -> PathBuf {
    data_dir().join("icefall.db")
}

pub fn container_runtime() -> String {
    "docker".to_string()
}

pub fn container_socket() -> String {
    detect_socket()
}

pub fn detect_socket() -> String {
    // Probe rootless Podman (per-user socket) before rootful Podman and Docker;
    // missing it makes a rootless host fall back to a non-existent Docker socket.
    for path in rootless_podman_socket_paths() {
        if std::path::Path::new(&path).exists() {
            return path;
        }
    }

    let rootful_paths = [
        "/run/podman/podman.sock",
        "/var/run/podman/podman.sock",
        "/var/run/docker.sock",
    ];
    for path in &rootful_paths {
        if std::path::Path::new(path).exists() {
            return path.to_string();
        }
    }
    "/var/run/docker.sock".to_string()
}

/// Candidate rootless Podman socket paths, derived from `XDG_RUNTIME_DIR`
/// (set in every systemd user session); non-standard setups use the override.
fn rootless_podman_socket_paths() -> Vec<String> {
    let mut paths = Vec::new();
    if let Ok(dir) = std::env::var("XDG_RUNTIME_DIR") {
        if !dir.is_empty() {
            paths.push(format!("{}/podman/podman.sock", dir.trim_end_matches('/')));
        }
    }
    paths
}

pub fn detect_runtime_from_socket(socket: &str) -> String {
    if socket.contains("podman") {
        "podman".to_string()
    } else {
        "docker".to_string()
    }
}

pub fn caddy_admin_url() -> String {
    "http://localhost:2019".to_string()
}

pub fn pid_file() -> PathBuf {
    PathBuf::from("/var/run/icefall.pid")
}

pub fn log_level() -> String {
    "info".to_string()
}

pub fn backup_interval_hours() -> u32 {
    24
}

pub fn backup_retain_count() -> u32 {
    7
}

pub fn build_timeout_secs() -> u64 {
    600
}

pub fn keep_images() -> usize {
    5
}

pub fn health_check_attempts() -> u32 {
    30
}

pub fn health_check_interval_ms() -> u64 {
    1000
}

pub fn deploy_stop_timeout_secs() -> i64 {
    10
}

pub fn ssl_check_interval_hours() -> u64 {
    24
}

pub fn image_transfer_chunk_bytes() -> usize {
    8 * 1024 * 1024
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rootless_socket_path_derives_from_xdg_runtime_dir() {
        // SAFETY: single-threaded test; env is restored before returning.
        let prev = std::env::var("XDG_RUNTIME_DIR").ok();
        std::env::set_var("XDG_RUNTIME_DIR", "/run/user/1000");
        let paths = rootless_podman_socket_paths();
        assert_eq!(paths, vec!["/run/user/1000/podman/podman.sock"]);
        match prev {
            Some(v) => std::env::set_var("XDG_RUNTIME_DIR", v),
            None => std::env::remove_var("XDG_RUNTIME_DIR"),
        }
    }

    #[test]
    fn rootless_socket_path_trims_trailing_slash() {
        let prev = std::env::var("XDG_RUNTIME_DIR").ok();
        std::env::set_var("XDG_RUNTIME_DIR", "/run/user/1000/");
        let paths = rootless_podman_socket_paths();
        assert_eq!(paths, vec!["/run/user/1000/podman/podman.sock"]);
        match prev {
            Some(v) => std::env::set_var("XDG_RUNTIME_DIR", v),
            None => std::env::remove_var("XDG_RUNTIME_DIR"),
        }
    }

    #[test]
    fn detect_runtime_from_socket_identifies_podman() {
        assert_eq!(
            detect_runtime_from_socket("/run/user/1000/podman/podman.sock"),
            "podman"
        );
        assert_eq!(detect_runtime_from_socket("/var/run/docker.sock"), "docker");
    }
}
