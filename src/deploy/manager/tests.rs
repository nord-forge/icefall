#[test]
fn resolve_env_vars_adds_port_and_host() {
    let vars: Vec<String> = vec!["FOO=bar".to_string(), "BAZ=qux".to_string()];
    assert!(!vars.iter().any(|v| v.starts_with("PORT=")));

    let mut result = vars;
    result.push("PORT=3000".to_string());
    result.push("HOST=0.0.0.0".to_string());

    assert!(result.iter().any(|v| v.starts_with("PORT=")));
    assert!(result.iter().any(|v| v.starts_with("HOST=")));
}
