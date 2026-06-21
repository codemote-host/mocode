use mocode_mihomo_lint::{DiagnosticSeverity, SemanticIndex, validate_index};
use mocode_mihomo_schema::{BUILTIN_OUTBOUNDS, CompletionKind, SchemaCatalog};
use mocode_text::{TextBuffer, TextEdit, TextEditError, TextPosition, TextRange};
pub use mocode_yaml::SyntaxHighlightKind;
use mocode_yaml::{SyntaxToken, YamlDocument, YamlPath, YamlPathSegment};
use std::collections::{BTreeMap, BTreeSet};

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
pub struct LineDiagnostic {
    pub severity: DiagnosticSeverity,
    pub code: String,
    pub message: String,
    pub column: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticLine {
    pub number: u32,
    pub text: String,
    pub diagnostics: Vec<LineDiagnostic>,
    pub highlights: Vec<SyntaxHighlightSpan>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntaxHighlightSpan {
    pub start: u32,
    pub end: u32,
    pub kind: SyntaxHighlightKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HoverSummary {
    pub title: String,
    pub body: String,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProxyChainStatus {
    Complete,
    MissingReference,
    Cycle,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProxyChainPreview {
    pub steps: Vec<String>,
    pub is_definite: bool,
    pub status: ProxyChainStatus,
    pub message: Option<String>,
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
        self.refresh_indexes();
        Ok(())
    }

    pub fn insert_text_at(
        &mut self,
        position: TextPosition,
        text: &str,
    ) -> Result<TextPosition, EditorError> {
        let cursor = self.text.insert_text_at(position, text)?;
        self.refresh_indexes();
        Ok(cursor)
    }

    pub fn backspace_at(&mut self, position: TextPosition) -> Result<TextPosition, EditorError> {
        let cursor = self.text.backspace_at(position)?;
        self.refresh_indexes();
        Ok(cursor)
    }

    pub fn delete_at(&mut self, position: TextPosition) -> Result<TextPosition, EditorError> {
        let cursor = self.text.delete_at(position)?;
        self.refresh_indexes();
        Ok(cursor)
    }

    pub fn undo(&mut self) -> Result<Option<TextPosition>, EditorError> {
        let cursor = self.text.undo()?;
        if cursor.is_some() {
            self.refresh_indexes();
        }
        Ok(cursor)
    }

    pub fn redo(&mut self) -> Result<Option<TextPosition>, EditorError> {
        let cursor = self.text.redo()?;
        if cursor.is_some() {
            self.refresh_indexes();
        }
        Ok(cursor)
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

    pub fn hover_summary_at(&self, position: TextPosition) -> Option<HoverSummary> {
        let hover = self.hover_at(position)?;
        Some(HoverSummary {
            title: hover.title,
            body: first_markdown_paragraph(&hover.markdown),
        })
    }

    pub fn current_yaml_path(&self, position: TextPosition) -> Option<YamlPath> {
        self.yaml.path_at(position)
    }

    pub fn semantic_index(&self) -> &SemanticIndex {
        &self.semantic_index
    }

    pub fn proxy_chain_preview_at(&self, position: TextPosition) -> Option<ProxyChainPreview> {
        let path = self.current_yaml_path(position)?;
        let proxy_index = proxy_index_from_dialer_proxy_path(&path)?;

        // Reject if cursor is not on the value portion of "dialer-proxy: <value>"
        let line = self.line_text(position.line as usize)?;
        if !cursor_on_dialer_proxy_value(&line, position.character) {
            return None;
        }

        // Resolve proxy name via source_index (accounts for unnamed proxies
        // that the semantic index skips, which would misalign a simple
        // positional lookup into semantic_index.proxies).
        let entry = self
            .semantic_index
            .proxy_name_by_source_index(proxy_index)?;
        self.proxy_chain_preview_for_entry(&entry)
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

    pub fn text_in_range(&self, range: TextRange) -> Result<String, EditorError> {
        Ok(self.text.text_in_range(range)?)
    }

    pub fn line_end_position(&self, line: usize) -> Option<TextPosition> {
        self.text.line_end_position(line)
    }

    pub fn move_left(&self, position: TextPosition) -> Result<TextPosition, EditorError> {
        Ok(self.text.move_left(position)?)
    }

    pub fn move_right(&self, position: TextPosition) -> Result<TextPosition, EditorError> {
        Ok(self.text.move_right(position)?)
    }

    pub fn move_up(&self, position: TextPosition) -> Result<TextPosition, EditorError> {
        Ok(self.text.move_up(position)?)
    }

    pub fn move_down(&self, position: TextPosition) -> Result<TextPosition, EditorError> {
        Ok(self.text.move_down(position)?)
    }

    pub fn move_line_start(&self, position: TextPosition) -> Result<TextPosition, EditorError> {
        Ok(self.text.move_line_start(position)?)
    }

    pub fn move_line_end(&self, position: TextPosition) -> Result<TextPosition, EditorError> {
        Ok(self.text.move_line_end(position)?)
    }

    pub fn page_up(
        &self,
        position: TextPosition,
        visible_lines: u32,
    ) -> Result<TextPosition, EditorError> {
        Ok(self.text.page_up(position, visible_lines)?)
    }

    pub fn page_down(
        &self,
        position: TextPosition,
        visible_lines: u32,
    ) -> Result<TextPosition, EditorError> {
        Ok(self.text.page_down(position, visible_lines)?)
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

    pub fn semantic_lines(&self) -> Vec<SemanticLine> {
        let mut diagnostics_by_line = BTreeMap::<u32, Vec<LineDiagnostic>>::new();
        for diagnostic in self.diagnostics() {
            let Some(range) = diagnostic.range else {
                continue;
            };

            diagnostics_by_line
                .entry(range.start.line)
                .or_default()
                .push(LineDiagnostic {
                    severity: diagnostic.severity,
                    code: diagnostic.code,
                    message: diagnostic.message,
                    column: Some(range.start.character),
                });
        }

        (0..self.line_count())
            .filter_map(|line| {
                let number = u32::try_from(line + 1).ok()?;
                Some(SemanticLine {
                    number,
                    text: self.line_text(line)?,
                    diagnostics: diagnostics_by_line
                        .remove(&u32::try_from(line).ok()?)
                        .unwrap_or_default(),
                    highlights: Vec::new(),
                })
            })
            .collect()
    }

    pub fn semantic_lines_in_range(&self, start_line: usize, end_line: usize) -> Vec<SemanticLine> {
        let total = self.line_count();
        if start_line >= total {
            return Vec::new();
        }
        let end = end_line.min(total);

        let mut diagnostics_by_line = BTreeMap::<u32, Vec<LineDiagnostic>>::new();
        for diagnostic in self.diagnostics() {
            let Some(range) = diagnostic.range else {
                continue;
            };

            diagnostics_by_line
                .entry(range.start.line)
                .or_default()
                .push(LineDiagnostic {
                    severity: diagnostic.severity,
                    code: diagnostic.code,
                    message: diagnostic.message,
                    column: Some(range.start.character),
                });
        }
        let mut highlights_by_line = self.syntax_highlights_by_line(start_line, end);

        (start_line..end)
            .filter_map(|line| {
                let number = u32::try_from(line + 1).ok()?;
                Some(SemanticLine {
                    number,
                    text: self.line_text(line)?,
                    diagnostics: diagnostics_by_line
                        .remove(&u32::try_from(line).ok()?)
                        .unwrap_or_default(),
                    highlights: highlights_by_line
                        .remove(&u32::try_from(line).ok()?)
                        .unwrap_or_default(),
                })
            })
            .collect()
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

    fn proxy_chain_preview_for_entry(&self, entry: &str) -> Option<ProxyChainPreview> {
        let dialer_targets: BTreeMap<_, _> = self
            .semantic_index
            .dialer_proxy_edges
            .iter()
            .map(|edge| (edge.from.as_str(), edge.to.as_str()))
            .collect();

        if !dialer_targets.contains_key(entry) {
            return None;
        }

        let known_outbounds: BTreeSet<_> =
            self.outbound_names_with_builtins().into_iter().collect();
        let proxy_names: BTreeSet<_> = self
            .semantic_index
            .proxies
            .iter()
            .map(|proxy| proxy.name.as_str())
            .collect();

        let mut steps = vec!["Local".to_string(), entry.to_string()];
        let mut visited = BTreeSet::new();
        let mut current = entry;

        loop {
            if !visited.insert(current) {
                return Some(ProxyChainPreview {
                    steps,
                    is_definite: false,
                    status: ProxyChainStatus::Cycle,
                    message: Some(format!(
                        "dialer-proxy chain contains a cycle at `{current}`"
                    )),
                });
            }

            let Some(next) = dialer_targets.get(current).copied() else {
                steps.push("Target".to_string());
                return Some(ProxyChainPreview {
                    steps,
                    is_definite: true,
                    status: ProxyChainStatus::Complete,
                    message: None,
                });
            };

            steps.push(next.to_string());

            if !known_outbounds.contains(next) {
                return Some(ProxyChainPreview {
                    steps,
                    is_definite: false,
                    status: ProxyChainStatus::MissingReference,
                    message: Some(format!(
                        "dialer-proxy `{next}` referenced by `{current}` does not exist"
                    )),
                });
            }

            if !proxy_names.contains(next) {
                steps.push("Target".to_string());
                return Some(ProxyChainPreview {
                    steps,
                    is_definite: true,
                    status: ProxyChainStatus::Complete,
                    message: None,
                });
            }

            current = next;
        }
    }

    fn refresh_indexes(&mut self) {
        let text = self.text.as_string();
        self.yaml = YamlDocument::parse(&text);
        self.semantic_index = SemanticIndex::from_yaml_str(&text);
    }

    fn syntax_highlights_by_line(
        &self,
        start_line: usize,
        end_line: usize,
    ) -> BTreeMap<u32, Vec<SyntaxHighlightSpan>> {
        let mut highlights_by_line = BTreeMap::<u32, Vec<SyntaxHighlightSpan>>::new();
        for token in self.yaml.syntax_tokens_in_line_range(start_line, end_line) {
            if let Some((line, highlight)) = self.line_highlight_span(token) {
                highlights_by_line.entry(line).or_default().push(highlight);
            }
        }
        highlights_by_line
    }

    fn line_highlight_span(&self, token: SyntaxToken) -> Option<(u32, SyntaxHighlightSpan)> {
        let line = token.range.start.line;
        let text = self.line_text(line as usize)?;
        let line_length = text.chars().count() as u32;
        let start = token.range.start.character.min(line_length);
        let end = if token.range.end.line == line {
            token.range.end.character.min(line_length)
        } else {
            line_length
        };
        (start < end).then_some((
            line,
            SyntaxHighlightSpan {
                start,
                end,
                kind: token.kind,
            },
        ))
    }
}

fn is_proxy_group_member_path(path: &str) -> bool {
    path.starts_with("proxy-groups[") && (path.ends_with(".proxies") || path.contains(".proxies["))
}

fn is_dialer_proxy_path(path: &str) -> bool {
    path.starts_with("proxies[") && path.ends_with(".dialer-proxy")
}

fn proxy_index_from_dialer_proxy_path(path: &YamlPath) -> Option<usize> {
    match path.segments.as_slice() {
        [
            YamlPathSegment::Key(root),
            YamlPathSegment::Index(index),
            YamlPathSegment::Key(field),
        ] if root == "proxies" && field == "dialer-proxy" => Some(*index),
        _ => None,
    }
}

fn reference_completion(name: &str) -> mocode_mihomo_schema::SchemaCompletion {
    mocode_mihomo_schema::SchemaCompletion {
        label: name.to_string(),
        insert_text: name.to_string(),
        kind: CompletionKind::Reference,
        documentation: Some("Mihomo outbound reference".to_string()),
    }
}

fn first_markdown_paragraph(markdown: &str) -> String {
    markdown
        .split("\n\n")
        .find_map(|paragraph| {
            let trimmed = paragraph.trim();
            (!trimmed.is_empty()).then(|| trimmed.replace('\n', " "))
        })
        .unwrap_or_default()
}

/// Returns `true` when `character` is at or after the colon of `dialer-proxy:`.
/// This rejects cursor positions on the key name itself, leading whitespace, comments, etc.
fn cursor_on_dialer_proxy_value(line: &str, character: u32) -> bool {
    let indent = line.len() - line.trim_start().len();
    let trimmed = line.trim_start();
    if !trimmed.starts_with("dialer-proxy:") {
        return false;
    }
    // Cursor must be at or past the end of "dialer-proxy" (i.e. on the `:` or value)
    let key_end = indent + "dialer-proxy".len();
    (character as usize) >= key_end
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
    fn returns_compact_hover_summary_for_ui_adapters() {
        let editor = MocodeEditor::open_text("tun:\n  stack: system\n");
        let hover = editor.hover_summary_at(TextPosition::new(1, 4)).unwrap();

        assert_eq!(hover.title, "tun.stack");
        assert!(hover.body.contains("TUN network stack"));
        assert!(!hover.body.contains("\n\n"));
    }

    #[test]
    fn groups_ranged_diagnostics_by_line_for_ui_adapters() {
        let editor =
            MocodeEditor::open_text(include_str!("../../../examples/configs/invalid-yaml.yaml"));
        let lines = editor.semantic_lines();

        assert!(lines.iter().any(|line| {
            line.diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "yaml.syntax")
        }));
        assert!(
            lines
                .iter()
                .flat_map(|line| &line.diagnostics)
                .all(|diagnostic| diagnostic.column.is_some())
        );
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
    fn missing_reference_diagnostics_keep_source_ranges() {
        let editor = MocodeEditor::open_text(include_str!(
            "../../../examples/configs/invalid-reference.yaml"
        ));
        let diagnostics = editor.diagnostics();

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

            assert!(
                diagnostic.range.is_some(),
                "diagnostic for {missing} should keep lint range"
            );
        }
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
    fn returns_proxy_chain_preview_for_dialer_proxy() {
        let editor = MocodeEditor::open_text(
            "proxies:\n  - name: entry\n    type: ss\n    dialer-proxy: mid\n  - name: mid\n    type: ss\n    dialer-proxy: exit\n  - name: exit\n    type: ss\n",
        );

        let preview = editor
            .proxy_chain_preview_at(TextPosition::new(3, 20))
            .unwrap();

        assert_eq!(
            preview.steps,
            vec!["Local", "entry", "mid", "exit", "Target"]
        );
        assert_eq!(preview.status, ProxyChainStatus::Complete);
        assert_eq!(preview.message, None);
        assert!(preview.is_definite);
    }

    #[test]
    fn proxy_chain_preview_reports_missing_dialer_target() {
        let editor = MocodeEditor::open_text(
            "proxies:\n  - name: entry\n    type: ss\n    dialer-proxy: missing\n",
        );

        let preview = editor
            .proxy_chain_preview_at(TextPosition::new(3, 22))
            .unwrap();

        assert_eq!(preview.steps, vec!["Local", "entry", "missing"]);
        assert_eq!(preview.status, ProxyChainStatus::MissingReference);
        assert_eq!(
            preview.message.as_deref(),
            Some("dialer-proxy `missing` referenced by `entry` does not exist")
        );
        assert!(!preview.is_definite);
    }

    #[test]
    fn proxy_chain_preview_reports_dialer_cycle() {
        let editor =
            MocodeEditor::open_text(include_str!("../../../tests/fixtures/dialer-cycle.yaml"));

        let preview = editor
            .proxy_chain_preview_at(TextPosition::new(10, 20))
            .unwrap();

        assert_eq!(preview.steps, vec!["Local", "a", "b", "a"]);
        assert_eq!(preview.status, ProxyChainStatus::Cycle);
        assert_eq!(
            preview.message.as_deref(),
            Some("dialer-proxy chain contains a cycle at `a`")
        );
        assert!(!preview.is_definite);
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
    fn loads_twenty_thousand_line_fixture_for_validation_baseline() {
        let text = include_str!("../../../examples/configs/large-20000.yaml");
        let editor = MocodeEditor::open_text(text);

        assert!(text.lines().count() >= 20_000);
        assert!(editor.line_count() >= 20_000);
        assert_eq!(editor.line_text(0), Some("mixed-port: 7890".to_string()));
        let diagnostics = editor.diagnostics();
        assert!(diagnostics.is_empty(), "{diagnostics:#?}");
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
    fn inserts_text_through_core_and_recomputes_semantics() {
        let mut editor = MocodeEditor::open_text(
            "proxy-groups:\n  - name: Proxy\n    type: select\n    proxies:\n      - missing\n",
        );
        assert!(
            editor
                .diagnostics()
                .iter()
                .any(|diagnostic| diagnostic.code == "mihomo.reference.missing")
        );

        let cursor = editor
            .insert_text_at(
                TextPosition::new(0, 0),
                "proxies:\n  - name: missing\n    type: ss\n",
            )
            .unwrap();

        assert_eq!(cursor, TextPosition::new(3, 0));
        assert!(
            editor
                .diagnostics()
                .iter()
                .all(|diagnostic| diagnostic.code != "mihomo.reference.missing")
        );
    }

    #[test]
    fn undo_redo_through_core_recomputes_semantics() {
        let mut editor = MocodeEditor::open_text(
            "proxy-groups:\n  - name: Proxy\n    type: select\n    proxies:\n      - missing\n",
        );

        assert!(
            editor
                .diagnostics()
                .iter()
                .any(|diagnostic| diagnostic.code == "mihomo.reference.missing")
        );

        let cursor = editor
            .insert_text_at(
                TextPosition::new(0, 0),
                "proxies:\n  - name: missing\n    type: ss\n",
            )
            .unwrap();
        assert_eq!(cursor, TextPosition::new(3, 0));
        assert!(
            editor
                .diagnostics()
                .iter()
                .all(|diagnostic| diagnostic.code != "mihomo.reference.missing")
        );

        let undo_cursor = editor.undo().unwrap();
        assert_eq!(undo_cursor, Some(TextPosition::new(0, 0)));
        assert!(
            editor
                .diagnostics()
                .iter()
                .any(|diagnostic| diagnostic.code == "mihomo.reference.missing")
        );

        let redo_cursor = editor.redo().unwrap();
        assert_eq!(redo_cursor, Some(TextPosition::new(3, 0)));
        assert!(
            editor
                .diagnostics()
                .iter()
                .all(|diagnostic| diagnostic.code != "mihomo.reference.missing")
        );
    }

    #[test]
    fn exposes_core_cursor_movement_and_delete_helpers() {
        let mut editor = MocodeEditor::open_text("dns:\n  enable: true\n");

        assert_eq!(editor.line_end_position(0), Some(TextPosition::new(0, 4)));
        assert_eq!(
            editor.move_left(TextPosition::new(1, 0)).unwrap(),
            TextPosition::new(0, 4)
        );
        assert_eq!(
            editor.move_right(TextPosition::new(0, 4)).unwrap(),
            TextPosition::new(1, 0)
        );

        assert_eq!(
            editor.backspace_at(TextPosition::new(1, 2)).unwrap(),
            TextPosition::new(1, 1)
        );
        assert_eq!(editor.line_text(1), Some(" enable: true".to_string()));

        assert_eq!(
            editor.delete_at(TextPosition::new(0, 4)).unwrap(),
            TextPosition::new(0, 4)
        );
        assert_eq!(editor.line_text(0), Some("dns: enable: true".to_string()));
    }

    #[test]
    fn text_in_range_delegates_to_shared_text_buffer() {
        let editor = MocodeEditor::open_text("alpha\nbeta\ngamma\n");

        assert_eq!(
            editor
                .text_in_range(TextRange::new(
                    TextPosition::new(2, 2),
                    TextPosition::new(0, 2)
                ))
                .unwrap(),
            "pha\nbeta\nga"
        );
    }

    #[test]
    fn vertical_navigation_delegates_to_text_buffer() {
        let editor = MocodeEditor::open_text("short\nvery long line here\n");

        // move_up with column clamp
        assert_eq!(
            editor.move_up(TextPosition::new(1, 10)).unwrap(),
            TextPosition::new(0, 5) // "short" is 5 chars
        );
        // move_down preserves column
        assert_eq!(
            editor.move_down(TextPosition::new(0, 3)).unwrap(),
            TextPosition::new(1, 3)
        );
        // home / end
        assert_eq!(
            editor.move_line_start(TextPosition::new(1, 8)).unwrap(),
            TextPosition::new(1, 0)
        );
        assert_eq!(
            editor.move_line_end(TextPosition::new(0, 1)).unwrap(),
            TextPosition::new(0, 5)
        );
        // page
        assert_eq!(
            editor.page_up(TextPosition::new(1, 0), 1).unwrap(),
            TextPosition::new(0, 0)
        );
        assert_eq!(
            editor.page_down(TextPosition::new(0, 0), 1).unwrap(),
            TextPosition::new(1, 0)
        );
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

    // ── proxy_chain_preview_at negative cursor tests ──

    #[test]
    fn proxy_chain_preview_returns_none_on_key_text() {
        let editor = MocodeEditor::open_text(
            "proxies:\n  - name: entry\n    type: ss\n    dialer-proxy: mid\n  - name: mid\n    type: ss\n",
        );

        // cursor on the 'd' of "dialer-proxy" — not a value position
        assert_eq!(editor.proxy_chain_preview_at(TextPosition::new(3, 4)), None);
    }

    #[test]
    fn proxy_chain_preview_returns_none_on_indentation() {
        let editor = MocodeEditor::open_text(
            "proxies:\n  - name: entry\n    type: ss\n    dialer-proxy: mid\n  - name: mid\n    type: ss\n",
        );

        // cursor at column 0 on the dialer-proxy line — leading whitespace
        assert_eq!(editor.proxy_chain_preview_at(TextPosition::new(3, 0)), None);
    }

    #[test]
    fn proxy_chain_preview_returns_none_on_blank_line() {
        let editor = MocodeEditor::open_text(
            "proxies:\n  - name: entry\n    type: ss\n    dialer-proxy: mid\n\n  - name: mid\n    type: ss\n",
        );

        // cursor on blank line 4 — should not return preview
        assert_eq!(editor.proxy_chain_preview_at(TextPosition::new(4, 0)), None);
    }

    #[test]
    fn proxy_chain_preview_returns_none_on_comment_line() {
        let editor = MocodeEditor::open_text(
            "proxies:\n  - name: entry\n    type: ss\n    # dialer-proxy: mid\n    dialer-proxy: mid\n  - name: mid\n    type: ss\n",
        );

        // cursor on the comment line (line 3) — should not return preview
        assert_eq!(
            editor.proxy_chain_preview_at(TextPosition::new(3, 16)),
            None
        );
    }

    #[test]
    fn proxy_chain_preview_returns_none_on_unrelated_field() {
        let editor = MocodeEditor::open_text(
            "proxies:\n  - name: entry\n    type: ss\n    dialer-proxy: mid\n  - name: mid\n    type: ss\n",
        );

        // cursor on "type: ss" line — not a dialer-proxy field
        assert_eq!(editor.proxy_chain_preview_at(TextPosition::new(2, 8)), None);
    }

    // ── built-in outbound tests ──

    #[test]
    fn proxy_chain_preview_completes_for_direct_outbound() {
        let editor = MocodeEditor::open_text(
            "proxies:\n  - name: entry\n    type: ss\n    dialer-proxy: DIRECT\n",
        );

        let preview = editor
            .proxy_chain_preview_at(TextPosition::new(3, 22))
            .unwrap();

        assert_eq!(preview.steps, vec!["Local", "entry", "DIRECT", "Target"]);
        assert_eq!(preview.status, ProxyChainStatus::Complete);
        assert!(preview.is_definite);
    }

    #[test]
    fn proxy_chain_preview_completes_for_reject_outbound() {
        let editor = MocodeEditor::open_text(
            "proxies:\n  - name: entry\n    type: ss\n    dialer-proxy: REJECT\n",
        );

        let preview = editor
            .proxy_chain_preview_at(TextPosition::new(3, 22))
            .unwrap();

        assert_eq!(preview.steps, vec!["Local", "entry", "REJECT", "Target"]);
        assert_eq!(preview.status, ProxyChainStatus::Complete);
        assert!(preview.is_definite);
    }

    // ── incomplete editing test: unnamed proxy before named proxy ──

    #[test]
    fn proxy_chain_preview_targets_correct_proxy_with_unnamed_preceding_item() {
        // YAML sequence: index 0 has no name → skipped by lint
        //               index 1 is "alpha" → proxies[0] in semantic_index
        //               index 2 is "beta"  → proxies[1] in semantic_index
        //               index 3 is "mid"   → proxies[2] in semantic_index
        let editor = MocodeEditor::open_text(
            "proxies:\n  - type: ss\n    dialer-proxy: x\n  - name: alpha\n    type: ss\n    dialer-proxy: mid\n  - name: beta\n    type: ss\n    dialer-proxy: exit\n  - name: mid\n    type: ss\n",
        );

        // Cursor on "dialer-proxy: mid" under alpha (line 5).
        // YAML path = proxies[1].dialer-proxy
        // Current bug: uses semantic_index.proxies[1] = "beta" (wrong!)
        // Correct: must resolve to "alpha"
        let preview = editor
            .proxy_chain_preview_at(TextPosition::new(5, 22))
            .unwrap();

        assert_eq!(preview.steps, vec!["Local", "alpha", "mid", "Target"]);
        assert_eq!(preview.status, ProxyChainStatus::Complete);
        assert!(preview.is_definite);
    }

    // ── viewport slice tests ──

    #[test]
    fn semantic_lines_in_range_returns_only_requested_range() {
        let editor = MocodeEditor::open_text(
            "mixed-port: 7890\nmode: rule\nproxies:\n  - name: hk-1\n    type: ss\n",
        );
        // Lines 1-3 (0-indexed: 1,2)
        let slice = editor.semantic_lines_in_range(1, 3);

        assert_eq!(slice.len(), 2);
        assert_eq!(slice[0].number, 2);
        assert_eq!(slice[0].text, "mode: rule");
        assert_eq!(slice[1].number, 3);
        assert_eq!(slice[1].text, "proxies:");
    }

    #[test]
    fn semantic_lines_in_range_respects_line_count_bound() {
        let editor = MocodeEditor::open_text("a\nb\nc\n");
        // "a\nb\nc\n" has 4 lines (trailing newline → empty last line).
        // Request beyond EOF — should clamp to line_count (4).
        let slice = editor.semantic_lines_in_range(1, 100);
        assert_eq!(slice.len(), 3); // lines at index 1,2,3
        assert_eq!(slice[0].number, 2);
        assert_eq!(slice[2].number, 4);
    }

    #[test]
    fn semantic_lines_in_range_empty_for_out_of_bounds() {
        let editor = MocodeEditor::open_text("a\nb\n");
        let slice = editor.semantic_lines_in_range(5, 10);
        assert!(slice.is_empty());
    }

    #[test]
    fn semantic_lines_in_range_includes_diagnostics() {
        let editor =
            MocodeEditor::open_text(include_str!("../../../examples/configs/invalid-yaml.yaml"));
        // Line 2 (0-indexed) has a YAML syntax error
        let slice = editor.semantic_lines_in_range(0, 5);

        assert!(!slice.is_empty());
        assert!(
            slice.iter().any(|line| !line.diagnostics.is_empty()),
            "slice should include at least one line with diagnostics"
        );
    }

    #[test]
    fn semantic_lines_in_range_carries_syntax_highlight_spans_for_visible_lines_only() {
        let editor = MocodeEditor::open_text("# hidden\nmixed-port: 7890\nallow-lan: true\n");

        let slice = editor.semantic_lines_in_range(1, 3);

        assert_eq!(slice.len(), 2);
        assert!(
            slice
                .iter()
                .flat_map(|line| &line.highlights)
                .all(|highlight| highlight.start < highlight.end)
        );
        assert!(
            slice[0].highlights.iter().any(|highlight| {
                highlight.start == 0
                    && highlight.end == 10
                    && highlight.kind == SyntaxHighlightKind::Key
            }),
            "expected key highlight on visible line: {:#?}",
            slice[0].highlights
        );
        assert!(
            slice[0].highlights.iter().any(|highlight| {
                highlight.start == 12
                    && highlight.end == 16
                    && highlight.kind == SyntaxHighlightKind::Number
            }),
            "expected number highlight on visible line: {:#?}",
            slice[0].highlights
        );
        assert!(
            slice[1].highlights.iter().any(|highlight| {
                highlight.start == 11
                    && highlight.end == 15
                    && highlight.kind == SyntaxHighlightKind::Boolean
            }),
            "expected boolean highlight on visible line: {:#?}",
            slice[1].highlights
        );
        assert!(
            slice
                .iter()
                .flat_map(|line| &line.highlights)
                .all(|highlight| highlight.kind != SyntaxHighlightKind::Comment),
            "line 0 comment must not be included in requested visible slice"
        );
    }

    #[test]
    fn semantic_lines_in_range_highlights_block_scalar_when_viewport_starts_inside_token() {
        let editor = MocodeEditor::open_text("payload: |\n  alpha\n  beta\n  gamma\nnext: true\n");

        let slice = editor.semantic_lines_in_range(2, 4);

        assert_eq!(slice.len(), 2);
        assert!(slice[0].highlights.iter().any(|highlight| {
            highlight.start == 0
                && highlight.end == 6
                && highlight.kind == SyntaxHighlightKind::String
        }));
        assert!(slice[1].highlights.iter().any(|highlight| {
            highlight.start == 0
                && highlight.end == 7
                && highlight.kind == SyntaxHighlightKind::String
        }));
    }

    #[test]
    fn semantic_lines_keeps_highlights_empty_for_non_viewport_snapshot_path() {
        let editor = MocodeEditor::open_text("mixed-port: 7890\nbad: [\n");

        let lines = editor.semantic_lines();

        assert!(lines.iter().all(|line| line.highlights.is_empty()));
        assert!(
            lines.iter().any(|line| !line.diagnostics.is_empty()),
            "diagnostic aggregation should remain available on full semantic_lines path"
        );
    }

    #[test]
    fn semantic_lines_in_range_highlights_small_slice_of_large_fixture() {
        let text = include_str!("../../../examples/configs/large-20000.yaml");
        let editor = MocodeEditor::open_text(text);

        let slice = editor.semantic_lines_in_range(0, 3);

        assert_eq!(slice.len(), 3);
        assert!(slice.iter().any(|line| !line.highlights.is_empty()));
        assert!(
            slice
                .iter()
                .flat_map(|line| &line.highlights)
                .all(|highlight| highlight.start < highlight.end)
        );
    }

    #[test]
    fn semantic_lines_in_range_handles_bottom_slice_of_large_fixture() {
        let text = include_str!("../../../examples/configs/large-20000.yaml");
        let editor = MocodeEditor::open_text(text);

        let slice = editor.semantic_lines_in_range(19_990, 20_000);

        assert_eq!(slice.len(), 10);
        assert_eq!(slice[0].number, 19_991);
        assert!(
            slice
                .iter()
                .flat_map(|line| &line.highlights)
                .any(|highlight| highlight.kind == SyntaxHighlightKind::Comment)
        );
        assert!(
            slice
                .iter()
                .flat_map(|line| &line.highlights)
                .all(|highlight| highlight.start < highlight.end)
        );
    }

    #[test]
    fn semantic_lines_in_range_highlights_constructed_twenty_thousand_line_tail() {
        let mut text = String::new();
        for _ in 0..19_990 {
            text.push('\n');
        }
        text.push_str("tail-number: 9000\n");
        text.push_str("tail-bool: true\n");
        text.push_str("tail-string: done\n");

        let editor = MocodeEditor::open_text(text);
        let slice = editor.semantic_lines_in_range(19_990, 20_000);

        assert_eq!(slice[0].number, 19_991);
        assert!(slice[0].highlights.iter().any(|highlight| {
            highlight.kind == SyntaxHighlightKind::Number && highlight.start == 13
        }));
        assert!(slice[1].highlights.iter().any(|highlight| {
            highlight.kind == SyntaxHighlightKind::Boolean && highlight.start == 11
        }));
        assert!(slice[2].highlights.iter().any(|highlight| {
            highlight.kind == SyntaxHighlightKind::String && highlight.start == 13
        }));
    }
}
