use mocode_mihomo_lint::{DiagnosticSeverity, SemanticIndex, validate_index};
use mocode_mihomo_schema::{BUILTIN_OUTBOUNDS, CompletionKind, SchemaCatalog};
use mocode_text::{TextBuffer, TextEdit, TextEditError, TextPosition, TextRange};
use mocode_yaml::{YamlDocument, YamlPath};
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditorError {
    TextEdit(TextEditError),
}

impl From<TextEditError> for EditorError {
    fn from(error: TextEditError) -> Self {
        Self::TextEdit(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Completion {
    pub label: String,
    pub insert_text: String,
    pub kind: CompletionKind,
    pub documentation: Option<String>,
    pub replace_range: Option<TextRange>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hover {
    pub title: String,
    pub markdown: String,
    pub range: Option<TextRange>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub severity: DiagnosticSeverity,
    pub code: String,
    pub message: String,
    pub range: Option<TextRange>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorLine {
    pub number: u32,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorSnapshot {
    pub lines: Vec<EditorLine>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reference {
    pub name: String,
    pub range: TextRange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProxyChainPreview {
    pub steps: Vec<String>,
    pub is_definite: bool,
}

#[derive(Debug, Clone)]
pub struct MocodeEditor {
    text: TextBuffer,
    yaml: YamlDocument,
    schema: SchemaCatalog,
    semantic_index: SemanticIndex,
}

impl MocodeEditor {
    pub fn open_text(text: impl Into<String>) -> Self {
        let text = text.into();
        Self {
            text: TextBuffer::open_text(&text),
            yaml: YamlDocument::parse(&text),
            schema: SchemaCatalog::default_catalog(),
            semantic_index: SemanticIndex::from_yaml_str(&text),
        }
    }

    pub fn apply_edit(&mut self, edit: TextEdit) -> Result<(), EditorError> {
        self.text.apply_edit(edit)?;
        let text = self.text.as_string();
        self.yaml = YamlDocument::parse(&text);
        self.semantic_index = SemanticIndex::from_yaml_str(&text);
        Ok(())
    }

    pub fn format(&self) -> Result<String, EditorError> {
        Ok(self.text.as_string())
    }

    pub fn diagnostics(&self) -> Vec<Diagnostic> {
        let yaml_errors = self
            .yaml
            .syntax_errors()
            .into_iter()
            .map(|error| Diagnostic {
                severity: DiagnosticSeverity::Error,
                code: "yaml.syntax".to_string(),
                message: error.message,
                range: Some(error.range),
            });

        let lint_errors = validate_index(&self.semantic_index)
            .into_iter()
            .map(|diagnostic| Diagnostic {
                severity: diagnostic.severity,
                code: diagnostic.code.to_string(),
                message: diagnostic.message,
                range: diagnostic.range,
            });

        yaml_errors.chain(lint_errors).collect()
    }

    pub fn completions_at(&self, position: TextPosition) -> Vec<Completion> {
        let path = self
            .current_yaml_path(position)
            .map(|path| path.to_string());
        let schema_completions = path
            .as_deref()
            .and_then(|path| self.reference_completions(path))
            .or_else(|| {
                path.as_deref()
                    .map(|path| self.schema.enum_completions(path))
                    .filter(|completions| !completions.is_empty())
            })
            .unwrap_or_else(|| self.schema.root_field_completions());

        schema_completions
            .into_iter()
            .map(|completion| Completion {
                label: completion.label,
                insert_text: completion.insert_text,
                kind: completion.kind,
                documentation: completion.documentation,
                replace_range: None,
            })
            .collect()
    }

    pub fn hover_at(&self, position: TextPosition) -> Option<Hover> {
        let path = self.current_yaml_path(position)?;
        let doc = self.schema.field_doc(&path.to_string())?;
        Some(Hover {
            title: doc.path.to_string(),
            markdown: format!("{}\n\n{}", doc.summary, doc.details),
            range: None,
        })
    }

    pub fn current_yaml_path(&self, position: TextPosition) -> Option<YamlPath> {
        self.yaml.path_at(position)
    }

    pub fn semantic_index(&self) -> &SemanticIndex {
        &self.semantic_index
    }

    pub fn proxy_chain_preview_at(&self, _position: TextPosition) -> Option<ProxyChainPreview> {
        None
    }

    pub fn references_at(&self, _position: TextPosition) -> Vec<Reference> {
        Vec::new()
    }

    pub fn validate(&self) -> Vec<Diagnostic> {
        self.diagnostics()
    }

    pub fn line_count(&self) -> usize {
        self.text.line_count()
    }

    pub fn line_text(&self, line: usize) -> Option<String> {
        self.text.line_text(line)
    }

    pub fn snapshot(&self) -> EditorSnapshot {
        EditorSnapshot {
            lines: (0..self.line_count())
                .filter_map(|line| {
                    Some(EditorLine {
                        number: u32::try_from(line + 1).ok()?,
                        text: self.line_text(line)?,
                    })
                })
                .collect(),
            diagnostics: self.diagnostics(),
        }
    }

    pub fn text(&self) -> String {
        self.text.as_string()
    }

    fn reference_completions(
        &self,
        path: &str,
    ) -> Option<Vec<mocode_mihomo_schema::SchemaCompletion>> {
        if is_proxy_group_member_path(path) {
            return Some(
                self.outbound_names_with_builtins()
                    .into_iter()
                    .map(reference_completion)
                    .collect(),
            );
        }

        if is_dialer_proxy_path(path) {
            return Some(
                self.semantic_index
                    .known_outbounds()
                    .into_iter()
                    .map(reference_completion)
                    .collect(),
            );
        }

        None
    }

    fn outbound_names_with_builtins(&self) -> Vec<&str> {
        self.semantic_index
            .known_outbounds()
            .into_iter()
            .chain(BUILTIN_OUTBOUNDS.iter().copied())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }
}

fn is_proxy_group_member_path(path: &str) -> bool {
    path.starts_with("proxy-groups[") && (path.ends_with(".proxies") || path.contains(".proxies["))
}

fn is_dialer_proxy_path(path: &str) -> bool {
    path.starts_with("proxies[") && path.ends_with(".dialer-proxy")
}

fn reference_completion(name: &str) -> mocode_mihomo_schema::SchemaCompletion {
    mocode_mihomo_schema::SchemaCompletion {
        label: name.to_string(),
        insert_text: name.to_string(),
        kind: CompletionKind::Reference,
        documentation: Some("Mihomo outbound reference".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opens_text_and_returns_root_completions() {
        let editor = MocodeEditor::open_text("mixed-port: 7890\n");
        let labels: Vec<_> = editor
            .completions_at(TextPosition::new(0, 0))
            .into_iter()
            .map(|item| item.label)
            .collect();

        assert!(labels.contains(&"mixed-port".to_string()));
    }

    #[test]
    fn returns_enum_completions_for_current_yaml_path() {
        let editor = MocodeEditor::open_text("dns:\n  enhanced-mode: \n");
        let labels: Vec<_> = editor
            .completions_at(TextPosition::new(1, 17))
            .into_iter()
            .map(|item| item.label)
            .collect();

        assert!(labels.contains(&"fake-ip".to_string()));
    }

    #[test]
    fn returns_hover_for_nested_field() {
        let editor = MocodeEditor::open_text("tun:\n  stack: system\n");
        let hover = editor.hover_at(TextPosition::new(1, 4)).unwrap();

        assert_eq!(hover.title, "tun.stack");
        assert!(hover.markdown.contains("TUN network stack"));
    }

    #[test]
    fn validates_missing_reference_from_core() {
        let editor = MocodeEditor::open_text(include_str!(
            "../../../tests/fixtures/invalid-reference.yaml"
        ));

        assert!(
            editor
                .diagnostics()
                .iter()
                .any(|diagnostic| diagnostic.code == "mihomo.reference.missing")
        );
    }

    #[test]
    fn reports_dialer_proxy_cycle_from_core() {
        let editor =
            MocodeEditor::open_text(include_str!("../../../tests/fixtures/dialer-cycle.yaml"));

        assert!(
            editor
                .diagnostics()
                .iter()
                .any(|diagnostic| diagnostic.code == "mihomo.dialer_proxy.cycle")
        );
    }

    #[test]
    fn indexes_semantics_on_open() {
        let editor = MocodeEditor::open_text(include_str!("../../../tests/fixtures/minimal.yaml"));

        assert!(
            editor
                .semantic_index()
                .proxies
                .iter()
                .any(|proxy| proxy.name == "hk-1")
        );
    }

    #[test]
    fn returns_read_only_snapshot_for_ui_adapters() {
        let editor = MocodeEditor::open_text("mixed-port: 7890\nproxy-groups:\n  - name: Auto\n");
        let snapshot = editor.snapshot();

        assert_eq!(snapshot.lines[0].number, 1);
        assert_eq!(snapshot.lines[0].text, "mixed-port: 7890");
        assert_eq!(snapshot.lines[1].number, 2);
        assert!(snapshot.diagnostics.is_empty());
    }

    #[test]
    fn returns_reference_completions_for_group_members() {
        let editor = MocodeEditor::open_text(
            "proxies:\n  - name: hk-1\n    type: ss\nproxy-groups:\n  - name: Proxy\n    type: select\n    proxies:\n      - \n",
        );
        let labels: Vec<_> = editor
            .completions_at(TextPosition::new(7, 8))
            .into_iter()
            .map(|item| item.label)
            .collect();

        assert!(labels.contains(&"hk-1".to_string()));
        assert!(labels.contains(&"DIRECT".to_string()));
    }

    #[test]
    fn returns_reference_completions_for_dialer_proxy() {
        let editor = MocodeEditor::open_text(
            "proxies:\n  - name: entry\n    type: ss\n    dialer-proxy: \n  - name: exit\n    type: ss\nproxy-groups:\n  - name: relay\n    type: select\n    proxies:\n      - exit\n",
        );
        let labels: Vec<_> = editor
            .completions_at(TextPosition::new(3, 18))
            .into_iter()
            .map(|item| item.label)
            .collect();

        assert!(labels.contains(&"exit".to_string()));
        assert!(labels.contains(&"relay".to_string()));
    }
}
