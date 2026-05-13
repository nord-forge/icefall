use std::io::{self, Write};
use std::path::PathBuf;

use crate::config::{ContainerRuntime, IcefallConfig};
use crate::db::encryption::Encryptor;

fn detect_runtime() -> (ContainerRuntime, String) {
    let podman_sockets = ["/run/podman/podman.sock", "/var/run/podman/podman.sock"];
    let docker_socket = "/var/run/docker.sock";

    for socket in &podman_sockets {
        if std::path::Path::new(socket).exists() {
            return (ContainerRuntime::Podman, socket.to_string());
        }
    }
    if std::path::Path::new(docker_socket).exists() {
        return (ContainerRuntime::Docker, docker_socket.to_string());
    }
    (ContainerRuntime::Docker, docker_socket.to_string())
}

pub async fn run() {
    println!("Icefall — Initial Setup\n");

    let (detected_runtime, detected_socket) = detect_runtime();
    println!(
        "Detected container runtime: {} ({})",
        detected_runtime, detected_socket
    );

    let data_dir = prompt("Data directory", "/var/lib/icefall");
    let listen_port: u16 = prompt("Listen port", "3000").parse().unwrap_or(3000);
    let base_domain = prompt_optional("Base domain (e.g. apps.example.com)");
    let runtime_input = prompt("Container runtime", &detected_runtime.to_string());
    let (runtime, container_socket) = match runtime_input.as_str() {
        "podman" => (ContainerRuntime::Podman, detected_socket.clone()),
        _ => (ContainerRuntime::Docker, detected_socket.clone()),
    };

    let encryption_key = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        Encryptor::generate_key(),
    );

    let config = IcefallConfig {
        listen_addr: "0.0.0.0".to_string(),
        listen_port,
        data_dir: PathBuf::from(&data_dir),
        sqlite_path: PathBuf::from(&data_dir).join("icefall.db"),
        runtime,
        container_socket,
        caddy_admin_url: "http://localhost:2019".to_string(),
        base_domain,
        encryption_key: Some(encryption_key),
        smtp: None,
        backup: Default::default(),
        pid_file: PathBuf::from("/var/run/icefall.pid"),
        log_level: "info".to_string(),
        build_timeout_secs: crate::config::defaults::build_timeout_secs(),
        keep_images: crate::config::defaults::keep_images(),
        health_check_attempts: crate::config::defaults::health_check_attempts(),
        health_check_interval_ms: crate::config::defaults::health_check_interval_ms(),
        deploy_stop_timeout_secs: crate::config::defaults::deploy_stop_timeout_secs(),
    };

    let toml_str = toml::to_string_pretty(&config).expect("Failed to serialize config");

    let config_dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from("/etc/icefall"));
    let config_path = config_dir.join("icefall").join("config.toml");

    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent).ok();
    }

    std::fs::write(&config_path, &toml_str).unwrap_or_else(|e| {
        eprintln!("Failed to write config to {}: {e}", config_path.display());
        std::process::exit(1);
    });

    println!("\nConfiguration written to {}", config_path.display());
    println!("Run `icefall daemon start` to start the daemon.");
}

fn prompt(label: &str, default: &str) -> String {
    print!("{label} [{default}]: ");
    io::stdout().flush().ok();
    let mut input = String::new();
    io::stdin().read_line(&mut input).ok();
    let trimmed = input.trim();
    if trimmed.is_empty() {
        default.to_string()
    } else {
        trimmed.to_string()
    }
}

fn prompt_optional(label: &str) -> Option<String> {
    print!("{label}: ");
    io::stdout().flush().ok();
    let mut input = String::new();
    io::stdin().read_line(&mut input).ok();
    let trimmed = input.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}
