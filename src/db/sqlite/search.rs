use sqlx::SqlitePool;

use crate::db::DbError;

pub(super) async fn search(pool: &SqlitePool, query: &str) -> Result<serde_json::Value, DbError> {
    let pattern = format!("%{query}%");

    let apps = sqlx::query_as::<_, (String, String)>(
        "SELECT id, name FROM apps WHERE name LIKE ? LIMIT 10",
    )
    .bind(&pattern)
    .fetch_all(pool)
    .await?;

    let databases = sqlx::query_as::<_, (String, String)>(
        "SELECT id, name FROM databases WHERE name LIKE ? LIMIT 10",
    )
    .bind(&pattern)
    .fetch_all(pool)
    .await?;

    let servers = sqlx::query_as::<_, (String, String)>(
        "SELECT id, name FROM servers WHERE name LIKE ? OR host LIKE ? LIMIT 10",
    )
    .bind(&pattern)
    .bind(&pattern)
    .fetch_all(pool)
    .await?;

    let projects = sqlx::query_as::<_, (String, String)>(
        "SELECT id, name FROM projects WHERE name LIKE ? LIMIT 10",
    )
    .bind(&pattern)
    .fetch_all(pool)
    .await?;

    let domains = sqlx::query_as::<_, (String, String)>(
        "SELECT id, domain FROM domains WHERE domain LIKE ? LIMIT 10",
    )
    .bind(&pattern)
    .fetch_all(pool)
    .await?;

    Ok(serde_json::json!({
        "apps": apps.iter().map(|(id, name)| serde_json::json!({"id": id, "name": name, "type": "app"})).collect::<Vec<_>>(),
        "databases": databases.iter().map(|(id, name)| serde_json::json!({"id": id, "name": name, "type": "database"})).collect::<Vec<_>>(),
        "servers": servers.iter().map(|(id, name)| serde_json::json!({"id": id, "name": name, "type": "server"})).collect::<Vec<_>>(),
        "projects": projects.iter().map(|(id, name)| serde_json::json!({"id": id, "name": name, "type": "project"})).collect::<Vec<_>>(),
        "domains": domains.iter().map(|(id, name)| serde_json::json!({"id": id, "name": name, "type": "domain"})).collect::<Vec<_>>(),
    }))
}
