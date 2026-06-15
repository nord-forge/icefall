use std::process::Command;

fn main() {
    ensure_dashboard_dist();

    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs");

    let commit = Command::new("git")
        .args(["rev-parse", "--short=7", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map_or_else(
            || "unknown".into(),
            |o| String::from_utf8_lossy(&o.stdout).trim().to_string(),
        );

    let build_date = Command::new("date")
        .args(["-u", "+%Y-%m-%d"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map_or_else(
            || "unknown".into(),
            |o| String::from_utf8_lossy(&o.stdout).trim().to_string(),
        );

    println!("cargo:rustc-env=ICEFALL_GIT_COMMIT={commit}");
    println!("cargo:rustc-env=ICEFALL_BUILD_DATE={build_date}");

    #[cfg(target_arch = "x86_64")]
    println!("cargo:rustc-env=ICEFALL_TARGET_TRIPLE=x86_64-unknown-linux-gnu");
    #[cfg(target_arch = "aarch64")]
    println!("cargo:rustc-env=ICEFALL_TARGET_TRIPLE=aarch64-unknown-linux-gnu");
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    println!("cargo:rustc-env=ICEFALL_TARGET_TRIPLE=unknown");

    if let Ok(target) = std::env::var("TARGET") {
        println!("cargo:rustc-env=ICEFALL_TARGET_TRIPLE={target}");
    }
}

/// The dashboard is embedded at compile time via `include_dir!(.../dashboard/dist)`
/// (IF-255), which panics if the directory is missing. The frontend is built in
/// a separate step (`bun run build`), so a plain `cargo check`/`cargo test` — or
/// a contributor who hasn't built the dashboard — would otherwise fail to
/// compile. Guarantee the directory exists with a placeholder so the Rust build
/// stands alone; the release pipeline builds the real dashboard first and embeds
/// that instead. We never overwrite a real build.
fn ensure_dashboard_dist() {
    use std::path::Path;

    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".into());
    let dist = Path::new(&manifest).join("dashboard").join("dist");
    let index = dist.join("index.html");

    // A real build leaves an index.html — leave it untouched.
    if index.is_file() {
        return;
    }

    // Missing or empty: create a minimal placeholder so include_dir! succeeds.
    if std::fs::create_dir_all(&dist).is_ok() {
        let placeholder = "<!doctype html><meta charset=utf-8>\
            <title>Icefall</title>\
            <body>Dashboard not built. Run <code>cd dashboard &amp;&amp; bun run build</code>, \
            then rebuild, or set <code>ICEFALL_DASHBOARD_DIR</code> to a built dist.</body>";
        let _ = std::fs::write(&index, placeholder);
        // An empty csp-hashes.json keeps the CSP loader happy (no inline hashes).
        let _ = std::fs::write(dist.join("csp-hashes.json"), "[]");
        println!(
            "cargo:warning=dashboard/dist not found — embedding a placeholder. \
             Build the dashboard (cd dashboard && bun run build) for the real UI."
        );
    }
}
