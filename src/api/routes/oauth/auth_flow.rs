use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, HeaderValue};
use axum::response::{IntoResponse, Redirect};
use serde::Deserialize;

use crate::api::error::ApiError;
use crate::api::routes::auth::authenticate_from_headers;
use crate::api::AppState;
use crate::db::models::NewUser;

use super::pkce::{store_pkce, take_pkce, OAuthIntent};
use super::providers::{fetch_user_profile, get_client_credentials, provider_config};

mod urlencoding {
    pub fn encode(input: &str) -> String {
        let mut result = String::with_capacity(input.len() * 3);
        for byte in input.bytes() {
            match byte {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                    result.push(byte as char);
                }
                _ => {
                    result.push('%');
                    result.push_str(&format!("{:02X}", byte));
                }
            }
        }
        result
    }
}

fn build_base_url(state: &AppState) -> String {
    state.config.base_domain.as_deref().map_or_else(
        || "http://localhost:3000".to_string(),
        |d| format!("https://{d}"),
    )
}

fn generate_pkce_and_state() -> (String, String, String) {
    use base64::Engine;
    use rand::Rng;
    use sha2::Digest;

    let mut rng = rand::rng();
    let mut verifier_bytes = [0u8; 32];
    rng.fill_bytes(&mut verifier_bytes);
    let verifier = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(verifier_bytes);

    let mut hasher = sha2::Sha256::new();
    hasher.update(verifier.as_bytes());
    let challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(hasher.finalize());

    let mut state_bytes = [0u8; 32];
    rng.fill_bytes(&mut state_bytes);
    let token = hex::encode(state_bytes);
    (verifier, challenge, token)
}

fn build_authorize_url(
    state: &AppState,
    provider: &str,
    client_id: &str,
    code_challenge: &str,
    state_token: &str,
) -> Result<String, ApiError> {
    let prov_config = provider_config(provider)
        .ok_or_else(|| ApiError::BadRequest(format!("Unsupported OAuth provider: {provider}")))?;

    let base = build_base_url(state);
    let redirect_uri = format!("{base}/api/v1/auth/oauth/{provider}/callback");
    let scope = prov_config.scopes.join(" ");

    let mut params = vec![
        ("client_id", client_id),
        ("redirect_uri", redirect_uri.as_str()),
        ("scope", scope.as_str()),
        ("state", state_token),
        ("response_type", "code"),
        ("code_challenge", code_challenge),
        ("code_challenge_method", "S256"),
    ];

    if provider == "google" {
        params.push(("access_type", "offline"));
        params.push(("prompt", "consent"));
    }

    let query_string: String = params
        .iter()
        .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
        .collect::<Vec<_>>()
        .join("&");

    Ok(format!("{}?{}", prov_config.authorize_url, query_string))
}

/// GET /api/v1/auth/oauth/{provider}/authorize
/// Generates PKCE challenge, builds authorization URL, redirects to provider.
pub(super) async fn oauth_authorize(
    State(state): State<AppState>,
    Path(provider): Path<String>,
) -> Result<Redirect, ApiError> {
    let oauth_settings = state.db.get_oauth_settings().await?.ok_or_else(|| {
        ApiError::BadRequest(
            "OAuth is not configured. Ask an admin to configure OAuth settings.".into(),
        )
    })?;

    let (client_id, _) = get_client_credentials(&oauth_settings, &provider).ok_or_else(|| {
        ApiError::BadRequest(format!(
            "OAuth provider '{provider}' is not enabled or configured"
        ))
    })?;

    let (code_verifier, code_challenge, state_token) = generate_pkce_and_state();
    store_pkce(&state_token, &code_verifier, OAuthIntent::Login).await;

    let url = build_authorize_url(&state, &provider, &client_id, &code_challenge, &state_token)?;
    Ok(Redirect::temporary(&url))
}

/// GET /api/v1/auth/oauth/{provider}/link
/// Same as authorize, but marks the intent as "link" so the callback attaches
/// the identity to the currently-authenticated user instead of logging in.
pub(super) async fn oauth_link(
    State(state): State<AppState>,
    Path(provider): Path<String>,
    headers: HeaderMap,
) -> Result<Redirect, ApiError> {
    authenticate_from_headers(&state, &headers)
        .await?
        .ok_or_else(|| ApiError::Forbidden("Not authenticated".into()))?;

    let oauth_settings = state
        .db
        .get_oauth_settings()
        .await?
        .ok_or_else(|| ApiError::BadRequest("OAuth is not configured".into()))?;

    let (client_id, _) = get_client_credentials(&oauth_settings, &provider).ok_or_else(|| {
        ApiError::BadRequest(format!(
            "OAuth provider '{provider}' is not enabled or configured"
        ))
    })?;

    let (code_verifier, code_challenge, state_token) = generate_pkce_and_state();
    store_pkce(&state_token, &code_verifier, OAuthIntent::Link).await;

    let url = build_authorize_url(&state, &provider, &client_id, &code_challenge, &state_token)?;
    Ok(Redirect::temporary(&url))
}

#[derive(Deserialize)]
pub(super) struct OAuthCallbackQuery {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
}

/// GET /api/v1/auth/oauth/{provider}/callback
/// Exchanges authorization code for token, fetches user info, creates/links account.
pub(super) async fn oauth_callback(
    State(state): State<AppState>,
    Path(provider): Path<String>,
    Query(query): Query<OAuthCallbackQuery>,
    headers: HeaderMap,
) -> Result<axum::response::Response, ApiError> {
    if let Some(ref err) = query.error {
        tracing::warn!("OAuth callback error from {provider}: {err}");
        return Ok(Redirect::temporary("/login?error=oauth_denied").into_response());
    }

    let code = query
        .code
        .as_deref()
        .ok_or_else(|| ApiError::BadRequest("Missing authorization code".into()))?;

    let state_token = query
        .state
        .as_deref()
        .ok_or_else(|| ApiError::BadRequest("Missing state parameter".into()))?;

    let (code_verifier, intent) = take_pkce(state_token).await.ok_or_else(|| {
        ApiError::BadRequest("Invalid or expired OAuth state. Please try again.".into())
    })?;

    let (provider_user_id, provider_email, _) =
        exchange_and_fetch(&state, &provider, code, &code_verifier).await?;

    match intent {
        OAuthIntent::Link => {
            handle_link_callback(
                &state,
                &provider,
                &provider_user_id,
                provider_email.as_deref(),
                &headers,
            )
            .await
        }
        OAuthIntent::Login => {
            handle_login_callback(&state, &provider, &provider_user_id, provider_email).await
        }
    }
}

/// Exchange the authorization code for an access token and fetch the user profile.
async fn exchange_and_fetch(
    state: &AppState,
    provider: &str,
    code: &str,
    code_verifier: &str,
) -> Result<(String, Option<String>, Option<String>), ApiError> {
    let prov_config = provider_config(provider)
        .ok_or_else(|| ApiError::BadRequest(format!("Unsupported OAuth provider: {provider}")))?;

    let oauth_settings = state
        .db
        .get_oauth_settings()
        .await?
        .ok_or_else(|| ApiError::BadRequest("OAuth is not configured".into()))?;

    let (client_id, client_secret) =
        get_client_credentials(&oauth_settings, provider).ok_or_else(|| {
            ApiError::BadRequest(format!("OAuth provider '{provider}' is not configured"))
        })?;

    let base = build_base_url(state);
    let redirect_uri = format!("{base}/api/v1/auth/oauth/{provider}/callback");

    let http_client = reqwest::Client::new();
    let token_response = http_client
        .post(prov_config.token_url)
        .header("Accept", "application/json")
        .form(&[
            ("client_id", client_id.as_str()),
            ("client_secret", client_secret.as_str()),
            ("code", code),
            ("redirect_uri", redirect_uri.as_str()),
            ("grant_type", "authorization_code"),
            ("code_verifier", code_verifier),
        ])
        .send()
        .await
        .map_err(|e| {
            tracing::error!("OAuth token exchange failed for {provider}: {e}");
            ApiError::internal(e)
        })?;

    if !token_response.status().is_success() {
        let body = token_response.text().await.unwrap_or_default();
        tracing::error!("OAuth token exchange returned error for {provider}: {body}");
        return Err(ApiError::BadRequest("OAuth token exchange failed".into()));
    }

    let token_data: serde_json::Value = token_response.json().await.map_err(|e| {
        tracing::error!("Failed to parse OAuth token response from {provider}: {e}");
        ApiError::internal(e)
    })?;

    let access_token = token_data["access_token"].as_str().ok_or_else(|| {
        tracing::error!("No access_token in OAuth response from {provider}");
        ApiError::BadRequest("OAuth token exchange did not return an access token".into())
    })?;

    fetch_user_profile(provider, access_token, &http_client).await
}

/// Handle the callback when intent is Link: attach identity to the current user.
async fn handle_link_callback(
    state: &AppState,
    provider: &str,
    provider_user_id: &str,
    provider_email: Option<&str>,
    headers: &HeaderMap,
) -> Result<axum::response::Response, ApiError> {
    let user = authenticate_from_headers(state, headers)
        .await?
        .ok_or_else(|| ApiError::Forbidden("Not authenticated. Log in first, then link.".into()))?;

    if let Some(existing) = state
        .db
        .get_oauth_identity(provider, provider_user_id)
        .await?
    {
        if existing.user_id == user.id {
            return Ok(Redirect::temporary("/profile?linked=already").into_response());
        }
        return Ok(Redirect::temporary("/profile?error=oauth_identity_taken").into_response());
    }

    state
        .db
        .create_oauth_identity(&user.id, provider, provider_user_id, provider_email)
        .await?;

    Ok(Redirect::temporary("/profile?linked=success").into_response())
}

/// Handle the callback when intent is Login: find or create user, start session.
async fn handle_login_callback(
    state: &AppState,
    provider: &str,
    provider_user_id: &str,
    provider_email: Option<String>,
) -> Result<axum::response::Response, ApiError> {
    let user = match state
        .db
        .get_oauth_identity(provider, provider_user_id)
        .await?
    {
        // Case 1: OAuth identity exists — log in as the linked user
        Some(identity) => state
            .db
            .get_user_by_id(&identity.user_id)
            .await?
            .ok_or_else(|| {
                ApiError::internal(std::io::Error::other("Linked user account not found"))
            })?,
        // Case 2: No OAuth identity yet
        None => {
            let existing_user = if let Some(ref email) = provider_email {
                state.db.get_user_by_email(email).await?
            } else {
                None
            };

            match existing_user {
                // Case 2a: Email matches existing user — do NOT auto-link.
                // The user must log in normally and link from their profile
                // to prove they own the existing account.
                Some(_) => {
                    tracing::warn!(
                        "OAuth {provider} email matches existing account but identity is not linked; \
                         user must link from profile"
                    );
                    return Ok(
                        Redirect::temporary("/login?error=oauth_link_required").into_response()
                    );
                }
                // Case 2b: New user — create account + link identity
                None => {
                    let email = provider_email.ok_or_else(|| {
                        ApiError::BadRequest(
                            "OAuth provider did not return an email address. Cannot create account."
                                .into(),
                        )
                    })?;

                    let reg = state.db.get_registration_settings().await?;
                    if !reg.allow_registration {
                        return Ok(Redirect::temporary("/login?error=registration_disabled")
                            .into_response());
                    }
                    if let Some(ref domains) = reg.allowed_domains {
                        let allowed: Vec<&str> = domains.split(',').map(str::trim).collect();
                        let email_domain = email.rsplit('@').next().unwrap_or("");
                        if !allowed.is_empty()
                            && !allowed.iter().any(|d| d.is_empty() || *d == email_domain)
                        {
                            return Ok(Redirect::temporary("/login?error=domain_not_allowed")
                                .into_response());
                        }
                    }

                    let random_password = format!("oauth-{}", uuid::Uuid::now_v7());
                    let password_hash = hash_password_for_oauth(&random_password)?;

                    let new_user = state
                        .db
                        .create_user(&NewUser {
                            email: email.clone(),
                            password_hash,
                            role: reg.default_role.clone(),
                        })
                        .await?;

                    state
                        .db
                        .create_oauth_identity(
                            &new_user.id,
                            provider,
                            provider_user_id,
                            Some(&email),
                        )
                        .await?;

                    new_user
                }
            }
        }
    };

    // If 2FA is enabled, redirect to the 2FA challenge instead of creating a
    // session. A single-use challenge token is passed in the URL — never the
    // raw user_id, which would otherwise leak via browser history / referrer
    // (audit M7). The frontend reads `challenge` and drives /2fa/validate.
    if user.totp_enabled {
        let challenge = crate::api::routes::two_factor_challenge::issue_challenge(&user.id).await;
        return Ok(
            Redirect::temporary(&format!("/login?requires_2fa=true&challenge={challenge}"))
                .into_response(),
        );
    }

    let expires_at = (chrono::Utc::now() + chrono::Duration::days(7))
        .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

    let session = state
        .db
        .create_session(&user.id, &expires_at)
        .await
        .map_err(ApiError::internal)?;

    let cookie =
        crate::api::routes::auth::session_cookie(&session.id, state.config.base_domain.as_deref());
    let mut headers = HeaderMap::new();
    headers.insert(
        "set-cookie",
        HeaderValue::from_str(&cookie).map_err(ApiError::internal)?,
    );
    headers.insert("location", HeaderValue::from_static("/"));

    Ok((axum::http::StatusCode::TEMPORARY_REDIRECT, headers).into_response())
}

pub(super) fn hash_password_for_oauth(password: &str) -> Result<String, ApiError> {
    use argon2::{password_hash::SaltString, Argon2, PasswordHasher};
    let salt = SaltString::generate(&mut argon2::password_hash::rand_core::OsRng);
    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| ApiError::internal(std::io::Error::other(e.to_string())))?;
    Ok(hash.to_string())
}
