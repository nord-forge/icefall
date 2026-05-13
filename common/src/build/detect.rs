use std::path::Path;

use super::{AstroMode, BuildConfig, DetectError, DetectionResult, Framework, PackageManager};

pub fn detect(
    project_dir: &Path,
    overrides: Option<&BuildConfig>,
) -> Result<DetectionResult, DetectError> {
    let framework = detect_framework(project_dir);
    let package_manager = detect_package_manager(project_dir);
    let node_version = detect_node_version(project_dir);
    let astro_mode = if framework == Framework::Astro {
        Some(detect_astro_mode(project_dir))
    } else {
        None
    };

    let (build_command, output_dir, start_command, detected_port) =
        framework_defaults(&framework, &package_manager, astro_mode.as_ref());

    let mut result = DetectionResult {
        framework,
        package_manager,
        node_version,
        build_command,
        output_dir,
        start_command,
        detected_port,
        astro_mode,
    };

    if let Some(ov) = overrides {
        apply_overrides(&mut result, ov);
    }

    Ok(result)
}

fn has_file(dir: &Path, name: &str) -> bool {
    dir.join(name).is_file()
}

fn read_file_string(dir: &Path, name: &str) -> Option<String> {
    std::fs::read_to_string(dir.join(name)).ok()
}

fn parse_package_json(dir: &Path) -> Option<serde_json::Value> {
    let content = read_file_string(dir, "package.json")?;
    serde_json::from_str(&content).ok()
}

fn has_dependency(pkg: &serde_json::Value, name: &str) -> bool {
    let check = |field: &str| {
        pkg.get(field)
            .and_then(|v| v.as_object())
            .is_some_and(|deps| deps.contains_key(name))
    };
    check("dependencies") || check("devDependencies")
}

fn detect_framework(dir: &Path) -> Framework {
    if has_file(dir, "Dockerfile") {
        return Framework::Dockerfile;
    }

    let Some(pkg) = parse_package_json(dir) else {
        if has_file(dir, "index.html") {
            return Framework::StaticSite;
        }
        return Framework::StaticSite;
    };

    if has_dependency(&pkg, "astro")
        || has_file(dir, "astro.config.mjs")
        || has_file(dir, "astro.config.ts")
        || has_file(dir, "astro.config.js")
    {
        return Framework::Astro;
    }

    if has_dependency(&pkg, "next")
        || has_file(dir, "next.config.mjs")
        || has_file(dir, "next.config.ts")
        || has_file(dir, "next.config.js")
    {
        return Framework::NextJs;
    }

    if has_dependency(&pkg, "nuxt")
        || has_file(dir, "nuxt.config.ts")
        || has_file(dir, "nuxt.config.js")
    {
        return Framework::Nuxt;
    }

    let has_vite = has_dependency(&pkg, "vite")
        || has_file(dir, "vite.config.ts")
        || has_file(dir, "vite.config.js")
        || has_file(dir, "vite.config.mts")
        || has_file(dir, "vite.config.mjs");

    if has_vite && (has_dependency(&pkg, "react") || has_dependency(&pkg, "react-dom")) {
        return Framework::ViteReact;
    }

    if has_vite && has_dependency(&pkg, "vue") {
        return Framework::ViteVue;
    }

    let has_start = pkg.get("scripts").and_then(|s| s.get("start")).is_some();
    let has_main = pkg.get("main").is_some();

    if has_start || has_main {
        return Framework::NodeApp;
    }

    Framework::StaticSite
}

fn detect_package_manager(dir: &Path) -> PackageManager {
    if has_file(dir, "bun.lock") || has_file(dir, "bun.lockb") {
        return PackageManager::Bun;
    }
    if has_file(dir, "pnpm-lock.yaml") {
        return PackageManager::Pnpm;
    }
    if has_file(dir, "yarn.lock") {
        return PackageManager::Yarn;
    }
    PackageManager::Npm
}

fn detect_node_version(dir: &Path) -> String {
    if let Some(content) = read_file_string(dir, ".nvmrc") {
        let v = content.trim().trim_start_matches('v');
        if !v.is_empty() {
            return extract_major_version(v).to_string();
        }
    }

    if let Some(content) = read_file_string(dir, ".node-version") {
        let v = content.trim().trim_start_matches('v');
        if !v.is_empty() {
            return extract_major_version(v).to_string();
        }
    }

    if let Some(pkg) = parse_package_json(dir) {
        if let Some(engines) = pkg.get("engines").and_then(|e| e.get("node")) {
            if let Some(range) = engines.as_str() {
                let major = parse_node_version_range(range);
                if !major.is_empty() {
                    return major;
                }
            }
        }
    }

    "22".to_string()
}

fn extract_major_version(version: &str) -> &str {
    version.split('.').next().unwrap_or(version)
}

fn parse_node_version_range(range: &str) -> String {
    let cleaned = range
        .trim()
        .trim_start_matches(">=")
        .trim_start_matches("^")
        .trim_start_matches("~")
        .trim_start_matches('>')
        .trim_start_matches('=')
        .trim_start_matches('v')
        .trim();

    extract_major_version(cleaned).to_string()
}

fn detect_astro_mode(dir: &Path) -> AstroMode {
    for config_file in ["astro.config.mjs", "astro.config.ts", "astro.config.js"] {
        if let Some(content) = read_file_string(dir, config_file) {
            if content.contains("output: 'server'")
                || content.contains("output: \"server\"")
                || content.contains("@astrojs/node")
                || content.contains("@astrojs/vercel")
                || content.contains("@astrojs/netlify")
                || content.contains("@astrojs/deno")
            {
                return AstroMode::Ssr;
            }
        }
    }
    AstroMode::Static
}

pub fn framework_defaults(
    framework: &Framework,
    pm: &PackageManager,
    astro_mode: Option<&AstroMode>,
) -> (Option<String>, Option<String>, Option<String>, u16) {
    let run = |script: &str| -> String {
        match pm {
            PackageManager::Npm => format!("npm run {script}"),
            PackageManager::Yarn => format!("yarn {script}"),
            PackageManager::Pnpm => format!("pnpm {script}"),
            PackageManager::Bun => format!("bun run {script}"),
        }
    };

    match framework {
        Framework::Dockerfile => (None, None, None, 3000),
        Framework::Astro => match astro_mode {
            Some(AstroMode::Ssr) => (
                Some(run("build")),
                Some("dist".to_string()),
                Some("node ./dist/server/entry.mjs".to_string()),
                4321,
            ),
            _ => (Some(run("build")), Some("dist".to_string()), None, 80),
        },
        Framework::NextJs => (
            Some(run("build")),
            Some(".next".to_string()),
            Some("node server.js".to_string()),
            3000,
        ),
        Framework::Nuxt => (
            Some(run("build")),
            Some(".output".to_string()),
            Some("node .output/server/index.mjs".to_string()),
            3000,
        ),
        Framework::ViteReact | Framework::ViteVue => {
            (Some(run("build")), Some("dist".to_string()), None, 80)
        }
        Framework::NodeApp => (None, None, None, 3000),
        Framework::StaticSite => (None, Some(".".to_string()), None, 80),
    }
}

fn apply_overrides(result: &mut DetectionResult, ov: &BuildConfig) {
    if let Some(ref fw) = ov.framework {
        result.framework = fw.clone();
    }
    if let Some(ref pm) = ov.package_manager {
        result.package_manager = pm.clone();
    }
    if let Some(ref nv) = ov.node_version {
        result.node_version = nv.clone();
    }
    if let Some(ref cmd) = ov.build_command {
        result.build_command = Some(cmd.clone());
    }
    if let Some(ref dir) = ov.output_dir {
        result.output_dir = Some(dir.clone());
    }
    if let Some(ref cmd) = ov.start_command {
        result.start_command = Some(cmd.clone());
    }
    if let Some(port) = ov.port {
        result.detected_port = port;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup_project(files: &[(&str, &str)]) -> TempDir {
        let dir = TempDir::new().unwrap();
        for (name, content) in files {
            let path = dir.path().join(name);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).ok();
            }
            fs::write(path, content).unwrap();
        }
        dir
    }

    #[test]
    fn detects_dockerfile_project() {
        let dir = setup_project(&[("Dockerfile", "FROM node:22")]);
        let result = detect(dir.path(), None).unwrap();
        assert_eq!(result.framework, Framework::Dockerfile);
    }

    #[test]
    fn detects_astro_static() {
        let pkg = r#"{"dependencies": {"astro": "^4.0.0"}}"#;
        let dir = setup_project(&[
            ("package.json", pkg),
            ("astro.config.mjs", "export default defineConfig({})"),
        ]);
        let result = detect(dir.path(), None).unwrap();
        assert_eq!(result.framework, Framework::Astro);
        assert_eq!(result.astro_mode, Some(AstroMode::Static));
        assert_eq!(result.detected_port, 80);
    }

    #[test]
    fn detects_nextjs() {
        let pkg = r#"{"dependencies": {"next": "^14.0.0", "react": "^18.0.0"}}"#;
        let dir = setup_project(&[("package.json", pkg), ("next.config.mjs", "")]);
        let result = detect(dir.path(), None).unwrap();
        assert_eq!(result.framework, Framework::NextJs);
    }

    #[test]
    fn detects_bun_from_lockfile() {
        let pkg = r#"{"dependencies": {"next": "^14.0.0"}}"#;
        let dir = setup_project(&[("package.json", pkg), ("bun.lock", "")]);
        let result = detect(dir.path(), None).unwrap();
        assert_eq!(result.package_manager, PackageManager::Bun);
    }

    #[test]
    fn overrides_apply_correctly() {
        let pkg = r#"{"dependencies": {"next": "^14.0.0"}}"#;
        let dir = setup_project(&[("package.json", pkg)]);

        let overrides = BuildConfig {
            framework: Some(Framework::NodeApp),
            package_manager: Some(PackageManager::Bun),
            node_version: Some("20".to_string()),
            port: Some(8080),
            build_command: Some("bun run build".to_string()),
            ..Default::default()
        };

        let result = detect(dir.path(), Some(&overrides)).unwrap();
        assert_eq!(result.framework, Framework::NodeApp);
        assert_eq!(result.package_manager, PackageManager::Bun);
        assert_eq!(result.detected_port, 8080);
    }
}
