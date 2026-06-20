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

    pub fn text_in_range(&self, range: TextRange) -> Result<String, TextEditError> {
        let range = self.ordered_range(range);
        let char_range = self.char_range(range)?;
        Ok(self.rope.slice(char_range).to_string())
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

    pub fn move_up(&self, position: TextPosition) -> Result<TextPosition, TextEditError> {
        self.char_index(position)?;
        if position.line == 0 {
            return Ok(position);
        }
        let target_line = position.line - 1;
        let target_len = self.line_len_without_ending(target_line as usize);
        let character = position
            .character
            .min(u32::try_from(target_len).unwrap_or(u32::MAX));
        Ok(TextPosition::new(target_line, character))
    }

    pub fn move_down(&self, position: TextPosition) -> Result<TextPosition, TextEditError> {
        self.char_index(position)?;
        let last_line = (self.rope.len_lines().saturating_sub(1)) as u32;
        if position.line >= last_line {
            return Ok(position);
        }
        let target_line = position.line + 1;
        let target_len = self.line_len_without_ending(target_line as usize);
        let character = position
            .character
            .min(u32::try_from(target_len).unwrap_or(u32::MAX));
        Ok(TextPosition::new(target_line, character))
    }

    pub fn move_line_start(&self, position: TextPosition) -> Result<TextPosition, TextEditError> {
        self.char_index(position)?;
        Ok(TextPosition::new(position.line, 0))
    }

    pub fn move_line_end(&self, position: TextPosition) -> Result<TextPosition, TextEditError> {
        self.char_index(position)?;
        self.line_end_position(position.line as usize)
            .ok_or(TextEditError::PositionOutOfBounds(position))
    }

    pub fn page_up(
        &self,
        position: TextPosition,
        visible_lines: u32,
    ) -> Result<TextPosition, TextEditError> {
        self.char_index(position)?;
        let target_line = position.line.saturating_sub(visible_lines);
        let target_len = self.line_len_without_ending(target_line as usize);
        let character = position
            .character
            .min(u32::try_from(target_len).unwrap_or(u32::MAX));
        Ok(TextPosition::new(target_line, character))
    }

    pub fn page_down(
        &self,
        position: TextPosition,
        visible_lines: u32,
    ) -> Result<TextPosition, TextEditError> {
        self.char_index(position)?;
        let last_line = (self.rope.len_lines().saturating_sub(1)) as u32;
        let target_line = (position.line + visible_lines).min(last_line);
        let target_len = self.line_len_without_ending(target_line as usize);
        let character = position
            .character
            .min(u32::try_from(target_len).unwrap_or(u32::MAX));
        Ok(TextPosition::new(target_line, character))
    }

    fn line_len_without_ending(&self, line: usize) -> usize {
        strip_line_ending(self.rope.line(line).to_string())
            .chars()
            .count()
    }

    fn ordered_range(&self, range: TextRange) -> TextRange {
        if range.start <= range.end {
            range
        } else {
            TextRange::new(range.end, range.start)
        }
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

    #[test]
    fn text_in_range_extracts_single_line_multi_line_and_reversed_ranges() {
        let buffer = TextBuffer::open_text("alpha\nbeta\ngamma\n");

        assert_eq!(
            buffer
                .text_in_range(TextRange::new(
                    TextPosition::new(0, 1),
                    TextPosition::new(0, 4)
                ))
                .unwrap(),
            "lph"
        );
        assert_eq!(
            buffer
                .text_in_range(TextRange::new(
                    TextPosition::new(0, 2),
                    TextPosition::new(2, 2)
                ))
                .unwrap(),
            "pha\nbeta\nga"
        );
        assert_eq!(
            buffer
                .text_in_range(TextRange::new(
                    TextPosition::new(2, 2),
                    TextPosition::new(0, 2)
                ))
                .unwrap(),
            "pha\nbeta\nga"
        );
    }

    // ── vertical navigation ──

    #[test]
    fn move_up_basic() {
        let buffer = TextBuffer::open_text("line one\nline two\nline three\n");

        assert_eq!(
            buffer.move_up(TextPosition::new(1, 4)).unwrap(),
            TextPosition::new(0, 4)
        );
        // Already at top: stay in place
        assert_eq!(
            buffer.move_up(TextPosition::new(0, 3)).unwrap(),
            TextPosition::new(0, 3)
        );
    }

    #[test]
    fn move_down_basic() {
        // Trailing newline creates 4 lines (0-3), line 3 is empty.
        let buffer = TextBuffer::open_text("line one\nline two\nline three\n");

        assert_eq!(
            buffer.move_down(TextPosition::new(0, 4)).unwrap(),
            TextPosition::new(1, 4)
        );
        // Already at last (empty) line: stay in place
        assert_eq!(
            buffer.move_down(TextPosition::new(3, 0)).unwrap(),
            TextPosition::new(3, 0)
        );
    }

    #[test]
    fn move_up_clamps_column_to_shorter_line() {
        // line 0: "abc" (len 3), line 1: "longer line" (len 11)
        let buffer = TextBuffer::open_text("abc\nlonger line\n");

        assert_eq!(
            buffer.move_up(TextPosition::new(1, 10)).unwrap(),
            TextPosition::new(0, 3) // clamped to "abc".len()
        );
    }

    #[test]
    fn move_down_clamps_column_to_shorter_line() {
        let buffer = TextBuffer::open_text("longer line\nabc\n");

        assert_eq!(
            buffer.move_down(TextPosition::new(0, 10)).unwrap(),
            TextPosition::new(1, 3) // clamped to "abc".len()
        );
    }

    #[test]
    fn move_line_start_and_end() {
        let buffer = TextBuffer::open_text("alpha\nbeta\n");

        // Home: moves to column 0
        assert_eq!(
            buffer.move_line_start(TextPosition::new(0, 3)).unwrap(),
            TextPosition::new(0, 0)
        );
        assert_eq!(
            buffer.move_line_start(TextPosition::new(1, 2)).unwrap(),
            TextPosition::new(1, 0)
        );

        // End: moves to line end
        assert_eq!(
            buffer.move_line_end(TextPosition::new(0, 1)).unwrap(),
            TextPosition::new(0, 5)
        );
        assert_eq!(
            buffer.move_line_end(TextPosition::new(1, 0)).unwrap(),
            TextPosition::new(1, 4)
        );
    }

    #[test]
    fn move_line_end_on_empty_line_returns_start_of_line() {
        let buffer = TextBuffer::open_text("alpha\n\nbeta\n");

        // Trailing newline creates an empty line at index 1
        assert_eq!(
            buffer.move_line_end(TextPosition::new(1, 0)).unwrap(),
            TextPosition::new(1, 0)
        );
    }

    #[test]
    fn page_up_moves_by_visible_lines_or_saturates_at_top() {
        let text: String = (0..30).map(|i| format!("line {i}\n")).collect();
        let buffer = TextBuffer::open_text(text);

        // Normal page up
        assert_eq!(
            buffer.page_up(TextPosition::new(20, 5), 5).unwrap(),
            TextPosition::new(15, 5)
        );
        // Saturate at top
        assert_eq!(
            buffer.page_up(TextPosition::new(3, 2), 10).unwrap(),
            TextPosition::new(0, 2)
        );
    }

    #[test]
    fn page_down_moves_by_visible_lines_or_clamps_at_bottom() {
        let text: String = (0..30).map(|i| format!("line {i}\n")).collect();
        // 30 content lines each with \n → ropey sees 31 lines (0-30), last is empty
        let buffer = TextBuffer::open_text(text);

        // Normal page down
        assert_eq!(
            buffer.page_down(TextPosition::new(10, 5), 5).unwrap(),
            TextPosition::new(15, 5)
        );
        // Clamp at last line (30), character clamps to 0 (empty line)
        assert_eq!(
            buffer.page_down(TextPosition::new(27, 5), 10).unwrap(),
            TextPosition::new(30, 0) // clamped to last line (empty), col→0
        );
    }

    #[test]
    fn page_up_down_clamps_column_to_target_line() {
        // Line 0: "short" (len 5), Line 1: "a very long line here" (len 22)
        let buffer = TextBuffer::open_text("short\na very long line here\n");

        // page_up from (1, 18) to line 0 (len 5) → column clamped to 5
        assert_eq!(
            buffer.page_up(TextPosition::new(1, 18), 1).unwrap(),
            TextPosition::new(0, 5)
        );

        // page_down from (0, 0) to line 1 → column 0 is fine
        assert_eq!(
            buffer.page_down(TextPosition::new(0, 0), 1).unwrap(),
            TextPosition::new(1, 0)
        );
    }

    #[test]
    fn vertical_moves_error_on_out_of_bounds_position() {
        let buffer = TextBuffer::open_text("abc\n");

        assert!(buffer.move_up(TextPosition::new(5, 0)).is_err());
        assert!(buffer.move_down(TextPosition::new(5, 0)).is_err());
        assert!(buffer.move_line_start(TextPosition::new(5, 0)).is_err());
        assert!(buffer.move_line_end(TextPosition::new(5, 0)).is_err());
        assert!(buffer.page_up(TextPosition::new(5, 0), 3).is_err());
        assert!(buffer.page_down(TextPosition::new(5, 0), 3).is_err());
    }
}
