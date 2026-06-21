use std::collections::{HashMap, HashSet};

use mocode_mihomo_schema::BUILTIN_OUTBOUNDS;
use mocode_text::{TextPosition, TextRange};
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
    /// Original YAML sequence index (e.g. in `proxies`), when available.
    pub source_index: Option<usize>,
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

        let reference_ranges = ReferenceRanges::new(text);
        let mut index = Self::default();
        index.extract_proxies(document, &reference_ranges);
        index.extract_proxy_groups(document, &reference_ranges);
        index.extract_providers(document);
        index.extract_rules(document, &reference_ranges);
        index
    }

    /// Look up a proxy name by its original YAML sequence index in `proxies`.
    ///
    /// This accounts for unnamed proxy items that were skipped during indexing,
    /// which would cause a simple array-index lookup into `self.proxies` to
    /// resolve the wrong proxy.
    pub fn proxy_name_by_source_index(&self, index: usize) -> Option<&str> {
        self.proxies
            .iter()
            .find(|proxy| proxy.source_index == Some(index))
            .map(|proxy| proxy.name.as_str())
    }

    pub fn known_outbounds(&self) -> Vec<&str> {
        self.proxies
            .iter()
            .chain(self.proxy_groups.iter())
            .map(|entity| entity.name.as_str())
            .collect()
    }

    fn extract_proxies(&mut self, document: &Yaml, ranges: &ReferenceRanges) {
        let Some(proxies) = document["proxies"].as_vec() else {
            return;
        };

        for (idx, proxy) in proxies.iter().enumerate() {
            let Some(name) = yaml_get(proxy, "name").and_then(yaml_string) else {
                continue;
            };

            self.proxies.push(NamedEntity {
                name: name.clone(),
                range: None,
                source_index: Some(idx),
            });

            if let Some(target) = yaml_get(proxy, "dialer-proxy").and_then(yaml_string) {
                self.dialer_proxy_edges.push(ReferenceEdge {
                    from: name,
                    to: target,
                    range: ranges.dialer_proxy_by_proxy_index.get(&idx).copied(),
                });
            }
        }
    }

    fn extract_proxy_groups(&mut self, document: &Yaml, ranges: &ReferenceRanges) {
        let Some(groups) = document["proxy-groups"].as_vec() else {
            return;
        };

        for (group_index, group) in groups.iter().enumerate() {
            let Some(name) = yaml_get(group, "name").and_then(yaml_string) else {
                continue;
            };

            self.proxy_groups.push(NamedEntity {
                name,
                range: None,
                source_index: None,
            });

            if let Some(proxies) = yaml_get(group, "proxies").and_then(Yaml::as_vec) {
                let proxy_ranges = ranges
                    .group_proxies_by_group_index
                    .get(&group_index)
                    .map(Vec::as_slice)
                    .unwrap_or(&[]);
                let mut proxy_range_index = 0;
                for proxy in proxies.iter().filter_map(yaml_string) {
                    self.references.push(NamedReference {
                        name: proxy,
                        kind: ReferenceKind::Outbound,
                        range: next_range(proxy_ranges, &mut proxy_range_index),
                    });
                }
            }

            if let Some(providers) = yaml_get(group, "use").and_then(Yaml::as_vec) {
                let provider_ranges = ranges
                    .group_uses_by_group_index
                    .get(&group_index)
                    .map(Vec::as_slice)
                    .unwrap_or(&[]);
                let mut provider_range_index = 0;
                for provider in providers.iter().filter_map(yaml_string) {
                    self.references.push(NamedReference {
                        name: provider,
                        kind: ReferenceKind::ProxyProvider,
                        range: next_range(provider_ranges, &mut provider_range_index),
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
                    source_index: None,
                });
            }
        }

        if let Some(providers) = document["rule-providers"].as_hash() {
            for provider_name in providers.keys().filter_map(yaml_string_ref) {
                self.rule_providers.push(NamedEntity {
                    name: provider_name.to_string(),
                    range: None,
                    source_index: None,
                });
            }
        }
    }

    fn extract_rules(&mut self, document: &Yaml, ranges: &ReferenceRanges) {
        let Some(rules) = document["rules"].as_vec() else {
            return;
        };

        for (rule_index, rule) in rules.iter().enumerate() {
            let Some(rule) = yaml_string(rule) else {
                continue;
            };

            let provider_ranges = ranges
                .rule_providers_by_rule_index
                .get(&rule_index)
                .map(Vec::as_slice)
                .unwrap_or(&[]);
            let target_ranges = ranges
                .rule_targets_by_rule_index
                .get(&rule_index)
                .map(Vec::as_slice)
                .unwrap_or(&[]);
            extract_rule_references(&rule, &mut self.references, provider_ranges, target_ranges);
        }
    }
}

#[derive(Debug, Default)]
struct ReferenceRanges {
    dialer_proxy_by_proxy_index: HashMap<usize, TextRange>,
    group_proxies_by_group_index: HashMap<usize, Vec<TextRange>>,
    group_uses_by_group_index: HashMap<usize, Vec<TextRange>>,
    rule_providers_by_rule_index: HashMap<usize, Vec<TextRange>>,
    rule_targets_by_rule_index: HashMap<usize, Vec<TextRange>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TopLevelSection {
    Proxies,
    ProxyGroups,
    Rules,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GroupSublist {
    Proxies,
    Use,
}

impl ReferenceRanges {
    fn new(text: &str) -> Self {
        let mut ranges = Self::default();
        let mut section = TopLevelSection::Other;
        let mut proxy_index = None;
        let mut group_index = None;
        let mut group_sublist = None;
        let mut rule_index = 0usize;

        for (line_index, line) in text.lines().enumerate() {
            let indent = leading_spaces(line);
            let trimmed = line.trim_start();

            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            if indent == 0 {
                section = match top_level_key(trimmed) {
                    Some("proxies") => TopLevelSection::Proxies,
                    Some("proxy-groups") => TopLevelSection::ProxyGroups,
                    Some("rules") => TopLevelSection::Rules,
                    _ => TopLevelSection::Other,
                };
                proxy_index = None;
                group_index = None;
                group_sublist = None;
                continue;
            }

            match section {
                TopLevelSection::Proxies => {
                    if indent == 2 && trimmed.starts_with("- ") {
                        proxy_index = Some(proxy_index.map_or(0, |index| index + 1));
                    }

                    if let Some(current_proxy_index) = proxy_index {
                        if let Some(range) = key_value_range(line_index, line, "dialer-proxy") {
                            ranges
                                .dialer_proxy_by_proxy_index
                                .insert(current_proxy_index, range);
                        }
                    }
                }
                TopLevelSection::ProxyGroups => {
                    if indent == 2 && trimmed.starts_with("- ") {
                        group_index = Some(group_index.map_or(0, |index| index + 1));
                        group_sublist = None;
                    }

                    let Some(current_group_index) = group_index else {
                        continue;
                    };

                    if indent <= 4 {
                        group_sublist = match field_key(line) {
                            Some("proxies") => Some(GroupSublist::Proxies),
                            Some("use") => Some(GroupSublist::Use),
                            _ => None,
                        };
                        continue;
                    }

                    let Some(range) = list_item_value_range(line_index, line) else {
                        continue;
                    };

                    match group_sublist {
                        Some(GroupSublist::Proxies) => ranges
                            .group_proxies_by_group_index
                            .entry(current_group_index)
                            .or_default()
                            .push(range),
                        Some(GroupSublist::Use) => ranges
                            .group_uses_by_group_index
                            .entry(current_group_index)
                            .or_default()
                            .push(range),
                        None => {}
                    }
                }
                TopLevelSection::Rules => {
                    if indent != 2 || !trimmed.starts_with("- ") {
                        continue;
                    }

                    let current_rule_index = rule_index;
                    rule_index += 1;

                    let Some((value_start, value_end)) = list_item_value_span(line) else {
                        continue;
                    };
                    collect_rule_reference_ranges(
                        line_index,
                        line,
                        value_start,
                        value_end,
                        current_rule_index,
                        &mut ranges,
                    );
                }
                TopLevelSection::Other => {}
            }
        }

        ranges
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

fn next_range(ranges: &[TextRange], index: &mut usize) -> Option<TextRange> {
    let range = ranges.get(*index).copied();
    *index += 1;
    range
}

fn leading_spaces(line: &str) -> usize {
    line.bytes().take_while(|byte| *byte == b' ').count()
}

fn top_level_key(trimmed_line: &str) -> Option<&str> {
    let colon_index = trimmed_line.find(':')?;
    Some(trimmed_line[..colon_index].trim())
}

fn field_key(line: &str) -> Option<&str> {
    let indent = leading_spaces(line);
    let mut content_start = indent;
    let mut content = &line[content_start..];

    if content.starts_with("- ") {
        content_start += 2;
        content = &line[content_start..];
    }

    let colon_index = content.find(':')?;
    Some(content[..colon_index].trim())
}

fn key_value_range(line_index: usize, line: &str, key: &str) -> Option<TextRange> {
    if field_key(line)? != key {
        return None;
    }

    let colon_index = line.find(':')?;
    let mut value_start = colon_index + 1;
    while line.as_bytes().get(value_start) == Some(&b' ') {
        value_start += 1;
    }

    value_range_from_byte_span(
        line_index,
        line,
        value_start,
        scalar_value_end(line, value_start),
    )
}

fn list_item_value_range(line_index: usize, line: &str) -> Option<TextRange> {
    let (start, end) = list_item_value_span(line)?;
    value_range_from_byte_span(line_index, line, start, end)
}

fn list_item_value_span(line: &str) -> Option<(usize, usize)> {
    let indent = leading_spaces(line);
    let rest = &line[indent..];
    if !rest.starts_with("- ") {
        return None;
    }

    let mut value_start = indent + 2;
    while line.as_bytes().get(value_start) == Some(&b' ') {
        value_start += 1;
    }

    let value_end = scalar_value_end(line, value_start);
    trim_optional_quotes(line, value_start, value_end)
}

fn scalar_value_end(line: &str, value_start: usize) -> usize {
    let mut value_end = line.len();
    if let Some(comment_index) = line[value_start..].find(" #") {
        value_end = value_start + comment_index;
    }

    while value_end > value_start
        && line
            .as_bytes()
            .get(value_end - 1)
            .is_some_and(u8::is_ascii_whitespace)
    {
        value_end -= 1;
    }

    value_end
}

fn trim_optional_quotes(line: &str, mut start: usize, mut end: usize) -> Option<(usize, usize)> {
    if start >= end {
        return None;
    }

    let bytes = line.as_bytes();
    if end > start + 1 {
        let first = bytes[start];
        let last = bytes[end - 1];
        if (first == b'"' && last == b'"') || (first == b'\'' && last == b'\'') {
            start += 1;
            end -= 1;
        }
    }

    (start < end).then_some((start, end))
}

fn value_range_from_byte_span(
    line_index: usize,
    line: &str,
    start: usize,
    end: usize,
) -> Option<TextRange> {
    let (start, end) = trim_optional_quotes(line, start, end)?;
    Some(text_range_from_byte_span(line_index, line, start, end))
}

fn text_range_from_byte_span(line_index: usize, line: &str, start: usize, end: usize) -> TextRange {
    TextRange::new(
        TextPosition::new(line_index as u32, line[..start].chars().count() as u32),
        TextPosition::new(line_index as u32, line[..end].chars().count() as u32),
    )
}

#[derive(Debug, Clone, Copy)]
struct RulePart<'a> {
    text: &'a str,
    start: usize,
    end: usize,
}

fn collect_rule_reference_ranges(
    line_index: usize,
    line: &str,
    value_start: usize,
    value_end: usize,
    rule_index: usize,
    ranges: &mut ReferenceRanges,
) {
    let parts = rule_parts(&line[value_start..value_end], value_start);
    let Some(rule_type) = parts.first() else {
        return;
    };

    if rule_type.text.eq_ignore_ascii_case("RULE-SET") {
        if let Some(provider) = parts.get(1).filter(|part| !part.text.is_empty()) {
            ranges
                .rule_providers_by_rule_index
                .entry(rule_index)
                .or_default()
                .push(text_range_from_byte_span(
                    line_index,
                    line,
                    provider.start,
                    provider.end,
                ));
        }
        if let Some(target) = parts.get(2).filter(|part| !part.text.is_empty()) {
            ranges
                .rule_targets_by_rule_index
                .entry(rule_index)
                .or_default()
                .push(text_range_from_byte_span(
                    line_index,
                    line,
                    target.start,
                    target.end,
                ));
        }
        return;
    }

    let target = match parts.as_slice() {
        [.., target, no_resolve] if no_resolve.text == "no-resolve" => Some(*target),
        [.., target] if parts.len() >= 2 => Some(*target),
        _ => None,
    };

    if let Some(target) = target.filter(|part| !part.text.is_empty()) {
        ranges
            .rule_targets_by_rule_index
            .entry(rule_index)
            .or_default()
            .push(text_range_from_byte_span(
                line_index,
                line,
                target.start,
                target.end,
            ));
    }
}

fn rule_parts(value: &str, value_start: usize) -> Vec<RulePart<'_>> {
    let mut parts = Vec::new();
    let mut segment_start = 0usize;

    for (index, character) in value.char_indices() {
        if character == ',' {
            push_rule_part(value, segment_start, index, value_start, &mut parts);
            segment_start = index + character.len_utf8();
        }
    }
    push_rule_part(value, segment_start, value.len(), value_start, &mut parts);

    parts
}

fn push_rule_part<'a>(
    value: &'a str,
    segment_start: usize,
    segment_end: usize,
    value_start: usize,
    parts: &mut Vec<RulePart<'a>>,
) {
    let segment = &value[segment_start..segment_end];
    let trimmed_start = segment.len() - segment.trim_start().len();
    let trimmed_end = segment.trim_end().len();
    let start = segment_start + trimmed_start;
    let end = segment_start + trimmed_end;

    parts.push(RulePart {
        text: &value[start..end],
        start: value_start + start,
        end: value_start + end,
    });
}

fn extract_rule_references(
    rule: &str,
    references: &mut Vec<NamedReference>,
    provider_ranges: &[TextRange],
    target_ranges: &[TextRange],
) {
    let parts: Vec<_> = rule.split(',').map(str::trim).collect();
    let Some(rule_type) = parts.first().copied() else {
        return;
    };

    if rule_type.eq_ignore_ascii_case("RULE-SET") {
        let mut provider_range_index = 0;
        let mut target_range_index = 0;
        if let Some(provider) = parts.get(1).filter(|provider| !provider.is_empty()) {
            references.push(NamedReference {
                name: (*provider).to_string(),
                kind: ReferenceKind::RuleProvider,
                range: next_range(provider_ranges, &mut provider_range_index),
            });
        }
        if let Some(target) = parts.get(2).filter(|target| !target.is_empty()) {
            references.push(NamedReference {
                name: (*target).to_string(),
                kind: ReferenceKind::Outbound,
                range: next_range(target_ranges, &mut target_range_index),
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
        let mut target_range_index = 0;
        references.push(NamedReference {
            name: target.to_string(),
            kind: ReferenceKind::Outbound,
            range: next_range(target_ranges, &mut target_range_index),
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
    fn missing_references_include_source_ranges() {
        let index = SemanticIndex::from_yaml_str(include_str!(
            "../../../examples/configs/invalid-reference.yaml"
        ));
        let diagnostics = validate_index(&index);

        for missing in [
            "missing-dialer",
            "missing-proxy",
            "missing-group",
            "missing-rule-provider",
        ] {
            let diagnostic = diagnostics
                .iter()
                .find(|diagnostic| diagnostic.message.contains(missing))
                .unwrap_or_else(|| panic!("missing diagnostic for {missing}"));
            let range = diagnostic
                .range
                .unwrap_or_else(|| panic!("diagnostic for {missing} should have a range"));

            assert!(
                range.end.character > range.start.character,
                "diagnostic range for {missing} should cover the reference value"
            );
        }
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
