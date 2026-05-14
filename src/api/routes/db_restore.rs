use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;

use crate::api::error::ApiError;
use crate::api::AppState;

#[derive(Deserialize)]
pub struct RestoreRequest {
    #[serde(default = "default_source_type")]
    source_type: String,
    source_ref: Option<String>,
    custom_command: Option<String>,
}

fn default_source_type() -> String {
    "file".to_string()
}

pub async fn restore_database(
    State(state): State<AppState>,
    Path(db_id): Path<String>,
    Json(body): Json<RestoreRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let managed_dbs = state.db.list_managed_dbs().await?;
    let db_record = managed_dbs
        .iter()
        .find(|d| d.id == db_id)
        .ok_or_else(|| ApiError::NotFound(format!("database {db_id}")))?;

    let container_id = db_record
        .container_id
        .as_ref()
        .ok_or_else(|| ApiError::BadRequest("Database has no running container".into()))?;

    let restore_record = state
        .db
        .create_restore_record(&db_id, &body.source_type, body.source_ref.as_deref())
        .await?;

    let restore_id = restore_record.id.clone();
    let db_type = db_record.db_type.clone();
    let container = container_id.clone();
    let custom_cmd = body.custom_command;
    let docker = state.docker.clone();
    let db = state.db.clone();

    tokio::spawn(async move {
        let restore_cmd = if let Some(cmd) = custom_cmd {
            cmd
        } else {
            match db_type.as_str() {
                "postgresql" => "pg_restore -U $POSTGRES_USER -d $POSTGRES_DB /tmp/restore_dump 2>&1 || psql -U $POSTGRES_USER -d $POSTGRES_DB -f /tmp/restore_dump 2>&1".to_string(),
                "mysql" => "mysql -u $MYSQL_USER -p$MYSQL_PASSWORD $MYSQL_DATABASE < /tmp/restore_dump 2>&1".to_string(),
                "mariadb" => "mariadb -u $MARIADB_USER -p$MARIADB_PASSWORD $MARIADB_DATABASE < /tmp/restore_dump 2>&1".to_string(),
                "mongodb" => "mongorestore --gzip --archive=/tmp/restore_dump 2>&1".to_string(),
                "redis" => "redis-cli --pipe < /tmp/restore_dump 2>&1".to_string(),
                _ => {
                    let _ = db.update_restore_record(&restore_id, "failed", Some(&format!("Unsupported database type: {db_type}"))).await;
                    return;
                }
            }
        };

        let cmd_parts = vec!["sh".to_string(), "-c".to_string(), restore_cmd];
        match docker.exec_in_container(&container, &cmd_parts).await {
            Ok(output) => {
                let _ = db
                    .update_restore_record(&restore_id, "success", Some(&output))
                    .await;
            }
            Err(e) => {
                let _ = db
                    .update_restore_record(&restore_id, "failed", Some(&e.to_string()))
                    .await;
            }
        }
    });

    Ok(Json(serde_json::json!({ "data": restore_record })))
}

pub async fn list_restore_history(
    State(state): State<AppState>,
    Path(db_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let history = state.db.list_restore_history(&db_id, 20).await?;
    Ok(Json(serde_json::json!({ "data": history })))
}
