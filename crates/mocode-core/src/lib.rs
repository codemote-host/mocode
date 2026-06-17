use mocode_mihomo_lint::{DiagnosticSeverity, SemanticIndex, validate_index};
use mocode_mihomo_schema::{CompletionKind, SchemaCatalog};
use mocode_text::{TextBuffer, TextEdit, TextEditError, TextPosition, TextRange};
use mocode_yaml::{YamlDocument, YamlPath};

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
            yaml: YamlDocument::parse(text),
            schema: SchemaCatalog::default_catalog(),
            semantic_index: SemanticIndex::default(),
        }
    }

    pub fn apply_edit(&mut self, edit: TextEdit) -> Result<(), EditorError> {
        self.text.apply_edit(edit)?;
        self.yaml = YamlDocument::parse(self.text.as_string());
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

    pub fn completions_at(&self, _position: TextPosition) -> Vec<Completion> {
        self.schema
            .root_field_completions()
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

    pub fn text(&self) -> String {
        self.text.as_string()
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
}
