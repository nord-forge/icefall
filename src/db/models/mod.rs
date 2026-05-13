mod apps;
mod audit;
mod auth;
mod databases;
mod deploys;
mod domains;
mod env_vars;
mod environments;
mod health;
mod notifications;
mod projects;
mod servers;
mod settings;
mod updates;
mod users;

pub use apps::*;
pub use audit::*;
pub use auth::*;
pub use databases::*;
pub use deploys::*;
pub use domains::*;
pub use env_vars::*;
pub use environments::*;
pub use health::*;
pub use notifications::*;
pub use projects::*;
pub use servers::*;
pub use settings::*;
pub use updates::*;
pub use users::*;

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
