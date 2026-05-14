use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Registry {
    pub id: String,
    pub name: String,
    pub url: String,
    pub username: String,
    #[serde(skip_serializing)]
    pub password: String,
    pub registry_type: String,
    pub created_at: String,
    pub updated_at: String,
}

pub struct NewRegistry {
    pub name: String,
    pub url: String,
    pub username: String,
    pub password: String,
    pub registry_type: String,
}
