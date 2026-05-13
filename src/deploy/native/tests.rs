use super::*;

#[test]
fn static_frameworks_are_native() {
    use crate::build::{AstroMode, DetectionResult, Framework, PackageManager};

    let make = |fw: Framework, astro: Option<AstroMode>| DetectionResult {
        framework: fw,
        package_manager: PackageManager::Npm,
        node_version: "22".to_string(),
        build_command: None,
        output_dir: None,
        start_command: None,
        detected_port: 80,
        astro_mode: astro,
    };

    assert!(should_use_native(&make(Framework::StaticSite, None)));
    assert!(should_use_native(&make(Framework::ViteReact, None)));
    assert!(should_use_native(&make(Framework::ViteVue, None)));
    assert!(should_use_native(&make(
        Framework::Astro,
        Some(AstroMode::Static)
    )));

    assert!(!should_use_native(&make(Framework::NextJs, None)));
    assert!(!should_use_native(&make(Framework::Nuxt, None)));
    assert!(!should_use_native(&make(Framework::NodeApp, None)));
    assert!(!should_use_native(&make(Framework::Dockerfile, None)));
    assert!(!should_use_native(&make(
        Framework::Astro,
        Some(AstroMode::Ssr)
    )));
}

#[test]
fn install_commands_correct() {
    use crate::build::PackageManager;
    use super::helpers::install_command;

    assert_eq!(install_command(&PackageManager::Npm), "npm ci");
    assert_eq!(
        install_command(&PackageManager::Yarn),
        "yarn install --frozen-lockfile"
    );
    assert_eq!(
        install_command(&PackageManager::Pnpm),
        "pnpm install --frozen-lockfile"
    );
    assert_eq!(
        install_command(&PackageManager::Bun),
        "bun install --frozen-lockfile"
    );
}
