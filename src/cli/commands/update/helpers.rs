pub(super) fn confirm(prompt: &str) -> bool {
    use std::io::{self, Write};

    print!("{prompt} [y/N] ");
    io::stdout().flush().ok();

    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_err() {
        return false;
    }

    matches!(input.trim().to_lowercase().as_str(), "y" | "yes")
}

pub(super) fn print_json_step(
    step: &str,
    status: &str,
    error: Option<&str>,
    data: Option<serde_json::Value>,
) {
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

pub(super) fn progress_bar(percent: u32, width: usize) -> String {
    let filled = (percent as usize * width) / 100;
    let empty = width.saturating_sub(filled);
    format!("[{}{}]", "█".repeat(filled), "░".repeat(empty))
}
