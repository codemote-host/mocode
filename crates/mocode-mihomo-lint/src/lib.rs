use std::collections::{HashMap, HashSet};

use mocode_mihomo_schema::BUILTIN_OUTBOUNDS;
use mocode_text::TextRange;
use yaml_rust2::{Yaml, YamlLoader};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Info,
    Hint,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LintDiagnostic {
    pub severity: DiagnosticSeverity,
    pub code: &'static str,
    pub message: String,
    pub range: Option<TextRange>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamedEntity {
    pub name: String,
    pub range: Option<TextRange>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceEdge {
    pub from: String,
    pub to: String,
    pub range: Option<TextRange>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReferenceKind {
    Outbound,
    ProxyProvider,
    RuleProvider,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamedReference {
    pub name: String,
    pub kind: ReferenceKind,
    pub range: Option<TextRange>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SemanticIndex {
    pub proxies: Vec<NamedEntity>,
    pub proxy_groups: Vec<NamedEntity>,
    pub proxy_providers: Vec<NamedEntity>,
    pub rule_providers: Vec<NamedEntity>,
    pub references: Vec<NamedReference>,
    pub dialer_proxy_edges: Vec<ReferenceEdge>,
}

impl SemanticIndex {
    pub fn from_yaml_str(text: &str) -> Self {
        let Ok(documents) = YamlLoader::load_from_str(text) else {
            return Self::default();
        };

        let Some(document) = documents.first() else {
            return Self::default();
        };

        let mut index = Self::default();
        index.extract_proxies(document);
        index.extract_proxy_groups(document);
        index.extract_providers(document);
        index.extract_rules(document);
        index
    }

    pub fn known_outbounds(&self) -> Vec<&str> {
        self.proxies
            .iter()
            .chain(self.proxy_groups.iter())
            .map(|entity| entity.name.as_str())
            .collect()
    }

    fn extract_proxies(&mut self, document: &Yaml) {
        let Some(proxies) = document["proxies"].as_vec() else {
            return;
        };

        for proxy in proxies {
            let Some(name) = yaml_get(proxy, "name").and_then(yaml_string) else {
                continue;
            };

            self.proxies.push(NamedEntity {
                name: name.clone(),
                range: None,
            });

            if let Some(target) = yaml_get(proxy, "dialer-proxy").and_then(yaml_string) {
                self.dialer_proxy_edges.push(ReferenceEdge {
                    from: name,
                    to: target,
                    range: None,
                });
            }
        }
    }

    fn extract_proxy_groups(&mut self, document: &Yaml) {
        let Some(groups) = document["proxy-groups"].as_vec() else {
            return;
        };

        for group in groups {
            let Some(name) = yaml_get(group, "name").and_then(yaml_string) else {
                continue;
            };

            self.proxy_groups.push(NamedEntity { name, range: None });

            if let Some(proxies) = yaml_get(group, "proxies").and_then(Yaml::as_vec) {
                for proxy in proxies.iter().filter_map(yaml_string) {
                    self.references.push(NamedReference {
                        name: proxy,
                        kind: ReferenceKind::Outbound,
                        range: None,
                    });
                }
            }

            if let Some(providers) = yaml_get(group, "use").and_then(Yaml::as_vec) {
                for provider in providers.iter().filter_map(yaml_string) {
                    self.references.push(NamedReference {
                        name: provider,
                        kind: ReferenceKind::ProxyProvider,
                        range: None,
                    });
                }
            }
        }
    }

    fn extract_providers(&mut self, document: &Yaml) {
        if let Some(providers) = document["proxy-providers"].as_hash() {
            for provider_name in providers.keys().filter_map(yaml_string_ref) {
                self.proxy_providers.push(NamedEntity {
                    name: provider_name.to_string(),
                    range: None,
                });
            }
        }

        if let Some(providers) = document["rule-providers"].as_hash() {
            for provider_name in providers.keys().filter_map(yaml_string_ref) {
                self.rule_providers.push(NamedEntity {
                    name: provider_name.to_string(),
                    range: None,
                });
            }
        }
    }

    fn extract_rules(&mut self, document: &Yaml) {
        let Some(rules) = document["rules"].as_vec() else {
            return;
        };

        for rule in rules.iter().filter_map(yaml_string) {
            extract_rule_references(&rule, &mut self.references);
        }
    }
}

pub fn validate_index(index: &SemanticIndex) -> Vec<LintDiagnostic> {
    let mut diagnostics = Vec::new();
    let outbounds = known_outbound_set(index);
    let proxy_providers = known_name_set(&index.proxy_providers);
    let rule_providers = known_name_set(&index.rule_providers);

    for reference in &index.references {
        let known = match reference.kind {
            ReferenceKind::Outbound => outbounds.contains(reference.name.as_str()),
            ReferenceKind::ProxyProvider => proxy_providers.contains(reference.name.as_str()),
            ReferenceKind::RuleProvider => rule_providers.contains(reference.name.as_str()),
        };

        if !known {
            diagnostics.push(LintDiagnostic {
                severity: DiagnosticSeverity::Error,
                code: "mihomo.reference.missing",
                message: format!("Mihomo reference `{}` does not exist", reference.name),
                range: reference.range,
            });
        }
    }

    for edge in &index.dialer_proxy_edges {
        if !outbounds.contains(edge.to.as_str()) {
            diagnostics.push(LintDiagnostic {
                severity: DiagnosticSeverity::Error,
                code: "mihomo.reference.missing",
                message: format!(
                    "dialer-proxy `{}` referenced by `{}` does not exist",
                    edge.to, edge.from
                ),
                range: edge.range,
            });
        }
    }

    if has_dialer_proxy_cycle(index) {
        diagnostics.push(LintDiagnostic {
            severity: DiagnosticSeverity::Error,
            code: "mihomo.dialer_proxy.cycle",
            message: "dialer-proxy chain contains a cycle".to_string(),
            range: None,
        });
    }

    diagnostics
}

fn yaml_get<'a>(node: &'a Yaml, key: &str) -> Option<&'a Yaml> {
    let value = &node[key];
    (!value.is_badvalue()).then_some(value)
}

fn yaml_string(node: &Yaml) -> Option<String> {
    node.as_str().map(ToOwned::to_owned)
}

fn yaml_string_ref(node: &Yaml) -> Option<&str> {
    node.as_str()
}

fn extract_rule_references(rule: &str, references: &mut Vec<NamedReference>) {
    let parts: Vec<_> = rule.split(',').map(str::trim).collect();
    let Some(rule_type) = parts.first().copied() else {
        return;
    };

    if rule_type.eq_ignore_ascii_case("RULE-SET") {
        if let Some(provider) = parts.get(1).filter(|provider| !provider.is_empty()) {
            references.push(NamedReference {
                name: (*provider).to_string(),
                kind: ReferenceKind::RuleProvider,
                range: None,
            });
        }
        if let Some(target) = parts.get(2).filter(|target| !target.is_empty()) {
            references.push(NamedReference {
                name: (*target).to_string(),
                kind: ReferenceKind::Outbound,
                range: None,
            });
        }
        return;
    }

    let target = match parts.as_slice() {
        [.., target, "no-resolve"] => Some(*target),
        [.., target] if parts.len() >= 2 => Some(*target),
        _ => None,
    };

    if let Some(target) = target.filter(|target| !target.is_empty()) {
        references.push(NamedReference {
            name: target.to_string(),
            kind: ReferenceKind::Outbound,
            range: None,
        });
    }
}

fn known_outbound_set(index: &SemanticIndex) -> HashSet<&str> {
    index
        .known_outbounds()
        .into_iter()
        .chain(BUILTIN_OUTBOUNDS.iter().copied())
        .collect()
}

fn known_name_set(entities: &[NamedEntity]) -> HashSet<&str> {
    entities.iter().map(|entity| entity.name.as_str()).collect()
}

fn has_dialer_proxy_cycle(index: &SemanticIndex) -> bool {
    let proxy_names: HashSet<_> = index
        .proxies
        .iter()
        .map(|proxy| proxy.name.as_str())
        .collect();
    let graph: HashMap<_, _> = index
        .dialer_proxy_edges
        .iter()
        .filter(|edge| proxy_names.contains(edge.to.as_str()))
        .map(|edge| (edge.from.as_str(), edge.to.as_str()))
        .collect();

    for start in graph.keys().copied() {
        let mut visiting = HashSet::new();
        let mut current = start;

        while let Some(next) = graph.get(current).copied() {
            if !visiting.insert(current) {
                return true;
            }
            current = next;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indexes_named_outbounds_and_groups() {
        let index =
            SemanticIndex::from_yaml_str(include_str!("../../../tests/fixtures/minimal.yaml"));

        assert!(index.proxies.iter().any(|proxy| proxy.name == "hk-1"));
        assert!(index.proxy_groups.iter().any(|group| group.name == "Proxy"));
    }

    #[test]
    fn reports_missing_group_member_reference() {
        let index = SemanticIndex::from_yaml_str(include_str!(
            "../../../tests/fixtures/invalid-reference.yaml"
        ));
        let diagnostics = validate_index(&index);

        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "mihomo.reference.missing")
        );
    }

    #[test]
    fn reports_dialer_proxy_cycle() {
        let index =
            SemanticIndex::from_yaml_str(include_str!("../../../tests/fixtures/dialer-cycle.yaml"));
        let diagnostics = validate_index(&index);

        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "mihomo.dialer_proxy.cycle")
        );
    }
}
