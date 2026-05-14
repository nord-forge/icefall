pub mod git;
pub mod orchestrator;

// Re-export shared build types and modules from icefall-common
pub use icefall_common::build::{
    detect, dockerfile, AstroMode, BuildConfig, BuildResult, BuildStep, BuildStepStatus,
    DetectError, DetectionResult, Framework, PackageManager,
};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum BuildError {
    #[error("git clone failed: {0}")]
    GitClone(String),
    #[error("git checkout failed: {0}")]
    GitCheckout(String),
    #[error("framework detection failed: {0}")]
    Detection(String),
    #[error("dockerfile generation failed: {0}")]
    DockerfileGeneration(String),
    #[error("docker build failed: {0}")]
    DockerBuild(String),
    #[error("build timeout after {0} seconds")]
    Timeout(u64),
    #[error("docker API error: {0}")]
    Docker(#[from] crate::docker::DockerError),
    #[error("database error: {0}")]
    Database(#[from] crate::db::DbError),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Detect(#[from] DetectError),
    #[error("build cancelled")]
    Cancelled,
}
