use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Incident {
    pub id: String,
    pub title: String,
    pub status: String,
    pub severity: String,
    pub affected_apps: Option<String>,
    pub affected_servers: Option<String>,
    pub root_cause: Option<String>,
    pub started_at: String,
    pub resolved_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

pub struct NewIncident {
    pub title: String,
    pub severity: String,
    pub affected_apps: Option<String>,
    pub affected_servers: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct IncidentNote {
    pub id: String,
    pub incident_id: String,
    pub content: String,
    pub author_id: Option<String>,
    pub created_at: String,
}
