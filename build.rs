use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs");

    let commit = Command::new("git")
        .args(["rev-parse", "--short=7", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success()).map_or_else(|| "unknown".into(), |o| String::from_utf8_lossy(&o.stdout).trim().to_string());

    let build_date = Command::new("date")
        .args(["-u", "+%Y-%m-%d"])
        .output()
        .ok()
        .filter(|o| o.status.success()).map_or_else(|| "unknown".into(), |o| String::from_utf8_lossy(&o.stdout).trim().to_string());

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
