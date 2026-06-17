use std::fmt;

use mocode_text::{TextPosition, TextRange};

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
}

impl YamlDocument {
    pub fn parse(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }

    pub fn source(&self) -> &str {
        &self.text
    }

    pub fn syntax_errors(&self) -> Vec<YamlSyntaxError> {
        Vec::new()
    }

    pub fn path_at(&self, _position: TextPosition) -> Option<YamlPath> {
        None
    }
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
}
