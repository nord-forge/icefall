use crate::cli::client::CliClient;

use super::helpers::confirm;
use super::*;

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

pub async fn rollback_check() {
    let data_dir = std::env::var("ICEFALL_DATA_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("/var/lib/icefall"));

    let rb = crate::update::rollback::UpdateRollback::new(&data_dir);
    let exit_code = rb.check_and_rollback();
    std::process::exit(exit_code);
}
