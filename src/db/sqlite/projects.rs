use sqlx::SqlitePool;

use crate::db::models::*;
use crate::db::DbError;

pub(super) async fn list_projects(pool: &SqlitePool) -> Result<Vec<Project>, DbError> {
    let projects = sqlx::query_as::<_, Project>("SELECT * FROM projects ORDER BY name ASC")
        .fetch_all(pool)
        .await?;
    Ok(projects)
}

pub(super) async fn create_project(
    pool: &SqlitePool,
    project: &NewProject,
) -> Result<Project, DbError> {
    let id = new_id();
    let now = now_iso8601();

    sqlx::query(
        "INSERT INTO projects (id, name, description, color, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&project.name)
    .bind(&project.description)
    .bind(&project.color)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await
    .map_err(|e| match e {
        sqlx::Error::Database(ref db_err) if db_err.message().contains("UNIQUE") => {
            DbError::Duplicate(format!("project '{}' already exists", project.name))
        }
        other => DbError::Sqlx(other),
    })?;

    get_project(pool, &id)
        .await?
        .ok_or_else(|| DbError::NotFound(id))
}

pub(super) async fn get_project(pool: &SqlitePool, id: &str) -> Result<Option<Project>, DbError> {
    let project = sqlx::query_as::<_, Project>("SELECT * FROM projects WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    Ok(project)
}

pub(super) async fn update_project(
    pool: &SqlitePool,
    id: &str,
    update: &UpdateProject,
) -> Result<Project, DbError> {
    let existing = get_project(pool, id)
        .await?
        .ok_or_else(|| DbError::NotFound(format!("project {id}")))?;

    let name = update.name.as_deref().unwrap_or(&existing.name);
    let description = match &update.description {
        Some(v) => v.as_deref(),
        None => existing.description.as_deref(),
    };
    let color = match &update.color {
        Some(v) => v.as_deref(),
        None => existing.color.as_deref(),
    };
    let now = now_iso8601();

    sqlx::query(
        "UPDATE projects SET name = ?, description = ?, color = ?, updated_at = ? WHERE id = ?",
    )
    .bind(name)
    .bind(description)
    .bind(color)
    .bind(&now)
    .bind(id)
    .execute(pool)
    .await?;

    get_project(pool, id)
        .await?
        .ok_or_else(|| DbError::NotFound(id.to_string()))
}

pub(super) async fn delete_project(pool: &SqlitePool, id: &str) -> Result<(), DbError> {
    // Unassign all apps and databases from this project (don't delete them)
    sqlx::query("UPDATE apps SET project_id = NULL WHERE project_id = ?")
        .bind(id)
        .execute(pool)
        .await?;

    sqlx::query("UPDATE databases SET project_id = NULL WHERE project_id = ?")
        .bind(id)
        .execute(pool)
        .await?;

    let result = sqlx::query("DELETE FROM projects WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(DbError::NotFound(format!("project {id}")));
    }
    Ok(())
}
