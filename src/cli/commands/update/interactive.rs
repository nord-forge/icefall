use crate::cli::client::CliClient;

use super::helpers::*;
use super::*;

pub async fn run(yes: bool, channel: Option<&str>, json: bool) {
    let client = CliClient::new_or_exit();
    let current = env!("CARGO_PKG_VERSION");

    if let Some(ch) = channel {
        let _ = client
            .post::<serde_json::Value>(
                "/system/update/preferences",
                &serde_json::json!({ "channel": ch }),
            )
            .await;
    }

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
                Some(serde_json::json!({ "up_to_date": true, "version": current })),
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

    if !yes && !json {
        if !confirm("  Apply this update?") {
            println!("  Update cancelled.");
            return;
        }
        println!();
    }

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
                        Some(serde_json::json!({ "progress": progress })),
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
                    println!(
                        "\r  {GREEN}{STEP_DONE}{RESET} Downloading update        {GREEN}done{RESET}  "
                    );
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
                Some(serde_json::json!({ "version": latest })),
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
