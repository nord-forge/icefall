//! Provisioning of least-privilege, read-only database accounts.
//!
//! The database browser executes user-supplied queries. Even with query
//! validation, connecting as the database's admin/root account means a
//! validation bypass is catastrophic (audit C3/C4). Defense in depth: each
//! managed database gets a second, SELECT/read-only account, and the browser
//! connects as that. The read-only credentials are stored alongside the
//! primary ones in the encrypted `credentials` JSON under a `readonly` key.

use std::time::Duration;

use serde_json::Value;

use crate::api::error::ApiError;
use crate::api::AppState;
use crate::db::models::ManagedDatabase;

use super::config::{db_configs, generate_password, READONLY_USER};

/// How long to wait for a freshly-started database container to accept
/// connections before giving up on read-only-user provisioning.
const READINESS_TIMEOUT: Duration = Duration::from_secs(30);
const READINESS_INTERVAL: Duration = Duration::from_millis(1000);

/// Read-only credentials for connecting the db_browser.
pub(crate) struct ReadonlyCreds {
    pub user: String,
    pub password: String,
}

/// Provision a read-only account inside an already-running database
/// container. Polls for readiness, then runs the engine's idempotent setup
/// commands. Returns the read-only credentials, or `None` for engines that
/// have no read-only-user setup (redis &c. — guarded by a verb allowlist).
///
/// Errors are surfaced to the caller: a database created without a working
/// read-only account would otherwise silently fall back to admin access.
pub(super) async fn provision_readonly_user(
    state: &AppState,
    container_name: &str,
    db_type: &str,
    admin_user: &str,
    admin_password: &str,
    db_name: &str,
) -> Result<Option<ReadonlyCreds>, ApiError> {
    let configs = db_configs();
    let Some(type_config) = configs.get(db_type) else {
        return Ok(None);
    };
    let Some(setup) = type_config.readonly_setup else {
        return Ok(None);
    };

    let ro_password = generate_password();
    let commands = setup(
        admin_user,
        admin_password,
        READONLY_USER,
        &ro_password,
        db_name,
    );

    wait_for_ready(state, container_name, &commands).await?;

    for cmd in &commands {
        state
            .docker
            .exec_in_container(container_name, cmd)
            .await
            .map_err(|e| ApiError::Internal(format!("read-only user setup failed: {e}").into()))?;
    }

    Ok(Some(ReadonlyCreds {
        user: READONLY_USER.to_string(),
        password: ro_password,
    }))
}

/// Poll the container until the database accepts a command, up to
/// `READINESS_TIMEOUT`. A freshly-started DB container is not immediately
/// ready; running setup against it would fail. We probe with the engine's
/// own client (the first setup command's program) doing a trivial check.
async fn wait_for_ready(
    state: &AppState,
    container_name: &str,
    commands: &[Vec<String>],
) -> Result<(), ApiError> {
    // The probe is the engine client name from the first setup command,
    // e.g. "psql"/"mysql"/"mongosh", invoked with a no-op.
    let Some(client) = commands.first().and_then(|c| c.first()) else {
        return Ok(());
    };
    let probe: Vec<String> = match client.as_str() {
        "psql" => vec!["pg_isready".into()],
        "mysql" => vec!["mysqladmin".into(), "ping".into()],
        "mongosh" => vec![
            "mongosh".into(),
            "--quiet".into(),
            "--eval".into(),
            "db.runCommand({ ping: 1 })".into(),
        ],
        _ => return Ok(()),
    };

    let deadline = std::time::Instant::now() + READINESS_TIMEOUT;
    loop {
        if state
            .docker
            .exec_in_container(container_name, &probe)
            .await
            .is_ok()
        {
            return Ok(());
        }
        if std::time::Instant::now() >= deadline {
            return Err(ApiError::ServiceUnavailable(
                "database container did not become ready in time".into(),
            ));
        }
        tokio::time::sleep(READINESS_INTERVAL).await;
    }
}

/// Return the read-only credentials for a managed database, provisioning
/// them on first use if the database predates read-only support (lazy
/// migration). Returns `None` for engines without a read-only account.
///
/// On success the merged credentials JSON is persisted so the work is done
/// at most once per database.
pub(crate) async fn ensure_readonly_user(
    state: &AppState,
    db: &ManagedDatabase,
) -> Result<Option<ReadonlyCreds>, ApiError> {
    let mut creds: Value = serde_json::from_str(&db.credentials).unwrap_or_default();

    // Already provisioned — return the stored read-only credentials.
    if let Some(ro) = creds.get("readonly") {
        if let (Some(user), Some(password)) = (
            ro.get("user").and_then(Value::as_str),
            ro.get("password").and_then(Value::as_str),
        ) {
            return Ok(Some(ReadonlyCreds {
                user: user.to_string(),
                password: password.to_string(),
            }));
        }
    }

    let container_name = creds
        .get("host")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let admin_user = creds
        .get("user")
        .and_then(Value::as_str)
        .unwrap_or("icefall")
        .to_string();
    let admin_password = creds
        .get("password")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    if container_name.is_empty() || admin_password.is_empty() {
        return Ok(None);
    }

    let Some(ro) = provision_readonly_user(
        state,
        &container_name,
        &db.db_type,
        &admin_user,
        &admin_password,
        &db.name,
    )
    .await?
    else {
        return Ok(None);
    };

    // Persist the merged credentials so this runs at most once per database.
    if let Some(obj) = creds.as_object_mut() {
        obj.insert(
            "readonly".to_string(),
            serde_json::json!({ "user": ro.user, "password": ro.password }),
        );
    }
    let container_id = db.container_id.clone().unwrap_or_default();
    state
        .db
        .update_managed_db_credentials(&db.id, &creds.to_string(), &container_id)
        .await?;

    Ok(Some(ro))
}
