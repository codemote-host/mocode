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
}

impl YamlDocument {
    pub fn parse(text: impl Into<String>) -> Self {
        let text = text.into();
        let tree = parse_tree(&text);
        Self { text, tree }
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

    fn point_to_position(&self, point: Point) -> TextPosition {
        TextPosition::new(
            point.row as u32,
            self.byte_column_to_char(point.row, point.column) as u32,
        )
    }

    fn byte_column_to_char(&self, line_index: usize, byte_column: usize) -> usize {
        let Some(line) = self.text.lines().nth(line_index) else {
            return 0;
        };
        let clamped = byte_column.min(line.len());
        line[..clamped].chars().count()
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
}
