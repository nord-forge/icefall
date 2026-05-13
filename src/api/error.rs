use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use thiserror::Error;

use crate::build::BuildError;
use crate::caddy::CaddyError;
use crate::db::DbError;
use crate::deploy::DeployError;
use crate::docker::DockerError;

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    BadRequest(String),
    #[error("{0}")]
    Forbidden(String),
    #[error("{0}")]
    Conflict(String),
    #[error("internal error")]
    Internal(#[source] Box<dyn std::error::Error + Send + Sync>),
    #[error("{0}")]
    ServiceUnavailable(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code, message) = match &self {
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, "not_found", msg.clone()),
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, "bad_request", msg.clone()),
            ApiError::Forbidden(msg) => (StatusCode::FORBIDDEN, "forbidden", msg.clone()),
            ApiError::Conflict(msg) => (StatusCode::CONFLICT, "conflict", msg.clone()),
            ApiError::Internal(err) => {
                tracing::error!(error = %err, "internal server error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal_error",
                    "Internal server error".to_string(),
                )
            }
            ApiError::ServiceUnavailable(msg) => (
                StatusCode::SERVICE_UNAVAILABLE,
                "service_unavailable",
                msg.clone(),
            ),
        };

        let body = serde_json::json!({ "error": code, "message": message });
        (status, Json(body)).into_response()
    }
}

impl ApiError {
    pub fn internal(err: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::Internal(Box::new(err))
    }
}

impl From<std::io::Error> for ApiError {
    fn from(err: std::io::Error) -> Self {
        ApiError::internal(err)
    }
}

impl From<reqwest::Error> for ApiError {
    fn from(err: reqwest::Error) -> Self {
        ApiError::internal(err)
    }
}

impl From<DbError> for ApiError {
    fn from(err: DbError) -> Self {
        match err {
            DbError::NotFound(msg) => ApiError::NotFound(msg),
            DbError::Duplicate(msg) => ApiError::Conflict(msg),
            other => ApiError::internal(other),
        }
    }
}

impl From<DockerError> for ApiError {
    fn from(err: DockerError) -> Self {
        match err {
            DockerError::ContainerNotFound(msg) => ApiError::NotFound(msg),
            DockerError::ImageNotFound(msg) => ApiError::NotFound(msg),
            DockerError::Unavailable(msg) => ApiError::ServiceUnavailable(msg),
            other => ApiError::internal(other),
        }
    }
}

impl From<CaddyError> for ApiError {
    fn from(err: CaddyError) -> Self {
        match err {
            CaddyError::RouteNotFound(msg) => ApiError::NotFound(msg),
            CaddyError::Unreachable(msg) => ApiError::ServiceUnavailable(msg),
            other => ApiError::internal(other),
        }
    }
}

impl From<BuildError> for ApiError {
    fn from(err: BuildError) -> Self {
        ApiError::internal(err)
    }
}

impl From<DeployError> for ApiError {
    fn from(err: DeployError) -> Self {
        ApiError::internal(err)
    }
}
