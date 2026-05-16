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
    let podman_paths = ["/run/podman/podman.sock", "/var/run/podman/podman.sock"];
    let docker_paths = ["/var/run/docker.sock"];

    for path in &podman_paths {
        if std::path::Path::new(path).exists() {
            return path.to_string();
        }
    }
    for path in &docker_paths {
        if std::path::Path::new(path).exists() {
            return path.to_string();
        }
    }
    "/var/run/docker.sock".to_string()
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
