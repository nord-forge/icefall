use std::collections::HashMap;

use serde::{Deserialize, Serialize};

// --- Compose file types (MVP subset of the Docker Compose spec) ---

/// Top-level Compose file structure.
#[derive(Debug, Clone, Deserialize)]
pub struct ComposeFile {
    #[serde(default)]
    pub services: HashMap<String, ComposeService>,
    #[serde(default)]
    pub volumes: HashMap<String, Option<serde_yaml::Value>>,
}

/// A single service within a compose file.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct ComposeService {
    pub image: Option<String>,
    #[serde(default)]
    pub environment: ComposeEnvironment,
    #[serde(default, deserialize_with = "deserialize_string_or_number_vec")]
    pub ports: Vec<String>,
    #[serde(default)]
    pub volumes: Vec<String>,
    #[serde(default)]
    pub depends_on: ComposeDependsOn,
    pub command: Option<ComposeCommand>,
    pub entrypoint: Option<ComposeCommand>,
    pub restart: Option<String>,
    // Ignored fields — accept but don't act on them
    #[serde(default)]
    pub build: Option<serde_yaml::Value>,
    #[serde(default)]
    pub profiles: Option<serde_yaml::Value>,
    #[serde(default)]
    pub configs: Option<serde_yaml::Value>,
    #[serde(default)]
    pub secrets: Option<serde_yaml::Value>,
    #[serde(default)]
    pub deploy: Option<serde_yaml::Value>,
    #[serde(default)]
    pub networks: Option<serde_yaml::Value>,
    #[serde(default)]
    pub healthcheck: Option<serde_yaml::Value>,
    #[serde(default)]
    pub labels: Option<serde_yaml::Value>,
    #[serde(default)]
    pub logging: Option<serde_yaml::Value>,
    // Catch any other unrecognised keys
    #[serde(flatten)]
    pub _extra: HashMap<String, serde_yaml::Value>,
}

/// Environment variables — either a list of "KEY=VALUE" strings or a map.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(untagged)]
pub enum ComposeEnvironment {
    List(Vec<String>),
    Map(HashMap<String, Option<String>>),
    #[default]
    Empty,
}

/// depends_on — either a list of service names or a map of service-name -> condition.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(untagged)]
pub enum ComposeDependsOn {
    List(Vec<String>),
    Map(HashMap<String, serde_yaml::Value>),
    #[default]
    Empty,
}

impl ComposeDependsOn {
    pub(super) fn names(&self) -> Vec<String> {
        match self {
            ComposeDependsOn::List(v) => v.clone(),
            ComposeDependsOn::Map(m) => m.keys().cloned().collect(),
            ComposeDependsOn::Empty => Vec::new(),
        }
    }
}

/// Command — either a single string or a list of arguments.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum ComposeCommand {
    Simple(String),
    Args(Vec<String>),
}

/// Deserialize a Vec where each element can be a string or a number (converted to string).
/// Handles compose ports like `- 3000` (number) and `- "80:80"` (string).
pub fn deserialize_string_or_number_vec<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let values: Vec<serde_yaml::Value> = Vec::deserialize(deserializer)?;
    Ok(values
        .into_iter()
        .filter_map(|v| match v {
            serde_yaml::Value::String(s) => Some(s),
            serde_yaml::Value::Number(n) => Some(n.to_string()),
            _ => None,
        })
        .collect())
}

// --- Deploy result types ---

/// Result of deploying a full compose stack.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComposeDeployResult {
    pub network_name: String,
    pub services: Vec<ComposeServiceResult>,
}

/// Result of deploying a single compose service.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComposeServiceResult {
    pub service_name: String,
    pub container_id: String,
    pub image: String,
}
