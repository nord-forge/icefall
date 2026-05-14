use sqlx::SqlitePool;

use crate::db::models::*;
use crate::db::DbError;

// --- Servers ---

pub(super) async fn create_server(
    pool: &SqlitePool,
    server: &NewServer,
) -> Result<Server, DbError> {
    let id = new_id();
    let now = now_iso8601();

    sqlx::query(
        "INSERT INTO servers (id, name, host, role, status, token_hash, labels, resources, public_key, registered_at, created_at, updated_at)
         VALUES (?, ?, ?, ?, 'enrolling', ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&server.name)
    .bind(&server.host)
    .bind(&server.role)
    .bind(&server.token_hash)
    .bind(&server.labels)
    .bind(&server.resources)
    .bind(&server.public_key)
    .bind(&now)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;

    get_server(pool, &id)
        .await?
        .ok_or_else(|| DbError::NotFound(id))
}

pub(super) async fn get_server(pool: &SqlitePool, id: &str) -> Result<Option<Server>, DbError> {
    let server = sqlx::query_as::<_, Server>("SELECT * FROM servers WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    Ok(server)
}

pub(super) async fn get_server_by_token_hash(
    pool: &SqlitePool,
    hash: &str,
) -> Result<Option<Server>, DbError> {
    let server = sqlx::query_as::<_, Server>("SELECT * FROM servers WHERE token_hash = ?")
        .bind(hash)
        .fetch_optional(pool)
        .await?;
    Ok(server)
}

pub(super) async fn list_servers(pool: &SqlitePool) -> Result<Vec<Server>, DbError> {
    let servers = sqlx::query_as::<_, Server>("SELECT * FROM servers ORDER BY created_at ASC")
        .fetch_all(pool)
        .await?;
    Ok(servers)
}

pub(super) async fn update_server(
    pool: &SqlitePool,
    id: &str,
    update: &ServerUpdate,
) -> Result<Server, DbError> {
    let existing = get_server(pool, id)
        .await?
        .ok_or_else(|| DbError::NotFound(id.to_string()))?;

    let now = now_iso8601();
    let name = update.name.as_deref().unwrap_or(&existing.name);
    let host = update.host.as_deref().unwrap_or(&existing.host);
    let status = update.status.as_deref().unwrap_or(&existing.status);
    let token_hash = match &update.token_hash {
        Some(v) => v.as_deref(),
        None => existing.token_hash.as_deref(),
    };
    let agent_version = match &update.agent_version {
        Some(v) => v.as_deref(),
        None => existing.agent_version.as_deref(),
    };
    let labels = match &update.labels {
        Some(v) => v.as_deref(),
        None => existing.labels.as_deref(),
    };
    let resources = match &update.resources {
        Some(v) => v.as_deref(),
        None => existing.resources.as_deref(),
    };
    let public_key = match &update.public_key {
        Some(v) => v.as_deref(),
        None => existing.public_key.as_deref(),
    };

    sqlx::query(
        "UPDATE servers SET name = ?, host = ?, status = ?, token_hash = ?, agent_version = ?, labels = ?, resources = ?, public_key = ?, updated_at = ? WHERE id = ?",
    )
    .bind(name)
    .bind(host)
    .bind(status)
    .bind(token_hash)
    .bind(agent_version)
    .bind(labels)
    .bind(resources)
    .bind(public_key)
    .bind(&now)
    .bind(id)
    .execute(pool)
    .await?;

    get_server(pool, id)
        .await?
        .ok_or_else(|| DbError::NotFound(id.to_string()))
}

pub(super) async fn delete_server(pool: &SqlitePool, id: &str) -> Result<(), DbError> {
    sqlx::query("DELETE FROM servers WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub(super) async fn update_server_heartbeat(pool: &SqlitePool, id: &str) -> Result<(), DbError> {
    let now = now_iso8601();
    sqlx::query("UPDATE servers SET last_heartbeat_at = ?, updated_at = ? WHERE id = ?")
        .bind(&now)
        .bind(&now)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub(super) async fn update_server_status(
    pool: &SqlitePool,
    id: &str,
    status: &str,
) -> Result<(), DbError> {
    let now = now_iso8601();
    sqlx::query("UPDATE servers SET status = ?, updated_at = ? WHERE id = ?")
        .bind(status)
        .bind(&now)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub(super) async fn update_server_disk_alert_state(
    pool: &SqlitePool,
    id: &str,
    state: &str,
) -> Result<(), DbError> {
    sqlx::query("UPDATE servers SET disk_alert_state = ? WHERE id = ?")
        .bind(state)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

// --- Server Metrics (legacy single-server) ---

pub(super) async fn insert_server_metric(
    pool: &SqlitePool,
    snapshot: &crate::api::routes::server::ServerMetricsSnapshot,
) -> Result<(), DbError> {
    let id = new_id();
    sqlx::query(
        "INSERT INTO server_metrics (id, timestamp, cpu_percent, memory_used_bytes, memory_total_bytes, disk_used_bytes, disk_total_bytes)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&snapshot.timestamp)
    .bind(snapshot.cpu_percent as f64)
    .bind(snapshot.memory_used_bytes as i64)
    .bind(snapshot.memory_total_bytes as i64)
    .bind(snapshot.disk_used_bytes as i64)
    .bind(snapshot.disk_total_bytes as i64)
    .execute(pool)
    .await?;
    Ok(())
}

pub(super) async fn query_server_metrics(
    pool: &SqlitePool,
    from: &str,
    to: &str,
    limit: usize,
) -> Result<Vec<crate::api::routes::server::ServerMetricsSnapshot>, DbError> {
    let rows = sqlx::query(
        "SELECT timestamp, cpu_percent, memory_used_bytes, memory_total_bytes, disk_used_bytes, disk_total_bytes
         FROM server_metrics
         WHERE timestamp >= ? AND timestamp <= ?
         ORDER BY timestamp ASC
         LIMIT ?",
    )
    .bind(from)
    .bind(to)
    .bind(limit as i64)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .iter()
        .map(|row| {
            use sqlx::Row;
            crate::api::routes::server::ServerMetricsSnapshot {
                timestamp: row.get("timestamp"),
                cpu_percent: row.get::<f64, _>("cpu_percent") as f32,
                memory_used_bytes: row.get::<i64, _>("memory_used_bytes") as u64,
                memory_total_bytes: row.get::<i64, _>("memory_total_bytes") as u64,
                disk_used_bytes: row.get::<i64, _>("disk_used_bytes") as u64,
                disk_total_bytes: row.get::<i64, _>("disk_total_bytes") as u64,
            }
        })
        .collect())
}

// --- Server Metrics History (multi-server) ---

pub(super) async fn insert_server_metrics_record(
    pool: &SqlitePool,
    record: &NewServerMetricsRecord,
) -> Result<ServerMetricsRecord, DbError> {
    let id = new_id();
    let now = now_iso8601();

    sqlx::query(
        "INSERT INTO server_metrics_history (id, server_id, cpu_percent, ram_used_bytes, ram_total_bytes, disk_used_bytes, disk_total_bytes, load_average, recorded_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&record.server_id)
    .bind(record.cpu_percent)
    .bind(record.ram_used_bytes)
    .bind(record.ram_total_bytes)
    .bind(record.disk_used_bytes)
    .bind(record.disk_total_bytes)
    .bind(&record.load_average)
    .bind(&now)
    .execute(pool)
    .await?;

    Ok(ServerMetricsRecord {
        id,
        server_id: record.server_id.clone(),
        cpu_percent: record.cpu_percent,
        ram_used_bytes: record.ram_used_bytes,
        ram_total_bytes: record.ram_total_bytes,
        disk_used_bytes: record.disk_used_bytes,
        disk_total_bytes: record.disk_total_bytes,
        load_average: record.load_average.clone(),
        recorded_at: now,
    })
}

pub(super) async fn query_server_metrics_history(
    pool: &SqlitePool,
    server_id: &str,
    from: &str,
    to: &str,
    limit: usize,
) -> Result<Vec<ServerMetricsRecord>, DbError> {
    let records = sqlx::query_as::<_, ServerMetricsRecord>(
        "SELECT * FROM server_metrics_history WHERE server_id = ? AND recorded_at >= ? AND recorded_at <= ? ORDER BY recorded_at DESC LIMIT ?",
    )
    .bind(server_id)
    .bind(from)
    .bind(to)
    .bind(limit as i64)
    .fetch_all(pool)
    .await?;
    Ok(records)
}
