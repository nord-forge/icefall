use crate::cli::client::CliClient;

use super::helpers::print_json_step;
use super::*;

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
