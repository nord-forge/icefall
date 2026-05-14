use sqlx::SqlitePool;

use crate::db::models::*;
use crate::db::DbError;

pub(super) async fn create_app(pool: &SqlitePool, app: &NewApp) -> Result<App, DbError> {
    let id = new_id();
    let now = now_iso8601();
    let deploy_mode = app.deploy_mode.as_deref().unwrap_or("auto");

    let server_id = app.server_id.as_deref().unwrap_or(CONTROL_PLANE_SERVER_ID);

    sqlx::query(
        "INSERT INTO apps (id, name, git_repo, git_branch, framework, image_ref, compose_content, deploy_mode, server_id, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&app.name)
    .bind(&app.git_repo)
    .bind(&app.git_branch)
    .bind(&app.framework)
    .bind(&app.image_ref)
    .bind(&app.compose_content)
    .bind(deploy_mode)
    .bind(server_id)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await
    .map_err(|e| match e {
        sqlx::Error::Database(ref db_err) if db_err.message().contains("UNIQUE") => {
            DbError::Duplicate(format!("app '{}' already exists", app.name))
        }
        other => DbError::Sqlx(other),
    })?;

    get_app(pool, &id)
        .await?
        .ok_or_else(|| DbError::NotFound(id))
}

pub(super) async fn get_app(pool: &SqlitePool, id: &str) -> Result<Option<App>, DbError> {
    let app = sqlx::query_as::<_, App>("SELECT * FROM apps WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    Ok(app)
}

pub(super) async fn get_app_by_name(pool: &SqlitePool, name: &str) -> Result<Option<App>, DbError> {
    let app = sqlx::query_as::<_, App>("SELECT * FROM apps WHERE name = ?")
        .bind(name)
        .fetch_optional(pool)
        .await?;
    Ok(app)
}

pub(super) async fn list_apps(pool: &SqlitePool) -> Result<Vec<App>, DbError> {
    let apps = sqlx::query_as::<_, App>("SELECT * FROM apps ORDER BY created_at DESC")
        .fetch_all(pool)
        .await?;
    Ok(apps)
}

pub(super) async fn list_apps_by_project(
    pool: &SqlitePool,
    project_id: &str,
) -> Result<Vec<App>, DbError> {
    let apps = sqlx::query_as::<_, App>(
        "SELECT * FROM apps WHERE project_id = ? ORDER BY created_at DESC",
    )
    .bind(project_id)
    .fetch_all(pool)
    .await?;
    Ok(apps)
}

pub(super) async fn update_app(
    pool: &SqlitePool,
    id: &str,
    update: &UpdateApp,
) -> Result<App, DbError> {
    let existing = get_app(pool, id)
        .await?
        .ok_or_else(|| DbError::NotFound(format!("app {id}")))?;

    let name = update.name.as_deref().unwrap_or(&existing.name);
    let git_repo = update.git_repo.as_deref().or(existing.git_repo.as_deref());
    let git_branch = update.git_branch.as_deref().unwrap_or(&existing.git_branch);
    let framework = update
        .framework
        .as_deref()
        .or(existing.framework.as_deref());
    let build_config = update
        .build_config
        .as_deref()
        .or(existing.build_config.as_deref());
    let resource_limits = update
        .resource_limits
        .as_deref()
        .or(existing.resource_limits.as_deref());
    let preview_enabled = update.preview_enabled.unwrap_or(existing.preview_enabled);
    let preview_branch_pattern = match &update.preview_branch_pattern {
        Some(v) => v.as_deref(),
        None => existing.preview_branch_pattern.as_deref(),
    };
    let tags = update.tags.as_deref().or(existing.tags.as_deref());
    let volumes = update.volumes.as_deref().or(existing.volumes.as_deref());
    let image_ref = match &update.image_ref {
        Some(v) => v.as_deref(),
        None => existing.image_ref.as_deref(),
    };
    let compose_content = match &update.compose_content {
        Some(v) => v.as_deref(),
        None => existing.compose_content.as_deref(),
    };
    let project_id = match &update.project_id {
        Some(v) => v.as_deref(),
        None => existing.project_id.as_deref(),
    };
    let deploy_mode = update
        .deploy_mode
        .as_deref()
        .unwrap_or(&existing.deploy_mode);
    let server_id = match &update.server_id {
        Some(v) => v.as_deref(),
        None => existing.server_id.as_deref(),
    };
    let base_directory = match &update.base_directory {
        Some(v) => v.as_deref(),
        None => existing.base_directory.as_deref(),
    };
    let disable_build_cache = update
        .disable_build_cache
        .unwrap_or(existing.disable_build_cache);
    let git_submodules_enabled = update
        .git_submodules_enabled
        .unwrap_or(existing.git_submodules_enabled);
    let git_lfs_enabled = update.git_lfs_enabled.unwrap_or(existing.git_lfs_enabled);
    let git_shallow_clone = update
        .git_shallow_clone
        .unwrap_or(existing.git_shallow_clone);
    let basic_auth_enabled = update
        .basic_auth_enabled
        .unwrap_or(existing.basic_auth_enabled);
    let basic_auth_username = match &update.basic_auth_username {
        Some(v) => v.as_deref(),
        None => existing.basic_auth_username.as_deref(),
    };
    let basic_auth_password_hash = match &update.basic_auth_password_hash {
        Some(v) => v.as_deref(),
        None => existing.basic_auth_password_hash.as_deref(),
    };
    let pre_deploy_commands = match &update.pre_deploy_commands {
        Some(v) => v.as_deref(),
        None => existing.pre_deploy_commands.as_deref(),
    };
    let now = now_iso8601();

    sqlx::query(
        "UPDATE apps SET name = ?, git_repo = ?, git_branch = ?, framework = ?,
         build_config = ?, resource_limits = ?, preview_enabled = ?,
         preview_branch_pattern = ?, tags = ?, volumes = ?, image_ref = ?,
         compose_content = ?, project_id = ?, deploy_mode = ?, server_id = ?,
         base_directory = ?, disable_build_cache = ?, git_submodules_enabled = ?,
         git_lfs_enabled = ?, git_shallow_clone = ?, basic_auth_enabled = ?,
         basic_auth_username = ?, basic_auth_password_hash = ?, pre_deploy_commands = ?,
         updated_at = ? WHERE id = ?",
    )
    .bind(name)
    .bind(git_repo)
    .bind(git_branch)
    .bind(framework)
    .bind(build_config)
    .bind(resource_limits)
    .bind(preview_enabled)
    .bind(preview_branch_pattern)
    .bind(tags)
    .bind(volumes)
    .bind(image_ref)
    .bind(compose_content)
    .bind(project_id)
    .bind(deploy_mode)
    .bind(server_id)
    .bind(base_directory)
    .bind(disable_build_cache)
    .bind(git_submodules_enabled)
    .bind(git_lfs_enabled)
    .bind(git_shallow_clone)
    .bind(basic_auth_enabled)
    .bind(basic_auth_username)
    .bind(basic_auth_password_hash)
    .bind(pre_deploy_commands)
    .bind(&now)
    .bind(id)
    .execute(pool)
    .await?;

    get_app(pool, id)
        .await?
        .ok_or_else(|| DbError::NotFound(id.to_string()))
}

pub(super) async fn delete_app(pool: &SqlitePool, id: &str) -> Result<(), DbError> {
    let result = sqlx::query("DELETE FROM apps WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(DbError::NotFound(format!("app {id}")));
    }
    Ok(())
}
