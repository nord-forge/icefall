use crate::cli::client::CliClient;

use super::helpers::confirm;
use super::*;

pub async fn from_file(tarball: &str, manifest_path: &str, signature_path: &str, yes: bool) {
    let current = env!("CARGO_PKG_VERSION");

    println!("  {BOLD}Offline update{RESET}");
    println!();

    let manifest_bytes = match std::fs::read(manifest_path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("  {RED}Error:{RESET} Failed to read manifest: {e}");
            std::process::exit(1);
        }
    };

    let signature_b64 = match std::fs::read_to_string(signature_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("  {RED}Error:{RESET} Failed to read signature: {e}");
            std::process::exit(1);
        }
    };

    print!("  {STEP_RUNNING} Verifying manifest signature  ");
    let now = crate::db::models::now_iso8601();
    match crate::update::verify::verify_manifest_signature(&manifest_bytes, &signature_b64, &now) {
        Ok(key_id) => {
            println!("{GREEN}{STEP_DONE}{RESET} Verified (key: {DIM}{key_id}{RESET})");
        }
        Err(e) => {
            println!("{RED}{STEP_FAILED}{RESET}");
            eprintln!("  {RED}Error:{RESET} Signature verification failed: {e}");
            std::process::exit(1);
        }
    }

    let manifest = match crate::update::manifest::ReleaseManifest::from_bytes(&manifest_bytes) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("  {RED}Error:{RESET} Failed to parse manifest: {e}");
            std::process::exit(1);
        }
    };

    println!(
        "  Version: {BOLD}v{current}{RESET} {DIM}->{RESET} {BOLD}{GREEN}v{}{RESET}",
        manifest.version
    );

    if manifest.breaking {
        println!("  {YELLOW}Warning: This update contains breaking changes.{RESET}");
    }

    print!("  {STEP_RUNNING} Verifying tarball integrity   ");
    let tarball_data = match std::fs::read(tarball) {
        Ok(b) => b,
        Err(e) => {
            println!("{RED}{STEP_FAILED}{RESET}");
            eprintln!("  {RED}Error:{RESET} Failed to read tarball: {e}");
            std::process::exit(1);
        }
    };

    let target = crate::update::artifact_target();
    let artifact = match manifest.artifact_for_target(target) {
        Some(a) => a,
        None => {
            println!("{RED}{STEP_FAILED}{RESET}");
            eprintln!("  {RED}Error:{RESET} No artifact for target {target} in manifest");
            std::process::exit(1);
        }
    };

    match crate::update::verify::verify_sha256(&tarball_data, &artifact.sha256) {
        Ok(()) => println!("{GREEN}{STEP_DONE}{RESET} SHA-256 verified"),
        Err(e) => {
            println!("{RED}{STEP_FAILED}{RESET}");
            eprintln!("  {RED}Error:{RESET} {e}");
            std::process::exit(1);
        }
    }

    println!();
    if !yes && !confirm("  Apply this offline update?") {
        println!("  Update cancelled.");
        return;
    }

    let _client = match CliClient::try_new() {
        Some(c) => c,
        None => {
            println!();
            println!("  {YELLOW}Daemon is not running.{RESET}");
            println!("  To apply offline, extract the tarball and replace the binary manually:");
            println!("    tar xzf {tarball}");
            println!("    sudo cp icefall /usr/local/bin/icefall");
            println!("    sudo systemctl restart icefall");
            return;
        }
    };

    println!();
    println!("  {DIM}Tarball verified. Triggering update via daemon...{RESET}");

    let tmp_dir = std::env::temp_dir().join("icefall-offline-update");
    let _ = std::fs::remove_dir_all(&tmp_dir);
    std::fs::create_dir_all(&tmp_dir).ok();

    let tarball_path = std::path::Path::new(tarball);
    let dest_tarball = tmp_dir.join("update.tar.gz");
    if let Err(e) = std::fs::copy(tarball_path, &dest_tarball) {
        eprintln!("  {RED}Error:{RESET} Failed to stage tarball: {e}");
        std::process::exit(1);
    }

    print!("  {STEP_RUNNING} Extracting update            ");
    let updates_dir = tmp_dir.clone();
    let extract_result = tokio::task::spawn_blocking(move || -> Result<(), String> {
        let file = std::fs::File::open(&dest_tarball).map_err(|e| e.to_string())?;
        let gz = flate2::read::GzDecoder::new(file);
        let mut archive = tar::Archive::new(gz);
        archive.set_overwrite(true);
        archive.unpack(&updates_dir).map_err(|e| e.to_string())?;
        Ok(())
    })
    .await;

    match extract_result {
        Ok(Ok(())) => println!("{GREEN}{STEP_DONE}{RESET} Extracted"),
        Ok(Err(e)) => {
            println!("{RED}{STEP_FAILED}{RESET}");
            eprintln!("  {RED}Error:{RESET} {e}");
            std::process::exit(1);
        }
        Err(e) => {
            println!("{RED}{STEP_FAILED}{RESET}");
            eprintln!("  {RED}Error:{RESET} extraction task failed: {e}");
            std::process::exit(1);
        }
    }

    println!();
    println!("  {GREEN}Files verified and extracted.{RESET}");
    println!("  {DIM}Use `icefall update` with the daemon running for the full apply flow.{RESET}");
}
