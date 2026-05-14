mod apps;
mod audit;
mod auth;
mod databases;
mod deploys;
mod domains;
mod env_vars;
mod environments;
mod github;
mod health;
mod notifications;
mod projects;
mod public_ports;
mod registries;
mod servers;
mod settings;
mod ssh_keys;
mod updates;
mod users;
mod webhooks;

pub use apps::*;
pub use audit::*;
pub use auth::*;
pub use databases::*;
pub use deploys::*;
pub use domains::*;
pub use env_vars::*;
pub use environments::*;
pub use github::*;
pub use health::*;
pub use notifications::*;
pub use projects::*;
pub use public_ports::*;
pub use registries::*;
pub use servers::*;
pub use settings::*;
pub use ssh_keys::*;
pub use updates::*;
pub use users::*;
pub use webhooks::*;

use chrono::Utc;

pub fn now_iso8601() -> String {
    Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

pub fn new_id() -> String {
    use rand::Rng;
    let mut rng = rand::rng();
    let chars: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
    (0..20)
        .map(|_| chars[rng.random_range(0..chars.len())] as char)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_id_is_20_chars() {
        let id = new_id();
        assert_eq!(id.len(), 20);
    }

    #[test]
    fn new_id_only_contains_lowercase_alphanumeric() {
        let id = new_id();
        assert!(id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()));
    }

    #[test]
    fn new_id_generates_unique_values() {
        let ids: std::collections::HashSet<String> = (0..100).map(|_| new_id()).collect();
        assert_eq!(ids.len(), 100);
    }

    #[test]
    fn now_iso8601_is_valid_rfc3339() {
        let ts = now_iso8601();
        assert!(chrono::DateTime::parse_from_rfc3339(&ts).is_ok());
    }

    #[test]
    fn now_iso8601_ends_with_z() {
        let ts = now_iso8601();
        assert!(ts.ends_with('Z'));
    }

    #[test]
    fn now_iso8601_has_millisecond_precision() {
        let ts = now_iso8601();
        let dot_pos = ts.rfind('.').expect("should have decimal point");
        let frac = &ts[dot_pos + 1..ts.len() - 1];
        assert_eq!(frac.len(), 3);
    }

    #[test]
    fn now_iso8601_is_recent() {
        let ts = now_iso8601();
        let parsed = chrono::DateTime::parse_from_rfc3339(&ts).unwrap();
        let now = chrono::Utc::now();
        let diff = now.signed_duration_since(parsed);
        assert!(diff.num_seconds().abs() < 5);
    }

    #[test]
    fn new_id_no_uppercase() {
        for _ in 0..50 {
            let id = new_id();
            assert_eq!(id, id.to_lowercase());
        }
    }

    #[test]
    fn new_id_is_not_empty() {
        let id = new_id();
        assert!(!id.is_empty());
    }
}
