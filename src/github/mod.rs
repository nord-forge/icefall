pub mod auth;
pub mod client;
pub mod pr_comment;
pub mod refresh;
pub mod status;
pub mod token;
pub mod webhook_setup;

/// Parse "owner/name" from a git repo URL (https or ssh form) by taking the last
/// two path segments. Used only for GitHub.com-style repos linked via a GitHub App.
pub fn owner_repo(git_repo: &str) -> Option<(String, String)> {
    let trimmed = git_repo
        .trim()
        .trim_end_matches('/')
        .trim_end_matches(".git")
        .trim_end_matches('/');
    let normalized = trimmed.replace(':', "/");
    let parts: Vec<&str> = normalized.split('/').filter(|s| !s.is_empty()).collect();
    if parts.len() >= 2 {
        Some((
            parts[parts.len() - 2].to_string(),
            parts[parts.len() - 1].to_string(),
        ))
    } else {
        None
    }
}

#[cfg(test)]
mod owner_repo_tests {
    use super::owner_repo;

    #[test]
    fn parses_https_ssh_and_suffixes() {
        assert_eq!(
            owner_repo("https://github.com/acme/widget"),
            Some(("acme".into(), "widget".into()))
        );
        assert_eq!(
            owner_repo("https://github.com/acme/widget.git"),
            Some(("acme".into(), "widget".into()))
        );
        assert_eq!(
            owner_repo("git@github.com:acme/widget.git"),
            Some(("acme".into(), "widget".into()))
        );
        assert_eq!(
            owner_repo("https://github.com/acme/widget.git/"),
            Some(("acme".into(), "widget".into()))
        );
    }

    #[test]
    fn rejects_unparseable() {
        assert_eq!(owner_repo("widget"), None);
        assert_eq!(owner_repo(""), None);
    }
}
