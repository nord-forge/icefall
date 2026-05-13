use std::io::Read as _;
use std::path::Path;
use std::process::Command;

use sha2::Digest;

pub(super) fn find_config() -> Option<String> {
    let paths = [
        "/etc/icefall/config.toml".to_string(),
        dirs::config_dir()
            .unwrap_or_default()
            .join("icefall/config.toml")
            .to_string_lossy()
            .to_string(),
    ];
    paths.into_iter().find(|p| Path::new(p).exists())
}

pub(super) fn is_daemon_running() -> bool {
    Command::new("systemctl")
        .args(["is-active", "--quiet", "icefall"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub(super) fn sha256_file(path: &Path) -> String {
    let mut file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return String::new(),
    };
    let mut hasher = sha2::Sha256::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = file.read(&mut buf).unwrap_or(0);
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    hex::encode(hasher.finalize())
}

pub(super) fn format_bytes(bytes: u64) -> String {
    if bytes == 0 {
        return "0 B".to_string();
    }
    let units = ["B", "KB", "MB", "GB"];
    let i = (bytes as f64).log(1024.0).floor() as usize;
    let i = i.min(units.len() - 1);
    let value = bytes as f64 / 1024_f64.powi(i as i32);
    if value < 10.0 {
        format!("{value:.1} {}", units[i])
    } else {
        format!("{value:.0} {}", units[i])
    }
}

pub(super) fn file_size(path: &Path) -> u64 {
    std::fs::metadata(path).map(|m| m.len()).unwrap_or(0)
}

pub(super) fn dir_size(path: &Path) -> u64 {
    if !path.exists() {
        return 0;
    }
    walkdir(path)
}

fn walkdir(path: &Path) -> u64 {
    let mut total = 0;
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                total += walkdir(&p);
            } else {
                total += entry.metadata().map(|m| m.len()).unwrap_or(0);
            }
        }
    }
    total
}

pub(super) fn export_managed_database_dumps(dumps_dir: &Path) {
    let db_containers = list_icefall_db_containers();

    if db_containers.is_empty() {
        println!("    No managed database containers found");
        return;
    }

    for (name, db_type) in &db_containers {
        let dump_file = dumps_dir.join(format!("{name}.sql.gz"));
        let dump_cmd = match db_type.as_str() {
            t if t.contains("postgres") => {
                format!(
                    "docker exec {name} pg_dumpall -U icefall | gzip > {}",
                    dump_file.display()
                )
            }
            t if t.contains("mysql") => {
                format!(
                    "docker exec {name} mysqldump -u icefall --all-databases | gzip > {}",
                    dump_file.display()
                )
            }
            t if t.contains("mongo") => {
                format!(
                    "docker exec {name} mongodump --archive --gzip --username icefall > {}",
                    dump_file.display()
                )
            }
            t if t.contains("redis") => {
                let rdb_file = dumps_dir.join(format!("{name}.rdb"));
                format!(
                    "docker exec {name} redis-cli BGSAVE && sleep 2 && docker cp {name}:/data/dump.rdb {}",
                    rdb_file.display()
                )
            }
            _ => continue,
        };

        print!("    Dumping {name} ({db_type})...");
        let result = Command::new("sh").arg("-c").arg(&dump_cmd).output();
        match result {
            Ok(out) if out.status.success() => println!(" ✓"),
            Ok(out) => println!(" ✗ {}", String::from_utf8_lossy(&out.stderr).trim()),
            Err(e) => println!(" ✗ {e}"),
        }
    }
}

pub(super) fn export_docker_volumes(volumes_dir: &Path) {
    let volumes = list_icefall_volumes();

    if volumes.is_empty() {
        println!("    No icefall volumes found");
        return;
    }

    for volume in &volumes {
        let tar_path = volumes_dir.join(format!("{volume}.tar.gz"));
        print!("    Exporting volume {volume}...");

        let result = Command::new("docker")
            .args([
                "run",
                "--rm",
                "-v",
                &format!("{volume}:/data"),
                "-v",
                &format!("{}:/backup", volumes_dir.display()),
                "alpine",
                "tar",
                "czf",
                &format!("/backup/{volume}.tar.gz"),
                "-C",
                "/data",
                ".",
            ])
            .output();

        match result {
            Ok(out) if out.status.success() => {
                let size = std::fs::metadata(&tar_path).map(|m| m.len()).unwrap_or(0);
                println!(" ✓ ({})", format_bytes(size));
            }
            Ok(out) => println!(" ✗ {}", String::from_utf8_lossy(&out.stderr).trim()),
            Err(e) => println!(" ✗ {e}"),
        }
    }
}

pub(super) fn import_docker_volumes(volumes_dir: &Path) {
    let entries: Vec<_> = std::fs::read_dir(volumes_dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().extension().map(|ext| ext == "gz").unwrap_or(false))
        .collect();

    if entries.is_empty() {
        println!("    No volume snapshots to restore");
        return;
    }

    for entry in &entries {
        let filename = entry.file_name().to_string_lossy().to_string();
        let volume_name = filename.trim_end_matches(".tar.gz");
        print!("    Restoring volume {volume_name}...");

        let _ = Command::new("docker")
            .args(["volume", "create", volume_name])
            .output();

        let result = Command::new("docker")
            .args([
                "run",
                "--rm",
                "-v",
                &format!("{volume_name}:/data"),
                "-v",
                &format!("{}:/backup", volumes_dir.display()),
                "alpine",
                "sh",
                "-c",
                &format!("cd /data && tar xzf /backup/{filename}"),
            ])
            .output();

        match result {
            Ok(out) if out.status.success() => println!(" ✓"),
            Ok(out) => println!(" ✗ {}", String::from_utf8_lossy(&out.stderr).trim()),
            Err(e) => println!(" ✗ {e}"),
        }
    }
}

pub(super) fn list_icefall_db_containers() -> Vec<(String, String)> {
    let output = Command::new("docker")
        .args([
            "ps",
            "--filter",
            "label=icefall.managed-db=true",
            "--format",
            "{{.Names}}\t{{.Image}}",
        ])
        .output();

    match output {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|line| {
                let parts: Vec<&str> = line.split('\t').collect();
                if parts.len() >= 2 {
                    Some((parts[0].to_string(), parts[1].to_string()))
                } else {
                    None
                }
            })
            .collect(),
        _ => Vec::new(),
    }
}

pub(super) fn list_icefall_volumes() -> Vec<String> {
    let output = Command::new("docker")
        .args([
            "volume",
            "ls",
            "--filter",
            "name=icefall-db-",
            "--format",
            "{{.Name}}",
        ])
        .output();

    match output {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout)
            .lines()
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty())
            .collect(),
        _ => Vec::new(),
    }
}

pub(super) fn copy_or_exit(src: &Path, dst: &Path) {
    std::fs::copy(src, dst).unwrap_or_else(|e| {
        eprintln!("Failed to copy {} → {}: {e}", src.display(), dst.display());
        std::process::exit(1);
    });
}

pub(super) fn copy_dir_recursive(src: &Path, dst: &Path) {
    std::fs::create_dir_all(dst).ok();
    if let Ok(entries) = std::fs::read_dir(src) {
        for entry in entries.flatten() {
            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());
            if src_path.is_dir() {
                copy_dir_recursive(&src_path, &dst_path);
            } else {
                let _ = std::fs::copy(&src_path, &dst_path);
            }
        }
    }
}
