use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedVariable {
    pub id: String,
    pub scope: String,
    pub scope_id: String,
    pub key: String,
    pub value: String,
    pub is_sensitive: bool,
    pub created_at: String,
    pub updated_at: String,
}

pub struct NewSharedVariable {
    pub scope: String,
    pub scope_id: String,
    pub key: String,
    pub value: String,
    pub is_sensitive: bool,
}
