use sqlx::SqlitePool;

use crate::db::models::*;
use crate::db::DbError;

pub(super) async fn list_shared_variables(
    pool: &SqlitePool,
    scope: &str,
    scope_id: &str,
) -> Result<Vec<SharedVariable>, DbError> {
    let vars = sqlx::query_as::<_, SharedVariable>(
        "SELECT * FROM shared_variables WHERE scope = ? AND scope_id = ? ORDER BY key",
    )
    .bind(scope)
    .bind(scope_id)
    .fetch_all(pool)
    .await?;
    Ok(vars)
}

pub(super) async fn create_shared_variable(
    pool: &SqlitePool,
    var: &NewSharedVariable,
) -> Result<SharedVariable, DbError> {
    let id = new_id();
    let now = now_iso8601();

    sqlx::query(
        "INSERT INTO shared_variables (id, scope, scope_id, key, value, is_sensitive, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(scope, scope_id, key) DO UPDATE SET value = excluded.value, is_sensitive = excluded.is_sensitive, updated_at = excluded.updated_at",
    )
    .bind(&id)
    .bind(&var.scope)
    .bind(&var.scope_id)
    .bind(&var.key)
    .bind(&var.value)
    .bind(var.is_sensitive)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;

    let result = sqlx::query_as::<_, SharedVariable>(
        "SELECT * FROM shared_variables WHERE scope = ? AND scope_id = ? AND key = ?",
    )
    .bind(&var.scope)
    .bind(&var.scope_id)
    .bind(&var.key)
    .fetch_one(pool)
    .await?;

    Ok(result)
}

pub(super) async fn delete_shared_variable(pool: &SqlitePool, id: &str) -> Result<(), DbError> {
    sqlx::query("DELETE FROM shared_variables WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Resolve shared variables for an app by walking the scope chain: server → project
/// Returns (key, value, source) tuples
pub(super) async fn resolve_shared_variables(
    pool: &SqlitePool,
    app_id: &str,
) -> Result<Vec<(String, String, String)>, DbError> {
    let app = sqlx::query_as::<_, App>("SELECT * FROM apps WHERE id = ?")
        .bind(app_id)
        .fetch_optional(pool)
        .await?;

    let Some(app) = app else {
        return Ok(Vec::new());
    };

    let mut result: std::collections::HashMap<String, (String, String)> = std::collections::HashMap::new();

    // Server-level variables (lowest priority)
    if let Some(ref server_id) = app.server_id {
        let server_vars = sqlx::query_as::<_, SharedVariable>(
            "SELECT * FROM shared_variables WHERE scope = 'server' AND scope_id = ?",
        )
        .bind(server_id)
        .fetch_all(pool)
        .await?;
        for v in server_vars {
            result.insert(v.key, (v.value, format!("server:{server_id}")));
        }
    }

    // Project-level variables (higher priority, overrides server)
    if let Some(ref project_id) = app.project_id {
        let project_vars = sqlx::query_as::<_, SharedVariable>(
            "SELECT * FROM shared_variables WHERE scope = 'project' AND scope_id = ?",
        )
        .bind(project_id)
        .fetch_all(pool)
        .await?;
        for v in project_vars {
            result.insert(v.key, (v.value, format!("project:{project_id}")));
        }
    }

    Ok(result
        .into_iter()
        .map(|(key, (value, source))| (key, value, source))
        .collect())
}
