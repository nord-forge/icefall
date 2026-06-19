//! Detect (and optionally strip) coupling to a foreign hosting platform in a
//! pasted compose file: external proxy networks, foreign routing labels, and
//! proxy-control sidecars. See AC6 in docs/plans/git-deploy-detection.md.

use serde::Serialize;

/// Label prefixes treated as foreign routing labels. Configurable so new
/// platforms can be added without code changes.
pub const DEFAULT_FOREIGN_LABEL_PREFIXES: &[&str] = &["traefik."];

/// Container-socket paths that, when bind-mounted by a service, mark it a
/// candidate proxy-control sidecar.
const SOCKET_PATHS: &[&str] = &["/var/run/docker.sock", "/run/podman/podman.sock"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ForeignLabels {
    pub service: String,
    pub keys: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct ForeignCoupling {
    /// Names of top-level networks marked `external: true`.
    pub external_networks: Vec<String>,
    /// Per-service routing labels matching a foreign prefix.
    pub foreign_labels: Vec<ForeignLabels>,
    /// Services that mount a container socket and look like proxy controllers.
    pub proxy_sidecars: Vec<String>,
}

impl ForeignCoupling {
    pub fn is_empty(&self) -> bool {
        self.external_networks.is_empty()
            && self.foreign_labels.is_empty()
            && self.proxy_sidecars.is_empty()
    }
}

/// User-selected removals to apply in `strip_foreign_coupling`.
#[derive(Debug, Clone, Default)]
pub struct StripSelection {
    pub external_networks: Vec<String>,
    pub label_services: Vec<String>,
    pub sidecar_services: Vec<String>,
}

impl StripSelection {
    /// Select everything found — the common "strip all coupling" path.
    pub fn all(c: &ForeignCoupling) -> Self {
        Self {
            external_networks: c.external_networks.clone(),
            label_services: c.foreign_labels.iter().map(|l| l.service.clone()).collect(),
            sidecar_services: c.proxy_sidecars.clone(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ComposeAuditError {
    #[error("invalid compose YAML: {0}")]
    Parse(String),
}

/// Inspect a compose file for foreign-platform coupling. Pure; no I/O.
pub fn analyze_foreign_coupling(yaml: &str) -> Result<ForeignCoupling, ComposeAuditError> {
    analyze_with_prefixes(yaml, DEFAULT_FOREIGN_LABEL_PREFIXES)
}

pub fn analyze_with_prefixes(
    yaml: &str,
    label_prefixes: &[&str],
) -> Result<ForeignCoupling, ComposeAuditError> {
    let doc: serde_yaml::Value =
        serde_yaml::from_str(yaml).map_err(|e| ComposeAuditError::Parse(e.to_string()))?;

    Ok(ForeignCoupling {
        external_networks: external_networks(&doc),
        foreign_labels: foreign_labels(&doc, label_prefixes),
        proxy_sidecars: proxy_sidecars(&doc),
    })
}

fn external_networks(doc: &serde_yaml::Value) -> Vec<String> {
    let Some(nets) = doc.get("networks").and_then(|n| n.as_mapping()) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (name, def) in nets {
        let external = def
            .get("external")
            .map(|v| v.as_bool() == Some(true) || v.is_mapping())
            .unwrap_or(false);
        if external {
            if let Some(name) = name.as_str() {
                out.push(name.to_string());
            }
        }
    }
    out.sort();
    out
}

fn foreign_labels(doc: &serde_yaml::Value, prefixes: &[&str]) -> Vec<ForeignLabels> {
    let Some(services) = doc.get("services").and_then(|s| s.as_mapping()) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (svc_name, svc) in services {
        let Some(svc_name) = svc_name.as_str() else {
            continue;
        };
        let keys: Vec<String> = label_keys(svc)
            .into_iter()
            .filter(|k| prefixes.iter().any(|p| k.starts_with(p)))
            .collect();
        if !keys.is_empty() {
            out.push(ForeignLabels {
                service: svc_name.to_string(),
                keys,
            });
        }
    }
    out.sort_by(|a, b| a.service.cmp(&b.service));
    out
}

/// Compose `labels:` accept both a map and a `KEY=value` sequence — return the
/// label keys from either form.
fn label_keys(svc: &serde_yaml::Value) -> Vec<String> {
    let Some(labels) = svc.get("labels") else {
        return Vec::new();
    };
    if let Some(map) = labels.as_mapping() {
        return map
            .keys()
            .filter_map(|k| k.as_str().map(str::to_string))
            .collect();
    }
    if let Some(seq) = labels.as_sequence() {
        return seq
            .iter()
            .filter_map(|v| v.as_str())
            .map(|s| s.split('=').next().unwrap_or(s).trim().to_string())
            .collect();
    }
    Vec::new()
}

fn proxy_sidecars(doc: &serde_yaml::Value) -> Vec<String> {
    let Some(services) = doc.get("services").and_then(|s| s.as_mapping()) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (svc_name, svc) in services {
        let Some(svc_name) = svc_name.as_str() else {
            continue;
        };
        if mounts_container_socket(svc) {
            out.push(svc_name.to_string());
        }
    }
    out.sort();
    out
}

fn mounts_container_socket(svc: &serde_yaml::Value) -> bool {
    let Some(volumes) = svc.get("volumes").and_then(|v| v.as_sequence()) else {
        return false;
    };
    volumes.iter().any(|v| {
        let src = match v {
            serde_yaml::Value::String(s) => s.split(':').next().unwrap_or("").to_string(),
            serde_yaml::Value::Mapping(_) => v
                .get("source")
                .and_then(|s| s.as_str())
                .unwrap_or("")
                .to_string(),
            _ => String::new(),
        };
        SOCKET_PATHS.iter().any(|p| src == *p)
    })
}

/// Apply the selected removals and re-serialize. Re-parses the result to
/// validate; unselected content is preserved (modulo serde_yaml reformatting).
pub fn strip_foreign_coupling(
    yaml: &str,
    sel: &StripSelection,
) -> Result<String, ComposeAuditError> {
    strip_with_prefixes(yaml, sel, DEFAULT_FOREIGN_LABEL_PREFIXES)
}

pub fn strip_with_prefixes(
    yaml: &str,
    sel: &StripSelection,
    label_prefixes: &[&str],
) -> Result<String, ComposeAuditError> {
    let mut doc: serde_yaml::Value =
        serde_yaml::from_str(yaml).map_err(|e| ComposeAuditError::Parse(e.to_string()))?;

    remove_external_networks(&mut doc, &sel.external_networks);
    remove_sidecar_services(&mut doc, &sel.sidecar_services);
    remove_foreign_labels(&mut doc, &sel.label_services, label_prefixes);
    drop_dangling_depends_on(&mut doc, &sel.sidecar_services);

    let out = serde_yaml::to_string(&doc).map_err(|e| ComposeAuditError::Parse(e.to_string()))?;
    serde_yaml::from_str::<serde_yaml::Value>(&out)
        .map_err(|e| ComposeAuditError::Parse(e.to_string()))?;
    Ok(out)
}

fn remove_external_networks(doc: &mut serde_yaml::Value, names: &[String]) {
    if names.is_empty() {
        return;
    }
    if let Some(nets) = doc.get_mut("networks").and_then(|n| n.as_mapping_mut()) {
        for name in names {
            nets.remove(serde_yaml::Value::String(name.clone()));
        }
    }
    // Drop the now-removed networks from each service's `networks:` reference.
    if let Some(services) = doc.get_mut("services").and_then(|s| s.as_mapping_mut()) {
        for (_, svc) in services.iter_mut() {
            remove_network_refs(svc, names);
        }
    }
}

fn remove_network_refs(svc: &mut serde_yaml::Value, names: &[String]) {
    let Some(nets) = svc.get_mut("networks") else {
        return;
    };
    if let Some(seq) = nets.as_sequence_mut() {
        seq.retain(|v| {
            v.as_str()
                .map(|s| !names.iter().any(|n| n == s))
                .unwrap_or(true)
        });
    } else if let Some(map) = nets.as_mapping_mut() {
        for name in names {
            map.remove(serde_yaml::Value::String(name.clone()));
        }
    }
}

fn remove_sidecar_services(doc: &mut serde_yaml::Value, services: &[String]) {
    if services.is_empty() {
        return;
    }
    if let Some(map) = doc.get_mut("services").and_then(|s| s.as_mapping_mut()) {
        for svc in services {
            map.remove(serde_yaml::Value::String(svc.clone()));
        }
    }
}

fn remove_foreign_labels(doc: &mut serde_yaml::Value, services: &[String], prefixes: &[&str]) {
    if services.is_empty() {
        return;
    }
    let Some(map) = doc.get_mut("services").and_then(|s| s.as_mapping_mut()) else {
        return;
    };
    for svc_name in services {
        if let Some(svc) = map.get_mut(serde_yaml::Value::String(svc_name.clone())) {
            strip_foreign_label_entries(svc, prefixes);
        }
    }
}

fn strip_foreign_label_entries(svc: &mut serde_yaml::Value, prefixes: &[&str]) {
    let Some(labels) = svc.get_mut("labels") else {
        return;
    };
    let is_foreign = |k: &str| prefixes.iter().any(|p| k.starts_with(p));
    if let Some(lmap) = labels.as_mapping_mut() {
        let drop: Vec<serde_yaml::Value> = lmap
            .keys()
            .filter(|k| k.as_str().map(is_foreign).unwrap_or(false))
            .cloned()
            .collect();
        for k in drop {
            lmap.remove(k);
        }
    } else if let Some(seq) = labels.as_sequence_mut() {
        seq.retain(|v| {
            v.as_str()
                .map(|s| !is_foreign(s.split('=').next().unwrap_or(s).trim()))
                .unwrap_or(true)
        });
    }
}

fn drop_dangling_depends_on(doc: &mut serde_yaml::Value, removed: &[String]) {
    if removed.is_empty() {
        return;
    }
    let Some(services) = doc.get_mut("services").and_then(|s| s.as_mapping_mut()) else {
        return;
    };
    for (_, svc) in services.iter_mut() {
        let Some(dep) = svc.get_mut("depends_on") else {
            continue;
        };
        if let Some(seq) = dep.as_sequence_mut() {
            seq.retain(|v| {
                v.as_str()
                    .map(|s| !removed.iter().any(|r| r == s))
                    .unwrap_or(true)
            });
        } else if let Some(map) = dep.as_mapping_mut() {
            for r in removed {
                map.remove(serde_yaml::Value::String(r.clone()));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A compose file authored for a different platform: external proxy network,
    // foreign routing labels, and a socket-mounting proxy-control sidecar.
    const FOREIGN: &str = r#"
services:
  api:
    build:
      context: .
      dockerfile: Dockerfile.api
    networks: [default, edge]
    labels:
      - "router.enable=true"
      - "router.http.routers.api.rule=PathPrefix(`/api`)"
      - "app.role=api"
  web:
    build: { context: ., dockerfile: Dockerfile.web }
    networks: [default, edge]
    labels:
      router.enable: "true"
      router.http.routers.web.rule: "Host(`example.test`)"
  proxy-reload:
    image: docker:cli
    depends_on: [api, web]
    volumes:
      - /var/run/docker.sock:/var/run/docker.sock
  db:
    image: ghcr.io/example/db:latest
    volumes:
      - db-data:/var/lib/db
networks:
  default:
  edge:
    external: true
volumes:
  db-data:
"#;

    // The same shape but using "router." as the foreign prefix keeps the test
    // brand-free; analysis is parameterized on the prefix list.
    fn analyze(yaml: &str) -> ForeignCoupling {
        analyze_with_prefixes(yaml, &["router."]).unwrap()
    }

    #[test]
    fn detects_all_three_couplings() {
        let c = analyze(FOREIGN);
        assert_eq!(c.external_networks, vec!["edge".to_string()]);
        assert_eq!(c.proxy_sidecars, vec!["proxy-reload".to_string()]);
        let svcs: Vec<&str> = c
            .foreign_labels
            .iter()
            .map(|l| l.service.as_str())
            .collect();
        assert_eq!(svcs, vec!["api", "web"]);
        // The non-routing "app.role" label is left untouched.
        let api = c
            .foreign_labels
            .iter()
            .find(|l| l.service == "api")
            .unwrap();
        assert!(api.keys.iter().all(|k| k.starts_with("router.")));
        assert_eq!(api.keys.len(), 2);
    }

    #[test]
    fn clean_compose_flags_nothing() {
        let yaml = r#"
services:
  minio:
    image: minio/minio:latest
    ports: ["9000:9000"]
    volumes: [minio-data:/data]
  db:
    image: ghcr.io/example/db:latest
volumes:
  minio-data:
"#;
        let c = analyze(yaml);
        assert!(c.is_empty());
    }

    #[test]
    fn strip_removes_selected_and_validates() {
        let c = analyze(FOREIGN);
        let stripped = {
            let sel = StripSelection {
                external_networks: c.external_networks.clone(),
                label_services: c.foreign_labels.iter().map(|l| l.service.clone()).collect(),
                sidecar_services: c.proxy_sidecars.clone(),
            };
            // Strip uses the default prefix; align fixture prefix for this test.
            super::strip_with_prefixes(FOREIGN, &sel, &["router."]).unwrap()
        };

        let re = analyze(&stripped);
        assert!(re.is_empty(), "stripped file must have no coupling left");

        let doc: serde_yaml::Value = serde_yaml::from_str(&stripped).unwrap();
        let services = doc.get("services").unwrap().as_mapping().unwrap();
        assert!(services.contains_key(serde_yaml::Value::String("api".into())));
        assert!(services.contains_key(serde_yaml::Value::String("web".into())));
        assert!(services.contains_key(serde_yaml::Value::String("db".into())));
        assert!(!services.contains_key(serde_yaml::Value::String("proxy-reload".into())));

        // api keeps default network + non-routing label, loses edge.
        let api = services
            .get(serde_yaml::Value::String("api".into()))
            .unwrap();
        let nets: Vec<&str> = api
            .get("networks")
            .unwrap()
            .as_sequence()
            .unwrap()
            .iter()
            .filter_map(|v| v.as_str())
            .collect();
        assert_eq!(nets, vec!["default"]);
    }

    #[test]
    fn strip_drops_dangling_depends_on() {
        let c = analyze(FOREIGN);
        let sel = StripSelection::all(&c);
        let stripped = super::strip_with_prefixes(FOREIGN, &sel, &["router."]).unwrap();
        // proxy-reload removed; nothing should still depend on it.
        assert!(!stripped.contains("proxy-reload"));
    }

    #[test]
    fn rejects_invalid_yaml() {
        assert!(analyze_foreign_coupling("services: [unclosed").is_err());
    }
}
