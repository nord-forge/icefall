use crate::cli::client::CliClient;

const GREEN: &str = "\x1b[32m";
const RED: &str = "\x1b[31m";
const YELLOW: &str = "\x1b[33m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const RESET: &str = "\x1b[0m";

const STEP_DONE: &str = "✓";
const STEP_RUNNING: &str = "●";
const _STEP_PENDING: &str = "○";
const STEP_FAILED: &str = "✗";

/// Interactive update flow (default when no subcommand given).
pub async fn run(yes: bool, channel: Option<&str>, json: bool) {
    let client = CliClient::new_or_exit();
    let current = env!("CARGO_PKG_VERSION");

    // If a channel override was provided, set it first
    if let Some(ch) = channel {
        let _ = client
            .post::<serde_json::Value>(
                "/system/update/preferences",
                &serde_json::json!({ "channel": ch }),
            )
            .await;
    }

    // Step 1: Check for updates
    if !json {
        print!("Checking for updates...  ");
    }
    let check: serde_json::Value = match client.get("/system/update/check").await {
        Ok(v) => v,
        Err(e) => {
            if json {
                print_json_step("check", "failed", Some(&e.to_string()), None);
            } else {
                println!("{RED}failed{RESET}");
                eprintln!("{RED}Error:{RESET} {e}");
            }
            std::process::exit(1);
        }
    };

    let data = &check["data"];
    let available = data["available"].as_bool().unwrap_or(false);

    if !available {
        if json {
            print_json_step(
                "check",
                "done",
                None,
                Some(serde_json::json!({
                    "up_to_date": true,
                    "version": current,
                })),
            );
        } else {
            println!("{GREEN}up to date{RESET}");
            println!("\n  Icefall {GREEN}v{current}{RESET} is the latest version.");
        }
        return;
    }

    let latest = data["latest_version"].as_str().unwrap_or("unknown");
    let breaking = data["breaking"].as_bool().unwrap_or(false);

    if json {
        print_json_step(
            "check",
            "done",
            None,
            Some(serde_json::json!({
                "up_to_date": false,
                "current_version": current,
                "latest_version": latest,
                "breaking": breaking,
            })),
        );
    } else {
        println!("{GREEN}done{RESET}");

        println!();
        println!("  {BOLD}Icefall v{current}{RESET} {DIM}->{RESET} {BOLD}{GREEN}v{latest}{RESET}");

        if breaking {
            println!();
            println!("  {YELLOW}Warning: This update contains breaking changes.{RESET}");
            if let Some(changes) = data["breaking_changes"].as_str() {
                println!("  {changes}");
            }
        }

        if let Some(highlights) = data["changelog_highlights"].as_array() {
            if !highlights.is_empty() {
                println!();
                println!("  {BOLD}What's new:{RESET}");
                for item in highlights {
                    if let Some(s) = item.as_str() {
                        println!("    {DIM}-{RESET} {s}");
                    }
                }
            }
        }
        println!();
    }

    // Step 2: Confirm
    if !yes && !json {
        if !confirm("  Apply this update?") {
            println!("  Update cancelled.");
            return;
        }
        println!();
    }

    // Step 3: Download
    if !json {
        print!("  {STEP_RUNNING} Downloading update        ");
    }
    let download: serde_json::Value = match client
        .post("/system/update/download", &serde_json::json!({}))
        .await
    {
        Ok(v) => v,
        Err(e) => {
            if json {
                print_json_step("download", "failed", Some(&e.to_string()), None);
            } else {
                println!("{RED}{STEP_FAILED} failed{RESET}");
                eprintln!("  {RED}Error:{RESET} {e}");
            }
            std::process::exit(1);
        }
    };

    let dl_status = download["data"]["status"].as_str().unwrap_or("unknown");
    if dl_status != "download_started" && dl_status != "already_downloading" {
        if json {
            print_json_step(
                "download",
                "failed",
                Some(&format!("Unexpected status: {dl_status}")),
                None,
            );
        } else {
            println!("{RED}{STEP_FAILED} failed{RESET}");
            eprintln!("  Unexpected download status: {dl_status}");
        }
        std::process::exit(1);
    }

    // Poll download progress
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        let status: serde_json::Value = match client.get("/system/update/status").await {
            Ok(v) => v,
            Err(e) => {
                if json {
                    print_json_step("download", "failed", Some(&e.to_string()), None);
                } else {
                    println!("{RED}{STEP_FAILED} failed{RESET}");
                    eprintln!("  {RED}Error:{RESET} {e}");
                }
                std::process::exit(1);
            }
        };

        let state = status["data"]["download_state"]
            .as_str()
            .unwrap_or("unknown");
        let progress = status["data"]["download_progress"].as_i64().unwrap_or(0);

        match state {
            "downloading" => {
                if json {
                    print_json_step(
                        "download",
                        "running",
                        None,
                        Some(serde_json::json!({
                            "progress": progress,
                        })),
                    );
                } else {
                    let bar = progress_bar(progress as u32, 20);
                    print!("\r  {STEP_RUNNING} Downloading update        {bar} {progress}%  ");
                }
            }
            "ready" => {
                if json {
                    print_json_step("download", "done", None, None);
                } else {
                    println!("\r  {GREEN}{STEP_DONE}{RESET} Downloading update        {GREEN}done{RESET}  ");
                }
                break;
            }
            "error" => {
                let err_msg = status["data"]["error_message"]
                    .as_str()
                    .unwrap_or("Unknown error");
                if json {
                    print_json_step("download", "failed", Some(err_msg), None);
                } else {
                    println!("{RED}{STEP_FAILED} failed{RESET}");
                    eprintln!("  {RED}Error:{RESET} {err_msg}");
                }
                std::process::exit(1);
            }
            _ => {}
        }
    }

    // Step 4: Apply
    if !json {
        print!("  {STEP_RUNNING} Applying update           ");
    }
    let apply: serde_json::Value = match client
        .post("/system/update/apply", &serde_json::json!({}))
        .await
    {
        Ok(v) => v,
        Err(e) => {
            if json {
                print_json_step("apply", "failed", Some(&e.to_string()), None);
            } else {
                println!("{RED}{STEP_FAILED} failed{RESET}");
                eprintln!("  {RED}Error:{RESET} {e}");
            }
            std::process::exit(1);
        }
    };

    let apply_status = apply["data"]["status"].as_str().unwrap_or("unknown");
    if apply_status == "applying" {
        if json {
            print_json_step("apply", "done", None, None);
        } else {
            println!("{GREEN}{STEP_DONE}{RESET} Applying update           {GREEN}done{RESET}");
        }
    } else if json {
        print_json_step("apply", apply_status, None, None);
    } else {
        println!("{YELLOW}{apply_status}{RESET}");
    }

    // Step 5: Wait for health
    if !json {
        print!("  {STEP_RUNNING} Verifying health          ");
    }

    let mut health_ok = false;
    for _ in 0..30 {
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        match client.get::<serde_json::Value>("/server/status").await {
            Ok(_) => {
                health_ok = true;
                break;
            }
            Err(_) => continue,
        }
    }

    if health_ok {
        if json {
            print_json_step("health", "done", None, None);
            print_json_step(
                "complete",
                "done",
                None,
                Some(serde_json::json!({
                    "version": latest,
                })),
            );
        } else {
            println!("{GREEN}{STEP_DONE}{RESET} Verifying health          {GREEN}done{RESET}");
            println!();
            println!("  {GREEN}Update complete!{RESET} Icefall is now running v{latest}.");
        }
    } else if json {
        print_json_step("health", "timeout", None, None);
    } else {
        println!("{YELLOW}timeout{RESET}");
        println!();
        println!("  {YELLOW}Icefall didn't respond within 60 seconds.{RESET}");
        println!("  {DIM}Run: icefall update rollback{RESET}");
    }
}

/// Check for updates without applying. Prints result and exits.
pub async fn check(json: bool) {
    let client = CliClient::new_or_exit();
    let current = env!("CARGO_PKG_VERSION");

    if !json {
        print!("Checking for updates...  ");
    }
    let check: serde_json::Value = match client.get("/system/update/check").await {
        Ok(v) => v,
        Err(e) => {
            if json {
                print_json_step("check", "failed", Some(&e.to_string()), None);
            } else {
                println!("{RED}failed{RESET}");
                eprintln!("{RED}Error:{RESET} {e}");
            }
            std::process::exit(2);
        }
    };

    let data = &check["data"];
    let available = data["available"].as_bool().unwrap_or(false);

    if !available {
        if json {
            println!(
                "{}",
                serde_json::json!({
                    "up_to_date": true,
                    "current_version": current,
                })
            );
        } else {
            println!("{GREEN}up to date{RESET}");
            println!("\n  Icefall {GREEN}v{current}{RESET} is the latest version.");
        }
        std::process::exit(0);
    }

    let latest = data["latest_version"].as_str().unwrap_or("unknown");
    let breaking = data["breaking"].as_bool().unwrap_or(false);

    if json {
        println!(
            "{}",
            serde_json::json!({
                "up_to_date": false,
                "current_version": current,
                "latest_version": latest,
                "breaking": breaking,
                "changelog_highlights": data["changelog_highlights"],
            })
        );
    } else {
        println!("{GREEN}done{RESET}");

        println!();
        println!("  {BOLD}Update available:{RESET} v{current} -> {GREEN}v{latest}{RESET}");

        if breaking {
            println!("  {YELLOW}This update contains breaking changes.{RESET}");
        }

        if let Some(highlights) = data["changelog_highlights"].as_array() {
            if !highlights.is_empty() {
                println!();
                println!("  {BOLD}What's new:{RESET}");
                for item in highlights {
                    if let Some(s) = item.as_str() {
                        println!("    {DIM}-{RESET} {s}");
                    }
                }
            }
        }

        println!();
        println!("  Run {BOLD}icefall update{RESET} to apply this update.");
    }

    std::process::exit(1);
}

/// Roll back to the previous version.
pub async fn rollback(yes: bool) {
    let client = CliClient::new_or_exit();

    print!("Checking rollback availability...  ");
    let status: serde_json::Value = match client.get("/system/update/status").await {
        Ok(v) => v,
        Err(e) => {
            println!("{RED}failed{RESET}");
            eprintln!("{RED}Error:{RESET} {e}");
            std::process::exit(1);
        }
    };

    let rollback_available = status["data"]["rollback_available"]
        .as_bool()
        .unwrap_or(false);

    if !rollback_available {
        println!("{RED}unavailable{RESET}");
        eprintln!(
            "\n  {RED}No rollback binary found.{RESET} A rollback is only available after a successful update."
        );
        std::process::exit(1);
    }
    println!("{GREEN}available{RESET}");

    let current = env!("CARGO_PKG_VERSION");
    println!();
    println!("  {YELLOW}This will roll back Icefall v{current} to the previous version.{RESET}");
    println!("  The database will also be restored from the pre-update backup.");

    if !yes {
        println!();
        if !confirm("  Proceed with rollback?") {
            println!("  Rollback cancelled.");
            return;
        }
    }

    println!();
    print!("  Rolling back...  ");
    let result: serde_json::Value = match client
        .post("/system/update/rollback", &serde_json::json!({}))
        .await
    {
        Ok(v) => v,
        Err(e) => {
            println!("{RED}failed{RESET}");
            eprintln!("  {RED}Error:{RESET} {e}");
            std::process::exit(1);
        }
    };

    let rb_status = result["data"]["status"].as_str().unwrap_or("unknown");
    if rb_status == "rolling_back" {
        println!("{GREEN}done{RESET}");
        println!();
        println!(
            "  {GREEN}Rollback initiated.{RESET} The service will restart with the previous version."
        );
    } else {
        println!("{YELLOW}{rb_status}{RESET}");
    }
}

/// Offline update from local files.
pub async fn from_file(tarball: &str, manifest_path: &str, signature_path: &str, yes: bool) {
    let current = env!("CARGO_PKG_VERSION");

    println!("  {BOLD}Offline update{RESET}");
    println!();

    // Read manifest and signature
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

    // Verify signature
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

    // Parse manifest
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

    // Verify SHA-256 of tarball
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

    // Apply via the daemon API if running, otherwise instruct manual steps
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

    // Upload tarball path info and trigger apply
    println!();
    println!("  {DIM}Tarball verified. Triggering update via daemon...{RESET}");

    // Extract to updates dir, then apply
    let tmp_dir = std::env::temp_dir().join("icefall-offline-update");
    let _ = std::fs::remove_dir_all(&tmp_dir);
    std::fs::create_dir_all(&tmp_dir).ok();

    let tarball_path = std::path::Path::new(tarball);
    let dest_tarball = tmp_dir.join("update.tar.gz");
    if let Err(e) = std::fs::copy(tarball_path, &dest_tarball) {
        eprintln!("  {RED}Error:{RESET} Failed to stage tarball: {e}");
        std::process::exit(1);
    }

    // Extract
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

fn confirm(prompt: &str) -> bool {
    use std::io::{self, Write};

    print!("{prompt} [y/N] ");
    io::stdout().flush().ok();

    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_err() {
        return false;
    }

    matches!(input.trim().to_lowercase().as_str(), "y" | "yes")
}

fn print_json_step(step: &str, status: &str, error: Option<&str>, data: Option<serde_json::Value>) {
    let mut obj = serde_json::json!({
        "step": step,
        "status": status,
    });
    if let Some(e) = error {
        obj["error"] = serde_json::Value::String(e.to_string());
    }
    if let Some(d) = data {
        obj["data"] = d;
    }
    println!("{obj}");
}

fn progress_bar(percent: u32, width: usize) -> String {
    let filled = (percent as usize * width) / 100;
    let empty = width.saturating_sub(filled);
    format!("[{}{}]", "█".repeat(filled), "░".repeat(empty))
}

/// Automated rollback check for `ExecStopPost` in the systemd unit.
///
/// Checks if a pending update marker exists and is recent (< 5 min).
/// If so, executes a full rollback. Operates directly on the filesystem
/// without contacting the daemon (which may be crashed).
pub async fn rollback_check() {
    let data_dir = std::env::var("ICEFALL_DATA_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("/var/lib/icefall"));

    let rb = crate::update::rollback::UpdateRollback::new(&data_dir);
    let exit_code = rb.check_and_rollback();
    std::process::exit(exit_code);
}
