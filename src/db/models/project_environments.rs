use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ProjectEnvironment {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub slug: String,
    pub color: Option<String>,
    pub sort_order: i32,
    pub created_at: String,
    pub updated_at: String,
}

pub struct NewProjectEnvironment {
    pub project_id: String,
    pub name: String,
    pub slug: String,
    pub color: Option<String>,
}
