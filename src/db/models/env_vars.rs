use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvVar {
    pub id: String,
    pub environment_id: String,
    pub key: String,
    pub value: String,
    pub scope: String,
    pub created_at: String,
}

pub struct NewEnvVar {
    pub environment_id: String,
    pub key: String,
    pub value: String,
    pub scope: String,
}
