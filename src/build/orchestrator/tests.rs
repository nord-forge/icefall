#[cfg(test)]
mod orchestrator_tests {
    use crate::build::orchestrator::context::{create_build_context, redact_secrets};
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn creates_valid_tar_archive() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("Dockerfile"), "FROM node:22").unwrap();
        fs::write(dir.path().join("index.js"), "console.log('hi')").unwrap();
        fs::create_dir_all(dir.path().join("src")).unwrap();
        fs::write(dir.path().join("src/app.js"), "module.exports = {}").unwrap();

        let context = create_build_context(dir.path()).unwrap();
        assert!(!context.is_empty());

        let decoder = flate2::read::GzDecoder::new(&context[..]);
        let mut archive = tar::Archive::new(decoder);

        let entries: Vec<String> = archive
            .entries()
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.path().unwrap().to_string_lossy().to_string())
            .collect();

        assert!(entries.iter().any(|e| e.contains("Dockerfile")));
        assert!(entries.iter().any(|e| e.contains("index.js")));
        assert!(entries.iter().any(|e| e.contains("src/app.js")));
    }

    #[test]
    fn redacts_single_secret() {
        let line = "DATABASE_URL=postgres://user:s3cret@host/db";
        let result = redact_secrets(line, &["s3cret".to_string()]);
        assert_eq!(result, "DATABASE_URL=postgres://user:[REDACTED]@host/db");
        assert!(!result.contains("s3cret"));
    }

    #[test]
    fn redacts_multiple_secrets() {
        let line = "API_KEY=abc123 SECRET=xyz789";
        let result = redact_secrets(line, &["abc123".to_string(), "xyz789".to_string()]);
        assert_eq!(result, "API_KEY=[REDACTED] SECRET=[REDACTED]");
    }

    #[test]
    fn skips_short_secrets() {
        let line = "PORT=80";
        let result = redact_secrets(line, &["80".to_string()]);
        assert_eq!(result, "PORT=80");
    }

    #[test]
    fn handles_empty_secrets() {
        let line = "some build output";
        let result = redact_secrets(line, &[]);
        assert_eq!(result, "some build output");
    }
}
