pub mod apply;
pub mod discovery;
pub mod download;
pub mod keys;
pub mod manifest;
pub mod rollback;
pub mod scheduler;
pub mod verify;

use thiserror::Error;

pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const BUILD_COMMIT: &str = env!("ICEFALL_GIT_COMMIT");
pub const BUILD_TARGET: &str = env!("ICEFALL_TARGET_TRIPLE");
pub const BUILD_DATE: &str = env!("ICEFALL_BUILD_DATE");

pub fn artifact_target() -> &'static str {
    match BUILD_TARGET {
        t if t.contains("x86_64") && t.contains("linux") => "x86_64-linux",
        t if t.contains("aarch64") && t.contains("linux") => "aarch64-linux",
        other => other,
    }
}

/// GitHub repository ("org/icefall") to query for releases.
pub fn github_repo() -> String {
    "nord-forge/icefall".to_string()
}

#[derive(Debug, Error)]
pub enum UpdateError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("verification failed: {0}")]
    Verify(#[from] verify::VerifyError),
    #[error("no update available")]
    NoUpdate,
    #[error("insufficient disk space: need {needed} bytes, have {available} bytes")]
    DiskSpace { needed: u64, available: u64 },
    #[error("download failed: {0}")]
    Download(String),
    #[error("extraction failed: {0}")]
    Extract(String),
    #[error("no artifact for target {0}")]
    NoArtifact(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("database error: {0}")]
    Db(String),
    #[error("update apply failed: {0}")]
    Apply(String),
    #[error("rollback failed: {0}")]
    Rollback(String),
}
