//! Provisioning of least-privilege, read-only database accounts — defense in depth for the
//! query browser. Stored in the encrypted `credentials` JSON under a `readonly` key.

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

/// Provision a read-only account in a running container: polls readiness then runs idempotent
/// setup. `None` for engines without support; errors surface rather than fall back to admin.
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

/// Poll the container until the database accepts a command, up to `READINESS_TIMEOUT` —
/// a freshly-started DB isn't immediately ready. Probes with the engine's own client.
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

/// Return the read-only credentials for a managed database, lazily provisioning them
/// on first use (persisted, so done at most once). `None` for engines without one.
/// Probe whether the given read-only creds can authenticate, by running a no-op
/// query as that user inside the container. Engines without a RO account return
/// true (nothing to validate). Any failure → false, triggering re-provisioning.
async fn readonly_creds_work(
    state: &AppState,
    container_name: &str,
    db_type: &str,
    user: &str,
    password: &str,
) -> bool {
    let probe: Vec<String> = match db_type {
        "postgres" => vec![
            "psql".into(),
            format!("postgresql://{user}:{password}@localhost:5432/{user}"),
            "-tAc".into(),
            "SELECT 1".into(),
        ],
        "mysql" | "mariadb" => vec![
            "mysql".into(),
            format!("-u{user}"),
            format!("-p{password}"),
            "-e".into(),
            "SELECT 1".into(),
        ],
        // No read-only account for other engines — nothing to validate.
        _ => return true,
    };
    state
        .docker
        .exec_in_container(container_name, &probe)
        .await
        .is_ok()
}

pub(crate) async fn ensure_readonly_user(
    state: &AppState,
    db: &ManagedDatabase,
) -> Result<Option<ReadonlyCreds>, ApiError> {
    let mut creds: Value = serde_json::from_str(&db.credentials).unwrap_or_default();

    let cached_container = creds
        .get("host")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();

    // Already provisioned — return the stored read-only credentials, but only if
    // they still actually authenticate. They can go stale if the container was
    // recreated (wiping users) or provisioning previously cached creds for a user
    // that was never created. In that case, fall through and re-provision.
    if let Some(ro) = creds.get("readonly") {
        if let (Some(user), Some(password)) = (
            ro.get("user").and_then(Value::as_str),
            ro.get("password").and_then(Value::as_str),
        ) {
            if cached_container.is_empty()
                || readonly_creds_work(state, &cached_container, &db.db_type, user, password).await
            {
                return Ok(Some(ReadonlyCreds {
                    user: user.to_string(),
                    password: password.to_string(),
                }));
            }
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

    // Validate before caching. exec_in_container reports Ok as long as the client
    // process ran, even if the CREATE USER/GRANT SQL failed inside the engine
    // (e.g. the admin user lacks CREATE USER privilege). Caching phantom creds
    // here is what produced the silent "Access denied for 'icefall_ro'" — so we
    // only persist creds we've confirmed actually authenticate.
    if !readonly_creds_work(state, &container_name, &db.db_type, &ro.user, &ro.password).await {
        return Err(ApiError::Internal(
            "read-only database user could not be provisioned (admin lacks privileges?)".into(),
        ));
    }

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
