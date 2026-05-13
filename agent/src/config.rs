use serde::Deserialize;

const DEFAULT_CONFIG_PATH: &str = "/etc/icefall-agent/config.toml";

#[derive(Debug, Deserialize, Clone, PartialEq)]
pub enum ContainerRuntime {
    #[serde(rename = "docker")]
    Docker,
    #[serde(rename = "podman")]
    Podman,
}

impl Default for ContainerRuntime {
    fn default() -> Self {
        Self::Docker
    }
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
    pub fn from_socket(socket: &str) -> Self {
        if socket.contains("podman") {
            Self::Podman
        } else {
            Self::Docker
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct AgentConfig {
    pub control_plane_url: String,
    pub token: String,
    pub server_id: String,
    #[serde(default = "default_log_level")]
    pub log_level: String,
    #[serde(default)]
    pub runtime: ContainerRuntime,
    #[serde(default = "default_container_socket", alias = "docker_socket")]
    pub container_socket: String,
    #[serde(default = "default_caddy_admin_url")]
    pub caddy_admin_url: String,
    #[serde(default = "default_data_dir")]
    pub data_dir: String,
}

fn default_container_socket() -> String {
    let podman_sockets = ["/run/podman/podman.sock", "/var/run/podman/podman.sock"];
    for socket in &podman_sockets {
        if std::path::Path::new(socket).exists() {
            return socket.to_string();
        }
    }
    "/var/run/docker.sock".to_string()
}

fn default_caddy_admin_url() -> String {
    "http://localhost:2019".to_string()
}

fn default_data_dir() -> String {
    "/var/lib/icefall-agent".to_string()
}

fn default_log_level() -> String {
    "info".to_string()
}

impl AgentConfig {
    pub fn load(config_path: Option<&str>) -> Result<Self, String> {
        let path = config_path.unwrap_or(DEFAULT_CONFIG_PATH);

        let contents = std::fs::read_to_string(path).map_err(|e| {
            format!(
                "Failed to read config file at {path}: {e}\n\n\
                 Create {DEFAULT_CONFIG_PATH} with:\n\n\
                 control_plane_url = \"https://your-icefall-server.com\"\n\
                 token = \"your-agent-token\"\n\
                 server_id = \"your-server-id\"\n\
                 log_level = \"info\"\n"
            )
        })?;

        let mut config: AgentConfig =
            toml::from_str(&contents).map_err(|e| format!("Invalid config file: {e}"))?;

        // Environment variable overrides
        if let Ok(url) = std::env::var("ICEFALL_CONTROL_PLANE_URL") {
            config.control_plane_url = url;
        }
        if let Ok(token) = std::env::var("ICEFALL_TOKEN") {
            config.token = token;
        }

        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> Result<(), String> {
        if self.control_plane_url.is_empty() {
            return Err("control_plane_url must not be empty".into());
        }
        if self.token.is_empty() {
            return Err("token must not be empty".into());
        }
        if self.server_id.is_empty() {
            return Err("server_id must not be empty".into());
        }
        Ok(())
    }

    pub fn ws_url(&self) -> String {
        let base = self.control_plane_url.trim_end_matches('/');
        let scheme = if base.starts_with("https://") {
            base.replacen("https://", "wss://", 1)
        } else {
            base.replacen("http://", "ws://", 1)
        };
        format!("{scheme}/api/v1/agent/ws")
    }
}
