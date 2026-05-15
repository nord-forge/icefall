use sqlx::SqlitePool;

use crate::db::models::{
    new_id, now_iso8601, NewTeam, Server, ServerTeamAccess, Team, TeamInvitation, TeamMember,
    TeamMembership, UpdateTeam,
};
use crate::db::DbError;

pub(super) async fn create_team(pool: &SqlitePool, team: &NewTeam) -> Result<Team, DbError> {
    let id = new_id();
    let now = now_iso8601();

    sqlx::query(
        "INSERT INTO teams (id, name, slug, owner_id, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&team.name)
    .bind(&team.slug)
    .bind(&team.owner_id)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;

    let membership_id = new_id();
    sqlx::query(
        "INSERT INTO team_memberships (id, team_id, user_id, role, accepted_at, created_at) VALUES (?, ?, ?, 'owner', ?, ?)",
    )
    .bind(&membership_id)
    .bind(&id)
    .bind(&team.owner_id)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;

    get_team(pool, &id)
        .await?
        .ok_or_else(|| DbError::NotFound("team".into()))
}

pub(super) async fn get_team(pool: &SqlitePool, id: &str) -> Result<Option<Team>, DbError> {
    Ok(
        sqlx::query_as::<_, Team>("SELECT * FROM teams WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await?,
    )
}

pub(super) async fn get_team_by_slug(
    pool: &SqlitePool,
    slug: &str,
) -> Result<Option<Team>, DbError> {
    Ok(
        sqlx::query_as::<_, Team>("SELECT * FROM teams WHERE slug = ?")
            .bind(slug)
            .fetch_optional(pool)
            .await?,
    )
}

pub(super) async fn list_teams_for_user(
    pool: &SqlitePool,
    user_id: &str,
) -> Result<Vec<Team>, DbError> {
    Ok(sqlx::query_as::<_, Team>(
        "SELECT t.* FROM teams t
         INNER JOIN team_memberships tm ON tm.team_id = t.id
         WHERE tm.user_id = ?
         ORDER BY t.created_at ASC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?)
}

pub(super) async fn update_team(
    pool: &SqlitePool,
    id: &str,
    update: &UpdateTeam,
) -> Result<Team, DbError> {
    let existing = get_team(pool, id)
        .await?
        .ok_or_else(|| DbError::NotFound(format!("team {id}")))?;

    let name = update.name.as_deref().unwrap_or(&existing.name);
    let slug = update.slug.as_deref().unwrap_or(&existing.slug);
    let settings = match &update.settings {
        Some(v) => v.as_deref(),
        None => existing.settings.as_deref(),
    };
    let now = now_iso8601();

    sqlx::query("UPDATE teams SET name = ?, slug = ?, settings = ?, updated_at = ? WHERE id = ?")
        .bind(name)
        .bind(slug)
        .bind(settings)
        .bind(&now)
        .bind(id)
        .execute(pool)
        .await?;

    get_team(pool, id)
        .await?
        .ok_or_else(|| DbError::NotFound(id.to_string()))
}

pub(super) async fn delete_team(pool: &SqlitePool, id: &str) -> Result<(), DbError> {
    let result = sqlx::query("DELETE FROM teams WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(DbError::NotFound(format!("team {id}")));
    }
    Ok(())
}

pub(super) async fn count_team_resources(pool: &SqlitePool, team_id: &str) -> Result<i64, DbError> {
    let row: (i64,) = sqlx::query_as(
        "SELECT
            (SELECT COUNT(*) FROM apps WHERE team_id = ?) +
            (SELECT COUNT(*) FROM projects WHERE team_id = ?) +
            (SELECT COUNT(*) FROM databases WHERE team_id = ?)",
    )
    .bind(team_id)
    .bind(team_id)
    .bind(team_id)
    .fetch_one(pool)
    .await?;

    Ok(row.0)
}

pub(super) async fn add_team_member(
    pool: &SqlitePool,
    team_id: &str,
    user_id: &str,
    role: &str,
    invited_by: Option<&str>,
) -> Result<TeamMembership, DbError> {
    let id = new_id();
    let now = now_iso8601();

    sqlx::query(
        "INSERT INTO team_memberships (id, team_id, user_id, role, invited_by, accepted_at, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(team_id)
    .bind(user_id)
    .bind(role)
    .bind(invited_by)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await
    .map_err(|e| {
        if let sqlx::Error::Database(ref db_err) = e {
            if db_err.message().contains("UNIQUE") {
                return DbError::Duplicate(format!("user {user_id} already in team {team_id}"));
            }
        }
        DbError::Sqlx(e)
    })?;

    Ok(TeamMembership {
        id,
        team_id: team_id.to_string(),
        user_id: user_id.to_string(),
        role: role.to_string(),
        invited_by: invited_by.map(String::from),
        accepted_at: Some(now.clone()),
        created_at: now,
    })
}

pub(super) async fn list_team_members(
    pool: &SqlitePool,
    team_id: &str,
) -> Result<Vec<TeamMember>, DbError> {
    Ok(sqlx::query_as::<_, TeamMember>(
        "SELECT tm.id, tm.user_id, u.email, tm.role, tm.accepted_at, tm.created_at
         FROM team_memberships tm
         INNER JOIN users u ON u.id = tm.user_id
         WHERE tm.team_id = ?
         ORDER BY tm.created_at ASC",
    )
    .bind(team_id)
    .fetch_all(pool)
    .await?)
}

pub(super) async fn get_team_membership(
    pool: &SqlitePool,
    team_id: &str,
    user_id: &str,
) -> Result<Option<TeamMembership>, DbError> {
    Ok(sqlx::query_as::<_, TeamMembership>(
        "SELECT * FROM team_memberships WHERE team_id = ? AND user_id = ?",
    )
    .bind(team_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await?)
}

pub(super) async fn update_team_member_role(
    pool: &SqlitePool,
    team_id: &str,
    user_id: &str,
    role: &str,
) -> Result<(), DbError> {
    let result =
        sqlx::query("UPDATE team_memberships SET role = ? WHERE team_id = ? AND user_id = ?")
            .bind(role)
            .bind(team_id)
            .bind(user_id)
            .execute(pool)
            .await?;

    if result.rows_affected() == 0 {
        return Err(DbError::NotFound(format!(
            "membership for user {user_id} in team {team_id}"
        )));
    }
    Ok(())
}

pub(super) async fn remove_team_member(
    pool: &SqlitePool,
    team_id: &str,
    user_id: &str,
) -> Result<(), DbError> {
    let result = sqlx::query("DELETE FROM team_memberships WHERE team_id = ? AND user_id = ?")
        .bind(team_id)
        .bind(user_id)
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(DbError::NotFound(format!(
            "membership for user {user_id} in team {team_id}"
        )));
    }
    Ok(())
}

pub(super) async fn create_team_invitation(
    pool: &SqlitePool,
    team_id: &str,
    email: &str,
    role: &str,
    token: &str,
    invited_by: &str,
    expires_at: &str,
) -> Result<TeamInvitation, DbError> {
    let id = new_id();
    let now = now_iso8601();

    sqlx::query(
        "INSERT INTO team_invitations (id, team_id, email, role, token, invited_by, expires_at, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(team_id)
    .bind(email)
    .bind(role)
    .bind(token)
    .bind(invited_by)
    .bind(expires_at)
    .bind(&now)
    .execute(pool)
    .await?;

    Ok(TeamInvitation {
        id,
        team_id: team_id.to_string(),
        email: email.to_string(),
        role: role.to_string(),
        token: token.to_string(),
        invited_by: invited_by.to_string(),
        expires_at: expires_at.to_string(),
        created_at: now,
    })
}

pub(super) async fn get_team_invitation_by_token(
    pool: &SqlitePool,
    token: &str,
) -> Result<Option<TeamInvitation>, DbError> {
    Ok(
        sqlx::query_as::<_, TeamInvitation>("SELECT * FROM team_invitations WHERE token = ?")
            .bind(token)
            .fetch_optional(pool)
            .await?,
    )
}

pub(super) async fn list_team_invitations(
    pool: &SqlitePool,
    team_id: &str,
) -> Result<Vec<TeamInvitation>, DbError> {
    Ok(sqlx::query_as::<_, TeamInvitation>(
        "SELECT * FROM team_invitations WHERE team_id = ? ORDER BY created_at DESC",
    )
    .bind(team_id)
    .fetch_all(pool)
    .await?)
}

pub(super) async fn delete_team_invitation(pool: &SqlitePool, id: &str) -> Result<(), DbError> {
    sqlx::query("DELETE FROM team_invitations WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub(super) async fn set_session_team(
    pool: &SqlitePool,
    session_id: &str,
    team_id: &str,
) -> Result<(), DbError> {
    let result = sqlx::query("UPDATE sessions SET active_team_id = ? WHERE id = ?")
        .bind(team_id)
        .bind(session_id)
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(DbError::NotFound(format!("session {session_id}")));
    }
    Ok(())
}

pub(super) async fn share_server_with_team(
    pool: &SqlitePool,
    server_id: &str,
    team_id: &str,
    access_level: &str,
    granted_by: &str,
) -> Result<ServerTeamAccess, DbError> {
    let id = new_id();
    let now = now_iso8601();

    sqlx::query(
        "INSERT INTO server_team_access (id, server_id, team_id, access_level, granted_by, created_at)
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(server_id)
    .bind(team_id)
    .bind(access_level)
    .bind(granted_by)
    .bind(&now)
    .execute(pool)
    .await
    .map_err(|e| {
        if let sqlx::Error::Database(ref db_err) = e {
            if db_err.message().contains("UNIQUE") {
                return DbError::Duplicate(format!(
                    "server {server_id} already shared with team {team_id}"
                ));
            }
        }
        DbError::Sqlx(e)
    })?;

    Ok(ServerTeamAccess {
        id,
        server_id: server_id.to_string(),
        team_id: team_id.to_string(),
        access_level: access_level.to_string(),
        granted_by: granted_by.to_string(),
        created_at: now,
    })
}

pub(super) async fn revoke_server_share(
    pool: &SqlitePool,
    server_id: &str,
    team_id: &str,
) -> Result<(), DbError> {
    let result = sqlx::query("DELETE FROM server_team_access WHERE server_id = ? AND team_id = ?")
        .bind(server_id)
        .bind(team_id)
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(DbError::NotFound(format!(
            "share for server {server_id} with team {team_id}"
        )));
    }
    Ok(())
}

pub(super) async fn list_server_shares(
    pool: &SqlitePool,
    server_id: &str,
) -> Result<Vec<ServerTeamAccess>, DbError> {
    Ok(sqlx::query_as::<_, ServerTeamAccess>(
        "SELECT * FROM server_team_access WHERE server_id = ? ORDER BY created_at ASC",
    )
    .bind(server_id)
    .fetch_all(pool)
    .await?)
}

pub(super) async fn list_servers_shared_with_team(
    pool: &SqlitePool,
    team_id: &str,
) -> Result<Vec<(Server, String)>, DbError> {
    let rows = sqlx::query_as::<_, (String, String)>(
        "SELECT s.id, sta.access_level
         FROM servers s
         INNER JOIN server_team_access sta ON sta.server_id = s.id
         WHERE sta.team_id = ?
         ORDER BY s.created_at ASC",
    )
    .bind(team_id)
    .fetch_all(pool)
    .await?;

    let mut results = Vec::new();
    for (server_id, access_level) in rows {
        if let Some(server) = sqlx::query_as::<_, Server>("SELECT * FROM servers WHERE id = ?")
            .bind(&server_id)
            .fetch_optional(pool)
            .await?
        {
            results.push((server, access_level));
        }
    }
    Ok(results)
}
