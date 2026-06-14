use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ServerTeamAccess {
    pub id: String,
    pub server_id: String,
    pub team_id: String,
    pub access_level: String,
    pub granted_by: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Team {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub owner_id: String,
    pub settings: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

pub struct NewTeam {
    pub name: String,
    pub slug: String,
    pub owner_id: String,
}

#[derive(Default)]
pub struct UpdateTeam {
    pub name: Option<String>,
    pub slug: Option<String>,
    pub settings: Option<Option<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TeamMembership {
    pub id: String,
    pub team_id: String,
    pub user_id: String,
    pub role: String,
    pub invited_by: Option<String>,
    pub accepted_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TeamInvitation {
    pub id: String,
    pub team_id: String,
    pub email: String,
    pub role: String,
    pub token: String,
    pub invited_by: String,
    pub expires_at: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TeamMember {
    pub id: String,
    pub user_id: String,
    pub email: String,
    pub role: String,
    pub accepted_at: Option<String>,
    pub created_at: String,
}
