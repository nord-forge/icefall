use std::path::Path;

use super::{
    AstroMode, BuildConfig, DetectError, DetectionResult, Framework, PackageManager, RepoHints,
};

pub fn detect(
    project_dir: &Path,
    overrides: Option<&BuildConfig>,
) -> Result<DetectionResult, DetectError> {
    let framework = detect_framework(project_dir);
    let package_manager = detect_package_manager(project_dir);
    let yarn_berry = package_manager == PackageManager::Yarn && detect_yarn_berry(project_dir);
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
        yarn_berry,
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

/// Detect Yarn 2+ (Berry). Two reliable signals: a `packageManager: "yarn@2|3|4..."`
/// field in package.json (Corepack pin), or the Berry-only `__metadata:` block in
/// yarn.lock (Yarn 1 lockfiles never contain it).
fn detect_yarn_berry(dir: &Path) -> bool {
    if let Some(pkg) = read_file_string(dir, "package.json") {
        if let Some(idx) = pkg.find("\"packageManager\"") {
            let tail = &pkg[idx..];
            if let Some(at) = tail.find("yarn@") {
                let ver = &tail[at + 5..];
                // First version digit > 1 means Berry.
                if let Some(major) = ver.chars().find(|c| c.is_ascii_digit()) {
                    return major != '1';
                }
            }
        }
    }
    read_file_string(dir, "yarn.lock").is_some_and(|lock| lock.contains("__metadata:"))
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

/// Enumerate repo-shape hints (AC1/AC2/AC3) at `dir`. Surfaces Dockerfile
/// variants, root compose files, and a monorepo signal. Takes the already-run
/// detection so the monorepo "no deployable app at root" condition can key off
/// the resolved framework.
pub fn detect_repo_hints(dir: &Path, detection: &DetectionResult) -> RepoHints {
    let mut dockerfiles = dockerfile_names(dir);
    dockerfiles.sort();
    let has_plain_dockerfile = dockerfiles.iter().any(|n| n == "Dockerfile");

    let mut compose_files = compose_file_names(dir);
    compose_files.sort();

    let workspaces = workspace_dirs(dir);
    // Monorepo guardrail: workspaces declared AND detection fell through to a
    // bare static site (no app resolved at root). A root app that happens to
    // live in a workspace repo (framework != StaticSite) is not flagged.
    let is_monorepo = !workspaces.is_empty() && detection.framework == Framework::StaticSite;

    RepoHints {
        dockerfiles,
        has_plain_dockerfile,
        compose_files,
        is_monorepo,
        workspaces,
    }
}

/// Names of `Dockerfile` and `Dockerfile.*` files directly in `dir`.
/// `Dockerfile` (exact) and variants like `Dockerfile.api` count; `*.dockerfile`
/// or `Dockerfile`-prefixed-but-not-dot (e.g. `Dockerfilefoo`) do not.
fn dockerfile_names(dir: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|name| name == "Dockerfile" || name.starts_with("Dockerfile."))
        .collect()
}

/// Compose file names directly in `dir`: `docker-compose.yml`/`.yaml`,
/// `docker-compose.<env>.yml`/`.yaml`, and `compose.yml`/`.yaml`.
fn compose_file_names(dir: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|name| is_compose_file(name))
        .collect()
}

fn is_compose_file(name: &str) -> bool {
    let Some(stem) = name
        .strip_suffix(".yml")
        .or_else(|| name.strip_suffix(".yaml"))
    else {
        return false;
    };
    // "compose", "docker-compose", or "docker-compose.<anything>".
    stem == "compose" || stem == "docker-compose" || stem.starts_with("docker-compose.")
}

/// Resolve workspace directories from the root `package.json` `workspaces`
/// field. Supports the array form and the `{ "packages": [...] }` object form.
/// Only `<dir>/*` globs and literal dirs are resolved (the common cases); each
/// resolved path must exist on disk and contain a `package.json`.
fn workspace_dirs(dir: &Path) -> Vec<String> {
    let Some(pkg) = parse_package_json(dir) else {
        return Vec::new();
    };
    let globs = match pkg.get("workspaces") {
        Some(serde_json::Value::Array(arr)) => arr.clone(),
        Some(serde_json::Value::Object(obj)) => obj
            .get("packages")
            .and_then(|p| p.as_array())
            .cloned()
            .unwrap_or_default(),
        _ => return Vec::new(),
    };

    let mut dirs = Vec::new();
    for glob in globs.iter().filter_map(|g| g.as_str()) {
        if let Some(prefix) = glob.strip_suffix("/*") {
            // Expand one level: prefix/* -> each child dir with a package.json.
            if let Ok(entries) = std::fs::read_dir(dir.join(prefix)) {
                for entry in entries.filter_map(Result::ok) {
                    if entry.file_type().is_ok_and(|t| t.is_dir())
                        && entry.path().join("package.json").is_file()
                    {
                        if let Some(name) = entry.file_name().to_str() {
                            dirs.push(format!("{prefix}/{name}"));
                        }
                    }
                }
            }
        } else if dir.join(glob).join("package.json").is_file() {
            dirs.push(glob.to_string());
        }
    }
    dirs.sort();
    dirs
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

    fn hints_for(dir: &Path) -> RepoHints {
        let detection = detect(dir, None).unwrap();
        detect_repo_hints(dir, &detection)
    }

    #[test]
    fn hints_find_plain_dockerfile() {
        let dir = setup_project(&[("Dockerfile", "FROM node:22")]);
        let hints = hints_for(dir.path());
        assert_eq!(hints.dockerfiles, vec!["Dockerfile".to_string()]);
        assert!(hints.has_plain_dockerfile);
    }

    #[test]
    fn hints_find_variant_dockerfiles_without_plain() {
        // kaartje case: Dockerfile.api + Dockerfile.web, no plain Dockerfile.
        let dir = setup_project(&[
            ("Dockerfile.api", "FROM oven/bun:1"),
            ("Dockerfile.web", "FROM caddy:2"),
            ("package.json", r#"{"workspaces": ["packages/*"]}"#),
        ]);
        let hints = hints_for(dir.path());
        assert_eq!(
            hints.dockerfiles,
            vec!["Dockerfile.api".to_string(), "Dockerfile.web".to_string()]
        );
        assert!(
            !hints.has_plain_dockerfile,
            "variant-only must be flagged ambiguous"
        );
    }

    #[test]
    fn hints_plain_plus_variants() {
        let dir = setup_project(&[
            ("Dockerfile", "FROM node:22"),
            ("Dockerfile.dev", "FROM node:22"),
        ]);
        let hints = hints_for(dir.path());
        assert_eq!(hints.dockerfiles.len(), 2);
        assert!(hints.has_plain_dockerfile);
    }

    #[test]
    fn hints_ignore_non_dockerfile_names() {
        let dir = setup_project(&[
            ("Dockerfilefoo", "x"),
            ("app.dockerfile", "x"),
            ("README.md", "x"),
        ]);
        let hints = hints_for(dir.path());
        assert!(hints.dockerfiles.is_empty());
        assert!(!hints.has_plain_dockerfile);
    }

    #[test]
    fn hints_empty_on_bare_repo() {
        let dir = setup_project(&[("README.md", "hi")]);
        let hints = hints_for(dir.path());
        assert!(hints.dockerfiles.is_empty());
        assert!(!hints.has_plain_dockerfile);
        assert!(hints.compose_files.is_empty());
        assert!(!hints.is_monorepo);
    }

    #[test]
    fn hints_find_compose_files() {
        let dir = setup_project(&[
            ("docker-compose.yml", "services: {}"),
            ("docker-compose.prod.yml", "services: {}"),
            ("compose.yaml", "services: {}"),
            ("not-compose.yml", "x"),
        ]);
        let hints = hints_for(dir.path());
        assert_eq!(
            hints.compose_files,
            vec![
                "compose.yaml".to_string(),
                "docker-compose.prod.yml".to_string(),
                "docker-compose.yml".to_string(),
            ]
        );
    }

    #[test]
    fn hints_monorepo_when_workspaces_and_no_root_app() {
        // kaartje root: workspaces declared, no astro/next/etc at root, no
        // start/main -> detection is StaticSite -> must flag monorepo.
        let dir = setup_project(&[
            (
                "package.json",
                r#"{"workspaces": ["packages/*"], "devDependencies": {"oxlint": "latest"}}"#,
            ),
            ("packages/api/package.json", r#"{"name": "@k/api"}"#),
            ("packages/web/package.json", r#"{"name": "@k/web"}"#),
        ]);
        let hints = hints_for(dir.path());
        assert!(hints.is_monorepo, "must not silently ship root as static");
        assert_eq!(
            hints.workspaces,
            vec!["packages/api".to_string(), "packages/web".to_string()]
        );
    }

    #[test]
    fn hints_not_monorepo_when_root_app_resolves() {
        // workspaces declared but root is itself a deployable app (Astro).
        let dir = setup_project(&[
            (
                "package.json",
                r#"{"workspaces": ["packages/*"], "dependencies": {"astro": "^4"}}"#,
            ),
            ("packages/ui/package.json", r#"{"name": "@k/ui"}"#),
        ]);
        let hints = hints_for(dir.path());
        assert!(!hints.is_monorepo);
    }

    #[test]
    fn hints_workspaces_object_form() {
        let dir = setup_project(&[
            (
                "package.json",
                r#"{"workspaces": {"packages": ["apps/*"]}}"#,
            ),
            ("apps/site/package.json", r#"{"name": "site"}"#),
        ]);
        let hints = hints_for(dir.path());
        assert_eq!(hints.workspaces, vec!["apps/site".to_string()]);
        assert!(hints.is_monorepo);
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
