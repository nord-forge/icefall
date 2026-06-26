pub mod compose_audit;
pub mod detect;
pub mod dockerfile;

use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum DetectError {
    #[error("framework detection failed: {0}")]
    Detection(String),
    #[error("dockerfile generation failed: {0}")]
    DockerfileGeneration(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Framework {
    Dockerfile,
    Astro,
    NextJs,
    Nuxt,
    ViteReact,
    ViteVue,
    NodeApp,
    StaticSite,
}

impl std::fmt::Display for Framework {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Dockerfile => write!(f, "dockerfile"),
            Self::Astro => write!(f, "astro"),
            Self::NextJs => write!(f, "next-js"),
            Self::Nuxt => write!(f, "nuxt"),
            Self::ViteReact => write!(f, "vite-react"),
            Self::ViteVue => write!(f, "vite-vue"),
            Self::NodeApp => write!(f, "node-app"),
            Self::StaticSite => write!(f, "static-site"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PackageManager {
    Npm,
    Yarn,
    Pnpm,
    Bun,
}

impl std::fmt::Display for PackageManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Npm => write!(f, "npm"),
            Self::Yarn => write!(f, "yarn"),
            Self::Pnpm => write!(f, "pnpm"),
            Self::Bun => write!(f, "bun"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AstroMode {
    Static,
    Ssr,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionResult {
    pub framework: Framework,
    pub package_manager: PackageManager,
    pub node_version: String,
    pub build_command: Option<String>,
    pub output_dir: Option<String>,
    pub start_command: Option<String>,
    pub detected_port: u16,
    pub astro_mode: Option<AstroMode>,
    /// True when the project uses Yarn 2+ (Berry). Berry lockfiles can't be read
    /// by the image's bundled Yarn 1, and the install flag differs (`--immutable`
    /// vs `--frozen-lockfile`), so the Dockerfile must enable Corepack first.
    #[serde(default)]
    pub yarn_berry: bool,
}

/// Repo-shape hints surfaced to the create wizard alongside `DetectionResult`.
/// Kept separate from `DetectionResult` so the core detection contract stays
/// stable; these drive "how should this deploy" resolution (AC1/AC2/AC3).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepoHints {
    /// Dockerfile names found at the probed directory. Includes the plain
    /// `Dockerfile` and any `Dockerfile.*` variants (e.g. `Dockerfile.api`).
    pub dockerfiles: Vec<String>,
    /// True when a plain `Dockerfile` exists (an unambiguous single build
    /// target). When false but `dockerfiles` is non-empty, the variants are
    /// ambiguous and the wizard must ask which to use.
    pub has_plain_dockerfile: bool,
    /// Compose files found at the probed directory (`docker-compose.yml`,
    /// `docker-compose.*.yml`, `compose.yaml`, `compose.yml`). When non-empty
    /// the wizard offers to deploy as a Compose stack instead (AC1).
    pub compose_files: Vec<String>,
    /// True when the probed directory looks like a workspace monorepo with no
    /// deployable app at its root — root `package.json` declares `workspaces`
    /// and framework detection fell through to a bare static site. The wizard
    /// must require a `base_directory` selection rather than shipping the repo
    /// root as a static site (AC3 guardrail).
    pub is_monorepo: bool,
    /// Workspace directories resolved from the root `package.json`
    /// `workspaces` globs (e.g. `packages/api`, `packages/web`). Empty when not
    /// a monorepo or when no workspace dirs exist on disk.
    pub workspaces: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BuildConfig {
    pub framework: Option<Framework>,
    pub package_manager: Option<PackageManager>,
    pub node_version: Option<String>,
    pub build_command: Option<String>,
    pub output_dir: Option<String>,
    pub start_command: Option<String>,
    pub port: Option<u16>,
    pub base_image: Option<String>,
    pub build_timeout_secs: Option<u64>,
    pub keep_images: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildStep {
    pub name: String,
    pub status: BuildStepStatus,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub output: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BuildStepStatus {
    Pending,
    Running,
    Done,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildResult {
    pub image_ref: String,
    pub image_tags: Vec<String>,
    pub detection: DetectionResult,
    pub steps: Vec<BuildStep>,
    pub total_duration_secs: f64,
}
