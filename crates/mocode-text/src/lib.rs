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

    pub fn line_end_position(&self, line: usize) -> Option<TextPosition> {
        if line >= self.rope.len_lines() {
            return None;
        }

        Some(TextPosition::new(
            u32::try_from(line).ok()?,
            u32::try_from(self.line_len_without_ending(line)).ok()?,
        ))
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

    pub fn insert_text_at(
        &mut self,
        position: TextPosition,
        text: &str,
    ) -> Result<TextPosition, TextEditError> {
        self.apply_edit(TextEdit::insert(position, text))?;
        Ok(position_after_insert(position, text))
    }

    pub fn backspace_at(&mut self, position: TextPosition) -> Result<TextPosition, TextEditError> {
        self.char_index(position)?;
        if position.line == 0 && position.character == 0 {
            return Ok(position);
        }

        let previous = self.move_left(position)?;
        self.apply_edit(TextEdit::delete(TextRange::new(previous, position)))?;
        Ok(previous)
    }

    pub fn delete_at(&mut self, position: TextPosition) -> Result<TextPosition, TextEditError> {
        self.char_index(position)?;
        let next = self.move_right(position)?;
        if next == position {
            return Ok(position);
        }

        self.apply_edit(TextEdit::delete(TextRange::new(position, next)))?;
        Ok(position)
    }

    pub fn move_left(&self, position: TextPosition) -> Result<TextPosition, TextEditError> {
        self.char_index(position)?;
        if position.character > 0 {
            return Ok(TextPosition::new(position.line, position.character - 1));
        }

        if position.line == 0 {
            return Ok(position);
        }

        let previous_line = position.line - 1;
        self.line_end_position(previous_line as usize)
            .ok_or(TextEditError::PositionOutOfBounds(position))
    }

    pub fn move_right(&self, position: TextPosition) -> Result<TextPosition, TextEditError> {
        self.char_index(position)?;
        let line_end = self
            .line_end_position(position.line as usize)
            .ok_or(TextEditError::PositionOutOfBounds(position))?;
        if position.character < line_end.character {
            return Ok(TextPosition::new(position.line, position.character + 1));
        }

        let next_line = position.line + 1;
        if next_line as usize >= self.rope.len_lines() {
            return Ok(position);
        }

        Ok(TextPosition::new(next_line, 0))
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

fn position_after_insert(start: TextPosition, text: &str) -> TextPosition {
    let mut line = start.line;
    let mut character = start.character;
    let mut saw_carriage_return = false;

    for ch in text.chars() {
        match ch {
            '\r' => {
                line += 1;
                character = 0;
                saw_carriage_return = true;
            }
            '\n' if saw_carriage_return => {
                saw_carriage_return = false;
            }
            '\n' => {
                line += 1;
                character = 0;
                saw_carriage_return = false;
            }
            _ => {
                character += 1;
                saw_carriage_return = false;
            }
        }
    }

    TextPosition::new(line, character)
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

    #[test]
    fn returns_line_end_positions() {
        let buffer = TextBuffer::open_text("mixed-port: 7890\ndns:\n");

        assert_eq!(buffer.line_end_position(0), Some(TextPosition::new(0, 16)));
        assert_eq!(buffer.line_end_position(1), Some(TextPosition::new(1, 4)));
        assert_eq!(buffer.line_end_position(2), Some(TextPosition::new(2, 0)));
        assert_eq!(buffer.line_end_position(3), None);
    }

    #[test]
    fn inserts_text_and_returns_cursor_after_insert() {
        let mut buffer = TextBuffer::open_text("dns:\n");
        let cursor = buffer
            .insert_text_at(TextPosition::new(0, 4), "\n  enable: true")
            .unwrap();

        assert_eq!(buffer.as_string(), "dns:\n  enable: true\n");
        assert_eq!(cursor, TextPosition::new(1, 14));
    }

    #[test]
    fn backspaces_inside_line_and_across_line_boundary() {
        let mut buffer = TextBuffer::open_text("dns:\n  enable: true\n");

        let cursor = buffer.backspace_at(TextPosition::new(1, 2)).unwrap();
        assert_eq!(buffer.as_string(), "dns:\n enable: true\n");
        assert_eq!(cursor, TextPosition::new(1, 1));

        let cursor = buffer.backspace_at(TextPosition::new(1, 0)).unwrap();
        assert_eq!(buffer.as_string(), "dns: enable: true\n");
        assert_eq!(cursor, TextPosition::new(0, 4));
    }

    #[test]
    fn deletes_inside_line_and_across_line_boundary() {
        let mut buffer = TextBuffer::open_text("dns:\n  enable: true\n");

        let cursor = buffer.delete_at(TextPosition::new(1, 1)).unwrap();
        assert_eq!(buffer.as_string(), "dns:\n enable: true\n");
        assert_eq!(cursor, TextPosition::new(1, 1));

        let cursor = buffer.delete_at(TextPosition::new(0, 4)).unwrap();
        assert_eq!(buffer.as_string(), "dns: enable: true\n");
        assert_eq!(cursor, TextPosition::new(0, 4));
    }

    #[test]
    fn moves_cursor_left_and_right_across_lines() {
        let buffer = TextBuffer::open_text("dns:\n  enable: true\n");

        assert_eq!(
            buffer.move_left(TextPosition::new(1, 0)).unwrap(),
            TextPosition::new(0, 4)
        );
        assert_eq!(
            buffer.move_left(TextPosition::new(0, 0)).unwrap(),
            TextPosition::new(0, 0)
        );
        assert_eq!(
            buffer.move_right(TextPosition::new(0, 4)).unwrap(),
            TextPosition::new(1, 0)
        );
        assert_eq!(
            buffer.move_right(TextPosition::new(1, 0)).unwrap(),
            TextPosition::new(1, 1)
        );
    }
}
