use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;

use crate::api::error::ApiError;
use crate::api::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/databases/{id}/tables", get(list_tables))
        .route("/databases/{id}/query", post(execute_query))
}

#[derive(Deserialize)]
struct QueryBody {
    #[serde(alias = "sql")]
    query: String,
}

struct DbInfo {
    container_name: String,
    db_type: String,
    user: String,
    password: String,
    connection_string: String,
}

async fn get_db_info(state: &AppState, id: &str) -> Result<DbInfo, ApiError> {
    let dbs = state.db.list_managed_dbs().await?;
    let db = dbs
        .iter()
        .find(|d| d.id == id)
        .ok_or_else(|| ApiError::NotFound(format!("database {id}")))?;

    let creds: serde_json::Value = serde_json::from_str(&db.credentials).unwrap_or_default();
    let conn_str = creds
        .get("connection_string")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    if conn_str.is_empty() {
        return Err(ApiError::BadRequest(
            "No credentials stored for this database".into(),
        ));
    }

    let container_name = creds
        .get("host")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let user = creds
        .get("user")
        .and_then(|v| v.as_str())
        .unwrap_or("icefall")
        .to_string();
    let password = creds
        .get("password")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    Ok(DbInfo {
        container_name,
        db_type: db.db_type.clone(),
        user,
        password,
        connection_string: conn_str,
    })
}

async fn list_tables(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let info = get_db_info(&state, &id).await?;

    let cmd: Vec<String> = match info.db_type.as_str() {
        "postgres" => vec![
            "psql".into(), info.connection_string.replace(&info.container_name, "localhost"),
            "-t".into(), "-A".into(), "-c".into(),
            "SELECT table_name FROM information_schema.tables WHERE table_schema = 'public' ORDER BY table_name".into(),
        ],
        "mysql" => vec![
            "mysql".into(), format!("-u{}", info.user), format!("-p{}", info.password),
            "-e".into(), "SELECT table_name FROM information_schema.tables WHERE table_schema = DATABASE() ORDER BY table_name".into(),
            "--batch".into(), "--skip-column-names".into(),
        ],
        "mongo" => vec![
            "mongosh".into(), "--quiet".into(),
            format!("mongodb://{}:{}@localhost:27017/{}?authSource=admin", info.user, info.password, info.user),
            "--eval".into(), "db.getCollectionNames().forEach(c => print(c))".into(),
        ],
        "redis" => vec![
            "redis-cli".into(), "-a".into(), info.password.clone(), "--no-auth-warning".into(),
            "--scan".into(), "--pattern".into(), "*".into(), "--count".into(), "500".into(),
        ],
        _ => return Err(ApiError::BadRequest("Unsupported database type".into())),
    };

    let output = state
        .docker
        .exec_in_container(&info.container_name, &cmd)
        .await
        .map_err(|e| ApiError::Internal(Box::new(e)))?;

    let items: Vec<&str> = output.trim().lines().filter(|l| !l.is_empty()).collect();

    if info.db_type == "redis" && !items.is_empty() {
        let capped: Vec<&str> = items.into_iter().take(500).collect();
        let mut types = serde_json::Map::new();
        for key in &capped {
            let type_cmd = vec![
                "redis-cli".to_string(),
                "-a".to_string(),
                info.password.clone(),
                "--no-auth-warning".to_string(),
                "TYPE".to_string(),
                key.to_string(),
            ];
            let type_out = state
                .docker
                .exec_in_container(&info.container_name, &type_cmd)
                .await
                .unwrap_or_default();
            types.insert(
                key.to_string(),
                serde_json::Value::String(type_out.trim().to_string()),
            );
        }
        return Ok(Json(serde_json::json!({ "data": capped, "types": types })));
    }

    Ok(Json(serde_json::json!({ "data": items })))
}

async fn execute_query(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<QueryBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let query = body.query.trim();
    let info = get_db_info(&state, &id).await?;

    match info.db_type.as_str() {
        "postgres" | "mysql" => execute_sql_query(&state, &info, query).await,
        "mongo" => execute_mongo_query(&state, &info, query).await,
        "redis" => execute_redis_query(&state, &info, query).await,
        _ => Err(ApiError::BadRequest("Unsupported database type".into())),
    }
}

async fn execute_sql_query(
    state: &AppState,
    info: &DbInfo,
    query: &str,
) -> Result<Json<serde_json::Value>, ApiError> {
    let lower = query.to_lowercase();
    if !lower.starts_with("select") {
        return Err(ApiError::BadRequest(
            "Only SELECT queries are allowed".into(),
        ));
    }
    let limited = if lower.contains("limit") {
        query.to_string()
    } else {
        format!("{query} LIMIT 100")
    };

    let cmd: Vec<String> = match info.db_type.as_str() {
        "postgres" => vec![
            "psql".into(),
            info.connection_string
                .replace(&info.container_name, "localhost"),
            "--csv".into(),
            "-c".into(),
            limited,
        ],
        _ => vec![
            "mysql".into(),
            format!("-u{}", info.user),
            format!("-p{}", info.password),
            "-e".into(),
            limited,
            "--batch".into(),
        ],
    };

    let output = state
        .docker
        .exec_in_container(&info.container_name, &cmd)
        .await
        .map_err(|e| ApiError::Internal(Box::new(e)))?;

    let sep = if info.db_type == "postgres" {
        ','
    } else {
        '\t'
    };
    let mut lines = output.lines();
    let headers: Vec<String> = match lines.next() {
        Some(h) => h.split(sep).map(|s| s.trim().to_string()).collect(),
        None => {
            return Ok(Json(
                serde_json::json!({ "columns": [], "rows": [], "row_count": 0 }),
            ))
        }
    };
    let rows: Vec<Vec<String>> = lines
        .filter(|l| !l.is_empty())
        .map(|l| l.split(sep).map(|s| s.trim().to_string()).collect())
        .collect();

    Ok(Json(
        serde_json::json!({ "columns": headers, "rows": rows, "row_count": rows.len() }),
    ))
}

async fn execute_mongo_query(
    state: &AppState,
    info: &DbInfo,
    query: &str,
) -> Result<Json<serde_json::Value>, ApiError> {
    let blocked = [
        "insert",
        "update",
        "delete",
        "drop",
        "remove",
        "replace",
        "bulkwrite",
        "createindex",
        "createcollection",
    ];
    let lower = query.to_lowercase();
    if blocked.iter().any(|b| lower.contains(b)) {
        return Err(ApiError::BadRequest(
            "Only read operations are allowed".into(),
        ));
    }

    let eval_expr = format!(
        "JSON.stringify(({}).toArray ? ({}).toArray() : [{}])",
        query, query, query
    );
    let conn = format!(
        "mongodb://{}:{}@localhost:27017/{}?authSource=admin",
        info.user, info.password, info.user
    );
    let cmd = vec![
        "mongosh".to_string(),
        "--quiet".into(),
        conn,
        "--eval".into(),
        eval_expr,
    ];

    let output = state
        .docker
        .exec_in_container(&info.container_name, &cmd)
        .await
        .map_err(|e| ApiError::Internal(Box::new(e)))?;

    let trimmed = output.trim();
    let docs: Vec<serde_json::Value> = serde_json::from_str(trimmed)
        .unwrap_or_else(|_| vec![serde_json::json!({ "result": trimmed })]);

    Ok(Json(
        serde_json::json!({ "documents": docs, "row_count": docs.len() }),
    ))
}

const REDIS_ALLOWED: &[&str] = &[
    "get",
    "mget",
    "hgetall",
    "hget",
    "hkeys",
    "hvals",
    "hlen",
    "lrange",
    "llen",
    "lindex",
    "smembers",
    "scard",
    "sismember",
    "zrange",
    "zcard",
    "zscore",
    "zrangebyscore",
    "type",
    "ttl",
    "pttl",
    "strlen",
    "exists",
    "dbsize",
    "info",
    "keys",
    "scan",
];

async fn execute_redis_query(
    state: &AppState,
    info: &DbInfo,
    query: &str,
) -> Result<Json<serde_json::Value>, ApiError> {
    let parts: Vec<&str> = query.split_whitespace().collect();
    if parts.is_empty() {
        return Err(ApiError::BadRequest("Empty query".into()));
    }
    let verb = parts[0].to_lowercase();
    if !REDIS_ALLOWED.contains(&verb.as_str()) {
        return Err(ApiError::BadRequest(format!(
            "Command '{}' is not allowed. Only read commands are permitted.",
            parts[0]
        )));
    }

    let mut cmd = vec![
        "redis-cli".to_string(),
        "-a".into(),
        info.password.clone(),
        "--no-auth-warning".into(),
    ];
    cmd.extend(parts.iter().map(std::string::ToString::to_string));

    let output = state
        .docker
        .exec_in_container(&info.container_name, &cmd)
        .await
        .map_err(|e| ApiError::Internal(Box::new(e)))?;

    let trimmed = output.trim();
    let lines: Vec<&str> = trimmed.lines().filter(|l| !l.is_empty()).collect();

    let (columns, rows): (Vec<String>, Vec<Vec<String>>) = match verb.as_str() {
        "get" | "strlen" | "ttl" | "pttl" | "type" | "exists" | "dbsize" | "llen" | "scard"
        | "zcard" => {
            let key = parts.get(1).unwrap_or(&"");
            (
                vec!["key".into(), "value".into()],
                vec![vec![key.to_string(), trimmed.to_string()]],
            )
        }
        "hgetall" => {
            let mut pairs = Vec::new();
            let mut i = 0;
            while i + 1 < lines.len() {
                pairs.push(vec![lines[i].to_string(), lines[i + 1].to_string()]);
                i += 2;
            }
            (vec!["field".into(), "value".into()], pairs)
        }
        "hget" | "zscore" => {
            let field = parts.get(2).unwrap_or(&"");
            (
                vec!["field".into(), "value".into()],
                vec![vec![field.to_string(), trimmed.to_string()]],
            )
        }
        "hkeys" | "hvals" | "smembers" => (
            vec!["value".into()],
            lines.iter().map(|l| vec![l.to_string()]).collect(),
        ),
        "lrange" => (
            vec!["index".into(), "value".into()],
            lines
                .iter()
                .enumerate()
                .map(|(i, l)| vec![i.to_string(), l.to_string()])
                .collect(),
        ),
        "zrange" => {
            if query.to_lowercase().contains("withscores") {
                let mut pairs = Vec::new();
                let mut i = 0;
                while i + 1 < lines.len() {
                    pairs.push(vec![lines[i].to_string(), lines[i + 1].to_string()]);
                    i += 2;
                }
                (vec!["member".into(), "score".into()], pairs)
            } else {
                (
                    vec!["member".into()],
                    lines.iter().map(|l| vec![l.to_string()]).collect(),
                )
            }
        }
        "mget" | "keys" | "scan" => (
            vec!["value".into()],
            lines.iter().map(|l| vec![l.to_string()]).collect(),
        ),
        "info" => {
            let mut pairs = Vec::new();
            for line in &lines {
                if line.starts_with('#') || line.is_empty() {
                    continue;
                }
                if let Some((k, v)) = line.split_once(':') {
                    pairs.push(vec![k.to_string(), v.to_string()]);
                }
            }
            (vec!["key".into(), "value".into()], pairs)
        }
        _ => (
            vec!["output".into()],
            lines.iter().map(|l| vec![l.to_string()]).collect(),
        ),
    };

    Ok(Json(
        serde_json::json!({ "columns": columns, "rows": rows, "row_count": rows.len() }),
    ))
}
