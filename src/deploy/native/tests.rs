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
    use super::helpers::install_command;
    use crate::build::PackageManager;

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

// --- copy_dir_recursive ---

#[tokio::test]
async fn copy_dir_recursive_copies_files() {
    use super::helpers::copy_dir_recursive;

    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");

    tokio::fs::create_dir_all(&src).await.unwrap();
    tokio::fs::write(src.join("hello.txt"), "world").await.unwrap();

    copy_dir_recursive(&src, &dst).await.unwrap();

    let content = tokio::fs::read_to_string(dst.join("hello.txt")).await.unwrap();
    assert_eq!(content, "world");
}

#[tokio::test]
async fn copy_dir_recursive_copies_nested_dirs() {
    use super::helpers::copy_dir_recursive;

    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("src");
    let nested = src.join("sub").join("deep");

    tokio::fs::create_dir_all(&nested).await.unwrap();
    tokio::fs::write(nested.join("file.txt"), "nested content").await.unwrap();
    tokio::fs::write(src.join("root.txt"), "root content").await.unwrap();

    let dst = tmp.path().join("dst");
    copy_dir_recursive(&src, &dst).await.unwrap();

    let root_content = tokio::fs::read_to_string(dst.join("root.txt")).await.unwrap();
    assert_eq!(root_content, "root content");

    let nested_content = tokio::fs::read_to_string(dst.join("sub").join("deep").join("file.txt"))
        .await
        .unwrap();
    assert_eq!(nested_content, "nested content");
}

#[tokio::test]
async fn copy_dir_recursive_creates_dst_if_missing() {
    use super::helpers::copy_dir_recursive;

    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("src");
    tokio::fs::create_dir_all(&src).await.unwrap();
    tokio::fs::write(src.join("a.txt"), "data").await.unwrap();

    let dst = tmp.path().join("nonexistent").join("dst");
    copy_dir_recursive(&src, &dst).await.unwrap();

    assert!(dst.join("a.txt").exists());
}

// --- atomic_symlink ---

#[tokio::test]
async fn atomic_symlink_creates_symlink() {
    use super::helpers::atomic_symlink;

    let tmp = tempfile::tempdir().unwrap();
    let target = tmp.path().join("target_dir");
    tokio::fs::create_dir_all(&target).await.unwrap();
    tokio::fs::write(target.join("index.html"), "<h1>hello</h1>").await.unwrap();

    let link = tmp.path().join("current");
    atomic_symlink(&target, &link).await.unwrap();

    assert!(link.is_symlink());
    let resolved = tokio::fs::read_link(&link).await.unwrap();
    assert_eq!(resolved, target);
}

#[tokio::test]
async fn atomic_symlink_replaces_existing_symlink() {
    use super::helpers::atomic_symlink;

    let tmp = tempfile::tempdir().unwrap();
    let old_target = tmp.path().join("old");
    let new_target = tmp.path().join("new");
    tokio::fs::create_dir_all(&old_target).await.unwrap();
    tokio::fs::create_dir_all(&new_target).await.unwrap();

    let link = tmp.path().join("current");

    // Create initial symlink
    atomic_symlink(&old_target, &link).await.unwrap();
    let resolved = tokio::fs::read_link(&link).await.unwrap();
    assert_eq!(resolved, old_target);

    // Replace with new target
    atomic_symlink(&new_target, &link).await.unwrap();
    let resolved = tokio::fs::read_link(&link).await.unwrap();
    assert_eq!(resolved, new_target);
}

// --- cleanup_old_deploys ---

#[tokio::test]
async fn cleanup_old_deploys_keeps_recent_and_current() {
    use super::helpers::cleanup_old_deploys;

    let tmp = tempfile::tempdir().unwrap();
    let sites_dir = tmp.path();

    // Create deploy directories (v7 UUIDs sort chronologically, simulate with sorted names)
    let dirs = ["01-oldest", "02-old", "03-recent", "04-current"];
    for name in &dirs {
        tokio::fs::create_dir_all(sites_dir.join(name)).await.unwrap();
    }

    // Also create a "current" symlink that should be skipped
    tokio::fs::symlink(sites_dir.join("04-current"), sites_dir.join("current"))
        .await
        .unwrap();

    // Keep 2 most recent (besides current), should remove 01-oldest
    cleanup_old_deploys(sites_dir, "04-current", 2).await.unwrap();

    // current deploy dir should still exist
    assert!(sites_dir.join("04-current").exists());
    // 2 most recent should be kept
    assert!(sites_dir.join("03-recent").exists());
    assert!(sites_dir.join("02-old").exists());
    // oldest should be removed
    assert!(!sites_dir.join("01-oldest").exists());
    // current symlink untouched
    assert!(sites_dir.join("current").exists());
}

#[tokio::test]
async fn cleanup_old_deploys_nothing_to_remove() {
    use super::helpers::cleanup_old_deploys;

    let tmp = tempfile::tempdir().unwrap();
    let sites_dir = tmp.path();

    tokio::fs::create_dir_all(sites_dir.join("deploy-1")).await.unwrap();
    tokio::fs::create_dir_all(sites_dir.join("deploy-2")).await.unwrap();

    // Keep 5 but only 1 non-current deploy exists — nothing should be removed
    cleanup_old_deploys(sites_dir, "deploy-2", 5).await.unwrap();

    assert!(sites_dir.join("deploy-1").exists());
    assert!(sites_dir.join("deploy-2").exists());
}
