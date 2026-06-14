use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshKey {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub public_key: String,
    #[serde(skip_serializing)]
    pub private_key_encrypted: Vec<u8>,
    pub fingerprint: String,
    pub key_type: String,
    pub last_used_at: Option<String>,
    pub created_at: String,
}

pub struct NewSshKey {
    pub user_id: String,
    pub name: String,
    pub public_key: String,
    pub private_key_encrypted: Vec<u8>,
    pub fingerprint: String,
    pub key_type: String,
}
