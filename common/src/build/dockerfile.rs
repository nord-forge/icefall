use super::{AstroMode, BuildConfig, DetectError, DetectionResult, Framework, PackageManager};

pub fn generate_dockerfile(
    detection: &DetectionResult,
    overrides: Option<&BuildConfig>,
) -> Result<String, DetectError> {
    let base_image = overrides.and_then(|o| o.base_image.as_deref());

    let dockerfile = match (&detection.framework, &detection.astro_mode) {
        (Framework::Dockerfile, _) => {
            return Err(DetectError::DockerfileGeneration(
                "cannot generate Dockerfile for a project that already has one".to_string(),
            ));
        }
        (Framework::StaticSite, _) => {
            if detection.build_command.is_some() {
                dockerfile_static_build(detection, base_image)
            } else {
                dockerfile_static(detection)
            }
        }
        (Framework::Astro, Some(AstroMode::Ssr)) => dockerfile_astro_ssr(detection, base_image),
        (Framework::Astro, _) => dockerfile_static_build(detection, base_image),
        (Framework::NextJs, _) => dockerfile_nextjs(detection, base_image),
        (Framework::Nuxt, _) => dockerfile_nuxt(detection, base_image),
        (Framework::ViteReact, _) | (Framework::ViteVue, _) => {
            dockerfile_static_build(detection, base_image)
        }
        (Framework::NodeApp, _) => dockerfile_node_app(detection, base_image),
    };

    Ok(dockerfile)
}

pub fn generate_dockerignore(_detection: &DetectionResult) -> String {
    [
        "node_modules",
        ".git",
        ".env",
        ".env.*",
        "*.md",
        ".DS_Store",
        ".next",
        ".nuxt",
        ".output",
        "dist",
        ".turbo",
        ".cache",
        "coverage",
    ]
    .join("\n")
}

fn base_image_for(pm: &PackageManager, node_version: &str, base_override: Option<&str>) -> String {
    if let Some(img) = base_override {
        return img.to_string();
    }
    match pm {
        PackageManager::Bun => "oven/bun:latest".to_string(),
        _ => format!("node:{node_version}-slim"),
    }
}

fn lockfile_name(pm: &PackageManager) -> &'static str {
    match pm {
        PackageManager::Npm => "package-lock.json",
        PackageManager::Yarn => "yarn.lock",
        PackageManager::Pnpm => "pnpm-lock.yaml",
        PackageManager::Bun => "bun.lock",
    }
}

fn install_cmd(pm: &PackageManager) -> &'static str {
    match pm {
        PackageManager::Npm => "npm ci",
        PackageManager::Yarn => "yarn install --frozen-lockfile",
        PackageManager::Pnpm => "corepack enable && pnpm install --frozen-lockfile",
        PackageManager::Bun => "bun install --frozen-lockfile",
    }
}

fn install_prod_cmd(pm: &PackageManager) -> &'static str {
    match pm {
        PackageManager::Npm => "npm ci --omit=dev",
        PackageManager::Yarn => "yarn install --frozen-lockfile --production",
        PackageManager::Pnpm => "corepack enable && pnpm install --frozen-lockfile --prod",
        PackageManager::Bun => "bun install --frozen-lockfile --production",
    }
}

fn dockerfile_static(detection: &DetectionResult) -> String {
    let output = detection.output_dir.as_deref().unwrap_or(".");
    format!(
        r#"# Static site served by Caddy
FROM caddy:2-alpine
COPY {output} /usr/share/caddy
EXPOSE {port}
"#,
        port = detection.detected_port,
    )
}

fn dockerfile_static_build(detection: &DetectionResult, base_override: Option<&str>) -> String {
    let pm = &detection.package_manager;
    let base = base_image_for(pm, &detection.node_version, base_override);
    let lockfile = lockfile_name(pm);
    let install = install_cmd(pm);
    let build_cmd = detection
        .build_command
        .as_deref()
        .unwrap_or("npm run build");
    let output = detection.output_dir.as_deref().unwrap_or("dist");

    format!(
        r#"# Build stage
FROM {base} AS builder
WORKDIR /app
COPY package.json {lockfile} ./
RUN {install}
COPY . .
RUN {build_cmd}

# Serve stage
FROM caddy:2-alpine
COPY --from=builder /app/{output} /usr/share/caddy
EXPOSE 80
"#,
    )
}

fn dockerfile_nextjs(detection: &DetectionResult, base_override: Option<&str>) -> String {
    let pm = &detection.package_manager;
    let base = base_image_for(pm, &detection.node_version, base_override);
    let runtime_base = base_image_for(&PackageManager::Npm, &detection.node_version, None);
    let lockfile = lockfile_name(pm);
    let install = install_cmd(pm);
    let build_cmd = detection
        .build_command
        .as_deref()
        .unwrap_or("npm run build");
    let port = detection.detected_port;

    format!(
        r#"# Build stage
FROM {base} AS builder
WORKDIR /app
COPY package.json {lockfile} ./
RUN {install}
COPY . .
RUN {build_cmd}

# Runtime stage
FROM {runtime_base}
WORKDIR /app
RUN addgroup --system --gid 1001 nodejs && \
    adduser --system --uid 1001 nextjs
COPY --from=builder /app/public ./public
COPY --from=builder --chown=nextjs:nodejs /app/.next/standalone ./
COPY --from=builder --chown=nextjs:nodejs /app/.next/static ./.next/static
USER nextjs
EXPOSE {port}
ENV PORT={port} HOSTNAME="0.0.0.0"
CMD ["node", "server.js"]
"#,
    )
}

fn dockerfile_nuxt(detection: &DetectionResult, base_override: Option<&str>) -> String {
    let pm = &detection.package_manager;
    let base = base_image_for(pm, &detection.node_version, base_override);
    let runtime_base = base_image_for(&PackageManager::Npm, &detection.node_version, None);
    let lockfile = lockfile_name(pm);
    let install = install_cmd(pm);
    let build_cmd = detection
        .build_command
        .as_deref()
        .unwrap_or("npm run build");
    let port = detection.detected_port;

    format!(
        r#"# Build stage
FROM {base} AS builder
WORKDIR /app
COPY package.json {lockfile} ./
RUN {install}
COPY . .
RUN {build_cmd}

# Runtime stage
FROM {runtime_base}
WORKDIR /app
RUN addgroup --system --gid 1001 nuxt && \
    adduser --system --uid 1001 nuxt
COPY --from=builder --chown=nuxt:nuxt /app/.output ./
USER nuxt
EXPOSE {port}
CMD ["node", "server/index.mjs"]
"#,
    )
}

fn dockerfile_astro_ssr(detection: &DetectionResult, base_override: Option<&str>) -> String {
    let pm = &detection.package_manager;
    let base = base_image_for(pm, &detection.node_version, base_override);
    let runtime_base = base_image_for(&PackageManager::Npm, &detection.node_version, None);
    let lockfile = lockfile_name(pm);
    let install = install_cmd(pm);
    let build_cmd = detection
        .build_command
        .as_deref()
        .unwrap_or("npm run build");
    let port = detection.detected_port;
    let start_cmd = detection
        .start_command
        .as_deref()
        .unwrap_or("node ./dist/server/entry.mjs");

    format!(
        r#"# Build stage
FROM {base} AS builder
WORKDIR /app
COPY package.json {lockfile} ./
RUN {install}
COPY . .
RUN {build_cmd}

# Runtime stage
FROM {runtime_base}
WORKDIR /app
RUN addgroup --system --gid 1001 astro && \
    adduser --system --uid 1001 astro
COPY --from=builder --chown=astro:astro /app/dist ./dist
COPY --from=builder --chown=astro:astro /app/node_modules ./node_modules
COPY --from=builder --chown=astro:astro /app/package.json ./
USER astro
EXPOSE {port}
CMD {start_cmd_json}
"#,
        start_cmd_json = shell_to_cmd_json(start_cmd),
    )
}

fn dockerfile_node_app(detection: &DetectionResult, base_override: Option<&str>) -> String {
    let pm = &detection.package_manager;
    let base = base_image_for(pm, &detection.node_version, base_override);
    let lockfile = lockfile_name(pm);
    let install = install_prod_cmd(pm);
    let port = detection.detected_port;
    let start_cmd = detection
        .start_command
        .as_deref()
        .unwrap_or("node index.js");

    format!(
        r#"# Node.js application
FROM {base}
WORKDIR /app
COPY package.json {lockfile} ./
RUN {install}
COPY . .
RUN addgroup --system --gid 1001 app && \
    adduser --system --uid 1001 app
USER app
EXPOSE {port}
CMD {start_cmd_json}
"#,
        start_cmd_json = shell_to_cmd_json(start_cmd),
    )
}

fn shell_to_cmd_json(cmd: &str) -> String {
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    let json_parts: Vec<String> = parts.iter().map(|p| format!("\"{p}\"")).collect();
    format!("[{}]", json_parts.join(", "))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::build::detect::framework_defaults;

    fn make_detection(framework: Framework, pm: PackageManager) -> DetectionResult {
        let (build_command, output_dir, start_command, detected_port) =
            framework_defaults(&framework, &pm, None);
        DetectionResult {
            framework,
            package_manager: pm,
            node_version: "22".to_string(),
            build_command,
            output_dir,
            start_command,
            detected_port,
            astro_mode: None,
        }
    }

    #[test]
    fn errors_on_dockerfile_project() {
        let det = make_detection(Framework::Dockerfile, PackageManager::Npm);
        assert!(generate_dockerfile(&det, None).is_err());
    }

    #[test]
    fn generates_nextjs_standalone() {
        let det = make_detection(Framework::NextJs, PackageManager::Npm);
        let result = generate_dockerfile(&det, None).unwrap();
        assert!(result.contains("FROM node:22-slim AS builder"));
        assert!(result.contains(".next/standalone"));
        assert!(result.contains("USER nextjs"));
    }

    #[test]
    fn generates_node_app() {
        let det = DetectionResult {
            framework: Framework::NodeApp,
            package_manager: PackageManager::Npm,
            node_version: "20".to_string(),
            build_command: None,
            output_dir: None,
            start_command: Some("node server.js".to_string()),
            detected_port: 3000,
            astro_mode: None,
        };
        let result = generate_dockerfile(&det, None).unwrap();
        assert!(result.contains("FROM node:20-slim"));
        assert!(result.contains(r#"["node", "server.js"]"#));
    }

    #[test]
    fn dockerignore_has_essentials() {
        let det = make_detection(Framework::NextJs, PackageManager::Npm);
        let ignore = generate_dockerignore(&det);
        assert!(ignore.contains("node_modules"));
        assert!(ignore.contains(".git"));
    }
}
