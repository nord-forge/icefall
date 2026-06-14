use sqlx::SqlitePool;

use crate::db::models::*;
use crate::db::DbError;

pub(super) async fn create_project_environment(
    pool: &SqlitePool,
    env: &NewProjectEnvironment,
) -> Result<ProjectEnvironment, DbError> {
    let id = new_id();
    let now = now_iso8601();

    // Determine next sort_order for this project
    let max_order: Option<i32> =
        sqlx::query_scalar("SELECT MAX(sort_order) FROM project_environments WHERE project_id = ?")
            .bind(&env.project_id)
            .fetch_one(pool)
            .await?;
    let sort_order = max_order.unwrap_or(0) + 1;

    sqlx::query(
        "INSERT INTO project_environments (id, project_id, name, slug, color, sort_order, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&env.project_id)
    .bind(&env.name)
    .bind(&env.slug)
    .bind(&env.color)
    .bind(sort_order)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;

    Ok(ProjectEnvironment {
        id,
        project_id: env.project_id.clone(),
        name: env.name.clone(),
        slug: env.slug.clone(),
        color: env.color.clone(),
        sort_order,
        created_at: now.clone(),
        updated_at: now,
    })
}

pub(super) async fn list_project_environments(
    pool: &SqlitePool,
    project_id: &str,
) -> Result<Vec<ProjectEnvironment>, DbError> {
    let envs = sqlx::query_as::<_, ProjectEnvironment>(
        "SELECT * FROM project_environments WHERE project_id = ? ORDER BY sort_order ASC",
    )
    .bind(project_id)
    .fetch_all(pool)
    .await?;
    Ok(envs)
}

pub(super) async fn update_project_environment(
    pool: &SqlitePool,
    id: &str,
    name: &str,
    color: Option<&str>,
) -> Result<ProjectEnvironment, DbError> {
    let now = now_iso8601();
    let slug = name.to_lowercase().replace(' ', "-");

    sqlx::query(
        "UPDATE project_environments SET name = ?, slug = ?, color = ?, updated_at = ? WHERE id = ?",
    )
    .bind(name)
    .bind(&slug)
    .bind(color)
    .bind(&now)
    .bind(id)
    .execute(pool)
    .await?;

    get_project_environment(pool, id)
        .await?
        .ok_or_else(|| DbError::NotFound(format!("project_environment {id}")))
}

pub(super) async fn delete_project_environment(pool: &SqlitePool, id: &str) -> Result<(), DbError> {
    sqlx::query("DELETE FROM project_environments WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub(super) async fn get_project_environment(
    pool: &SqlitePool,
    id: &str,
) -> Result<Option<ProjectEnvironment>, DbError> {
    let env =
        sqlx::query_as::<_, ProjectEnvironment>("SELECT * FROM project_environments WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await?;
    Ok(env)
}
