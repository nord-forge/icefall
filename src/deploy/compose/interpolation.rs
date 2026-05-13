use std::collections::HashMap;

use super::types::ComposeFile;

/// Extract all `${VAR}` and `${VAR:-default}` references from the raw YAML.
pub(super) fn extract_variables(yaml: &str) -> Vec<(String, Option<String>)> {
    let mut vars = Vec::new();
    let mut seen = std::collections::HashSet::new();

    let mut chars = yaml.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '$' && chars.peek() == Some(&'{') {
            chars.next(); // consume '{'
            let mut name = String::new();
            let mut default = None;
            let mut in_default = false;
            let mut default_buf = String::new();

            loop {
                match chars.next() {
                    None | Some('}') => break,
                    Some(':') if !in_default => {
                        if chars.peek() == Some(&'-') {
                            chars.next();
                            in_default = true;
                        } else {
                            name.push(':');
                        }
                    }
                    Some(c) => {
                        if in_default {
                            default_buf.push(c);
                        } else {
                            name.push(c);
                        }
                    }
                }
            }

            if in_default {
                default = Some(default_buf);
            }

            if !name.is_empty() && seen.insert(name.clone()) {
                vars.push((name, default));
            }
        }
    }

    vars
}

/// Interpolate `${VAR}` and `${VAR:-default}` in the YAML string with provided values.
pub(super) fn interpolate(yaml: &str, env_vars: &HashMap<String, String>) -> String {
    let mut result = String::with_capacity(yaml.len());
    let mut chars = yaml.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '$' && chars.peek() == Some(&'{') {
            chars.next(); // consume '{'
            let mut name = String::new();
            let mut default = None;
            let mut in_default = false;
            let mut default_buf = String::new();

            loop {
                match chars.next() {
                    None | Some('}') => break,
                    Some(':') if !in_default => {
                        if chars.peek() == Some(&'-') {
                            chars.next();
                            in_default = true;
                        } else {
                            name.push(':');
                        }
                    }
                    Some(c) => {
                        if in_default {
                            default_buf.push(c);
                        } else {
                            name.push(c);
                        }
                    }
                }
            }

            if in_default {
                default = Some(default_buf);
            }

            if let Some(val) = env_vars.get(&name) {
                result.push_str(val);
            } else if let Some(def) = default {
                result.push_str(&def);
            }
            // If no value and no default, omit (empty string)
        } else {
            result.push(ch);
        }
    }

    result
}

/// Return service names in dependency order (dependencies first).
pub(super) fn dependency_order(compose: &ComposeFile) -> Vec<String> {
    let all_names: Vec<String> = compose.services.keys().cloned().collect();

    // Build: service -> list of services it depends on
    let mut deps: HashMap<String, Vec<String>> = HashMap::new();
    for (name, service) in &compose.services {
        deps.insert(name.clone(), service.depends_on.names());
    }

    // Kahn's algorithm: nodes with in-degree 0 have no unmet dependencies.
    // If A depends_on B, then A has in-degree 1 (needs B first).
    let mut in_degree: HashMap<String, usize> = HashMap::new();
    for name in &all_names {
        in_degree.entry(name.clone()).or_insert(0);
    }
    for (name, service_deps) in &deps {
        // Each dependency of `name` increases name's in-degree
        let count = service_deps
            .iter()
            .filter(|d| compose.services.contains_key(d.as_str()))
            .count();
        *in_degree.entry(name.clone()).or_insert(0) += count;
    }

    let mut queue: Vec<String> = in_degree
        .iter()
        .filter(|(_, &deg)| deg == 0)
        .map(|(name, _)| name.clone())
        .collect();
    queue.sort(); // deterministic

    let mut ordered = Vec::new();
    while let Some(current) = queue.pop() {
        ordered.push(current.clone());
        // For every service that depends on `current`, decrease its in-degree
        for (name, service_deps) in &deps {
            if service_deps.contains(&current) {
                if let Some(count) = in_degree.get_mut(name) {
                    *count = count.saturating_sub(1);
                    if *count == 0 {
                        queue.push(name.clone());
                        queue.sort();
                    }
                }
            }
        }
    }

    // Append any remaining (circular deps — best-effort)
    for name in &all_names {
        if !ordered.contains(name) {
            ordered.push(name.clone());
        }
    }

    ordered
}
