#[cfg(test)]
use std::path::Path;

#[cfg(test)]
use bytes::Bytes;

#[cfg(test)]
use crate::build::BuildError;
use crate::build::{BuildStep, BuildStepStatus};
use crate::db::models::now_iso8601;

pub(super) fn new_step(name: &str) -> BuildStep {
    BuildStep {
        name: name.to_string(),
        status: BuildStepStatus::Running,
        started_at: Some(now_iso8601()),
        finished_at: None,
        output: Vec::new(),
    }
}

pub(super) fn finish_step(step: &mut BuildStep, status: BuildStepStatus) {
    step.status = status;
    step.finished_at = Some(now_iso8601());
}

// Retained for the build-context archive test; the live build path now streams
// via the runtime CLI from the source directory, so this is test-only.
#[cfg(test)]
pub(super) const IGNORE_DIRS: &[&str] = &[
    "node_modules",
    ".git",
    ".next",
    ".nuxt",
    ".output",
    "target",
    ".turbo",
    ".cache",
    "coverage",
    "dist",
];

#[cfg(test)]
pub(super) fn create_build_context(project_dir: &Path) -> Result<Bytes, BuildError> {
    let buf = Vec::new();
    let encoder = flate2::write::GzEncoder::new(buf, flate2::Compression::fast());
    let mut archive = tar::Builder::new(encoder);

    fn walk_and_add(
        archive: &mut tar::Builder<flate2::write::GzEncoder<Vec<u8>>>,
        dir: &Path,
        base: &Path,
    ) -> std::io::Result<()> {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let name = entry.file_name();
            let name_str = name.to_string_lossy();

            if IGNORE_DIRS.contains(&name_str.as_ref()) {
                continue;
            }

            let relative = path.strip_prefix(base).unwrap_or(&path);
            if path.is_dir() {
                archive.append_dir(relative, &path)?;
                walk_and_add(archive, &path, base)?;
            } else {
                archive.append_path_with_name(&path, relative)?;
            }
        }
        Ok(())
    }

    walk_and_add(&mut archive, project_dir, project_dir)
        .map_err(|e| BuildError::DockerBuild(format!("failed to create tar archive: {e}")))?;

    let encoder = archive
        .into_inner()
        .map_err(|e| BuildError::DockerBuild(format!("failed to finalize tar archive: {e}")))?;
    let compressed = encoder
        .finish()
        .map_err(|e| BuildError::DockerBuild(format!("failed to compress archive: {e}")))?;

    Ok(Bytes::from(compressed))
}

pub(super) fn redact_secrets(line: &str, secrets: &[String]) -> String {
    let mut result = line.to_string();
    for secret in secrets {
        if secret.len() >= 4 {
            result = result.replace(secret, "[REDACTED]");
        }
    }
    result
}

/// Coerce an app name into a valid Docker image-name component (lowercase
/// alphanumerics and `-_.`, trimmed at the edges). Falls back to "app" if
/// nothing usable remains.
pub(super) fn sanitize_image_name(name: &str) -> String {
    let mapped: String = name
        .to_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.') {
                c
            } else {
                '-'
            }
        })
        .collect();
    let trimmed = mapped.trim_matches(|c: char| !c.is_ascii_alphanumeric());
    if trimmed.is_empty() {
        "app".to_string()
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::sanitize_image_name;

    #[test]
    fn sanitizes_bad_names() {
        assert_eq!(sanitize_image_name("My App"), "my-app");
        assert_eq!(sanitize_image_name("webgl-portfolio"), "webgl-portfolio");
        assert_eq!(sanitize_image_name("  weird:name!! "), "weird-name");
        assert_eq!(sanitize_image_name("***"), "app");
    }
}
