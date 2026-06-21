use std::fmt;

use mocode_text::{TextPosition, TextRange};
use tree_sitter::{Node, Parser, Point, Tree};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum YamlPathSegment {
    Key(String),
    Index(usize),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct YamlPath {
    pub segments: Vec<YamlPathSegment>,
}

impl YamlPath {
    pub fn root() -> Self {
        Self::default()
    }

    pub fn key(mut self, key: impl Into<String>) -> Self {
        self.segments.push(YamlPathSegment::Key(key.into()));
        self
    }

    pub fn index(mut self, index: usize) -> Self {
        self.segments.push(YamlPathSegment::Index(index));
        self
    }
}

impl fmt::Display for YamlPath {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.segments.is_empty() {
            return formatter.write_str("<root>");
        }

        let mut first = true;
        for segment in &self.segments {
            match segment {
                YamlPathSegment::Key(key) => {
                    if !first {
                        formatter.write_str(".")?;
                    }
                    formatter.write_str(key)?;
                }
                YamlPathSegment::Index(index) => {
                    write!(formatter, "[{index}]")?;
                }
            }
            first = false;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YamlSyntaxError {
    pub range: TextRange,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SyntaxHighlightKind {
    Comment,
    Key,
    String,
    Number,
    Boolean,
    Null,
    Anchor,
    Alias,
    Tag,
    Punctuation,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntaxToken {
    pub range: TextRange,
    pub kind: SyntaxHighlightKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FormatOptions {
    pub indent_width: u8,
    pub preserve_comments: bool,
}

impl Default for FormatOptions {
    fn default() -> Self {
        Self {
            indent_width: 2,
            preserve_comments: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct YamlDocument {
    text: String,
    tree: Option<Tree>,
    line_starts: Vec<usize>,
}

impl YamlDocument {
    pub fn parse(text: impl Into<String>) -> Self {
        let text = text.into();
        let tree = parse_tree(&text);
        let line_starts = line_starts(&text);
        Self {
            text,
            tree,
            line_starts,
        }
    }

    pub fn source(&self) -> &str {
        &self.text
    }

    pub fn syntax_errors(&self) -> Vec<YamlSyntaxError> {
        let Some(tree) = &self.tree else {
            return vec![YamlSyntaxError {
                range: TextRange::empty(TextPosition::new(0, 0)),
                message: "failed to initialize YAML parser".to_string(),
            }];
        };

        let mut errors = Vec::new();
        collect_error_nodes(self, tree.root_node(), &mut errors);
        errors
    }

    pub fn syntax_tokens_in_line_range(
        &self,
        start_line: usize,
        end_line: usize,
    ) -> Vec<SyntaxToken> {
        if start_line >= end_line {
            return Vec::new();
        }

        let Some(tree) = &self.tree else {
            return vec![SyntaxToken {
                range: TextRange::empty(TextPosition::new(0, 0)),
                kind: SyntaxHighlightKind::Error,
            }];
        };

        let mut tokens = Vec::new();
        collect_syntax_tokens(self, tree.root_node(), start_line, end_line, &mut tokens);
        tokens.sort_by_key(|token| (token.range.start.line, token.range.start.character));
        tokens.dedup();
        tokens
    }

    pub fn path_at(&self, position: TextPosition) -> Option<YamlPath> {
        let target_line = position.line as usize;
        let mut stack = Vec::<PathFrame>::new();
        let mut sequence_indexes = Vec::<SequenceCounter>::new();
        let mut current_path = None;

        for (line_index, line) in self.text.lines().enumerate() {
            if line_index > target_line {
                break;
            }

            let Some(line_info) = parse_yaml_line(line) else {
                continue;
            };

            stack.retain(|frame| frame.indent < line_info.indent);

            match line_info.kind {
                LineKind::MapKey {
                    key,
                    has_inline_value,
                } => {
                    let mut path = path_from_stack(&stack);
                    path.segments.push(YamlPathSegment::Key(key.clone()));
                    current_path = Some(path);

                    if !has_inline_value {
                        stack.push(PathFrame {
                            indent: line_info.indent,
                            segment: YamlPathSegment::Key(key),
                        });
                    }
                }
                LineKind::SequenceItem { inline_key } => {
                    let parent_path = path_from_stack(&stack);
                    let index =
                        next_sequence_index(&mut sequence_indexes, line_info.indent, &parent_path);

                    stack.push(PathFrame {
                        indent: line_info.indent,
                        segment: YamlPathSegment::Index(index),
                    });

                    let mut path = parent_path.index(index);
                    if let Some((key, has_inline_value)) = inline_key {
                        path.segments.push(YamlPathSegment::Key(key.clone()));
                        if !has_inline_value {
                            stack.push(PathFrame {
                                indent: line_info.indent + 1,
                                segment: YamlPathSegment::Key(key),
                            });
                        }
                    }
                    current_path = Some(path);
                }
            }
        }

        current_path
    }
}

fn collect_syntax_tokens(
    document: &YamlDocument,
    node: Node<'_>,
    start_line: usize,
    end_line: usize,
    tokens: &mut Vec<SyntaxToken>,
) {
    if !node_overlaps_line_range(node, start_line, end_line) {
        return;
    }

    if node.is_error() || node.is_missing() {
        push_syntax_token(
            document,
            node,
            SyntaxHighlightKind::Error,
            start_line,
            end_line,
            tokens,
        );
    }

    if matches!(node.kind(), "block_mapping_pair" | "flow_pair") {
        let key = node.child_by_field_name("key");
        if let Some(key) = key {
            push_key_token(document, key, start_line, end_line, tokens);
        }

        for index in 0..node.child_count() {
            if let Some(child) = node.child(index) {
                if key.is_some_and(|key| same_node_range(key, child)) {
                    continue;
                }
                if child_ends_before_line_range(child, start_line) {
                    continue;
                }
                if child_starts_after_line_range(child, end_line) {
                    break;
                }
                collect_syntax_tokens(document, child, start_line, end_line, tokens);
            }
        }
        return;
    }

    if let Some(kind) = syntax_kind_for_node(node) {
        push_syntax_token(document, node, kind, start_line, end_line, tokens);
        return;
    }

    for index in 0..node.child_count() {
        if let Some(child) = node.child(index) {
            if child_ends_before_line_range(child, start_line) {
                continue;
            }
            if child_starts_after_line_range(child, end_line) {
                break;
            }
            collect_syntax_tokens(document, child, start_line, end_line, tokens);
        }
    }
}

fn syntax_kind_for_node(node: Node<'_>) -> Option<SyntaxHighlightKind> {
    match node.kind() {
        "comment" => Some(SyntaxHighlightKind::Comment),
        "double_quote_scalar"
        | "single_quote_scalar"
        | "block_scalar"
        | "string_scalar"
        | "timestamp_scalar" => Some(SyntaxHighlightKind::String),
        "integer_scalar" | "float_scalar" => Some(SyntaxHighlightKind::Number),
        "boolean_scalar" => Some(SyntaxHighlightKind::Boolean),
        "null_scalar" => Some(SyntaxHighlightKind::Null),
        "anchor" => Some(SyntaxHighlightKind::Anchor),
        "alias" => Some(SyntaxHighlightKind::Alias),
        "tag" | "tag_handle" | "tag_prefix" => Some(SyntaxHighlightKind::Tag),
        ":" | "-" | "?" | "," | "[" | "]" | "{" | "}" | "---" | "..." => {
            Some(SyntaxHighlightKind::Punctuation)
        }
        _ => None,
    }
}

fn push_key_token(
    document: &YamlDocument,
    node: Node<'_>,
    start_line: usize,
    end_line: usize,
    tokens: &mut Vec<SyntaxToken>,
) {
    let range = document.trimmed_node_range(node);
    push_syntax_range(
        document,
        range,
        SyntaxHighlightKind::Key,
        start_line,
        end_line,
        tokens,
    );
}

fn push_syntax_token(
    document: &YamlDocument,
    node: Node<'_>,
    kind: SyntaxHighlightKind,
    start_line: usize,
    end_line: usize,
    tokens: &mut Vec<SyntaxToken>,
) {
    push_syntax_range(
        document,
        document.node_range(node),
        kind,
        start_line,
        end_line,
        tokens,
    );
}

fn push_syntax_range(
    document: &YamlDocument,
    range: TextRange,
    kind: SyntaxHighlightKind,
    start_line: usize,
    end_line: usize,
    tokens: &mut Vec<SyntaxToken>,
) {
    let first_line = (range.start.line as usize).max(start_line);
    let last_line = (range.end.line as usize).min(end_line.saturating_sub(1));
    if first_line > last_line {
        return;
    }

    for line in first_line..=last_line {
        let line_u32 = line as u32;
        let start_character = if line_u32 == range.start.line {
            range.start.character
        } else {
            0
        };
        let end_character = if line_u32 == range.end.line {
            range.end.character
        } else {
            document.line_char_count(line) as u32
        };
        if start_character < end_character {
            tokens.push(SyntaxToken {
                range: TextRange::new(
                    TextPosition::new(line_u32, start_character),
                    TextPosition::new(line_u32, end_character),
                ),
                kind,
            });
        }
    }
}

fn node_overlaps_line_range(node: Node<'_>, start_line: usize, end_line: usize) -> bool {
    node.start_position().row < end_line && node.end_position().row >= start_line
}

fn child_ends_before_line_range(child: Node<'_>, start_line: usize) -> bool {
    child.end_position().row < start_line
}

fn child_starts_after_line_range(child: Node<'_>, end_line: usize) -> bool {
    child.start_position().row >= end_line
}

fn same_node_range(left: Node<'_>, right: Node<'_>) -> bool {
    left.start_byte() == right.start_byte() && left.end_byte() == right.end_byte()
}

fn parse_tree(text: &str) -> Option<Tree> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_yaml::LANGUAGE.into())
        .ok()?;
    parser.parse(text, None)
}

fn collect_error_nodes(document: &YamlDocument, node: Node<'_>, errors: &mut Vec<YamlSyntaxError>) {
    if node.is_error() || node.is_missing() {
        errors.push(YamlSyntaxError {
            range: document.node_range(node),
            message: format!("YAML syntax error near {}", node.kind()),
        });
    }

    if !node.has_error() {
        return;
    }

    for index in 0..node.child_count() {
        if let Some(child) = node.child(index) {
            collect_error_nodes(document, child, errors);
        }
    }
}

impl YamlDocument {
    fn node_range(&self, node: Node<'_>) -> TextRange {
        let start = self.point_to_position(node.start_position());
        let mut end = self.point_to_position(node.end_position());
        if start == end {
            end.character = end.character.saturating_add(1);
        }
        TextRange::new(start, end)
    }

    fn trimmed_node_range(&self, node: Node<'_>) -> TextRange {
        let range = self.node_range(node);
        let text = &self.text[node.start_byte()..node.end_byte()];
        let leading_chars = text
            .chars()
            .take_while(|char| char.is_whitespace() || *char == '"' || *char == '\'')
            .count() as u32;
        let trailing_chars = text
            .chars()
            .rev()
            .take_while(|char| char.is_whitespace() || *char == '"' || *char == '\'')
            .count() as u32;

        let start = TextPosition::new(
            range.start.line,
            range.start.character.saturating_add(leading_chars),
        );
        let end = TextPosition::new(
            range.end.line,
            range.end.character.saturating_sub(trailing_chars),
        );
        if start < end {
            TextRange::new(start, end)
        } else {
            range
        }
    }

    fn point_to_position(&self, point: Point) -> TextPosition {
        TextPosition::new(
            point.row as u32,
            self.byte_column_to_char(point.row, point.column) as u32,
        )
    }

    fn byte_column_to_char(&self, line_index: usize, byte_column: usize) -> usize {
        let Some(line) = self.line_slice(line_index) else {
            return 0;
        };
        let clamped = byte_column.min(line.len());
        line[..clamped].chars().count()
    }

    fn line_char_count(&self, line_index: usize) -> usize {
        self.line_slice(line_index)
            .map(|line| line.chars().count())
            .unwrap_or_default()
    }

    fn line_slice(&self, line_index: usize) -> Option<&str> {
        let start = *self.line_starts.get(line_index)?;
        let end = self
            .line_starts
            .get(line_index + 1)
            .map(|next_start| {
                next_start.saturating_sub(line_ending_len_before(&self.text, *next_start))
            })
            .unwrap_or(self.text.len());
        self.text.get(start..end)
    }
}

fn line_starts(text: &str) -> Vec<usize> {
    let mut starts = vec![0];
    for (index, byte) in text.bytes().enumerate() {
        if byte == b'\n' {
            starts.push(index + 1);
        }
    }
    starts
}

fn line_ending_len_before(text: &str, next_start: usize) -> usize {
    if next_start == 0 {
        return 0;
    }
    let newline = next_start - 1;
    if newline > 0 && text.as_bytes().get(newline - 1) == Some(&b'\r') {
        2
    } else {
        1
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PathFrame {
    indent: usize,
    segment: YamlPathSegment,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SequenceCounter {
    indent: usize,
    parent: YamlPath,
    next_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LineInfo {
    indent: usize,
    kind: LineKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum LineKind {
    MapKey { key: String, has_inline_value: bool },
    SequenceItem { inline_key: Option<(String, bool)> },
}

fn parse_yaml_line(line: &str) -> Option<LineInfo> {
    let indent = line.chars().take_while(|char| *char == ' ').count();
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }

    if let Some(rest) = trimmed.strip_prefix("- ") {
        return Some(LineInfo {
            indent,
            kind: LineKind::SequenceItem {
                inline_key: parse_map_key(rest),
            },
        });
    }

    parse_map_key(trimmed).map(|(key, has_inline_value)| LineInfo {
        indent,
        kind: LineKind::MapKey {
            key,
            has_inline_value,
        },
    })
}

fn parse_map_key(text: &str) -> Option<(String, bool)> {
    let colon_index = text.find(':')?;
    let key = text[..colon_index]
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .to_string();

    if key.is_empty() || key.contains(' ') {
        return None;
    }

    let value = text[colon_index + 1..].trim();
    Some((key, !value.is_empty()))
}

fn path_from_stack(stack: &[PathFrame]) -> YamlPath {
    YamlPath {
        segments: stack.iter().map(|frame| frame.segment.clone()).collect(),
    }
}

fn next_sequence_index(
    counters: &mut Vec<SequenceCounter>,
    indent: usize,
    parent: &YamlPath,
) -> usize {
    if let Some(counter) = counters
        .iter_mut()
        .find(|counter| counter.indent == indent && counter.parent == *parent)
    {
        let index = counter.next_index;
        counter.next_index += 1;
        return index;
    }

    counters.push(SequenceCounter {
        indent,
        parent: parent.clone(),
        next_index: 1,
    });
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn displays_yaml_paths() {
        let path = YamlPath::root()
            .key("proxy-groups")
            .index(0)
            .key("proxies")
            .index(2);

        assert_eq!(path.to_string(), "proxy-groups[0].proxies[2]");
    }

    #[test]
    fn computes_path_for_sequence_member() {
        let doc = YamlDocument::parse(
            "proxy-groups:\n  - name: Proxy\n    proxies:\n      - hk-1\n      - DIRECT\n",
        );

        assert_eq!(
            doc.path_at(TextPosition::new(4, 9)).unwrap().to_string(),
            "proxy-groups[0].proxies[1]"
        );
    }

    #[test]
    fn computes_path_for_nested_dns_sequence_member() {
        let doc = YamlDocument::parse("dns:\n  nameserver:\n    - 1.1.1.1\n");

        assert_eq!(
            doc.path_at(TextPosition::new(2, 7)).unwrap().to_string(),
            "dns.nameserver[0]"
        );
    }

    #[test]
    fn reports_syntax_error_for_bad_yaml() {
        let doc = YamlDocument::parse(
            "proxies:\n  - name: broken\n    type: ss\n      server: example.com\n",
        );

        assert!(!doc.syntax_errors().is_empty());
    }

    #[test]
    fn line_slice_returns_requested_line_without_scanning_from_start() {
        let doc = YamlDocument::parse("first\nsecond\nthird\n");

        assert_eq!(doc.line_slice(0), Some("first"));
        assert_eq!(doc.line_slice(2), Some("third"));
        assert_eq!(doc.line_slice(3), Some(""));
        assert_eq!(doc.line_slice(4), None);
    }

    #[test]
    fn classifies_yaml_syntax_tokens_in_requested_line_range() {
        let doc = YamlDocument::parse(
            "# top\nmixed-port: 7890\nallow-lan: true\nproxies:\n  - name: &entry \"hk-1\"\n    dialer-proxy: *entry\n    udp: null\nbad: [\n",
        );

        let tokens = doc.syntax_tokens_in_line_range(1, 7);

        assert!(tokens.iter().all(|token| token.range.start.line >= 1));
        assert!(tokens.iter().all(|token| token.range.start.line < 7));
        assert!(tokens.contains(&SyntaxToken {
            range: TextRange::new(TextPosition::new(1, 0), TextPosition::new(1, 10)),
            kind: SyntaxHighlightKind::Key,
        }));
        assert!(tokens.contains(&SyntaxToken {
            range: TextRange::new(TextPosition::new(1, 12), TextPosition::new(1, 16)),
            kind: SyntaxHighlightKind::Number,
        }));
        assert!(tokens.contains(&SyntaxToken {
            range: TextRange::new(TextPosition::new(2, 11), TextPosition::new(2, 15)),
            kind: SyntaxHighlightKind::Boolean,
        }));
        assert!(tokens.contains(&SyntaxToken {
            range: TextRange::new(TextPosition::new(4, 10), TextPosition::new(4, 16)),
            kind: SyntaxHighlightKind::Anchor,
        }));
        assert!(tokens.contains(&SyntaxToken {
            range: TextRange::new(TextPosition::new(4, 17), TextPosition::new(4, 23)),
            kind: SyntaxHighlightKind::String,
        }));
        assert!(tokens.contains(&SyntaxToken {
            range: TextRange::new(TextPosition::new(5, 18), TextPosition::new(5, 24)),
            kind: SyntaxHighlightKind::Alias,
        }));
        assert!(tokens.contains(&SyntaxToken {
            range: TextRange::new(TextPosition::new(6, 9), TextPosition::new(6, 13)),
            kind: SyntaxHighlightKind::Null,
        }));
    }

    #[test]
    fn classifies_comments_punctuation_tags_and_errors() {
        let doc = YamlDocument::parse("# top\nvalue: !secret [broken\n");

        let tokens = doc.syntax_tokens_in_line_range(0, 2);

        assert!(tokens.contains(&SyntaxToken {
            range: TextRange::new(TextPosition::new(0, 0), TextPosition::new(0, 5)),
            kind: SyntaxHighlightKind::Comment,
        }));
        assert!(tokens.contains(&SyntaxToken {
            range: TextRange::new(TextPosition::new(1, 5), TextPosition::new(1, 6)),
            kind: SyntaxHighlightKind::Punctuation,
        }));
        assert!(tokens.contains(&SyntaxToken {
            range: TextRange::new(TextPosition::new(1, 7), TextPosition::new(1, 14)),
            kind: SyntaxHighlightKind::Tag,
        }));
        assert!(
            tokens
                .iter()
                .any(|token| token.kind == SyntaxHighlightKind::Error)
        );
    }

    #[test]
    fn returns_visible_segments_for_block_scalar_tokens_that_start_above_range() {
        let doc = YamlDocument::parse("payload: |\n  alpha\n  beta\n  gamma\nnext: true\n");

        let tokens = doc.syntax_tokens_in_line_range(2, 4);

        assert!(tokens.contains(&SyntaxToken {
            range: TextRange::new(TextPosition::new(2, 0), TextPosition::new(2, 6)),
            kind: SyntaxHighlightKind::String,
        }));
        assert!(tokens.contains(&SyntaxToken {
            range: TextRange::new(TextPosition::new(3, 0), TextPosition::new(3, 7)),
            kind: SyntaxHighlightKind::String,
        }));
        assert!(tokens.iter().all(|token| token.range.start.line >= 2));
        assert!(tokens.iter().all(|token| token.range.start.line < 4));
    }
}
