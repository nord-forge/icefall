use sha2::{Digest, Sha256};

use crate::db::models::App;

pub fn compute_config_hash(app: &App, env_vars: &[(String, String)], domains: &[String]) -> String {
    let mut hasher = Sha256::new();

    if let Some(ref bc) = app.build_config {
        hasher.update(bc.as_bytes());
    }
    if let Some(ref rl) = app.resource_limits {
        hasher.update(rl.as_bytes());
    }
    if let Some(ref v) = app.volumes {
        hasher.update(v.as_bytes());
    }
    if let Some(ref img) = app.image_ref {
        hasher.update(img.as_bytes());
    }
    if let Some(ref cc) = app.compose_content {
        hasher.update(cc.as_bytes());
    }

    hasher.update(app.git_branch.as_bytes());
    if let Some(ref repo) = app.git_repo {
        hasher.update(repo.as_bytes());
    }
    hasher.update(app.deploy_mode.as_bytes());

    let mut sorted_vars = env_vars.to_vec();
    sorted_vars.sort_by(|a, b| a.0.cmp(&b.0));
    for (key, value) in &sorted_vars {
        hasher.update(key.as_bytes());
        hasher.update(b"=");
        hasher.update(value.as_bytes());
        hasher.update(b"\n");
    }

    let mut sorted_domains = domains.to_vec();
    sorted_domains.sort();
    for domain in &sorted_domains {
        hasher.update(domain.as_bytes());
        hasher.update(b"\n");
    }

    let result = hasher.finalize();
    result.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_app() -> App {
        App {
            id: "test".into(),
            name: "my-app".into(),
            git_repo: Some("https://github.com/user/repo".into()),
            git_branch: "main".into(),
            framework: None,
            build_config: Some(r#"{"build_command":"npm run build"}"#.into()),
            resource_limits: Some(r#"{"memory_bytes":536870912}"#.into()),
            preview_enabled: false,
            preview_branch_pattern: None,
            webhook_secret: None,
            tags: None,
            volumes: None,
            image_ref: None,
            compose_content: None,
            project_id: None,
            deploy_mode: "auto".into(),
            server_id: None,
            base_directory: None,
            disable_build_cache: false,
            git_submodules_enabled: false,
            git_lfs_enabled: false,
            git_shallow_clone: true,
            basic_auth_enabled: false,
            basic_auth_username: None,
            basic_auth_password_hash: None,
            pre_deploy_commands: None,
            post_deploy_commands: None,
            ssh_key_id: None,
            last_request_at: None,
            exempt_from_inactivity: false,
            require_deploy_approval: false,
            canary_enabled: false,
            canary_config: None,
            drift_monitoring_enabled: true,
            log_noise_patterns: None,
            log_highlight_patterns: None,
            ghost_mode_enabled: false,
            ghost_mode_idle_minutes: 30,
            ghost_mode_status: "active".into(),
            status_page_enabled: false,
            power_nap_priority: "standard".into(),
            power_nap_custom_schedule: None,
            created_at: "2026-01-01".into(),
            updated_at: "2026-01-01".into(),
        }
    }

    #[test]
    fn deterministic_output() {
        let app = make_app();
        let vars = vec![("KEY".into(), "val".into())];
        let domains = vec!["app.example.com".into()];
        let h1 = compute_config_hash(&app, &vars, &domains);
        let h2 = compute_config_hash(&app, &vars, &domains);
        assert_eq!(h1, h2);
    }

    #[test]
    fn env_var_order_independent() {
        let app = make_app();
        let vars_a = vec![("A".into(), "1".into()), ("B".into(), "2".into())];
        let vars_b = vec![("B".into(), "2".into()), ("A".into(), "1".into())];
        let h1 = compute_config_hash(&app, &vars_a, &[]);
        let h2 = compute_config_hash(&app, &vars_b, &[]);
        assert_eq!(h1, h2);
    }

    #[test]
    fn domain_order_independent() {
        let app = make_app();
        let d1 = vec!["a.example.com".into(), "b.example.com".into()];
        let d2 = vec!["b.example.com".into(), "a.example.com".into()];
        let h1 = compute_config_hash(&app, &[], &d1);
        let h2 = compute_config_hash(&app, &[], &d2);
        assert_eq!(h1, h2);
    }

    #[test]
    fn changes_on_env_var_change() {
        let app = make_app();
        let v1 = vec![("KEY".into(), "old".into())];
        let v2 = vec![("KEY".into(), "new".into())];
        let h1 = compute_config_hash(&app, &v1, &[]);
        let h2 = compute_config_hash(&app, &v2, &[]);
        assert_ne!(h1, h2);
    }

    #[test]
    fn changes_on_build_config_change() {
        let mut app = make_app();
        let h1 = compute_config_hash(&app, &[], &[]);
        app.build_config = Some(r#"{"build_command":"npm run build:prod"}"#.into());
        let h2 = compute_config_hash(&app, &[], &[]);
        assert_ne!(h1, h2);
    }

    #[test]
    fn ignores_non_deploy_fields() {
        let mut app = make_app();
        let h1 = compute_config_hash(&app, &[], &[]);
        app.name = "renamed-app".into();
        app.tags = Some("tag1,tag2".into());
        app.webhook_secret = Some("new-secret".into());
        let h2 = compute_config_hash(&app, &[], &[]);
        assert_eq!(h1, h2);
    }
}
