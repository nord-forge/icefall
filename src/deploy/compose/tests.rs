use std::collections::HashMap;

use super::helpers::parse_image_ref;
use super::interpolation;
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
