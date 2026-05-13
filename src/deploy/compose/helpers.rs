use crate::docker::containers::{PortMapping, VolumeMount};

use super::types::{ComposeCommand, ComposeEnvironment, ComposeService};

/// Parse "nginx:latest" into ("nginx", "latest"), "postgres" into ("postgres", "latest").
pub(super) fn parse_image_ref(image: &str) -> (String, String) {
    if let Some((name, tag)) = image.rsplit_once(':') {
        // Guard against registry URLs like registry.example.com:5000/image
        if tag.contains('/') {
            (image.to_string(), "latest".to_string())
        } else {
            (name.to_string(), tag.to_string())
        }
    } else {
        (image.to_string(), "latest".to_string())
    }
}

/// Extract environment variables from a compose service.
pub(super) fn resolve_service_env(service: &ComposeService) -> Vec<String> {
    match &service.environment {
        ComposeEnvironment::List(list) => list.clone(),
        ComposeEnvironment::Map(map) => map
            .iter()
            .map(|(k, v)| {
                let val = v.as_deref().unwrap_or("");
                format!("{k}={val}")
            })
            .collect(),
        ComposeEnvironment::Empty => Vec::new(),
    }
}

/// Extract port mappings from a compose service.
pub(super) fn resolve_service_ports(service: &ComposeService) -> Vec<PortMapping> {
    service
        .ports
        .iter()
        .filter_map(|p| {
            let p = p.trim();
            if p.contains(':') {
                let parts: Vec<&str> = p.split(':').collect();
                if parts.len() >= 2 {
                    let host_part = parts[parts.len() - 2];
                    let container_part = parts[parts.len() - 1];
                    let (container_port_str, protocol) =
                        if let Some((port, proto)) = container_part.rsplit_once('/') {
                            (port, proto)
                        } else {
                            (container_part, "tcp")
                        };
                    let container_port = container_port_str.parse::<u16>().ok()?;
                    let host_port = host_part.parse::<u16>().ok();
                    Some(PortMapping {
                        container_port,
                        host_port,
                        protocol: protocol.to_string(),
                    })
                } else {
                    None
                }
            } else {
                let (port_str, protocol) = if let Some((port, proto)) = p.rsplit_once('/') {
                    (port, proto)
                } else {
                    (p, "tcp")
                };
                let container_port = port_str.parse::<u16>().ok()?;
                Some(PortMapping {
                    container_port,
                    host_port: None,
                    protocol: protocol.to_string(),
                })
            }
        })
        .collect()
}

/// Extract volume mounts, prefixing named volumes with the app name.
pub(super) fn resolve_service_volumes(
    service: &ComposeService,
    app_name: &str,
) -> Vec<VolumeMount> {
    service
        .volumes
        .iter()
        .filter_map(|v| {
            let v = v.trim();
            if v.contains(':') {
                let parts: Vec<&str> = v.splitn(3, ':').collect();
                if parts.len() >= 2 {
                    let source = parts[0];
                    let target = parts[1];
                    let read_only = parts.get(2).is_some_and(|&s| s == "ro");

                    let resolved_source = if source.starts_with('/') || source.starts_with('.') {
                        source.to_string()
                    } else {
                        format!("icefall-{}-{}", app_name, source)
                    };

                    Some(VolumeMount {
                        source: resolved_source,
                        target: target.to_string(),
                        read_only,
                    })
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect()
}

/// Extract command override from a compose service.
pub(super) fn resolve_service_command(service: &ComposeService) -> Option<Vec<String>> {
    match &service.command {
        Some(ComposeCommand::Simple(s)) => Some(s.split_whitespace().map(String::from).collect()),
        Some(ComposeCommand::Args(args)) => Some(args.clone()),
        None => None,
    }
}

/// Extract restart policy from a compose service.
pub(super) fn resolve_restart_policy(service: &ComposeService) -> String {
    service
        .restart
        .as_deref()
        .unwrap_or("unless-stopped")
        .to_string()
}
