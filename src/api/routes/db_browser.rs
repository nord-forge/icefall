use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;

use crate::api::error::ApiError;
use crate::api::team_auth::TeamCtx;
use crate::api::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/databases/{id}/tables", get(list_tables))
        .route("/databases/{id}/query", post(execute_query))
        .route(
            "/databases/{id}/mongo-query",
            post(execute_mongo_query_route),
        )
}

#[derive(Deserialize)]
struct QueryBody {
    #[serde(alias = "sql")]
    query: String,
}

/// Connection details for the database browser.
/// `user`/`password` are the *least-privileged* credentials available — the
/// read-only account when one has been provisioned (C3/C4), otherwise the
/// primary account. The browser always connects with these.
struct DbInfo {
    container_name: String,
    db_type: String,
    user: String,
    password: String,
    connection_string: String,
    /// The managed database's own name (used to build connection URIs).
    db_name: String,
}

async fn get_db_info(state: &AppState, team_id: &str, id: &str) -> Result<DbInfo, ApiError> {
    // H6: only resolve the database if it belongs to the caller's team.
    let db = state
        .db
        .get_managed_db_for_team(team_id, id)
        .await?
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
    let admin_user = creds
        .get("user")
        .and_then(|v| v.as_str())
        .unwrap_or("icefall")
        .to_string();
    let admin_password = creds
        .get("password")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    // C3/C4: connect as the read-only account. ensure_readonly_user
    // provisions it on first use for databases created before read-only
    // support (lazy migration). Engines without a read-only account
    // (redis &c.) fall back to the primary user — they are guarded by a
    // command allowlist instead.
    let (user, password) =
        match crate::api::routes::databases::readonly::ensure_readonly_user(state, &db).await? {
            Some(ro) => (ro.user, ro.password),
            None => (admin_user, admin_password),
        };

    Ok(DbInfo {
        container_name,
        db_type: db.db_type.clone(),
        user,
        password,
        connection_string: conn_str,
        db_name: db.name.clone(),
    })
}

/// Rebuild a connection string so it targets `localhost` (the exec runs
/// inside the container) and uses the given credentials — never the
/// admin/root account baked into the stored connection string.
fn local_conn_string(info: &DbInfo) -> String {
    match info.db_type.as_str() {
        "postgres" => format!(
            "postgresql://{}:{}@localhost:5432/{}",
            info.user, info.password, info.user
        ),
        "mysql" | "mariadb" => format!(
            "mysql://{}:{}@localhost:3306/{}",
            info.user, info.password, info.db_name
        ),
        _ => info
            .connection_string
            .replace(&info.container_name, "localhost"),
    }
}

/// Build a mongodb:// URI for the browser's (read-only) account. The
/// read-only user is created on the managed database, so that is its
/// `authSource`; the primary account authenticates against `admin`.
fn mongo_conn_string(info: &DbInfo) -> String {
    let auth_source = if info.user == super::databases::READONLY_USER {
        info.db_name.as_str()
    } else {
        "admin"
    };
    format!(
        "mongodb://{}:{}@localhost:27017/{}?authSource={auth_source}",
        info.user, info.password, info.db_name
    )
}

async fn list_tables(
    State(state): State<AppState>,
    ctx: TeamCtx,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // H6: get_db_info scopes the database to the caller's team.
    let info = get_db_info(&state, &ctx.team_id, &id).await?;

    let cmd: Vec<String> = match info.db_type.as_str() {
        "postgres" => vec![
            "psql".into(), local_conn_string(&info),
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
            mongo_conn_string(&info),
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
        .map_err(ApiError::internal)?;

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
    ctx: TeamCtx,
    Path(id): Path<String>,
    Json(body): Json<QueryBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let query = body.query.trim();
    // H6: get_db_info scopes the database to the caller's team.
    let info = get_db_info(&state, &ctx.team_id, &id).await?;

    match info.db_type.as_str() {
        "postgres" | "mysql" => execute_sql_query(&state, &info, query).await,
        // MongoDB uses the structured /mongo-query endpoint — raw query
        // strings are no longer accepted (audit C3).
        "mongo" => Err(ApiError::BadRequest(
            "Use the /mongo-query endpoint for MongoDB databases".into(),
        )),
        "redis" => execute_redis_query(&state, &info, query).await,
        _ => Err(ApiError::BadRequest("Unsupported database type".into())),
    }
}

/// Tokens that have no place in a read-only browser query. Even though the
/// browser connects as a read-only account, these are rejected at the app
/// layer too (defense in depth): file I/O, program execution, and
/// engine-specific escapes (audit C4).
const SQL_FORBIDDEN: &[&str] = &[
    "into outfile",
    "into dumpfile",
    "load_file",
    "load data",
    "copy ",
    "pg_read_file",
    "pg_ls_dir",
    "pg_sleep",
    "dblink",
    "lo_import",
    "lo_export",
];

/// Validate a browser SQL query: a single read-only statement, no file or
/// program access. Returns the query with a `LIMIT` appended if absent.
fn validate_sql_query(query: &str) -> Result<String, ApiError> {
    let trimmed = query.trim().trim_end_matches(';');
    let lower = trimmed.to_lowercase();

    // Exactly one statement — `SELECT 1; DROP TABLE users` must not pass.
    // psql/mysql -e both execute stacked statements, so reject any
    // embedded semicolon.
    if trimmed.contains(';') {
        return Err(ApiError::BadRequest(
            "Only a single statement is allowed".into(),
        ));
    }

    // Read-only: SELECT, or a WITH ... SELECT CTE.
    if !(lower.starts_with("select") || lower.starts_with("with")) {
        return Err(ApiError::BadRequest(
            "Only SELECT (or WITH ... SELECT) queries are allowed".into(),
        ));
    }

    // psql meta-commands (\copy, \!, ...) bypass SQL entirely.
    if trimmed.contains('\\') {
        return Err(ApiError::BadRequest(
            "Backslash commands are not allowed".into(),
        ));
    }

    for needle in SQL_FORBIDDEN {
        if lower.contains(needle) {
            return Err(ApiError::BadRequest(format!(
                "Query contains a disallowed operation: '{}'",
                needle.trim()
            )));
        }
    }

    if lower.contains("limit") {
        Ok(trimmed.to_string())
    } else {
        Ok(format!("{trimmed} LIMIT 100"))
    }
}

async fn execute_sql_query(
    state: &AppState,
    info: &DbInfo,
    query: &str,
) -> Result<Json<serde_json::Value>, ApiError> {
    let limited = validate_sql_query(query)?;

    // Connect as the least-privileged account (read-only when provisioned).
    let cmd: Vec<String> = match info.db_type.as_str() {
        "postgres" => vec![
            "psql".into(),
            local_conn_string(info),
            "--csv".into(),
            "-c".into(),
            limited,
        ],
        _ => vec![
            "mysql".into(),
            format!("-u{}", info.user),
            format!("-p{}", info.password),
            info.db_name.clone(),
            "-e".into(),
            limited,
            "--batch".into(),
        ],
    };

    let output = state
        .docker
        .exec_in_container(&info.container_name, &cmd)
        .await
        .map_err(ApiError::internal)?;

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

/// Structured MongoDB query (audit C3).
///
/// The old browser passed the user's raw input straight into
/// `mongosh --eval`, which is JavaScript — `require('child_process')` and
/// friends meant full RCE on the database container. There is no safe way to
/// eval attacker JS. Instead the client now sends a *structured* query:
/// validated parts that are JSON-serialized (JSON is a safe JS literal) into
/// a fixed `find()` expression. No user-controlled JavaScript is executed.
#[derive(Deserialize)]
struct MongoQueryBody {
    collection: String,
    #[serde(default)]
    filter: serde_json::Value,
    #[serde(default)]
    projection: Option<serde_json::Value>,
    #[serde(default)]
    sort: Option<serde_json::Value>,
    #[serde(default)]
    limit: Option<i64>,
}

/// Recursively reject a filter that smuggles server-side JavaScript via the
/// `$where` or `$function` operators — those run JS even inside a `find()`.
fn filter_has_js_operator(v: &serde_json::Value) -> bool {
    match v {
        serde_json::Value::Object(map) => map
            .iter()
            .any(|(k, val)| k == "$where" || k == "$function" || filter_has_js_operator(val)),
        serde_json::Value::Array(items) => items.iter().any(filter_has_js_operator),
        _ => false,
    }
}

async fn execute_mongo_query_route(
    State(state): State<AppState>,
    ctx: TeamCtx,
    Path(id): Path<String>,
    Json(body): Json<MongoQueryBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // H6: get_db_info scopes the database to the caller's team.
    let info = get_db_info(&state, &ctx.team_id, &id).await?;
    if info.db_type != "mongo" {
        return Err(ApiError::BadRequest(
            "This endpoint is for MongoDB databases only".into(),
        ));
    }
    execute_mongo_query(&state, &info, &body).await
}

async fn execute_mongo_query(
    state: &AppState,
    info: &DbInfo,
    body: &MongoQueryBody,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Collection is the one identifier that cannot be a JSON literal — it is
    // a bare name in the expression, so it is strictly validated instead.
    if body.collection.is_empty()
        || !body
            .collection
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
    {
        return Err(ApiError::BadRequest("Invalid collection name".into()));
    }

    if filter_has_js_operator(&body.filter) {
        return Err(ApiError::BadRequest(
            "The $where and $function operators are not allowed".into(),
        ));
    }

    // Each part is JSON — a valid, inert JS literal that cannot break out of
    // the expression. The limit is clamped to a sane range.
    let filter_json = serde_json::to_string(&body.filter).unwrap_or_else(|_| "{}".into());
    let projection_json = body
        .projection
        .as_ref()
        .map(|p| serde_json::to_string(p).unwrap_or_else(|_| "{}".into()))
        .unwrap_or_else(|| "{}".into());
    let sort_json = body
        .sort
        .as_ref()
        .map(|s| serde_json::to_string(s).unwrap_or_else(|_| "{}".into()))
        .unwrap_or_else(|| "{}".into());
    let limit = body.limit.unwrap_or(100).clamp(1, 500);

    let eval_expr = format!(
        "JSON.stringify(db.getCollection({collection}).find({filter}, {projection})\
         .sort({sort}).limit({limit}).toArray())",
        collection = serde_json::Value::String(body.collection.clone()),
        filter = filter_json,
        projection = projection_json,
        sort = sort_json,
        limit = limit,
    );

    let cmd = vec![
        "mongosh".to_string(),
        "--quiet".into(),
        mongo_conn_string(info),
        "--eval".into(),
        eval_expr,
    ];

    let output = state
        .docker
        .exec_in_container(&info.container_name, &cmd)
        .await
        .map_err(ApiError::internal)?;

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
        .map_err(ApiError::internal)?;

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

#[cfg(test)]
mod tests {
    use super::*;

    // --- SQL query validation (audit C4) ---

    #[test]
    fn plain_select_is_accepted_and_gets_a_limit() {
        let q = validate_sql_query("SELECT * FROM users").unwrap();
        assert!(q.to_lowercase().contains("limit 100"));
    }

    #[test]
    fn with_cte_is_accepted() {
        assert!(validate_sql_query("WITH x AS (SELECT 1) SELECT * FROM x").is_ok());
    }

    #[test]
    fn stacked_statements_are_rejected() {
        // The classic injection: a benign SELECT then a destructive stmt.
        assert!(validate_sql_query("SELECT 1; DROP TABLE users").is_err());
        assert!(validate_sql_query("SELECT 1; DELETE FROM users").is_err());
    }

    #[test]
    fn non_select_is_rejected() {
        assert!(validate_sql_query("DROP TABLE users").is_err());
        assert!(validate_sql_query("UPDATE users SET admin = 1").is_err());
    }

    #[test]
    fn file_access_is_rejected() {
        assert!(validate_sql_query("SELECT * INTO OUTFILE '/tmp/x' FROM users").is_err());
        assert!(validate_sql_query("SELECT load_file('/etc/passwd')").is_err());
        assert!(validate_sql_query("SELECT pg_read_file('/etc/passwd')").is_err());
    }

    #[test]
    fn psql_meta_commands_are_rejected() {
        assert!(validate_sql_query("SELECT 1 \\! sh").is_err());
    }

    // --- Mongo structured query (audit C3) ---

    #[test]
    fn where_operator_in_filter_is_detected() {
        let f = serde_json::json!({ "$where": "sleep(1000)" });
        assert!(filter_has_js_operator(&f));
    }

    #[test]
    fn nested_where_operator_is_detected() {
        let f = serde_json::json!({ "a": { "b": { "$function": "x" } } });
        assert!(filter_has_js_operator(&f));
        let in_array = serde_json::json!({ "$or": [{ "$where": "1" }] });
        assert!(filter_has_js_operator(&in_array));
    }

    #[test]
    fn ordinary_filter_has_no_js_operator() {
        let f = serde_json::json!({ "active": true, "age": { "$gt": 18 } });
        assert!(!filter_has_js_operator(&f));
    }
}
