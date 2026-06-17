use std::ops::Range;

use ropey::Rope;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TextPosition {
    pub line: u32,
    pub character: u32,
}

impl TextPosition {
    pub const fn new(line: u32, character: u32) -> Self {
        Self { line, character }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextRange {
    pub start: TextPosition,
    pub end: TextPosition,
}

impl TextRange {
    pub const fn new(start: TextPosition, end: TextPosition) -> Self {
        Self { start, end }
    }

    pub const fn empty(position: TextPosition) -> Self {
        Self {
            start: position,
            end: position,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextEdit {
    pub range: TextRange,
    pub replacement: String,
}

impl TextEdit {
    pub fn replace(range: TextRange, replacement: impl Into<String>) -> Self {
        Self {
            range,
            replacement: replacement.into(),
        }
    }

    pub fn insert(position: TextPosition, text: impl Into<String>) -> Self {
        Self::replace(TextRange::empty(position), text)
    }

    pub fn delete(range: TextRange) -> Self {
        Self::replace(range, "")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Cursor {
    pub position: TextPosition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Selection {
    pub anchor: TextPosition,
    pub active: TextPosition,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextEditError {
    PositionOutOfBounds(TextPosition),
    ReversedRange(TextRange),
}

#[derive(Debug, Clone)]
pub struct TextBuffer {
    rope: Rope,
}

impl TextBuffer {
    pub fn open_text(text: impl AsRef<str>) -> Self {
        Self {
            rope: Rope::from_str(text.as_ref()),
        }
    }

    pub fn as_string(&self) -> String {
        self.rope.to_string()
    }

    pub fn line_count(&self) -> usize {
        self.rope.len_lines()
    }

    pub fn line_text(&self, line: usize) -> Option<String> {
        if line >= self.rope.len_lines() {
            return None;
        }

        Some(strip_line_ending(self.rope.line(line).to_string()))
    }

    pub fn len_chars(&self) -> usize {
        self.rope.len_chars()
    }

    pub fn apply_edit(&mut self, edit: TextEdit) -> Result<(), TextEditError> {
        let range = self.char_range(edit.range)?;
        self.rope.remove(range.clone());
        self.rope.insert(range.start, &edit.replacement);
        Ok(())
    }

    pub fn char_index(&self, position: TextPosition) -> Result<usize, TextEditError> {
        let line = position.line as usize;
        if line >= self.rope.len_lines() {
            return Err(TextEditError::PositionOutOfBounds(position));
        }

        let line_start = self.rope.line_to_char(line);
        let character = position.character as usize;
        let line_len = self.line_len_without_ending(line);

        if character > line_len {
            return Err(TextEditError::PositionOutOfBounds(position));
        }

        Ok(line_start + character)
    }

    pub fn char_range(&self, range: TextRange) -> Result<Range<usize>, TextEditError> {
        let start = self.char_index(range.start)?;
        let end = self.char_index(range.end)?;
        if start > end {
            return Err(TextEditError::ReversedRange(range));
        }
        Ok(start..end)
    }

    fn line_len_without_ending(&self, line: usize) -> usize {
        strip_line_ending(self.rope.line(line).to_string())
            .chars()
            .count()
    }
}

fn strip_line_ending(mut line: String) -> String {
    if line.ends_with('\n') {
        line.pop();
    }
    if line.ends_with('\r') {
        line.pop();
    }
    line
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn applies_insert_with_line_character_position() {
        let mut buffer = TextBuffer::open_text("mixed-port: 7890\n");
        buffer
            .apply_edit(TextEdit::insert(TextPosition::new(0, 11), " "))
            .unwrap();

        assert_eq!(buffer.as_string(), "mixed-port:  7890\n");
    }

    #[test]
    fn returns_line_text_without_line_endings() {
        let buffer = TextBuffer::open_text("mixed-port: 7890\r\ndns:\n");

        assert_eq!(buffer.line_text(0), Some("mixed-port: 7890".to_string()));
        assert_eq!(buffer.line_text(1), Some("dns:".to_string()));
        assert_eq!(buffer.line_text(2), Some(String::new()));
        assert_eq!(buffer.line_text(3), None);
    }
}
