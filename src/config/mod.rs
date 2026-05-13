pub mod defaults;

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::info;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("config file not found at {0}")]
    NotFound(PathBuf),
    #[error("failed to read config: {0}")]
    Read(#[from] std::io::Error),
    #[error("failed to parse config: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("validation failed: {0}")]
    Validation(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmtpConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub from_address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BackupConfig {
    #[serde(default = "defaults::backup_interval_hours")]
    pub interval_hours: u32,
    #[serde(default = "defaults::backup_retain_count")]
    pub retain_count: u32,
    pub s3_bucket: Option<String>,
    pub s3_endpoint: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub enum ContainerRuntime {
    #[default]
    #[serde(rename = "docker")]
    Docker,
    #[serde(rename = "podman")]
    Podman,
}

impl std::fmt::Display for ContainerRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Docker => write!(f, "docker"),
            Self::Podman => write!(f, "podman"),
        }
    }
}

impl ContainerRuntime {
    pub fn default_socket(&self) -> &'static str {
        match self {
            Self::Docker => "/var/run/docker.sock",
            Self::Podman => "/run/podman/podman.sock",
        }
    }

    pub fn compose_command(&self) -> &'static str {
        match self {
            Self::Docker => "docker compose",
            Self::Podman => "podman compose",
        }
    }

    pub fn from_socket(socket: &str) -> Self {
        if socket.contains("podman") {
            Self::Podman
        } else {
            Self::Docker
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IcefallConfig {
    #[serde(default = "defaults::listen_addr")]
    pub listen_addr: String,
    #[serde(default = "defaults::listen_port")]
    pub listen_port: u16,
    #[serde(default = "defaults::data_dir")]
    pub data_dir: PathBuf,
    #[serde(default = "defaults::sqlite_path")]
    pub sqlite_path: PathBuf,
    #[serde(default)]
    pub runtime: ContainerRuntime,
    #[serde(default = "defaults::container_socket", alias = "docker_socket")]
    pub container_socket: String,
    #[serde(default = "defaults::caddy_admin_url")]
    pub caddy_admin_url: String,
    pub base_domain: Option<String>,
    pub encryption_key: Option<String>,
    #[serde(default)]
    pub smtp: Option<SmtpConfig>,
    #[serde(default)]
    pub backup: BackupConfig,
    #[serde(default = "defaults::pid_file")]
    pub pid_file: PathBuf,
    #[serde(default = "defaults::log_level")]
    pub log_level: String,
    #[serde(default = "defaults::build_timeout_secs")]
    pub build_timeout_secs: u64,
    #[serde(default = "defaults::keep_images")]
    pub keep_images: usize,
    #[serde(default = "defaults::health_check_attempts")]
    pub health_check_attempts: u32,
    #[serde(default = "defaults::health_check_interval_ms")]
    pub health_check_interval_ms: u64,
    #[serde(default = "defaults::deploy_stop_timeout_secs")]
    pub deploy_stop_timeout_secs: i64,
}

impl IcefallConfig {
    pub fn load() -> Result<Self, ConfigError> {
        let config_path = Self::find_config_file()?;
        let contents = std::fs::read_to_string(&config_path)?;
        let mut config: Self = toml::from_str(&contents)?;
        config.apply_env_overrides();
        Ok(config)
    }

    pub fn load_or_default() -> Self {
        match Self::load() {
            Ok(config) => config,
            Err(_) => {
                let mut config = Self::default();
                config.apply_env_overrides();
                config
            }
        }
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.encryption_key.is_none() {
            return Err(ConfigError::Validation(
                "encryption_key is required — run `icefall init` to generate one".to_string(),
            ));
        }

        if let Some(ref key) = self.encryption_key {
            let decoded = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, key)
                .map_err(|_| {
                ConfigError::Validation("encryption_key is not valid base64".to_string())
            })?;
            if decoded.len() != 32 {
                return Err(ConfigError::Validation(
                    "encryption_key must be 32 bytes (256 bits)".to_string(),
                ));
            }
        }

        Ok(())
    }

    fn find_config_file() -> Result<PathBuf, ConfigError> {
        if let Ok(path) = std::env::var("ICEFALL_CONFIG") {
            let p = PathBuf::from(path);
            if p.exists() {
                return Ok(p);
            }
            return Err(ConfigError::NotFound(p));
        }

        let paths = [
            PathBuf::from("/etc/icefall/config.toml"),
            dirs::config_dir()
                .unwrap_or_default()
                .join("icefall")
                .join("config.toml"),
        ];

        for p in &paths {
            if p.exists() {
                return Ok(p.clone());
            }
        }

        Err(ConfigError::NotFound(paths[0].clone()))
    }

    fn apply_env_overrides(&mut self) {
        if let Ok(val) = std::env::var("ICEFALL_LISTEN_ADDR") {
            self.listen_addr = val;
        }
        if let Ok(val) = std::env::var("ICEFALL_PORT") {
            if let Ok(port) = val.parse() {
                self.listen_port = port;
            }
        }
        if let Ok(val) = std::env::var("ICEFALL_DATA_DIR") {
            self.data_dir = PathBuf::from(val);
        }
        if let Ok(val) = std::env::var("ICEFALL_SQLITE_PATH") {
            self.sqlite_path = PathBuf::from(val);
        }
        if let Ok(val) = std::env::var("ICEFALL_CONTAINER_SOCKET") {
            self.container_socket = val.clone();
            self.runtime = ContainerRuntime::from_socket(&val);
        } else if let Ok(val) = std::env::var("ICEFALL_DOCKER_SOCKET") {
            self.container_socket = val.clone();
            self.runtime = ContainerRuntime::from_socket(&val);
        }
        if let Ok(val) = std::env::var("ICEFALL_RUNTIME") {
            match val.as_str() {
                "podman" => {
                    self.runtime = ContainerRuntime::Podman;
                    if self.container_socket == defaults::container_socket() {
                        self.container_socket =
                            ContainerRuntime::Podman.default_socket().to_string();
                    }
                }
                "docker" => {
                    self.runtime = ContainerRuntime::Docker;
                    if self.container_socket == defaults::container_socket() {
                        self.container_socket =
                            ContainerRuntime::Docker.default_socket().to_string();
                    }
                }
                other => {
                    info!("Unknown ICEFALL_RUNTIME value '{other}', using default");
                }
            }
        }
        if let Ok(val) = std::env::var("ICEFALL_CADDY_URL") {
            self.caddy_admin_url = val;
        }
        if let Ok(val) = std::env::var("ICEFALL_BASE_DOMAIN") {
            self.base_domain = Some(val);
        }
        if let Ok(val) = std::env::var("ICEFALL_ENCRYPTION_KEY") {
            self.encryption_key = Some(val);
        }
        if let Ok(val) = std::env::var("ICEFALL_LOG_LEVEL") {
            self.log_level = val;
        }
        if let Ok(val) = std::env::var("ICEFALL_PID_FILE") {
            self.pid_file = PathBuf::from(val);
        }
    }
}

impl Default for IcefallConfig {
    fn default() -> Self {
        Self {
            listen_addr: defaults::listen_addr(),
            listen_port: defaults::listen_port(),
            data_dir: defaults::data_dir(),
            sqlite_path: defaults::sqlite_path(),
            runtime: ContainerRuntime::default(),
            container_socket: defaults::container_socket(),
            caddy_admin_url: defaults::caddy_admin_url(),
            base_domain: None,
            encryption_key: None,
            smtp: None,
            backup: BackupConfig::default(),
            pid_file: defaults::pid_file(),
            log_level: defaults::log_level(),
            build_timeout_secs: defaults::build_timeout_secs(),
            keep_images: defaults::keep_images(),
            health_check_attempts: defaults::health_check_attempts(),
            health_check_interval_ms: defaults::health_check_interval_ms(),
            deploy_stop_timeout_secs: defaults::deploy_stop_timeout_secs(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_expected_port() {
        let config = IcefallConfig::default();
        assert_eq!(config.listen_port, 3000);
    }

    #[test]
    fn config_parses_from_toml() {
        let toml_str = r#"
listen_addr = "127.0.0.1"
listen_port = 8080
base_domain = "apps.example.com"
encryption_key = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA="
"#;
        let config: IcefallConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.listen_port, 8080);
        assert_eq!(config.base_domain, Some("apps.example.com".to_string()));
    }

    #[test]
    fn validation_requires_encryption_key() {
        let config = IcefallConfig::default();
        assert!(config.validate().is_err());
    }

    // --- ContainerRuntime::from_socket() ---

    #[test]
    fn from_socket_podman_path_returns_podman() {
        let runtime = ContainerRuntime::from_socket("/run/podman/podman.sock");
        assert_eq!(runtime, ContainerRuntime::Podman);
    }

    #[test]
    fn from_socket_docker_path_returns_docker() {
        let runtime = ContainerRuntime::from_socket("/var/run/docker.sock");
        assert_eq!(runtime, ContainerRuntime::Docker);
    }

    #[test]
    fn from_socket_unknown_path_defaults_to_docker() {
        let runtime = ContainerRuntime::from_socket("/some/random/path.sock");
        assert_eq!(runtime, ContainerRuntime::Docker);
    }

    #[test]
    fn from_socket_podman_in_alternate_location() {
        let runtime = ContainerRuntime::from_socket("/var/run/podman/podman.sock");
        assert_eq!(runtime, ContainerRuntime::Podman);
    }

    // --- ContainerRuntime::default_socket() ---

    #[test]
    fn docker_default_socket() {
        assert_eq!(
            ContainerRuntime::Docker.default_socket(),
            "/var/run/docker.sock"
        );
    }

    #[test]
    fn podman_default_socket() {
        assert_eq!(
            ContainerRuntime::Podman.default_socket(),
            "/run/podman/podman.sock"
        );
    }

    // --- ContainerRuntime::compose_command() ---

    #[test]
    fn docker_compose_command() {
        assert_eq!(ContainerRuntime::Docker.compose_command(), "docker compose");
    }

    #[test]
    fn podman_compose_command() {
        assert_eq!(ContainerRuntime::Podman.compose_command(), "podman compose");
    }

    // --- ContainerRuntime Display ---

    #[test]
    fn docker_display() {
        assert_eq!(format!("{}", ContainerRuntime::Docker), "docker");
    }

    #[test]
    fn podman_display() {
        assert_eq!(format!("{}", ContainerRuntime::Podman), "podman");
    }

    // --- ContainerRuntime Default ---

    #[test]
    fn default_runtime_is_docker() {
        assert_eq!(ContainerRuntime::default(), ContainerRuntime::Docker);
    }

    // --- apply_env_overrides() ---

    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn clear_icefall_env_vars() {
        for var in &[
            "ICEFALL_LISTEN_ADDR",
            "ICEFALL_PORT",
            "ICEFALL_DATA_DIR",
            "ICEFALL_SQLITE_PATH",
            "ICEFALL_CONTAINER_SOCKET",
            "ICEFALL_DOCKER_SOCKET",
            "ICEFALL_RUNTIME",
            "ICEFALL_CADDY_URL",
            "ICEFALL_BASE_DOMAIN",
            "ICEFALL_ENCRYPTION_KEY",
            "ICEFALL_LOG_LEVEL",
            "ICEFALL_PID_FILE",
        ] {
            std::env::remove_var(var);
        }
    }

    #[test]
    fn env_override_runtime_podman() {
        let _lock = ENV_LOCK.lock().unwrap();
        clear_icefall_env_vars();
        let mut config = IcefallConfig::default();

        std::env::set_var("ICEFALL_RUNTIME", "podman");
        config.apply_env_overrides();
        std::env::remove_var("ICEFALL_RUNTIME");

        assert_eq!(config.runtime, ContainerRuntime::Podman);
        assert_eq!(config.container_socket, "/run/podman/podman.sock");
    }

    #[test]
    fn env_override_runtime_docker() {
        let _lock = ENV_LOCK.lock().unwrap();
        clear_icefall_env_vars();
        let mut config = IcefallConfig::default();
        config.runtime = ContainerRuntime::Podman;
        config.container_socket = defaults::container_socket();

        std::env::set_var("ICEFALL_RUNTIME", "docker");
        config.apply_env_overrides();
        std::env::remove_var("ICEFALL_RUNTIME");

        assert_eq!(config.runtime, ContainerRuntime::Docker);
    }

    #[test]
    fn env_override_container_socket() {
        let _lock = ENV_LOCK.lock().unwrap();
        clear_icefall_env_vars();
        let mut config = IcefallConfig::default();
        let custom_socket = "/custom/podman.sock";

        std::env::set_var("ICEFALL_CONTAINER_SOCKET", custom_socket);
        config.apply_env_overrides();
        std::env::remove_var("ICEFALL_CONTAINER_SOCKET");

        assert_eq!(config.container_socket, custom_socket);
        assert_eq!(config.runtime, ContainerRuntime::Podman);
    }

    #[test]
    fn env_override_port() {
        let _lock = ENV_LOCK.lock().unwrap();
        clear_icefall_env_vars();
        let mut config = IcefallConfig::default();

        std::env::set_var("ICEFALL_PORT", "9090");
        config.apply_env_overrides();
        std::env::remove_var("ICEFALL_PORT");

        assert_eq!(config.listen_port, 9090);
    }

    #[test]
    fn env_override_port_invalid_ignored() {
        let _lock = ENV_LOCK.lock().unwrap();
        clear_icefall_env_vars();
        let mut config = IcefallConfig::default();
        let original_port = config.listen_port;

        std::env::set_var("ICEFALL_PORT", "not-a-number");
        config.apply_env_overrides();
        std::env::remove_var("ICEFALL_PORT");

        assert_eq!(config.listen_port, original_port);
    }

    #[test]
    fn env_override_data_dir() {
        let _lock = ENV_LOCK.lock().unwrap();
        clear_icefall_env_vars();
        let mut config = IcefallConfig::default();

        std::env::set_var("ICEFALL_DATA_DIR", "/tmp/icefall-test");
        config.apply_env_overrides();
        std::env::remove_var("ICEFALL_DATA_DIR");

        assert_eq!(config.data_dir, PathBuf::from("/tmp/icefall-test"));
    }

    // --- ContainerRuntime serde ---

    #[test]
    fn runtime_serializes_as_lowercase() {
        let docker_json = serde_json::to_string(&ContainerRuntime::Docker).unwrap();
        assert_eq!(docker_json, "\"docker\"");

        let podman_json = serde_json::to_string(&ContainerRuntime::Podman).unwrap();
        assert_eq!(podman_json, "\"podman\"");
    }

    #[test]
    fn runtime_deserializes_from_lowercase() {
        let docker: ContainerRuntime = serde_json::from_str("\"docker\"").unwrap();
        assert_eq!(docker, ContainerRuntime::Docker);

        let podman: ContainerRuntime = serde_json::from_str("\"podman\"").unwrap();
        assert_eq!(podman, ContainerRuntime::Podman);
    }
}
