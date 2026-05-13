use std::collections::HashMap;

use super::helpers::{
    parse_image_ref, resolve_restart_policy, resolve_service_command, resolve_service_env,
    resolve_service_ports, resolve_service_volumes,
};
use super::interpolation;
use super::types::{ComposeCommand, ComposeEnvironment, ComposeService};
use super::ComposeDeployer;

#[test]
fn parse_image_ref_with_tag() {
    let (name, tag) = parse_image_ref("postgres:16-alpine");
    assert_eq!(name, "postgres");
    assert_eq!(tag, "16-alpine");
}

#[test]
fn parse_image_ref_without_tag() {
    let (name, tag) = parse_image_ref("nginx");
    assert_eq!(name, "nginx");
    assert_eq!(tag, "latest");
}

#[test]
fn parse_image_ref_with_registry_port() {
    let (name, tag) = parse_image_ref("registry.example.com:5000/myapp");
    assert_eq!(name, "registry.example.com:5000/myapp");
    assert_eq!(tag, "latest");
}

#[test]
fn extract_variables_from_yaml() {
    let yaml = r#"
services:
  db:
    image: postgres:16
    environment:
      POSTGRES_PASSWORD: ${DB_PASSWORD}
      POSTGRES_DB: ${DB_NAME:-mydb}
"#;
    let vars = ComposeDeployer::extract_variables(yaml);
    assert_eq!(vars.len(), 2);
    assert_eq!(vars[0], ("DB_PASSWORD".to_string(), None));
    assert_eq!(vars[1], ("DB_NAME".to_string(), Some("mydb".to_string())));
}

#[test]
fn interpolate_variables() {
    let yaml = "image: ${REGISTRY:-docker.io}/${IMAGE}:${TAG:-latest}";
    let mut env = HashMap::new();
    env.insert("IMAGE".to_string(), "myapp".to_string());

    let result = ComposeDeployer::interpolate(yaml, &env);
    assert_eq!(result, "image: docker.io/myapp:latest");
}

#[test]
fn parse_simple_compose() {
    let yaml = r#"
services:
  web:
    image: nginx:latest
    ports:
      - "80:80"
  db:
    image: postgres:16
    environment:
      POSTGRES_PASSWORD: secret
    volumes:
      - pgdata:/var/lib/postgresql/data

volumes:
  pgdata:
"#;
    let compose = ComposeDeployer::parse(yaml).unwrap();
    assert_eq!(compose.services.len(), 2);
    assert!(compose.services.contains_key("web"));
    assert!(compose.services.contains_key("db"));
    assert_eq!(compose.volumes.len(), 1);
}

#[test]
fn dependency_order_respects_depends_on() {
    let yaml = r#"
services:
  app:
    image: myapp:latest
    depends_on:
      - db
      - redis
  db:
    image: postgres:16
  redis:
    image: redis:7
"#;
    let compose = ComposeDeployer::parse(yaml).unwrap();
    let order = interpolation::dependency_order(&compose);
    let app_idx = order.iter().position(|n| n == "app").unwrap();
    let db_idx = order.iter().position(|n| n == "db").unwrap();
    let redis_idx = order.iter().position(|n| n == "redis").unwrap();
    assert!(db_idx < app_idx);
    assert!(redis_idx < app_idx);
}

// --- resolve_service_env ---

#[test]
fn resolve_env_from_list() {
    let service = ComposeService {
        environment: ComposeEnvironment::List(vec!["FOO=bar".to_string(), "BAZ=qux".to_string()]),
        ..Default::default()
    };
    let env = resolve_service_env(&service);
    assert_eq!(env, vec!["FOO=bar", "BAZ=qux"]);
}

#[test]
fn resolve_env_from_map() {
    let mut map = HashMap::new();
    map.insert("DB_HOST".to_string(), Some("localhost".to_string()));
    map.insert("DB_PORT".to_string(), Some("5432".to_string()));
    let service = ComposeService {
        environment: ComposeEnvironment::Map(map),
        ..Default::default()
    };
    let env = resolve_service_env(&service);
    assert_eq!(env.len(), 2);
    assert!(env.contains(&"DB_HOST=localhost".to_string()));
    assert!(env.contains(&"DB_PORT=5432".to_string()));
}

#[test]
fn resolve_env_from_map_with_none_value() {
    let mut map = HashMap::new();
    map.insert("EMPTY_VAR".to_string(), None);
    let service = ComposeService {
        environment: ComposeEnvironment::Map(map),
        ..Default::default()
    };
    let env = resolve_service_env(&service);
    assert_eq!(env, vec!["EMPTY_VAR="]);
}

#[test]
fn resolve_env_empty() {
    let service = ComposeService {
        environment: ComposeEnvironment::Empty,
        ..Default::default()
    };
    let env = resolve_service_env(&service);
    assert!(env.is_empty());
}

// --- resolve_service_ports ---

#[test]
fn resolve_ports_host_and_container() {
    let service = ComposeService {
        ports: vec!["8080:80".to_string()],
        ..Default::default()
    };
    let ports = resolve_service_ports(&service);
    assert_eq!(ports.len(), 1);
    assert_eq!(ports[0].container_port, 80);
    assert_eq!(ports[0].host_port, Some(8080));
    assert_eq!(ports[0].protocol, "tcp");
}

#[test]
fn resolve_ports_container_only() {
    let service = ComposeService {
        ports: vec!["3000".to_string()],
        ..Default::default()
    };
    let ports = resolve_service_ports(&service);
    assert_eq!(ports.len(), 1);
    assert_eq!(ports[0].container_port, 3000);
    assert_eq!(ports[0].host_port, None);
    assert_eq!(ports[0].protocol, "tcp");
}

#[test]
fn resolve_ports_with_protocol() {
    let service = ComposeService {
        ports: vec!["5353:53/udp".to_string()],
        ..Default::default()
    };
    let ports = resolve_service_ports(&service);
    assert_eq!(ports.len(), 1);
    assert_eq!(ports[0].container_port, 53);
    assert_eq!(ports[0].host_port, Some(5353));
    assert_eq!(ports[0].protocol, "udp");
}

#[test]
fn resolve_ports_container_only_with_protocol() {
    let service = ComposeService {
        ports: vec!["53/udp".to_string()],
        ..Default::default()
    };
    let ports = resolve_service_ports(&service);
    assert_eq!(ports.len(), 1);
    assert_eq!(ports[0].container_port, 53);
    assert_eq!(ports[0].host_port, None);
    assert_eq!(ports[0].protocol, "udp");
}

#[test]
fn resolve_ports_multiple() {
    let service = ComposeService {
        ports: vec!["80:80".to_string(), "443:443".to_string()],
        ..Default::default()
    };
    let ports = resolve_service_ports(&service);
    assert_eq!(ports.len(), 2);
}

#[test]
fn resolve_ports_empty() {
    let service = ComposeService {
        ports: vec![],
        ..Default::default()
    };
    let ports = resolve_service_ports(&service);
    assert!(ports.is_empty());
}

// --- resolve_service_volumes ---

#[test]
fn resolve_volumes_named_volume() {
    let service = ComposeService {
        volumes: vec!["pgdata:/var/lib/postgresql/data".to_string()],
        ..Default::default()
    };
    let vols = resolve_service_volumes(&service, "myapp");
    assert_eq!(vols.len(), 1);
    assert_eq!(vols[0].source, "icefall-myapp-pgdata");
    assert_eq!(vols[0].target, "/var/lib/postgresql/data");
    assert!(!vols[0].read_only);
}

#[test]
fn resolve_volumes_bind_mount_absolute() {
    let service = ComposeService {
        volumes: vec!["/host/path:/container/path".to_string()],
        ..Default::default()
    };
    let vols = resolve_service_volumes(&service, "myapp");
    assert_eq!(vols.len(), 1);
    assert_eq!(vols[0].source, "/host/path");
    assert_eq!(vols[0].target, "/container/path");
}

#[test]
fn resolve_volumes_bind_mount_relative() {
    let service = ComposeService {
        volumes: vec!["./data:/app/data".to_string()],
        ..Default::default()
    };
    let vols = resolve_service_volumes(&service, "myapp");
    assert_eq!(vols.len(), 1);
    assert_eq!(vols[0].source, "./data");
    assert_eq!(vols[0].target, "/app/data");
}

#[test]
fn resolve_volumes_read_only() {
    let service = ComposeService {
        volumes: vec!["config:/etc/app/config:ro".to_string()],
        ..Default::default()
    };
    let vols = resolve_service_volumes(&service, "myapp");
    assert_eq!(vols.len(), 1);
    assert!(vols[0].read_only);
}

#[test]
fn resolve_volumes_no_colon_skipped() {
    let service = ComposeService {
        volumes: vec!["justvolumename".to_string()],
        ..Default::default()
    };
    let vols = resolve_service_volumes(&service, "myapp");
    assert!(vols.is_empty());
}

#[test]
fn resolve_volumes_empty() {
    let service = ComposeService {
        volumes: vec![],
        ..Default::default()
    };
    let vols = resolve_service_volumes(&service, "myapp");
    assert!(vols.is_empty());
}

// --- resolve_service_command ---

#[test]
fn resolve_command_simple_string() {
    let service = ComposeService {
        command: Some(ComposeCommand::Simple("python app.py --debug".to_string())),
        ..Default::default()
    };
    let cmd = resolve_service_command(&service);
    assert_eq!(
        cmd,
        Some(vec![
            "python".to_string(),
            "app.py".to_string(),
            "--debug".to_string()
        ])
    );
}

#[test]
fn resolve_command_args_list() {
    let service = ComposeService {
        command: Some(ComposeCommand::Args(vec![
            "node".to_string(),
            "server.js".to_string(),
        ])),
        ..Default::default()
    };
    let cmd = resolve_service_command(&service);
    assert_eq!(cmd, Some(vec!["node".to_string(), "server.js".to_string()]));
}

#[test]
fn resolve_command_none() {
    let service = ComposeService {
        command: None,
        ..Default::default()
    };
    let cmd = resolve_service_command(&service);
    assert!(cmd.is_none());
}

// --- resolve_restart_policy ---

#[test]
fn resolve_restart_explicit() {
    let service = ComposeService {
        restart: Some("always".to_string()),
        ..Default::default()
    };
    assert_eq!(resolve_restart_policy(&service), "always");
}

#[test]
fn resolve_restart_default() {
    let service = ComposeService {
        restart: None,
        ..Default::default()
    };
    assert_eq!(resolve_restart_policy(&service), "unless-stopped");
}

// --- additional interpolation edge cases ---

#[test]
fn extract_variables_deduplicates() {
    let yaml = "${FOO} and ${FOO} again";
    let vars = interpolation::extract_variables(yaml);
    assert_eq!(vars.len(), 1);
    assert_eq!(vars[0].0, "FOO");
}

#[test]
fn extract_variables_no_variables() {
    let yaml = "image: nginx:latest";
    let vars = interpolation::extract_variables(yaml);
    assert!(vars.is_empty());
}

#[test]
fn interpolate_missing_var_no_default_is_empty() {
    let yaml = "host: ${MISSING}";
    let env = HashMap::new();
    let result = interpolation::interpolate(yaml, &env);
    assert_eq!(result, "host: ");
}

#[test]
fn interpolate_provided_value_overrides_default() {
    let yaml = "port: ${PORT:-3000}";
    let mut env = HashMap::new();
    env.insert("PORT".to_string(), "8080".to_string());
    let result = interpolation::interpolate(yaml, &env);
    assert_eq!(result, "port: 8080");
}

// --- additional dependency_order edge cases ---

#[test]
fn dependency_order_no_deps() {
    let yaml = r#"
services:
  alpha:
    image: alpha:1
  beta:
    image: beta:1
"#;
    let compose = ComposeDeployer::parse(yaml).unwrap();
    let order = interpolation::dependency_order(&compose);
    assert_eq!(order.len(), 2);
    // Both should appear (order is alphabetical when no deps)
    assert!(order.contains(&"alpha".to_string()));
    assert!(order.contains(&"beta".to_string()));
}

#[test]
fn dependency_order_chain() {
    let yaml = r#"
services:
  c:
    image: c:1
    depends_on:
      - b
  b:
    image: b:1
    depends_on:
      - a
  a:
    image: a:1
"#;
    let compose = ComposeDeployer::parse(yaml).unwrap();
    let order = interpolation::dependency_order(&compose);
    let a_idx = order.iter().position(|n| n == "a").unwrap();
    let b_idx = order.iter().position(|n| n == "b").unwrap();
    let c_idx = order.iter().position(|n| n == "c").unwrap();
    assert!(a_idx < b_idx);
    assert!(b_idx < c_idx);
}

#[test]
fn extract_variables_with_empty_default() {
    let yaml = "val: ${EMPTY:-}";
    let vars = interpolation::extract_variables(yaml);
    assert_eq!(vars.len(), 1);
    assert_eq!(vars[0], ("EMPTY".to_string(), Some(String::new())));
}

#[test]
fn interpolate_empty_default_produces_empty_string() {
    let yaml = "val: ${UNSET:-}";
    let env = HashMap::new();
    let result = interpolation::interpolate(yaml, &env);
    assert_eq!(result, "val: ");
}

#[test]
fn interpolate_preserves_literal_dollar_without_brace() {
    let yaml = "price: $5 or ${CURRENCY:-USD}";
    let env = HashMap::new();
    let result = interpolation::interpolate(yaml, &env);
    assert_eq!(result, "price: $5 or USD");
}

#[test]
fn dependency_order_map_style_depends_on() {
    let yaml = r#"
services:
  app:
    image: app:1
    depends_on:
      db:
        condition: service_healthy
  db:
    image: postgres:16
"#;
    let compose = ComposeDeployer::parse(yaml).unwrap();
    let order = interpolation::dependency_order(&compose);
    let app_idx = order.iter().position(|n| n == "app").unwrap();
    let db_idx = order.iter().position(|n| n == "db").unwrap();
    assert!(db_idx < app_idx);
}

// --- parse_image_ref additional edge cases ---

#[test]
fn parse_image_ref_registry_with_tag() {
    let (name, tag) = parse_image_ref("ghcr.io/owner/image:v1.2.3");
    assert_eq!(name, "ghcr.io/owner/image");
    assert_eq!(tag, "v1.2.3");
}

#[test]
fn parse_image_ref_registry_without_tag() {
    let (name, tag) = parse_image_ref("ghcr.io/owner/image");
    assert_eq!(name, "ghcr.io/owner/image");
    assert_eq!(tag, "latest");
}
