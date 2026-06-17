use mocode_text::TextRange;

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

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SemanticIndex {
    pub proxies: Vec<NamedEntity>,
    pub proxy_groups: Vec<NamedEntity>,
    pub proxy_providers: Vec<NamedEntity>,
    pub rule_providers: Vec<NamedEntity>,
    pub dialer_proxy_edges: Vec<ReferenceEdge>,
}

impl SemanticIndex {
    pub fn known_outbounds(&self) -> Vec<&str> {
        self.proxies
            .iter()
            .chain(self.proxy_groups.iter())
            .map(|entity| entity.name.as_str())
            .collect()
    }
}

pub fn validate_index(_index: &SemanticIndex) -> Vec<LintDiagnostic> {
    Vec::new()
}
