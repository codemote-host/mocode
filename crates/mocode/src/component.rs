use std::{
    cell::RefCell,
    collections::BTreeMap,
    fs, io,
    ops::Range,
    path::{Path, PathBuf},
    rc::Rc,
};

use mocode_api::{
    CompletionKind, DiagnosticSeverity, EditorError, MocodeEditor, ProxyChainPreview,
    ProxyChainStatus, SyntaxHighlightKind, TextEdit, TextPosition, TextRange,
};

use gpui::{
    AnyElement, App, Bounds, ClipboardItem, Context, ElementInputHandler, EntityInputHandler,
    FocusHandle, IntoElement, KeyBinding, ListHorizontalSizingBehavior, MouseButton,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels, Point, ScrollStrategy,
    UniformListScrollHandle, Window, actions, canvas, div, point, prelude::*, px, rgb,
    uniform_list,
};

const LINE_HEIGHT_PX: f32 = 22.0;
const GUTTER_WIDTH_PX: f32 = 64.0;
const CHAR_WIDTH_PX: f32 = 7.5;
const TAB_WIDTH: &str = "  ";

fn reveal_line(scroll_handle: &UniformListScrollHandle, line_count: usize, line: u32) {
    if line_count == 0 {
        scroll_handle.0.borrow_mut().deferred_scroll_to_item = None;
        return;
    }

    let item_index = (line as usize).min(line_count - 1);
    scroll_handle.scroll_to_item(item_index, ScrollStrategy::Center);
}

actions!(
    mocode_editor,
    [
        Backspace,
        Delete,
        Tab,
        ShiftTab,
        ToggleComment,
        Left,
        Right,
        SelectLeft,
        SelectRight,
        SelectAll,
        SelectNextMatch,
        SelectLineStart,
        SelectLineEnd,
        Up,
        Down,
        SelectUp,
        SelectDown,
        Home,
        End,
        PageUp,
        PageDown,
        Paste,
        Copy,
        Open,
        Save,
        SaveAs,
        Undo,
        Redo,
        Find,
        GoToLine,
        FindNext,
        FindPrevious,
        EscapeSearch,
        Enter
    ]
);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GpuiEditorLine {
    pub(crate) number: u32,
    pub(crate) text: String,
    pub(crate) diagnostic_count: usize,
    pub(crate) diagnostic_severity: Option<String>,
    pub(crate) diagnostic_message: Option<String>,
    pub(crate) syntax_highlights: Vec<GpuiSyntaxHighlight>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GpuiSyntaxHighlight {
    pub(crate) start: u32,
    pub(crate) end: u32,
    pub(crate) kind: SyntaxHighlightKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GpuiEditorDiagnostic {
    pub(crate) severity: String,
    pub(crate) code: String,
    pub(crate) message: String,
    pub(crate) line: Option<u32>,
    pub(crate) column: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GpuiEditorCompletion {
    pub(crate) label: String,
    pub(crate) insert_text: String,
    pub(crate) kind: String,
    pub(crate) documentation: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct GpuiEditorCompletionPopup {
    pub(crate) anchor_line: u32,
    pub(crate) anchor_column: u32,
    pub(crate) left_px: f32,
    pub(crate) top_px: f32,
    pub(crate) selected_index: usize,
    pub(crate) items: Vec<GpuiEditorCompletion>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct GpuiSearchHighlight {
    pub(crate) start: u32,
    pub(crate) end: u32,
    pub(crate) active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum GpuiEditorSaveError {
    MissingPath,
    Io { path: PathBuf, message: String },
}

#[derive(Debug, Clone)]
pub(crate) struct GpuiEditorDocument {
    pub(crate) title: String,
    pub(crate) path: Option<PathBuf>,
    pub(crate) path_display: String,
    pub(crate) dirty: bool,
    pub(crate) save_status: String,
    saved_text: String,
    editor: MocodeEditor,
    pub(crate) cursor: TextPosition,
    pub(crate) line_count: usize,
    pub(crate) current_yaml_path: String,
    pub(crate) diagnostics: Vec<GpuiEditorDiagnostic>,
    pub(crate) completion_labels: Vec<String>,
    pub(crate) completion_items: Vec<GpuiEditorCompletion>,
    pub(crate) completion_popup: Option<GpuiEditorCompletionPopup>,
    completion_popup_suppressed_at: Option<TextPosition>,
    pub(crate) hover_title: String,
    pub(crate) hover_body: String,
    pub(crate) chain_preview: Option<ProxyChainPreview>,
    selection_anchor: Option<TextPosition>,
    pub(crate) selection_summary: String,
    pub(crate) search_active: bool,
    pub(crate) search_query: String,
    pub(crate) search_summary: String,
    pub(crate) go_to_line_active: bool,
    pub(crate) go_to_line_query: String,
    pub(crate) go_to_line_summary: String,
    ime_marked_range: Option<TextRange>,
}

impl GpuiEditorDocument {
    pub(crate) fn from_text(
        title: impl Into<String>,
        text: &str,
        inspect_position: TextPosition,
    ) -> Self {
        Self::from_text_with_path(title, text, inspect_position, None)
    }

    pub(crate) fn from_path(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref();
        let text = fs::read_to_string(path)?;
        let title = path
            .file_name()
            .and_then(|name| name.to_str())
            .map(str::to_string)
            .unwrap_or_else(|| path.display().to_string());
        let mut document = Self::from_text_with_path(
            title,
            &text,
            TextPosition::new(0, 0),
            Some(path.to_path_buf()),
        );
        document.save_status = format!("Opened {}", document.path_display);
        Ok(document)
    }

    pub(crate) fn open_path_if_clean(&mut self, path: impl AsRef<Path>) -> io::Result<bool> {
        let path = path.as_ref();
        if self.dirty {
            self.save_status = format!("Unsaved changes; save before opening {}", path.display());
            return Ok(false);
        }

        *self = Self::from_path(path)?;
        Ok(true)
    }

    pub(crate) fn from_text_with_path(
        title: impl Into<String>,
        text: &str,
        inspect_position: TextPosition,
        path: Option<PathBuf>,
    ) -> Self {
        let editor = MocodeEditor::open_text(text);
        let path_display = path
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "<built-in fixture>".to_string());
        let save_status = if path.is_some() {
            format!("Opened {path_display}")
        } else {
            "Built-in fixture is not saveable".to_string()
        };
        let mut document = Self {
            title: title.into(),
            path,
            path_display,
            dirty: false,
            save_status,
            saved_text: text.to_string(),
            editor,
            cursor: inspect_position,
            line_count: 0,
            current_yaml_path: String::new(),
            diagnostics: Vec::new(),
            completion_labels: Vec::new(),
            completion_items: Vec::new(),
            completion_popup: None,
            completion_popup_suppressed_at: None,
            hover_title: String::new(),
            hover_body: String::new(),
            chain_preview: None,
            selection_anchor: None,
            selection_summary: String::new(),
            search_active: false,
            search_query: String::new(),
            search_summary: "<none>".to_string(),
            go_to_line_active: false,
            go_to_line_query: String::new(),
            go_to_line_summary: "<none>".to_string(),
            ime_marked_range: None,
        };
        document.refresh_derived();
        document
    }

    #[cfg(test)]
    pub(crate) fn insert_text(&mut self, text: &str) -> Result<(), EditorError> {
        self.replace_utf16_range(None, text)
    }

    pub(crate) fn insert_pasted_text(&mut self, text: &str) -> Result<(), EditorError> {
        let normalized = normalize_pasted_yaml_indentation(text, &self.current_line_prefix());
        self.replace_utf16_range(None, &normalized)
    }

    pub(crate) fn accept_completion(&mut self) -> Result<bool, EditorError> {
        if self.completion_popup_suppressed_at == Some(self.cursor) {
            return Ok(false);
        }

        let Some((prefix, replace_range)) = self.completion_prefix_range() else {
            return Ok(false);
        };
        if prefix.is_empty() && !can_accept_empty_completion(&self.current_line_prefix()) {
            return Ok(false);
        }

        let Some(insert_text) = self
            .selected_completion_insert_text(&prefix)
            .or_else(|| completion_insert_text(&self.completion_items, &prefix))
        else {
            return Ok(false);
        };

        self.editor
            .apply_edit(TextEdit::replace(replace_range, &insert_text))?;
        self.cursor = position_after_insert(replace_range.start, &insert_text);
        self.ime_marked_range = None;
        self.clear_selection();
        self.mark_dirty();
        self.refresh_derived();
        self.completion_popup = None;
        Ok(true)
    }

    pub(crate) fn accept_completion_at(&mut self, index: usize) -> Result<bool, EditorError> {
        if !self.select_completion_index(index) {
            return Ok(false);
        }

        self.accept_completion()
    }

    pub(crate) fn jump_to_diagnostic(&mut self, index: usize) -> bool {
        let Some(diagnostic) = self.diagnostics.get(index).cloned() else {
            return false;
        };
        let (Some(line), Some(column)) = (diagnostic.line, diagnostic.column) else {
            return false;
        };
        if self.line_count == 0 {
            return false;
        }

        let line = line.saturating_sub(1).min(self.line_count as u32 - 1);
        let column = column.saturating_sub(1);
        let line_end = self
            .editor
            .line_end_position(line as usize)
            .unwrap_or_else(|| TextPosition::new(line, 0));
        self.cursor = TextPosition::new(line, column.min(line_end.character));
        self.ime_marked_range = None;
        self.clear_selection();
        self.completion_popup = None;
        self.refresh_derived();
        true
    }

    pub(crate) fn select_next_completion(&mut self) -> bool {
        self.select_completion(CompletionSelection::Next)
    }

    pub(crate) fn select_previous_completion(&mut self) -> bool {
        self.select_completion(CompletionSelection::Previous)
    }

    pub(crate) fn close_completion_popup(&mut self) -> bool {
        if self.completion_popup.is_none() {
            return false;
        }

        self.completion_popup = None;
        self.completion_popup_suppressed_at = Some(self.cursor);
        true
    }

    pub(crate) fn commit_text(&mut self, text: &str) -> Result<(), EditorError> {
        self.replace_utf16_range(None, text)
    }

    pub(crate) fn insert_tab(&mut self) -> Result<(), EditorError> {
        self.commit_text(TAB_WIDTH)
    }

    pub(crate) fn insert_newline(&mut self) -> Result<(), EditorError> {
        let line_prefix = self.current_line_prefix();
        let indent = auto_indent_for_line_prefix(&line_prefix);
        self.commit_text(&format!("\n{indent}"))
    }

    pub(crate) fn outdent_current_line(&mut self) -> Result<(), EditorError> {
        let Some(line_text) = self.editor.line_text(self.cursor.line as usize) else {
            return Ok(());
        };
        let remove_count = line_text
            .chars()
            .take(TAB_WIDTH.chars().count())
            .take_while(|ch| *ch == ' ')
            .count();

        if remove_count == 0 {
            return Ok(());
        }

        let start = TextPosition::new(self.cursor.line, 0);
        let end = TextPosition::new(self.cursor.line, remove_count as u32);
        self.editor
            .apply_edit(TextEdit::delete(TextRange::new(start, end)))?;
        self.cursor = TextPosition::new(
            self.cursor.line,
            self.cursor.character.saturating_sub(remove_count as u32),
        );
        self.ime_marked_range = None;
        self.clear_selection();
        self.mark_dirty();
        self.refresh_derived();
        Ok(())
    }

    pub(crate) fn toggle_line_comment(&mut self) -> Result<(), EditorError> {
        let Some((start_line, end_line)) = self.comment_line_range() else {
            return Ok(());
        };

        let original_lines = (start_line..=end_line)
            .filter_map(|line| self.editor.line_text(line as usize))
            .collect::<Vec<_>>();
        if original_lines.iter().all(|line| line.trim().is_empty()) {
            return Ok(());
        }

        let uncomment = original_lines
            .iter()
            .filter(|line| !line.trim().is_empty())
            .all(|line| line_is_commented(line));
        let replacement_lines = original_lines
            .iter()
            .map(|line| toggle_yaml_line_comment(line, uncomment))
            .collect::<Vec<_>>();
        let replacement = replacement_lines.join("\n");
        let range = TextRange::new(
            TextPosition::new(start_line, 0),
            self.editor
                .line_end_position(end_line as usize)
                .unwrap_or_else(|| TextPosition::new(end_line, 0)),
        );
        let cursor = cursor_after_comment_toggle(
            self.cursor,
            start_line,
            end_line,
            &original_lines,
            uncomment,
        );

        self.editor
            .apply_edit(TextEdit::replace(range, &replacement))?;
        self.cursor = cursor;
        self.ime_marked_range = None;
        self.clear_selection();
        self.mark_dirty();
        self.refresh_derived();
        Ok(())
    }

    pub(crate) fn backspace(&mut self) -> Result<(), EditorError> {
        if self.delete_selected_text()? {
            return Ok(());
        }

        self.cursor = self.editor.backspace_at(self.cursor)?;
        self.clear_selection();
        self.mark_dirty();
        self.refresh_derived();
        Ok(())
    }

    pub(crate) fn delete(&mut self) -> Result<(), EditorError> {
        if self.delete_selected_text()? {
            return Ok(());
        }

        self.cursor = self.editor.delete_at(self.cursor)?;
        self.clear_selection();
        self.mark_dirty();
        self.refresh_derived();
        Ok(())
    }

    pub(crate) fn undo(&mut self) -> Result<(), EditorError> {
        if let Some(cursor) = self.editor.undo()? {
            self.cursor = cursor;
            self.clear_selection();
            self.mark_dirty();
            self.refresh_derived();
        }
        Ok(())
    }

    pub(crate) fn redo(&mut self) -> Result<(), EditorError> {
        if let Some(cursor) = self.editor.redo()? {
            self.cursor = cursor;
            self.clear_selection();
            self.mark_dirty();
            self.refresh_derived();
        }
        Ok(())
    }

    pub(crate) fn move_left(&mut self) -> Result<(), EditorError> {
        self.cursor = self.editor.move_left(self.cursor)?;
        self.clear_selection();
        self.refresh_derived();
        Ok(())
    }

    pub(crate) fn move_right(&mut self) -> Result<(), EditorError> {
        self.cursor = self.editor.move_right(self.cursor)?;
        self.clear_selection();
        self.refresh_derived();
        Ok(())
    }

    pub(crate) fn select_left(&mut self) -> Result<(), EditorError> {
        self.ensure_selection_anchor();
        self.cursor = self.editor.move_left(self.cursor)?;
        self.refresh_derived();
        Ok(())
    }

    pub(crate) fn select_right(&mut self) -> Result<(), EditorError> {
        self.ensure_selection_anchor();
        self.cursor = self.editor.move_right(self.cursor)?;
        self.refresh_derived();
        Ok(())
    }

    pub(crate) fn select_all(&mut self) -> Result<(), EditorError> {
        self.selection_anchor = Some(TextPosition::new(0, 0));
        self.cursor = self.document_end_position();
        self.ime_marked_range = None;
        self.refresh_derived();
        Ok(())
    }

    pub(crate) fn select_next_match(&mut self) -> bool {
        self.search_active = false;
        self.go_to_line_active = false;
        self.completion_popup = None;

        if let Some(range) = self.selected_range().map(ordered_text_range) {
            let Ok(query) = self.editor.text_in_range(range) else {
                return false;
            };
            if query.is_empty() {
                return false;
            }

            let Some(search_match) =
                find_next_match(&self.text(), &query, self.cursor, Some(range))
            else {
                return false;
            };
            if search_match.range == range {
                return false;
            }

            self.selection_anchor = Some(search_match.range.start);
            self.cursor = search_match.range.end;
            self.refresh_derived();
            return true;
        }

        let Some(range) = self.current_identifier_range() else {
            return false;
        };
        self.selection_anchor = Some(range.start);
        self.cursor = range.end;
        self.refresh_derived();
        true
    }

    pub(crate) const PAGE_LINES: u32 = 25;

    pub(crate) fn move_up(&mut self) -> Result<(), EditorError> {
        self.cursor = self.editor.move_up(self.cursor)?;
        self.clear_selection();
        self.refresh_derived();
        Ok(())
    }

    pub(crate) fn move_down(&mut self) -> Result<(), EditorError> {
        self.cursor = self.editor.move_down(self.cursor)?;
        self.clear_selection();
        self.refresh_derived();
        Ok(())
    }

    pub(crate) fn move_line_start(&mut self) -> Result<(), EditorError> {
        self.cursor = self.editor.move_line_start(self.cursor)?;
        self.clear_selection();
        self.refresh_derived();
        Ok(())
    }

    pub(crate) fn move_line_end(&mut self) -> Result<(), EditorError> {
        self.cursor = self.editor.move_line_end(self.cursor)?;
        self.clear_selection();
        self.refresh_derived();
        Ok(())
    }

    pub(crate) fn select_line_start(&mut self) -> Result<(), EditorError> {
        self.ensure_selection_anchor();
        self.cursor = self.editor.move_line_start(self.cursor)?;
        self.refresh_derived();
        Ok(())
    }

    pub(crate) fn select_line_end(&mut self) -> Result<(), EditorError> {
        self.ensure_selection_anchor();
        self.cursor = self.editor.move_line_end(self.cursor)?;
        self.refresh_derived();
        Ok(())
    }

    pub(crate) fn page_up(&mut self) -> Result<(), EditorError> {
        self.cursor = self.editor.page_up(self.cursor, Self::PAGE_LINES)?;
        self.clear_selection();
        self.refresh_derived();
        Ok(())
    }

    pub(crate) fn page_down(&mut self) -> Result<(), EditorError> {
        self.cursor = self.editor.page_down(self.cursor, Self::PAGE_LINES)?;
        self.clear_selection();
        self.refresh_derived();
        Ok(())
    }

    pub(crate) fn select_up(&mut self) -> Result<(), EditorError> {
        self.ensure_selection_anchor();
        self.cursor = self.editor.move_up(self.cursor)?;
        self.refresh_derived();
        Ok(())
    }

    pub(crate) fn select_down(&mut self) -> Result<(), EditorError> {
        self.ensure_selection_anchor();
        self.cursor = self.editor.move_down(self.cursor)?;
        self.refresh_derived();
        Ok(())
    }

    pub(crate) fn begin_selection_at(&mut self, position: TextPosition) {
        self.cursor = position;
        self.selection_anchor = Some(position);
        self.refresh_derived();
    }

    pub(crate) fn select_to(&mut self, position: TextPosition) {
        self.ensure_selection_anchor();
        self.cursor = position;
        self.refresh_derived();
    }

    pub(crate) fn finish_selection(&mut self) {
        if self.selected_range().is_none() {
            self.clear_selection();
            self.refresh_derived();
        }
    }

    pub(crate) fn selected_text(&self) -> Option<String> {
        let range = self.selected_range()?;
        self.editor.text_in_range(range).ok()
    }

    pub(crate) fn copy_selection_text(&self) -> Option<String> {
        self.selected_text()
    }

    pub(crate) fn text(&self) -> String {
        self.editor.text()
    }

    pub(crate) fn text_for_utf16_range(
        &self,
        range_utf16: Range<usize>,
        adjusted_range: &mut Option<Range<usize>>,
    ) -> Option<String> {
        let clamped = self.clamp_utf16_range(range_utf16);
        let range = self.text_range_from_utf16(clamped.clone());
        adjusted_range.replace(clamped);
        self.editor.text_in_range(range).ok()
    }

    pub(crate) fn selected_utf16_range(&self) -> (Range<usize>, bool) {
        let Some(range) = self.selected_range() else {
            let cursor = self.utf16_index_for_position(self.cursor);
            return (cursor..cursor, false);
        };
        let reversed = range.start > range.end;
        let ordered = ordered_text_range(range);
        (
            self.utf16_index_for_position(ordered.start)
                ..self.utf16_index_for_position(ordered.end),
            reversed,
        )
    }

    pub(crate) fn marked_utf16_range(&self) -> Option<Range<usize>> {
        let range = ordered_text_range(self.ime_marked_range?);
        Some(self.utf16_index_for_position(range.start)..self.utf16_index_for_position(range.end))
    }

    pub(crate) fn unmark_ime_text(&mut self) {
        self.ime_marked_range = None;
    }

    pub(crate) fn replace_utf16_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        text: &str,
    ) -> Result<(), EditorError> {
        let range = range_utf16
            .map(|range| self.text_range_from_utf16(self.clamp_utf16_range(range)))
            .or(self.ime_marked_range)
            .or_else(|| self.selected_range().map(ordered_text_range))
            .unwrap_or_else(|| TextRange::empty(self.cursor));
        let range = ordered_text_range(range);

        self.editor.apply_edit(TextEdit::replace(range, text))?;
        self.cursor = position_after_insert(range.start, text);
        self.ime_marked_range = None;
        self.clear_selection();
        self.mark_dirty();
        self.refresh_derived();
        Ok(())
    }

    pub(crate) fn replace_and_mark_utf16_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        text: &str,
        selected_range_utf16: Option<Range<usize>>,
    ) -> Result<(), EditorError> {
        let range = range_utf16
            .map(|range| self.text_range_from_utf16(self.clamp_utf16_range(range)))
            .or(self.ime_marked_range)
            .or_else(|| self.selected_range().map(ordered_text_range))
            .unwrap_or_else(|| TextRange::empty(self.cursor));
        let range = ordered_text_range(range);

        self.editor.apply_edit(TextEdit::replace(range, text))?;
        let inserted_start = range.start;
        let inserted_end = position_after_insert(inserted_start, text);
        self.ime_marked_range =
            (!text.is_empty()).then_some(TextRange::new(inserted_start, inserted_end));
        self.cursor = selected_range_utf16
            .map(|range| position_after_utf16_prefix(inserted_start, text, range.end))
            .unwrap_or(inserted_end);
        self.clear_selection();
        self.mark_dirty();
        self.refresh_derived();
        Ok(())
    }

    pub(crate) fn bounds_for_utf16_range(
        &self,
        range_utf16: Range<usize>,
        element_bounds: Bounds<Pixels>,
    ) -> Option<Bounds<Pixels>> {
        let position = self.position_for_utf16_index(range_utf16.start);
        if position.line as usize >= self.line_count {
            return None;
        }

        let left =
            element_bounds.left() + px(GUTTER_WIDTH_PX + position.character as f32 * CHAR_WIDTH_PX);
        let top = element_bounds.top() + px(position.line as f32 * LINE_HEIGHT_PX);
        Some(Bounds::from_corners(
            point(left, top),
            point(left + px(CHAR_WIDTH_PX), top + px(LINE_HEIGHT_PX)),
        ))
    }

    pub(crate) fn utf16_index_for_point(
        &self,
        point: Point<Pixels>,
        element_bounds: Bounds<Pixels>,
    ) -> Option<usize> {
        let y: f32 = point.y.into();
        let x: f32 = point.x.into();
        let origin_y: f32 = element_bounds.top().into();
        let origin_x: f32 = element_bounds.left().into();
        let position = mouse_to_text_position(
            y,
            x,
            origin_y,
            origin_x,
            GUTTER_WIDTH_PX,
            CHAR_WIDTH_PX,
            LINE_HEIGHT_PX,
            self.line_count,
            |line| {
                self.editor
                    .line_end_position(line as usize)
                    .map(|position| position.character)
                    .unwrap_or(0)
            },
        )?;
        Some(self.utf16_index_for_position(position))
    }

    fn utf16_index_for_position(&self, position: TextPosition) -> usize {
        self.editor
            .text_in_range(TextRange::new(TextPosition::new(0, 0), position))
            .map(|text| text.encode_utf16().count())
            .unwrap_or(0)
    }

    fn position_for_utf16_index(&self, target: usize) -> TextPosition {
        let mut utf16_index = 0usize;
        let mut line = 0u32;
        let mut character = 0u32;

        for ch in self.text().chars() {
            let width = ch.len_utf16();
            if utf16_index + width > target {
                break;
            }

            utf16_index += width;
            if ch == '\n' {
                line += 1;
                character = 0;
            } else {
                character += 1;
            }
        }

        TextPosition::new(line, character)
    }

    fn text_range_from_utf16(&self, range: Range<usize>) -> TextRange {
        TextRange::new(
            self.position_for_utf16_index(range.start),
            self.position_for_utf16_index(range.end),
        )
    }

    fn clamp_utf16_range(&self, range: Range<usize>) -> Range<usize> {
        let len = self.text().encode_utf16().count();
        let start = range.start.min(len);
        let end = range.end.min(len);
        if start <= end { start..end } else { end..start }
    }

    #[cfg(test)]
    pub(crate) fn line_at(&self, index: usize) -> Option<GpuiEditorLine> {
        let lines = self.editor.semantic_lines_in_range(index, index + 1);
        lines.into_iter().next().map(|line| GpuiEditorLine {
            number: line.number,
            text: line.text,
            diagnostic_count: line.diagnostics.len(),
            diagnostic_severity: line
                .diagnostics
                .first()
                .map(|diagnostic| severity_label(diagnostic.severity).to_string()),
            diagnostic_message: line
                .diagnostics
                .first()
                .map(|diagnostic| diagnostic.message.clone()),
            syntax_highlights: line
                .highlights
                .into_iter()
                .map(|highlight| GpuiSyntaxHighlight {
                    start: highlight.start,
                    end: highlight.end,
                    kind: highlight.kind,
                })
                .collect(),
        })
    }

    pub(crate) fn lines_in_range(&self, start: usize, end: usize) -> Vec<GpuiEditorLine> {
        self.editor
            .semantic_lines_in_range(start, end)
            .into_iter()
            .map(|line| GpuiEditorLine {
                number: line.number,
                text: line.text,
                diagnostic_count: line.diagnostics.len(),
                diagnostic_severity: line
                    .diagnostics
                    .first()
                    .map(|diagnostic| severity_label(diagnostic.severity).to_string()),
                diagnostic_message: line
                    .diagnostics
                    .first()
                    .map(|diagnostic| diagnostic.message.clone()),
                syntax_highlights: line
                    .highlights
                    .into_iter()
                    .map(|highlight| GpuiSyntaxHighlight {
                        start: highlight.start,
                        end: highlight.end,
                        kind: highlight.kind,
                    })
                    .collect(),
            })
            .collect()
    }

    pub(crate) fn current_line_diagnostic_summary(&self) -> Option<String> {
        let cursor_line = self.cursor.line + 1;
        self.diagnostics
            .iter()
            .find(|diagnostic| diagnostic.line == Some(cursor_line))
            .map(|diagnostic| format_diagnostic_summary(&diagnostic.severity, &diagnostic.message))
    }

    pub(crate) fn search_highlights_in_range(
        &self,
        start_line: usize,
        end_line: usize,
    ) -> BTreeMap<u32, Vec<GpuiSearchHighlight>> {
        if !self.search_active || self.search_query.is_empty() || start_line >= end_line {
            return BTreeMap::new();
        }

        let text = self.text();
        let current_range = self.selected_range().map(ordered_text_range);
        let mut highlights: BTreeMap<u32, Vec<GpuiSearchHighlight>> = BTreeMap::new();

        for start_byte in match_start_indices(&text, &self.search_query) {
            let Some(search_match) =
                build_search_match(&text, &self.search_query, start_byte, 0, 0)
            else {
                continue;
            };
            let active = current_range == Some(search_match.range);
            let first_line = search_match.range.start.line as usize;
            let last_line = search_match.range.end.line as usize;
            let visible_start = first_line.max(start_line);
            let visible_end = last_line.min(end_line.saturating_sub(1));

            for line in visible_start..=visible_end {
                let Some(line_end) = self.editor.line_end_position(line) else {
                    continue;
                };
                let Some((highlight_start, highlight_end)) =
                    text_range_on_line(line as u32, line_end.character, search_match.range)
                else {
                    continue;
                };
                highlights
                    .entry(line as u32)
                    .or_default()
                    .push(GpuiSearchHighlight {
                        start: highlight_start,
                        end: highlight_end,
                        active,
                    });
            }
        }

        highlights
    }

    pub(crate) fn save_to_original_path(&mut self) -> Result<(), GpuiEditorSaveError> {
        let Some(path) = self.path.clone() else {
            self.save_status = "Built-in fixture is not saveable".to_string();
            return Err(GpuiEditorSaveError::MissingPath);
        };

        self.save_to_path(&path, false)
    }

    pub(crate) fn save_as(&mut self, path: impl AsRef<Path>) -> Result<(), GpuiEditorSaveError> {
        self.save_to_path(path.as_ref(), true)
    }

    pub(crate) fn backup_path_for(path: impl AsRef<Path>) -> PathBuf {
        let path = path.as_ref();
        match path.file_name().and_then(|name| name.to_str()) {
            Some(file_name) => path.with_file_name(format!("{file_name}.bak")),
            None => path.with_extension("bak"),
        }
    }

    #[cfg(test)]
    pub(crate) fn set_search_query(&mut self, query: impl Into<String>) {
        self.search_active = true;
        self.search_query = query.into();
        self.update_search_summary_without_match();
    }

    pub(crate) fn start_search_from_selection(&mut self) {
        self.search_active = true;
        self.go_to_line_active = false;
        if let Some(selected) = self.selected_text()
            && !selected.is_empty()
        {
            self.search_query = selected;
        }

        if self.search_query.is_empty() {
            self.search_summary = "Type a search query".to_string();
        } else {
            self.find_next();
        }
    }

    pub(crate) fn append_search_input(&mut self, text: &str) {
        self.search_active = true;
        self.search_query.push_str(text);
        self.find_next();
    }

    pub(crate) fn search_backspace(&mut self) {
        self.search_active = true;
        self.search_query.pop();
        if self.search_query.is_empty() {
            self.clear_selection();
            self.search_summary = "Type a search query".to_string();
            self.refresh_derived();
        } else {
            self.find_next();
        }
    }

    pub(crate) fn close_search(&mut self) {
        self.search_active = false;
        if self.search_query.is_empty() {
            self.search_summary = "<none>".to_string();
        }
    }

    pub(crate) fn start_go_to_line(&mut self) {
        self.search_active = false;
        self.go_to_line_active = true;
        self.go_to_line_query.clear();
        self.go_to_line_summary = self.go_to_line_prompt();
        self.completion_popup = None;
    }

    pub(crate) fn append_go_to_line_input(&mut self, text: &str) {
        self.go_to_line_active = true;
        self.go_to_line_query
            .extend(text.chars().filter(char::is_ascii_digit));
        self.update_go_to_line_summary();
    }

    pub(crate) fn go_to_line_backspace(&mut self) {
        self.go_to_line_active = true;
        self.go_to_line_query.pop();
        self.update_go_to_line_summary();
    }

    pub(crate) fn close_go_to_line(&mut self) {
        self.go_to_line_active = false;
        if self.go_to_line_query.is_empty() {
            self.go_to_line_summary = "<none>".to_string();
        }
    }

    pub(crate) fn submit_go_to_line(&mut self) -> bool {
        let Some(line_number) = parse_go_to_line_query(&self.go_to_line_query) else {
            self.update_go_to_line_summary();
            return false;
        };
        if self.line_count == 0 {
            self.go_to_line_summary = "No lines".to_string();
            return false;
        }

        let last_line = self.line_count.saturating_sub(1).min(u32::MAX as usize);
        let target_line = line_number.saturating_sub(1).min(last_line);
        self.cursor = TextPosition::new(target_line as u32, 0);
        self.clear_selection();
        self.go_to_line_active = false;
        self.go_to_line_summary = format!("Line {}", target_line + 1);
        self.refresh_derived();
        true
    }

    pub(crate) fn find_next(&mut self) -> bool {
        let Some(search_match) = find_next_match(
            &self.text(),
            &self.search_query,
            self.cursor,
            self.selected_range(),
        ) else {
            self.clear_selection();
            self.update_search_summary_without_match();
            self.refresh_derived();
            return false;
        };

        self.apply_search_match(search_match);
        true
    }

    pub(crate) fn find_previous(&mut self) -> bool {
        let Some(search_match) = find_previous_match(
            &self.text(),
            &self.search_query,
            self.cursor,
            self.selected_range(),
        ) else {
            self.clear_selection();
            self.update_search_summary_without_match();
            self.refresh_derived();
            return false;
        };

        self.apply_search_match(search_match);
        true
    }

    fn save_to_path(&mut self, path: &Path, save_as: bool) -> Result<(), GpuiEditorSaveError> {
        let backup_path = path.exists().then(|| Self::backup_path_for(path));
        if let Some(backup_path) = &backup_path
            && let Err(error) = fs::copy(path, backup_path)
        {
            let message = error.to_string();
            self.save_status = format!("Failed to backup {}: {message}", path.display());
            return Err(GpuiEditorSaveError::Io {
                path: backup_path.clone(),
                message,
            });
        }

        let current_text = self.text();
        if let Err(error) = fs::write(path, &current_text) {
            let message = error.to_string();
            self.save_status = format!("Failed to save {}: {message}", path.display());
            return Err(GpuiEditorSaveError::Io {
                path: path.to_path_buf(),
                message,
            });
        }

        self.path = Some(path.to_path_buf());
        self.path_display = path.display().to_string();
        self.title = path
            .file_name()
            .and_then(|name| name.to_str())
            .map(str::to_string)
            .unwrap_or_else(|| path.display().to_string());
        self.saved_text = current_text;
        self.dirty = false;
        let verb = if save_as { "Saved as" } else { "Saved" };
        self.save_status = if let Some(backup_path) = backup_path {
            format!(
                "{verb} {}; Backup {}",
                path.display(),
                backup_path.display()
            )
        } else {
            format!("{verb} {}", path.display())
        };
        Ok(())
    }

    fn apply_search_match(&mut self, search_match: SearchMatch) {
        self.selection_anchor = Some(search_match.range.start);
        self.cursor = search_match.range.end;
        self.search_summary = format!(
            "{} - {}/{} at {}:{}",
            self.search_query,
            search_match.ordinal,
            search_match.total,
            search_match.range.start.line + 1,
            search_match.range.start.character + 1
        );
        self.refresh_derived();
    }

    fn update_search_summary_without_match(&mut self) {
        self.search_summary = if self.search_query.is_empty() {
            "Type a search query".to_string()
        } else {
            format!("{} - 0/0", self.search_query)
        };
    }

    fn update_go_to_line_summary(&mut self) {
        self.go_to_line_summary = if self.go_to_line_query.is_empty() {
            self.go_to_line_prompt()
        } else {
            format!("Go to line {}", self.go_to_line_query)
        };
    }

    fn go_to_line_prompt(&self) -> String {
        format!("Go to line 1-{}", self.line_count.max(1))
    }

    fn mark_dirty(&mut self) {
        self.refresh_dirty_state();
        if self.dirty {
            self.save_status = "Modified".to_string();
        } else {
            self.save_status = "No unsaved changes".to_string();
        }
    }

    fn refresh_dirty_state(&mut self) {
        self.dirty = self.text() != self.saved_text;
    }

    fn refresh_derived(&mut self) {
        self.line_count = self.editor.line_count();
        self.current_yaml_path = self
            .editor
            .current_yaml_path(self.cursor)
            .map(|path| path.to_string())
            .unwrap_or_else(|| "<none>".to_string());
        self.completion_items = self
            .editor
            .completions_at(self.cursor)
            .into_iter()
            .map(|completion| GpuiEditorCompletion {
                label: completion.label,
                insert_text: completion.insert_text,
                kind: completion_kind_label(completion.kind).to_string(),
                documentation: completion.documentation,
            })
            .collect();
        self.completion_labels = self
            .completion_items
            .iter()
            .map(|completion| completion.label.clone())
            .collect();
        if self
            .completion_popup_suppressed_at
            .is_some_and(|position| position != self.cursor)
        {
            self.completion_popup_suppressed_at = None;
        }
        let selected_index = self
            .completion_popup
            .as_ref()
            .filter(|popup| {
                popup.anchor_line == self.cursor.line + 1
                    && popup.anchor_column == self.cursor.character + 1
            })
            .map(|popup| popup.selected_index)
            .unwrap_or(0);
        let completion_prefix = self
            .completion_prefix_range()
            .map(|(prefix, _)| prefix)
            .unwrap_or_default();
        self.completion_popup = if self.completion_popup_suppressed_at == Some(self.cursor) {
            None
        } else {
            build_completion_popup(
                self.cursor,
                &self.completion_items,
                selected_index,
                &completion_prefix,
                can_accept_empty_completion(&self.current_line_prefix()),
            )
        };
        if let Some(hover) = self.editor.hover_summary_at(self.cursor) {
            self.hover_title = hover.title;
            self.hover_body = hover.body;
        } else {
            self.hover_title = "<none>".to_string();
            self.hover_body.clear();
        }
        self.diagnostics = self
            .editor
            .diagnostics()
            .into_iter()
            .map(|diagnostic| GpuiEditorDiagnostic {
                severity: severity_label(diagnostic.severity).to_string(),
                code: diagnostic.code,
                message: diagnostic.message,
                line: diagnostic.range.map(|range| range.start.line + 1),
                column: diagnostic.range.map(|range| range.start.character + 1),
            })
            .collect();
        self.selection_summary = self
            .selected_range()
            .map(format_selection_range)
            .unwrap_or_else(|| "<none>".to_string());
        self.chain_preview = self.editor.proxy_chain_preview_at(self.cursor);
    }

    fn ensure_selection_anchor(&mut self) {
        if self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.cursor);
        }
    }

    fn clear_selection(&mut self) {
        self.selection_anchor = None;
    }

    fn current_line_prefix(&self) -> String {
        let Some(line_text) = self.editor.line_text(self.cursor.line as usize) else {
            return String::new();
        };
        line_text
            .chars()
            .take(self.cursor.character as usize)
            .collect()
    }

    fn current_identifier_range(&self) -> Option<TextRange> {
        let line_text = self.editor.line_text(self.cursor.line as usize)?;
        identifier_range_at_position(self.cursor.line, &line_text, self.cursor.character)
    }

    fn completion_prefix_range(&self) -> Option<(String, TextRange)> {
        let line_text = self.editor.line_text(self.cursor.line as usize)?;
        let line_len = line_text.chars().count() as u32;
        let cursor_character = self.cursor.character.min(line_len);
        let prefix_start = completion_prefix_start(&line_text, cursor_character);
        let prefix = line_text
            .chars()
            .skip(prefix_start as usize)
            .take((cursor_character - prefix_start) as usize)
            .collect();

        Some((
            prefix,
            TextRange::new(
                TextPosition::new(self.cursor.line, prefix_start),
                TextPosition::new(self.cursor.line, cursor_character),
            ),
        ))
    }

    fn selected_completion_insert_text(&self, prefix: &str) -> Option<String> {
        let popup = self.completion_popup.as_ref()?;
        let selected = popup.items.get(popup.selected_index)?;

        if prefix.is_empty()
            || starts_with_ignore_ascii_case(&selected.insert_text, prefix)
            || starts_with_ignore_ascii_case(&selected.label, prefix)
        {
            Some(selected.insert_text.clone())
        } else {
            None
        }
    }

    fn select_completion(&mut self, selection: CompletionSelection) -> bool {
        let Some(popup) = self.completion_popup.as_mut() else {
            return false;
        };
        if popup.items.is_empty() {
            return false;
        }

        let len = popup.items.len();
        popup.selected_index = match selection {
            CompletionSelection::Next => (popup.selected_index + 1) % len,
            CompletionSelection::Previous => {
                if popup.selected_index == 0 {
                    len - 1
                } else {
                    popup.selected_index - 1
                }
            }
        };
        true
    }

    fn select_completion_index(&mut self, index: usize) -> bool {
        let Some(popup) = self.completion_popup.as_mut() else {
            return false;
        };
        if index >= popup.items.len() {
            return false;
        }

        popup.selected_index = index;
        true
    }

    fn comment_line_range(&self) -> Option<(u32, u32)> {
        let line_count = self.editor.line_count();
        if line_count == 0 {
            return None;
        }

        let range = self.selected_range().map(ordered_text_range);
        let start_line = range
            .as_ref()
            .map(|range| range.start.line)
            .unwrap_or(self.cursor.line);
        let end_line = range
            .as_ref()
            .map(|range| {
                if range.end.character == 0 && range.end.line > range.start.line {
                    range.end.line - 1
                } else {
                    range.end.line
                }
            })
            .unwrap_or(self.cursor.line);
        let last_line = line_count.saturating_sub(1) as u32;

        Some((start_line.min(last_line), end_line.min(last_line)))
    }

    fn document_end_position(&self) -> TextPosition {
        let last_line = self.editor.line_count().saturating_sub(1);
        self.editor
            .line_end_position(last_line)
            .unwrap_or_else(|| TextPosition::new(0, 0))
    }

    fn delete_selected_text(&mut self) -> Result<bool, EditorError> {
        let Some(range) = self.selected_range().map(ordered_text_range) else {
            return Ok(false);
        };

        self.editor.apply_edit(TextEdit::delete(range))?;
        self.cursor = range.start;
        self.ime_marked_range = None;
        self.clear_selection();
        self.mark_dirty();
        self.refresh_derived();
        Ok(true)
    }

    pub(crate) fn selected_range(&self) -> Option<TextRange> {
        let anchor = self.selection_anchor?;
        (anchor != self.cursor).then(|| TextRange::new(anchor, self.cursor))
    }
}

pub(crate) struct GpuiEditorComponent {
    document: GpuiEditorDocument,
    focus_handle: FocusHandle,
    scroll_handle: UniformListScrollHandle,
    line_list_bounds: Rc<RefCell<Option<Bounds<Pixels>>>>,
    mouse_selecting: bool,
}

impl GpuiEditorComponent {
    pub(crate) fn new(document: GpuiEditorDocument, focus_handle: FocusHandle) -> Self {
        Self {
            document,
            focus_handle,
            scroll_handle: UniformListScrollHandle::new(),
            line_list_bounds: Rc::new(RefCell::new(None)),
            mouse_selecting: false,
        }
    }

    pub(crate) fn document(&self) -> &GpuiEditorDocument {
        &self.document
    }

    pub(crate) fn document_mut(&mut self) -> &mut GpuiEditorDocument {
        &mut self.document
    }

    pub(crate) fn focus_handle(&self) -> &FocusHandle {
        &self.focus_handle
    }

    fn line_list_bounds_handle(&self) -> Rc<RefCell<Option<Bounds<Pixels>>>> {
        Rc::clone(&self.line_list_bounds)
    }

    pub(crate) fn line_list_bounds(&self) -> Option<Bounds<Pixels>> {
        *self.line_list_bounds.borrow()
    }

    fn scroll_handle(&self) -> UniformListScrollHandle {
        self.scroll_handle.clone()
    }

    fn reveal_cursor(&self) {
        reveal_line(
            &self.scroll_handle,
            self.document.line_count,
            self.document.cursor.line,
        );
    }

    fn reveal_if_ok(&self, result: Result<(), EditorError>) -> Result<(), EditorError> {
        if result.is_ok() {
            self.reveal_cursor();
        }
        result
    }

    fn begin_mouse_selection(&mut self, position: TextPosition) {
        self.mouse_selecting = true;
        self.document.begin_selection_at(position);
        self.reveal_cursor();
    }

    fn update_mouse_selection(&mut self, position: TextPosition) -> bool {
        if !self.mouse_selecting {
            return false;
        }

        self.document.select_to(position);
        self.reveal_cursor();
        true
    }

    fn finish_mouse_selection(&mut self) -> bool {
        if !self.mouse_selecting {
            return false;
        }

        self.mouse_selecting = false;
        self.document.finish_selection();
        true
    }

    fn insert_pasted_text(&mut self, text: &str) -> Result<(), EditorError> {
        let result = self.document.insert_pasted_text(text);
        self.reveal_if_ok(result)
    }

    fn accept_completion(&mut self) -> Result<bool, EditorError> {
        let accepted = self.document.accept_completion()?;
        if accepted {
            self.reveal_cursor();
        }
        Ok(accepted)
    }

    fn accept_completion_at(&mut self, index: usize) -> Result<bool, EditorError> {
        let accepted = self.document.accept_completion_at(index)?;
        if accepted {
            self.reveal_cursor();
        }
        Ok(accepted)
    }

    fn jump_to_diagnostic(&mut self, index: usize) -> bool {
        let jumped = self.document.jump_to_diagnostic(index);
        if jumped {
            self.reveal_cursor();
        }
        jumped
    }

    fn select_next_completion(&mut self) -> bool {
        self.document.select_next_completion()
    }

    fn select_previous_completion(&mut self) -> bool {
        self.document.select_previous_completion()
    }

    fn close_completion_popup(&mut self) -> bool {
        self.document.close_completion_popup()
    }

    fn insert_tab(&mut self) -> Result<(), EditorError> {
        let result = self.document.insert_tab();
        self.reveal_if_ok(result)
    }

    fn toggle_line_comment(&mut self) -> Result<(), EditorError> {
        let result = self.document.toggle_line_comment();
        self.reveal_if_ok(result)
    }

    fn insert_newline(&mut self) -> Result<(), EditorError> {
        let result = self.document.insert_newline();
        self.reveal_if_ok(result)
    }

    fn outdent_current_line(&mut self) -> Result<(), EditorError> {
        let result = self.document.outdent_current_line();
        self.reveal_if_ok(result)
    }

    fn backspace(&mut self) -> Result<(), EditorError> {
        let result = self.document.backspace();
        self.reveal_if_ok(result)
    }

    fn delete(&mut self) -> Result<(), EditorError> {
        let result = self.document.delete();
        self.reveal_if_ok(result)
    }

    fn undo(&mut self) -> Result<(), EditorError> {
        let result = self.document.undo();
        self.reveal_if_ok(result)
    }

    fn redo(&mut self) -> Result<(), EditorError> {
        let result = self.document.redo();
        self.reveal_if_ok(result)
    }

    fn move_left(&mut self) -> Result<(), EditorError> {
        let result = self.document.move_left();
        self.reveal_if_ok(result)
    }

    fn move_right(&mut self) -> Result<(), EditorError> {
        let result = self.document.move_right();
        self.reveal_if_ok(result)
    }

    fn select_left(&mut self) -> Result<(), EditorError> {
        let result = self.document.select_left();
        self.reveal_if_ok(result)
    }

    fn select_right(&mut self) -> Result<(), EditorError> {
        let result = self.document.select_right();
        self.reveal_if_ok(result)
    }

    fn select_all(&mut self) -> Result<(), EditorError> {
        let result = self.document.select_all();
        self.reveal_if_ok(result)
    }

    fn select_next_match(&mut self) -> bool {
        let selected = self.document.select_next_match();
        if selected {
            self.reveal_cursor();
        }
        selected
    }

    fn move_up(&mut self) -> Result<(), EditorError> {
        let result = self.document.move_up();
        self.reveal_if_ok(result)
    }

    fn move_down(&mut self) -> Result<(), EditorError> {
        let result = self.document.move_down();
        self.reveal_if_ok(result)
    }

    fn move_line_start(&mut self) -> Result<(), EditorError> {
        let result = self.document.move_line_start();
        self.reveal_if_ok(result)
    }

    fn move_line_end(&mut self) -> Result<(), EditorError> {
        let result = self.document.move_line_end();
        self.reveal_if_ok(result)
    }

    fn select_line_start(&mut self) -> Result<(), EditorError> {
        let result = self.document.select_line_start();
        self.reveal_if_ok(result)
    }

    fn select_line_end(&mut self) -> Result<(), EditorError> {
        let result = self.document.select_line_end();
        self.reveal_if_ok(result)
    }

    fn page_up(&mut self) -> Result<(), EditorError> {
        let result = self.document.page_up();
        self.reveal_if_ok(result)
    }

    fn page_down(&mut self) -> Result<(), EditorError> {
        let result = self.document.page_down();
        self.reveal_if_ok(result)
    }

    fn select_up(&mut self) -> Result<(), EditorError> {
        let result = self.document.select_up();
        self.reveal_if_ok(result)
    }

    fn select_down(&mut self) -> Result<(), EditorError> {
        let result = self.document.select_down();
        self.reveal_if_ok(result)
    }

    pub(crate) fn replace_utf16_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        text: &str,
    ) -> Result<(), EditorError> {
        let result = self.document.replace_utf16_range(range_utf16, text);
        self.reveal_if_ok(result)
    }

    pub(crate) fn replace_and_mark_utf16_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        text: &str,
        selected_range_utf16: Option<Range<usize>>,
    ) -> Result<(), EditorError> {
        let result =
            self.document
                .replace_and_mark_utf16_range(range_utf16, text, selected_range_utf16);
        self.reveal_if_ok(result)
    }

    fn copy_selection_text(&self) -> Option<String> {
        self.document.copy_selection_text()
    }

    pub(crate) fn open_path(&mut self, path: impl AsRef<Path>) -> io::Result<()> {
        if self.document.open_path_if_clean(path)? {
            self.reveal_cursor();
        }
        Ok(())
    }

    pub(crate) fn has_unsaved_changes(&self) -> bool {
        self.document.dirty
    }

    pub(crate) fn block_open_for_unsaved_changes(&mut self) {
        self.document.save_status = "Unsaved changes; save before opening another file".to_string();
    }

    fn start_search_from_selection(&mut self) {
        self.document.start_search_from_selection();
    }

    fn start_go_to_line(&mut self) {
        self.document.start_go_to_line();
    }

    fn search_backspace(&mut self) {
        self.document.search_backspace();
    }

    fn go_to_line_backspace(&mut self) {
        self.document.go_to_line_backspace();
    }

    fn close_search(&mut self) {
        self.document.close_search();
    }

    fn close_go_to_line(&mut self) {
        self.document.close_go_to_line();
    }

    fn search_active(&self) -> bool {
        self.document.search_active
    }

    fn go_to_line_active(&self) -> bool {
        self.document.go_to_line_active
    }

    fn submit_go_to_line(&mut self) -> bool {
        let jumped = self.document.submit_go_to_line();
        if jumped {
            self.reveal_cursor();
        }
        jumped
    }

    fn find_next(&mut self) -> bool {
        let found = self.document.find_next();
        if found {
            self.reveal_cursor();
        }
        found
    }

    fn find_previous(&mut self) -> bool {
        let found = self.document.find_previous();
        if found {
            self.reveal_cursor();
        }
        found
    }
}

pub(crate) trait GpuiEditorHost {
    fn editor_component(&self) -> &GpuiEditorComponent;
    fn editor_component_mut(&mut self) -> &mut GpuiEditorComponent;
    fn open_document(&mut self, window: &mut Window, cx: &mut Context<Self>)
    where
        Self: Sized;
    fn save_document(&mut self, window: &mut Window, cx: &mut Context<Self>)
    where
        Self: Sized;
    fn save_document_as(&mut self, window: &mut Window, cx: &mut Context<Self>)
    where
        Self: Sized;
}

pub(crate) fn bind_editor_keys(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("backspace", Backspace, Some("MocodeEditor")),
        KeyBinding::new("delete", Delete, Some("MocodeEditor")),
        KeyBinding::new("tab", Tab, Some("MocodeEditor")),
        KeyBinding::new("shift-tab", ShiftTab, Some("MocodeEditor")),
        KeyBinding::new("cmd-/", ToggleComment, Some("MocodeEditor")),
        KeyBinding::new("ctrl-/", ToggleComment, Some("MocodeEditor")),
        KeyBinding::new("left", Left, Some("MocodeEditor")),
        KeyBinding::new("right", Right, Some("MocodeEditor")),
        KeyBinding::new("shift-left", SelectLeft, Some("MocodeEditor")),
        KeyBinding::new("shift-right", SelectRight, Some("MocodeEditor")),
        KeyBinding::new("cmd-a", SelectAll, Some("MocodeEditor")),
        KeyBinding::new("ctrl-a", SelectAll, Some("MocodeEditor")),
        KeyBinding::new("cmd-d", SelectNextMatch, Some("MocodeEditor")),
        KeyBinding::new("ctrl-d", SelectNextMatch, Some("MocodeEditor")),
        KeyBinding::new("up", Up, Some("MocodeEditor")),
        KeyBinding::new("down", Down, Some("MocodeEditor")),
        KeyBinding::new("shift-up", SelectUp, Some("MocodeEditor")),
        KeyBinding::new("shift-down", SelectDown, Some("MocodeEditor")),
        KeyBinding::new("home", Home, Some("MocodeEditor")),
        KeyBinding::new("end", End, Some("MocodeEditor")),
        KeyBinding::new("shift-home", SelectLineStart, Some("MocodeEditor")),
        KeyBinding::new("shift-end", SelectLineEnd, Some("MocodeEditor")),
        KeyBinding::new("pageup", PageUp, Some("MocodeEditor")),
        KeyBinding::new("pagedown", PageDown, Some("MocodeEditor")),
        KeyBinding::new("cmd-v", Paste, Some("MocodeEditor")),
        KeyBinding::new("ctrl-v", Paste, Some("MocodeEditor")),
        KeyBinding::new("cmd-c", Copy, Some("MocodeEditor")),
        KeyBinding::new("ctrl-c", Copy, Some("MocodeEditor")),
        KeyBinding::new("cmd-o", Open, Some("MocodeEditor")),
        KeyBinding::new("ctrl-o", Open, Some("MocodeEditor")),
        KeyBinding::new("cmd-s", Save, Some("MocodeEditor")),
        KeyBinding::new("ctrl-s", Save, Some("MocodeEditor")),
        KeyBinding::new("cmd-shift-s", SaveAs, Some("MocodeEditor")),
        KeyBinding::new("ctrl-shift-s", SaveAs, Some("MocodeEditor")),
        KeyBinding::new("cmd-z", Undo, Some("MocodeEditor")),
        KeyBinding::new("ctrl-z", Undo, Some("MocodeEditor")),
        KeyBinding::new("cmd-shift-z", Redo, Some("MocodeEditor")),
        KeyBinding::new("ctrl-shift-z", Redo, Some("MocodeEditor")),
        KeyBinding::new("ctrl-y", Redo, Some("MocodeEditor")),
        KeyBinding::new("cmd-f", Find, Some("MocodeEditor")),
        KeyBinding::new("ctrl-f", Find, Some("MocodeEditor")),
        KeyBinding::new("ctrl-g", GoToLine, Some("MocodeEditor")),
        KeyBinding::new("cmd-g", FindNext, Some("MocodeEditor")),
        KeyBinding::new("f3", FindNext, Some("MocodeEditor")),
        KeyBinding::new("enter", Enter, Some("MocodeEditor")),
        KeyBinding::new("shift-enter", FindPrevious, Some("MocodeEditor")),
        KeyBinding::new("cmd-shift-g", FindPrevious, Some("MocodeEditor")),
        KeyBinding::new("ctrl-shift-g", FindPrevious, Some("MocodeEditor")),
        KeyBinding::new("escape", EscapeSearch, Some("MocodeEditor")),
    ]);
}

pub(crate) fn render_editor_component<T>(
    editor: &GpuiEditorComponent,
    window: &mut Window,
    cx: &mut Context<'_, T>,
) -> impl IntoElement
where
    T: GpuiEditorHost + EntityInputHandler + 'static,
{
    div()
        .flex()
        .flex_col()
        .h_full()
        .child(editor_surface(editor, window, cx))
        .when_some(
            diagnostics_strip::<T>(editor.document(), cx),
            |this, strip| this.child(strip),
        )
        .child(status_bar(editor.document()))
}

fn editor_surface<T>(
    editor: &GpuiEditorComponent,
    _window: &mut Window,
    cx: &mut Context<'_, T>,
) -> impl IntoElement
where
    T: GpuiEditorHost + EntityInputHandler + 'static,
{
    let document = editor.document();
    let line_count = document.line_count;
    let line_list_bounds = editor.line_list_bounds_handle();
    let scroll_handle = editor.scroll_handle();
    let focus_handle = editor.focus_handle().clone();
    let entity = cx.entity();
    let input_focus_handle = focus_handle.clone();
    let input_entity = entity.clone();
    div()
        .relative()
        .flex_1()
        .w_full()
        .h_full()
        .bg(rgb(0xffffff))
        .track_focus(editor.focus_handle())
        .key_context("MocodeEditor")
        .on_action(
            cx.listener(|this: &mut T, _: &Backspace, _: &mut Window, cx| {
                if this.editor_component().go_to_line_active() {
                    this.editor_component_mut().go_to_line_backspace();
                    cx.notify();
                } else if this.editor_component().search_active() {
                    this.editor_component_mut().search_backspace();
                    cx.notify();
                } else if this.editor_component_mut().backspace().is_ok() {
                    cx.notify();
                }
            }),
        )
        .on_action(cx.listener(|this: &mut T, _: &Delete, _: &mut Window, cx| {
            if this.editor_component().go_to_line_active()
                || this.editor_component_mut().delete().is_ok()
            {
                cx.notify();
            }
        }))
        .on_action(cx.listener(|this: &mut T, _: &Tab, _: &mut Window, cx| {
            if this
                .editor_component_mut()
                .accept_completion()
                .is_ok_and(|accepted| accepted)
            {
                cx.notify();
            } else if this.editor_component_mut().insert_tab().is_ok() {
                cx.notify();
            }
        }))
        .on_action(
            cx.listener(|this: &mut T, _: &ShiftTab, _: &mut Window, cx| {
                if this.editor_component_mut().outdent_current_line().is_ok() {
                    cx.notify();
                }
            }),
        )
        .on_action(
            cx.listener(|this: &mut T, _: &ToggleComment, _: &mut Window, cx| {
                if this.editor_component_mut().toggle_line_comment().is_ok() {
                    cx.notify();
                }
            }),
        )
        .on_action(cx.listener(|this: &mut T, _: &Undo, _: &mut Window, cx| {
            if this.editor_component_mut().undo().is_ok() {
                cx.notify();
            }
        }))
        .on_action(cx.listener(|this: &mut T, _: &Redo, _: &mut Window, cx| {
            if this.editor_component_mut().redo().is_ok() {
                cx.notify();
            }
        }))
        .on_action(cx.listener(|this: &mut T, _: &Left, _: &mut Window, cx| {
            if this.editor_component_mut().move_left().is_ok() {
                cx.notify();
            }
        }))
        .on_action(cx.listener(|this: &mut T, _: &Right, _: &mut Window, cx| {
            if this.editor_component_mut().move_right().is_ok() {
                cx.notify();
            }
        }))
        .on_action(
            cx.listener(|this: &mut T, _: &SelectLeft, _: &mut Window, cx| {
                if this.editor_component_mut().select_left().is_ok() {
                    cx.notify();
                }
            }),
        )
        .on_action(
            cx.listener(|this: &mut T, _: &SelectRight, _: &mut Window, cx| {
                if this.editor_component_mut().select_right().is_ok() {
                    cx.notify();
                }
            }),
        )
        .on_action(
            cx.listener(|this: &mut T, _: &SelectAll, _: &mut Window, cx| {
                if this.editor_component_mut().select_all().is_ok() {
                    cx.notify();
                }
            }),
        )
        .on_action(
            cx.listener(|this: &mut T, _: &SelectNextMatch, _: &mut Window, cx| {
                if this.editor_component_mut().select_next_match() {
                    cx.notify();
                }
            }),
        )
        .on_action(
            cx.listener(|this: &mut T, _: &Open, window: &mut Window, cx| {
                this.open_document(window, cx);
                cx.notify();
            }),
        )
        .on_action(
            cx.listener(|this: &mut T, _: &Save, window: &mut Window, cx| {
                this.save_document(window, cx);
                cx.notify();
            }),
        )
        .on_action(
            cx.listener(|this: &mut T, _: &SaveAs, window: &mut Window, cx| {
                this.save_document_as(window, cx);
                cx.notify();
            }),
        )
        .on_action(cx.listener(|this: &mut T, _: &Find, _: &mut Window, cx| {
            this.editor_component_mut().start_search_from_selection();
            cx.notify();
        }))
        .on_action(
            cx.listener(|this: &mut T, _: &GoToLine, _: &mut Window, cx| {
                this.editor_component_mut().start_go_to_line();
                cx.notify();
            }),
        )
        .on_action(
            cx.listener(|this: &mut T, _: &FindNext, _: &mut Window, cx| {
                this.editor_component_mut().find_next();
                cx.notify();
            }),
        )
        .on_action(
            cx.listener(|this: &mut T, _: &FindPrevious, _: &mut Window, cx| {
                this.editor_component_mut().find_previous();
                cx.notify();
            }),
        )
        .on_action(
            cx.listener(|this: &mut T, _: &EscapeSearch, _: &mut Window, cx| {
                if this.editor_component_mut().close_completion_popup() {
                    cx.notify();
                } else if this.editor_component().go_to_line_active() {
                    this.editor_component_mut().close_go_to_line();
                    cx.notify();
                } else {
                    this.editor_component_mut().close_search();
                    cx.notify();
                }
            }),
        )
        .on_action(cx.listener(|this: &mut T, _: &Enter, _: &mut Window, cx| {
            if this.editor_component().go_to_line_active() {
                this.editor_component_mut().submit_go_to_line();
                cx.notify();
            } else if this.editor_component().search_active() {
                this.editor_component_mut().find_next();
                cx.notify();
            } else if this
                .editor_component_mut()
                .accept_completion()
                .is_ok_and(|accepted| accepted)
            {
                cx.notify();
            } else if this.editor_component_mut().insert_newline().is_ok() {
                cx.notify();
            }
        }))
        .on_action(cx.listener(|this: &mut T, _: &Up, _: &mut Window, cx| {
            if this.editor_component_mut().select_previous_completion() {
                cx.notify();
            } else if this.editor_component_mut().move_up().is_ok() {
                cx.notify();
            }
        }))
        .on_action(cx.listener(|this: &mut T, _: &Down, _: &mut Window, cx| {
            if this.editor_component_mut().select_next_completion() {
                cx.notify();
            } else if this.editor_component_mut().move_down().is_ok() {
                cx.notify();
            }
        }))
        .on_action(cx.listener(|this: &mut T, _: &Home, _: &mut Window, cx| {
            if this.editor_component_mut().move_line_start().is_ok() {
                cx.notify();
            }
        }))
        .on_action(cx.listener(|this: &mut T, _: &End, _: &mut Window, cx| {
            if this.editor_component_mut().move_line_end().is_ok() {
                cx.notify();
            }
        }))
        .on_action(
            cx.listener(|this: &mut T, _: &SelectLineStart, _: &mut Window, cx| {
                if this.editor_component_mut().select_line_start().is_ok() {
                    cx.notify();
                }
            }),
        )
        .on_action(
            cx.listener(|this: &mut T, _: &SelectLineEnd, _: &mut Window, cx| {
                if this.editor_component_mut().select_line_end().is_ok() {
                    cx.notify();
                }
            }),
        )
        .on_action(cx.listener(|this: &mut T, _: &PageUp, _: &mut Window, cx| {
            if this.editor_component_mut().page_up().is_ok() {
                cx.notify();
            }
        }))
        .on_action(
            cx.listener(|this: &mut T, _: &PageDown, _: &mut Window, cx| {
                if this.editor_component_mut().page_down().is_ok() {
                    cx.notify();
                }
            }),
        )
        .on_action(
            cx.listener(|this: &mut T, _: &SelectUp, _: &mut Window, cx| {
                if this.editor_component_mut().select_up().is_ok() {
                    cx.notify();
                }
            }),
        )
        .on_action(
            cx.listener(|this: &mut T, _: &SelectDown, _: &mut Window, cx| {
                if this.editor_component_mut().select_down().is_ok() {
                    cx.notify();
                }
            }),
        )
        .on_action(cx.listener(|this: &mut T, _: &Paste, _: &mut Window, cx| {
            if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text())
                && this
                    .editor_component_mut()
                    .insert_pasted_text(&text)
                    .is_ok()
            {
                cx.notify();
            }
        }))
        .on_action(cx.listener(|this: &mut T, _: &Copy, _: &mut Window, cx| {
            if let Some(text) = this.editor_component().copy_selection_text() {
                cx.write_to_clipboard(ClipboardItem::new_string(text));
            }
        }))
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(
                |this: &mut T, event: &MouseDownEvent, window: &mut Window, cx: &mut Context<T>| {
                    let focus_handle = this.editor_component().focus_handle().clone();
                    focus_handle.focus(window);

                    let position = mouse_event_text_position(
                        this.editor_component().document(),
                        this.editor_component().line_list_bounds(),
                        event.position,
                    );
                    if let Some(position) = position {
                        this.editor_component_mut().begin_mouse_selection(position);
                        cx.notify();
                    }
                },
            ),
        )
        .on_mouse_move(cx.listener(
            |this: &mut T, event: &MouseMoveEvent, _: &mut Window, cx: &mut Context<T>| {
                if !event.dragging() {
                    return;
                }

                let position = mouse_event_text_position(
                    this.editor_component().document(),
                    this.editor_component().line_list_bounds(),
                    event.position,
                );
                if let Some(position) = position
                    && this.editor_component_mut().update_mouse_selection(position)
                {
                    cx.notify();
                }
            },
        ))
        .on_mouse_up(
            MouseButton::Left,
            cx.listener(
                |this: &mut T, _: &MouseUpEvent, _: &mut Window, cx: &mut Context<T>| {
                    if this.editor_component_mut().finish_mouse_selection() {
                        cx.notify();
                    }
                },
            ),
        )
        .on_children_prepainted(move |children_bounds, _, _| {
            if let Some(bounds) = children_bounds.first().copied() {
                *line_list_bounds.borrow_mut() = Some(bounds);
            }
        })
        .child(
            uniform_list(
                "mocode-lines",
                line_count,
                cx.processor(
                    |this: &mut T, range: std::ops::Range<usize>, _window, _cx| {
                        let document = this.editor_component().document();
                        let slice = document.lines_in_range(range.start, range.end);
                        let selection_range = document.selected_range();
                        let mut search_highlights =
                            document.search_highlights_in_range(range.start, range.end);
                        let mut rows = Vec::new();
                        for (offset, line) in slice.into_iter().enumerate() {
                            let index = range.start + offset;
                            let index_u32 = index as u32;
                            let current_line = document.cursor.line as usize == index;
                            let cursor = current_line.then_some(document.cursor.character);
                            let line_selection = selection_range.and_then(|r| {
                                selection_on_line(index_u32, line.text.chars().count() as u32, r)
                            });
                            let line_search_highlights =
                                search_highlights.remove(&index_u32).unwrap_or_default();
                            let diagnostic_hint = current_line
                                .then(|| document.current_line_diagnostic_summary())
                                .flatten();
                            rows.push(line_row(
                                index,
                                line.number,
                                line.text,
                                line.diagnostic_count,
                                line.diagnostic_severity,
                                line.syntax_highlights,
                                diagnostic_hint,
                                cursor,
                                line_selection,
                                line_search_highlights,
                            ));
                        }
                        rows
                    },
                ),
            )
            .with_horizontal_sizing_behavior(ListHorizontalSizingBehavior::Unconstrained)
            .track_scroll(scroll_handle)
            .h_full(),
        )
        .child(
            canvas(
                |_, _, _| (),
                move |bounds, _, window, cx| {
                    window.handle_input(
                        &input_focus_handle,
                        ElementInputHandler::new(bounds, input_entity.clone()),
                        cx,
                    );
                },
            )
            .absolute()
            .top_0()
            .left_0()
            .size_full(),
        )
        .when_some(
            completion_popup::<T>(document.completion_popup.as_ref(), cx),
            |this, popup| this.child(popup),
        )
}

fn status_bar(document: &GpuiEditorDocument) -> impl IntoElement {
    let cursor = format!(
        "Ln {}, Col {}",
        document.cursor.line + 1,
        document.cursor.character + 1
    );
    let selection = (document.selection_summary != "<none>")
        .then(|| format!("Sel {}", document.selection_summary));
    let find = find_bar_label(document);
    let go_to_line = go_to_line_bar_label(document);

    div()
        .flex()
        .flex_row()
        .items_center()
        .gap_3()
        .px_3()
        .py_1()
        .bg(rgb(0xf8fafc))
        .border_t_1()
        .border_color(rgb(0xd9e2ec))
        .text_color(rgb(0x475569))
        .text_size(px(11.0))
        .child(status_item(cursor))
        .when_some(selection, |this, selection| {
            this.child(status_item(selection))
        })
        .child(status_item(format!("Path {}", document.current_yaml_path)))
        .child(status_item(diagnostics_summary(document)))
        .child(status_item(completion_summary(document)))
        .when_some(find, |this, find| this.child(find_bar(find)))
        .when_some(go_to_line, |this, go_to_line| {
            this.child(find_bar(go_to_line))
        })
        .child(status_item(chain_preview_summary(document)))
}

fn status_item(text: String) -> impl IntoElement {
    div()
        .max_w(px(360.0))
        .whitespace_nowrap()
        .overflow_hidden()
        .text_ellipsis()
        .child(text)
}

fn completion_popup<T>(
    popup: Option<&GpuiEditorCompletionPopup>,
    cx: &mut Context<'_, T>,
) -> Option<AnyElement>
where
    T: GpuiEditorHost + EntityInputHandler + 'static,
{
    let popup = popup?;
    let mut items = Vec::new();
    for (index, item) in popup.items.iter().enumerate() {
        items.push(completion_popup_item::<T>(
            item,
            index,
            index == popup.selected_index,
            cx,
        ));
    }

    Some(
        div()
            .absolute()
            .left(px(popup.left_px))
            .top(px(popup.top_px))
            .w(px(300.0))
            .rounded_sm()
            .border_1()
            .border_color(rgb(0xcbd5e1))
            .bg(rgb(0xffffff))
            .overflow_hidden()
            .children(items)
            .into_any_element(),
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CompletionPopupItemState {
    CompletionItemSelected,
    CompletionItem,
}

fn completion_popup_item<T>(
    item: &GpuiEditorCompletion,
    index: usize,
    selected: bool,
    cx: &mut Context<'_, T>,
) -> AnyElement
where
    T: GpuiEditorHost + EntityInputHandler + 'static,
{
    let state = if selected {
        CompletionPopupItemState::CompletionItemSelected
    } else {
        CompletionPopupItemState::CompletionItem
    };
    let background = match state {
        CompletionPopupItemState::CompletionItemSelected => rgb(0xe0f2fe),
        CompletionPopupItemState::CompletionItem => rgb(0xffffff),
    };
    let text_color = match state {
        CompletionPopupItemState::CompletionItemSelected => rgb(0x0f172a),
        CompletionPopupItemState::CompletionItem => rgb(0x334155),
    };

    div()
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .gap_2()
        .px_2()
        .py_1()
        .bg(background)
        .text_color(text_color)
        .text_size(px(12.0))
        .child(
            div()
                .flex_1()
                .overflow_hidden()
                .text_ellipsis()
                .child(item.label.clone()),
        )
        .child(
            div()
                .text_size(px(10.0))
                .text_color(rgb(0x64748b))
                .child(item.kind.clone()),
        )
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(
                move |this: &mut T, _: &MouseDownEvent, _: &mut Window, cx| {
                    if this
                        .editor_component_mut()
                        .accept_completion_at(index)
                        .is_ok_and(|accepted| accepted)
                    {
                        cx.notify();
                    }
                },
            ),
        )
        .into_any_element()
}

fn diagnostics_strip<T>(
    document: &GpuiEditorDocument,
    cx: &mut Context<'_, T>,
) -> Option<AnyElement>
where
    T: GpuiEditorHost + EntityInputHandler + 'static,
{
    if document.diagnostics.is_empty() {
        return None;
    }

    let mut items = Vec::new();
    for (index, diagnostic) in document.diagnostics.iter().take(5).enumerate() {
        items.push(diagnostic_item::<T>(index, diagnostic, cx));
    }

    Some(
        div()
            .flex()
            .flex_row()
            .items_center()
            .gap_2()
            .px_3()
            .py_1()
            .border_t_1()
            .border_color(rgb(0xfecaca))
            .bg(rgb(0xfffbeb))
            .children(items)
            .into_any_element(),
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DiagnosticItemState {
    DiagnosticItem,
}

fn diagnostic_item<T>(
    index: usize,
    diagnostic: &GpuiEditorDiagnostic,
    cx: &mut Context<'_, T>,
) -> AnyElement
where
    T: GpuiEditorHost + EntityInputHandler + 'static,
{
    let state = DiagnosticItemState::DiagnosticItem;
    let border_color = match state {
        DiagnosticItemState::DiagnosticItem => rgb(0xfbbf24),
    };
    let location = diagnostic
        .line
        .zip(diagnostic.column)
        .map(|(line, column)| format!("{}:{}", line, column))
        .unwrap_or_else(|| "global".to_string());
    let text = format!(
        "{} {} {}",
        diagnostic.severity, location, diagnostic.message
    );

    div()
        .max_w(px(520.0))
        .px_2()
        .py_0p5()
        .rounded_sm()
        .border_1()
        .border_color(border_color)
        .bg(rgb(0xffffff))
        .text_color(rgb(0x78350f))
        .text_size(px(11.0))
        .whitespace_nowrap()
        .overflow_hidden()
        .text_ellipsis()
        .child(text)
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(
                move |this: &mut T, _: &MouseDownEvent, _: &mut Window, cx| {
                    if this.editor_component_mut().jump_to_diagnostic(index) {
                        cx.notify();
                    }
                },
            ),
        )
        .into_any_element()
}

pub(crate) fn find_bar_label(document: &GpuiEditorDocument) -> Option<String> {
    document
        .search_active
        .then(|| format!("Find: {}", document.search_summary))
}

pub(crate) fn go_to_line_bar_label(document: &GpuiEditorDocument) -> Option<String> {
    document
        .go_to_line_active
        .then(|| document.go_to_line_summary.clone())
}

fn find_bar(text: String) -> impl IntoElement {
    div()
        .flex()
        .flex_row()
        .items_center()
        .gap_2()
        .max_w(px(420.0))
        .px_2()
        .py_0p5()
        .rounded_sm()
        .border_1()
        .border_color(rgb(0xf59e0b))
        .bg(rgb(0xfffbeb))
        .text_color(rgb(0x78350f))
        .whitespace_nowrap()
        .overflow_hidden()
        .text_ellipsis()
        .child(text)
        .child(
            div()
                .px_1()
                .rounded_sm()
                .bg(rgb(0xfef3c7))
                .text_color(rgb(0x92400e))
                .child("Esc"),
        )
}

fn diagnostics_summary(document: &GpuiEditorDocument) -> String {
    if document.diagnostics.is_empty() {
        "Diagnostics 0".to_string()
    } else {
        let errors = document
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == "error")
            .count();
        let warnings = document
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == "warning")
            .count();
        format!("Diagnostics {errors} error, {warnings} warning")
    }
}

fn completion_summary(document: &GpuiEditorDocument) -> String {
    if document.completion_items.is_empty() {
        "Completions 0".to_string()
    } else {
        format!("Completions {}", document.completion_items.len())
    }
}

fn chain_preview_summary(document: &GpuiEditorDocument) -> String {
    let Some(preview) = &document.chain_preview else {
        return "Chain <none>".to_string();
    };

    let status = match preview.status {
        ProxyChainStatus::Complete => "complete",
        ProxyChainStatus::MissingReference => "missing",
        ProxyChainStatus::Cycle => "cycle",
    };
    format!("Chain {} ({status})", preview.steps.join(" -> "))
}
fn line_row(
    index: usize,
    number: u32,
    text: String,
    diagnostic_count: usize,
    diagnostic_severity: Option<String>,
    syntax_highlights: Vec<GpuiSyntaxHighlight>,
    diagnostic_hint: Option<String>,
    cursor: Option<u32>,
    selection: Option<(u32, u32)>,
    search_highlights: Vec<GpuiSearchHighlight>,
) -> impl IntoElement {
    let marker_color = diagnostic_severity
        .as_deref()
        .map(severity_color)
        .unwrap_or_else(|| rgb(0xf8fafc).into());

    div()
        .id(index)
        .flex()
        .flex_row()
        .h(px(22.0))
        .line_height(px(22.0))
        .border_b_1()
        .border_color(rgb(0xf1f5f9))
        .child(
            div()
                .w(px(64.0))
                .flex()
                .flex_row()
                .text_color(rgb(0x94a3b8))
                .bg(rgb(0xf8fafc))
                .child(div().w(px(4.0)).h_full().bg(marker_color))
                .child(div().w(px(60.0)).px_2().child(if diagnostic_count == 0 {
                    format!("{number:>4}")
                } else {
                    format!("{number:>3}!")
                })),
        )
        .child(render_line_text(
            text,
            cursor,
            selection,
            search_highlights,
            syntax_highlights,
        ))
        .when_some(diagnostic_hint, |this, hint| {
            this.child(line_diagnostic_hint(hint))
        })
}

fn line_diagnostic_hint(text: String) -> impl IntoElement {
    div()
        .ml_3()
        .max_w(px(520.0))
        .px_2()
        .rounded_sm()
        .bg(rgb(0xfff7ed))
        .text_color(rgb(0x9a3412))
        .text_size(px(11.0))
        .whitespace_nowrap()
        .overflow_hidden()
        .text_ellipsis()
        .child(text)
}

fn render_line_text(
    text: String,
    cursor: Option<u32>,
    selection: Option<(u32, u32)>,
    search_highlights: Vec<GpuiSearchHighlight>,
    syntax_highlights: Vec<GpuiSyntaxHighlight>,
) -> impl IntoElement {
    div()
        .px_3()
        .flex()
        .flex_row()
        .items_center()
        .text_color(rgb(0x0f172a))
        .whitespace_nowrap()
        .children(render_text_segments(
            &text,
            cursor,
            text_highlights_for_line(selection, search_highlights),
            syntax_highlights_for_line(syntax_highlights),
        ))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TextHighlightKind {
    Search,
    ActiveSearch,
    Selection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TextHighlight {
    start: u32,
    end: u32,
    kind: TextHighlightKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TextSyntaxHighlight {
    start: u32,
    end: u32,
    kind: SyntaxHighlightKind,
}

fn text_highlights_for_line(
    selection: Option<(u32, u32)>,
    search_highlights: Vec<GpuiSearchHighlight>,
) -> Vec<TextHighlight> {
    let selection = selection.and_then(|(start, end)| {
        (start < end).then_some(TextHighlight {
            start,
            end,
            kind: TextHighlightKind::Selection,
        })
    });

    let mut highlights: Vec<TextHighlight> = search_highlights
        .into_iter()
        .filter(|highlight| highlight.start < highlight.end)
        .filter(|highlight| {
            selection.is_none_or(|selection| {
                highlight.end <= selection.start || highlight.start >= selection.end
            })
        })
        .map(|highlight| TextHighlight {
            start: highlight.start,
            end: highlight.end,
            kind: if highlight.active {
                TextHighlightKind::ActiveSearch
            } else {
                TextHighlightKind::Search
            },
        })
        .collect();

    if let Some(selection) = selection {
        highlights.push(selection);
    }

    highlights.sort_by_key(|highlight| highlight.start);
    highlights
}

fn syntax_highlights_for_line(
    syntax_highlights: Vec<GpuiSyntaxHighlight>,
) -> Vec<TextSyntaxHighlight> {
    let mut highlights: Vec<TextSyntaxHighlight> = syntax_highlights
        .into_iter()
        .filter(|highlight| highlight.start < highlight.end)
        .map(|highlight| TextSyntaxHighlight {
            start: highlight.start,
            end: highlight.end,
            kind: highlight.kind,
        })
        .collect();

    highlights.sort_by_key(|highlight| (highlight.start, highlight.end));
    highlights
}

fn render_text_segments(
    text: &str,
    cursor: Option<u32>,
    highlights: Vec<TextHighlight>,
    syntax_highlights: Vec<TextSyntaxHighlight>,
) -> Vec<AnyElement> {
    let line_length = text.chars().count() as u32;
    let mut children = Vec::new();
    let mut cursor_inserted = false;

    let mut boundaries = vec![0, line_length];
    for highlight in &highlights {
        boundaries.push(highlight.start.min(line_length));
        boundaries.push(highlight.end.min(line_length));
    }
    for highlight in &syntax_highlights {
        boundaries.push(highlight.start.min(line_length));
        boundaries.push(highlight.end.min(line_length));
    }
    boundaries.sort_unstable();
    boundaries.dedup();

    for window in boundaries.windows(2) {
        let start = window[0];
        let end = window[1];
        push_text_segment(
            &mut children,
            text,
            start,
            end,
            cursor,
            &mut cursor_inserted,
            text_highlight_for_range(&highlights, start, end),
            syntax_highlight_for_range(&syntax_highlights, start, end),
        );
    }

    if !cursor_inserted
        && let Some(cursor) = cursor
        && cursor == line_length
    {
        push_cursor(&mut children);
    }

    children
}

fn push_text_segment(
    children: &mut Vec<AnyElement>,
    text: &str,
    start: u32,
    end: u32,
    cursor: Option<u32>,
    cursor_inserted: &mut bool,
    highlight: Option<TextHighlightKind>,
    syntax_highlight: Option<SyntaxHighlightKind>,
) {
    let cursor_in_segment =
        !*cursor_inserted && cursor.is_some_and(|cursor| cursor >= start && cursor <= end);

    if cursor_in_segment {
        let cursor = cursor.unwrap();
        push_text_piece(children, text, start, cursor, highlight, syntax_highlight);
        push_cursor(children);
        *cursor_inserted = true;
        push_text_piece(children, text, cursor, end, highlight, syntax_highlight);
    } else {
        push_text_piece(children, text, start, end, highlight, syntax_highlight);
    }
}

fn text_highlight_for_range(
    highlights: &[TextHighlight],
    start: u32,
    end: u32,
) -> Option<TextHighlightKind> {
    highlights
        .iter()
        .find(|highlight| highlight.start <= start && highlight.end >= end)
        .map(|highlight| highlight.kind)
}

fn syntax_highlight_for_range(
    highlights: &[TextSyntaxHighlight],
    start: u32,
    end: u32,
) -> Option<SyntaxHighlightKind> {
    let mut fallback = None;
    for highlight in highlights
        .iter()
        .filter(|highlight| highlight.start <= start && highlight.end >= end)
    {
        fallback.get_or_insert(highlight.kind);
        if highlight.kind == SyntaxHighlightKind::Error {
            return Some(SyntaxHighlightKind::Error);
        }
    }
    fallback
}

fn push_text_piece(
    children: &mut Vec<AnyElement>,
    text: &str,
    start: u32,
    end: u32,
    highlight: Option<TextHighlightKind>,
    syntax_highlight: Option<SyntaxHighlightKind>,
) {
    if start >= end {
        return;
    }

    let start_byte = char_to_byte_index(text, start);
    let end_byte = char_to_byte_index(text, end);
    let piece = text[start_byte..end_byte].to_string();
    let mut element = div();
    if let Some(color) = syntax_highlight.and_then(syntax_highlight_color) {
        element = element.text_color(color);
    }
    if let Some(color) = text_highlight_color(highlight) {
        element = element.bg(color);
    }
    let element = element.child(piece).into_any_element();
    children.push(element);
}

fn syntax_highlight_color(kind: SyntaxHighlightKind) -> Option<gpui::Hsla> {
    match kind {
        SyntaxHighlightKind::Comment => Some(rgb(0x64748b).into()),
        SyntaxHighlightKind::Key => Some(rgb(0x1d4ed8).into()),
        SyntaxHighlightKind::String => Some(rgb(0x047857).into()),
        SyntaxHighlightKind::Number => Some(rgb(0xb45309).into()),
        SyntaxHighlightKind::Boolean => Some(rgb(0x7c3aed).into()),
        SyntaxHighlightKind::Null => Some(rgb(0xbe123c).into()),
        SyntaxHighlightKind::Anchor | SyntaxHighlightKind::Alias => Some(rgb(0x0e7490).into()),
        SyntaxHighlightKind::Tag => Some(rgb(0x9333ea).into()),
        SyntaxHighlightKind::Error => Some(rgb(0xdc2626).into()),
        SyntaxHighlightKind::Punctuation => None,
    }
}

fn text_highlight_color(kind: Option<TextHighlightKind>) -> Option<gpui::Hsla> {
    match kind {
        Some(TextHighlightKind::Search) => Some(rgb(0xfef3c7).into()),
        Some(TextHighlightKind::ActiveSearch) => Some(rgb(0xfbbf24).into()),
        Some(TextHighlightKind::Selection) => Some(rgb(0xdbeafe).into()),
        None => None,
    }
}

fn push_cursor(children: &mut Vec<AnyElement>) {
    children.push(
        div()
            .w(px(1.0))
            .h(px(16.0))
            .bg(rgb(0x2563eb))
            .into_any_element(),
    );
}

fn auto_indent_for_line_prefix(line_prefix: &str) -> String {
    let mut indent = leading_spaces(line_prefix);
    let trimmed = line_prefix.trim_end();
    let trimmed_start = trimmed.trim_start();

    if trimmed.ends_with(':') || trimmed_start == "-" || trimmed_start.starts_with("- ") {
        indent.push_str(TAB_WIDTH);
    }

    indent
}

fn toggle_yaml_line_comment(line: &str, uncomment: bool) -> String {
    if line.trim().is_empty() {
        return line.to_string();
    }

    let indent_len = leading_space_count(line);
    let indent_byte = char_to_byte_index(line, indent_len as u32);

    if uncomment {
        let Some(remove_count) = comment_remove_count(line) else {
            return line.to_string();
        };
        let remove_end = indent_byte + remove_count;
        format!("{}{}", &line[..indent_byte], &line[remove_end..])
    } else {
        format!("{}# {}", &line[..indent_byte], &line[indent_byte..])
    }
}

fn line_is_commented(line: &str) -> bool {
    let indent_len = leading_space_count(line);
    let indent_byte = char_to_byte_index(line, indent_len as u32);
    line[indent_byte..].starts_with('#')
}

fn comment_remove_count(line: &str) -> Option<usize> {
    let indent_len = leading_space_count(line);
    let indent_byte = char_to_byte_index(line, indent_len as u32);
    let suffix = &line[indent_byte..];

    if suffix.starts_with("# ") {
        Some(2)
    } else if suffix.starts_with('#') {
        Some(1)
    } else {
        None
    }
}

fn cursor_after_comment_toggle(
    cursor: TextPosition,
    start_line: u32,
    end_line: u32,
    original_lines: &[String],
    uncomment: bool,
) -> TextPosition {
    if cursor.line < start_line || cursor.line > end_line {
        return cursor;
    }

    let Some(line) = original_lines.get((cursor.line - start_line) as usize) else {
        return cursor;
    };
    if line.trim().is_empty() {
        return cursor;
    }

    let indent_len = leading_space_count(line) as u32;
    if cursor.character < indent_len {
        return cursor;
    }

    if uncomment {
        let Some(remove_count) = comment_remove_count(line) else {
            return cursor;
        };
        TextPosition::new(
            cursor.line,
            cursor
                .character
                .saturating_sub(remove_count as u32)
                .max(indent_len),
        )
    } else {
        TextPosition::new(cursor.line, cursor.character + 2)
    }
}

fn completion_insert_text(
    completion_items: &[GpuiEditorCompletion],
    prefix: &str,
) -> Option<String> {
    if prefix.is_empty() {
        return completion_items
            .first()
            .map(|completion| completion.insert_text.clone());
    }

    completion_items
        .iter()
        .find(|completion| {
            starts_with_ignore_ascii_case(&completion.insert_text, prefix)
                || starts_with_ignore_ascii_case(&completion.label, prefix)
        })
        .map(|completion| completion.insert_text.clone())
}

fn completion_prefix_start(line_text: &str, cursor_character: u32) -> u32 {
    let chars = line_text.chars().collect::<Vec<_>>();
    let mut index = (cursor_character as usize).min(chars.len());

    while index > 0 && is_completion_prefix_char(chars[index - 1]) {
        index -= 1;
    }

    index as u32
}

fn is_completion_prefix_char(ch: char) -> bool {
    !ch.is_whitespace() && !matches!(ch, ':' | ',' | '[' | ']' | '{' | '}' | '"' | '\'' | '#')
}

fn can_accept_empty_completion(line_prefix: &str) -> bool {
    let trimmed = line_prefix.trim_end();
    !line_prefix.trim().is_empty() && (trimmed.ends_with(':') || trimmed.ends_with('-'))
}

fn starts_with_ignore_ascii_case(text: &str, prefix: &str) -> bool {
    text.get(..prefix.len())
        .is_some_and(|head| head.eq_ignore_ascii_case(prefix))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CompletionSelection {
    Next,
    Previous,
}

fn normalize_pasted_yaml_indentation(text: &str, line_prefix: &str) -> String {
    if !text.contains('\n') {
        return text.to_string();
    }

    let base_indent = leading_spaces(line_prefix);
    let lines: Vec<&str> = text.split('\n').collect();
    let mut normalized = strip_trailing_carriage_return(lines[0]).to_string();
    let last_index = lines.len().saturating_sub(1);

    for (index, line) in lines.iter().enumerate().skip(1) {
        let line = strip_trailing_carriage_return(line);
        normalized.push('\n');

        if index == last_index && line.is_empty() && text.ends_with('\n') {
            continue;
        }

        normalized.push_str(&base_indent);
        normalized.push_str(line.trim_start_matches(' '));
    }

    normalized
}

fn leading_spaces(text: &str) -> String {
    text.chars().take_while(|ch| *ch == ' ').collect()
}

fn leading_space_count(text: &str) -> usize {
    text.chars().take_while(|ch| *ch == ' ').count()
}

fn strip_trailing_carriage_return(line: &str) -> &str {
    line.strip_suffix('\r').unwrap_or(line)
}

fn char_to_byte_index(text: &str, char_index: u32) -> usize {
    text.char_indices()
        .nth(char_index as usize)
        .map(|(i, _)| i)
        .unwrap_or(text.len())
}

fn mouse_event_text_position(
    document: &GpuiEditorDocument,
    line_list_bounds: Option<Bounds<Pixels>>,
    position: Point<Pixels>,
) -> Option<TextPosition> {
    let line_list_bounds = line_list_bounds?;
    let y: f32 = position.y.into();
    let x: f32 = position.x.into();
    let editor_origin_y: f32 = line_list_bounds.top().into();
    let editor_origin_x: f32 = line_list_bounds.left().into();

    mouse_to_text_position(
        y,
        x,
        editor_origin_y,
        editor_origin_x,
        GUTTER_WIDTH_PX,
        CHAR_WIDTH_PX,
        LINE_HEIGHT_PX,
        document.line_count,
        |line| {
            document
                .editor
                .line_end_position(line as usize)
                .map(|pos| pos.character)
                .unwrap_or(0)
        },
    )
}

/// Convert a window-space mouse coordinate into a `TextPosition`.
///
/// `editor_origin_x` and `editor_origin_y` are the actual window-space origin
/// of the rendered line list, captured from GPUI layout bounds during prepaint.
fn mouse_to_text_position(
    window_y: f32,
    window_x: f32,
    editor_origin_y: f32,
    editor_origin_x: f32,
    gutter_width: f32,
    char_width: f32,
    line_height: f32,
    line_count: usize,
    get_line_length: impl Fn(u32) -> u32,
) -> Option<TextPosition> {
    let local_x = window_x - editor_origin_x;
    let text_x = local_x - gutter_width;
    if text_x < 0.0 || window_y < editor_origin_y {
        return None;
    }

    let max_line = line_count.saturating_sub(1) as u32;
    let line = (((window_y - editor_origin_y) / line_height) as u32).min(max_line);
    let character = if text_x > 0.0 {
        (text_x / char_width) as u32
    } else {
        0
    };
    let character = character.min(get_line_length(line));
    Some(TextPosition::new(line, character))
}

#[cfg(test)]
mod scroll_tests {
    use super::*;

    #[test]
    fn reveal_line_requests_center_scroll_to_cursor_line() {
        let handle = UniformListScrollHandle::new();

        reveal_line(&handle, 100, 42);

        let request = handle
            .0
            .borrow()
            .deferred_scroll_to_item
            .expect("cursor reveal should request a scroll target");
        assert_eq!(request.item_index, 42);
        assert_eq!(request.strategy, ScrollStrategy::Center);
        assert_eq!(request.offset, 0);
        assert!(!request.scroll_strict);
    }

    #[test]
    fn reveal_line_clamps_past_end_to_last_line() {
        let handle = UniformListScrollHandle::new();

        reveal_line(&handle, 3, 99);

        let request = handle
            .0
            .borrow()
            .deferred_scroll_to_item
            .expect("cursor reveal should request a scroll target");
        assert_eq!(request.item_index, 2);
    }

    #[test]
    fn reveal_line_clears_request_for_empty_documents() {
        let handle = UniformListScrollHandle::new();
        handle.scroll_to_item(5, ScrollStrategy::Center);

        reveal_line(&handle, 0, 5);

        assert!(handle.0.borrow().deferred_scroll_to_item.is_none());
    }
}

#[cfg(test)]
mod mouse_tests {
    use super::*;

    fn test_line_lengths(line: u32) -> u32 {
        match line {
            0 => 8,  // "line one"
            1 => 8,  // "line two"
            2 => 10, // "line three"
            _ => 0,
        }
    }

    #[test]
    fn click_top_of_text_area_maps_to_line_zero() {
        // editor origin at y=120, clicking at y=125 → line 0
        let pos = mouse_to_text_position(
            125.0,
            100.0, // window y, x
            120.0, // editor_origin_y
            0.0,   // editor_origin_x
            64.0,  // gutter
            7.5,   // char width
            22.0,  // line height
            3,     // line_count
            test_line_lengths,
        )
        .unwrap();
        assert_eq!(pos.line, 0);
        assert!(pos.character > 0); // x=100 > gutter=64, so some column
    }

    #[test]
    fn click_below_editor_origin_is_rejected() {
        assert!(
            mouse_to_text_position(
                10.0,
                100.0, // y < editor_origin_y
                120.0,
                0.0,
                64.0,
                7.5,
                22.0,
                3,
                test_line_lengths
            )
            .is_none()
        );
    }

    #[test]
    fn click_left_of_gutter_is_rejected() {
        let pos = mouse_to_text_position(
            130.0,
            50.0, // x=50 < gutter=64 → text_x negative → None
            120.0,
            0.0,
            64.0,
            7.5,
            22.0,
            3,
            test_line_lengths,
        );
        // text_x < 0 should return None (click on gutter, don't move)
        assert!(pos.is_none());
    }

    #[test]
    fn click_past_line_end_clamps_to_line_end() {
        let pos = mouse_to_text_position(
            125.0,
            500.0, // x far to the right
            120.0,
            0.0,
            64.0,
            7.5,
            22.0,
            3,
            test_line_lengths,
        )
        .unwrap();
        assert_eq!(pos.line, 0);
        // column clamped to line 0 length (8)
        assert_eq!(pos.character, 8);
    }

    #[test]
    fn click_below_last_line_clamps_to_last_line() {
        let pos = mouse_to_text_position(
            300.0,
            100.0, // y far down
            120.0,
            0.0,
            64.0,
            7.5,
            22.0,
            3,
            test_line_lengths,
        )
        .unwrap();
        assert_eq!(pos.line, 2); // last line (line_count=3, indices 0-2)
    }

    #[test]
    fn click_on_second_line_returns_line_one() {
        let pos = mouse_to_text_position(
            120.0 + 22.0 + 5.0, // origin + 1 line + 5px into line 1
            100.0,
            120.0,
            0.0,
            64.0,
            7.5,
            22.0,
            3,
            test_line_lengths,
        )
        .unwrap();
        assert_eq!(pos.line, 1);
    }

    #[test]
    fn click_with_nonzero_editor_x_uses_local_text_area() {
        let editor_origin_x = 40.0;
        let gutter_width = 64.0;
        let pos = mouse_to_text_position(
            125.0,
            editor_origin_x + gutter_width,
            120.0,
            editor_origin_x,
            gutter_width,
            7.5,
            22.0,
            3,
            test_line_lengths,
        )
        .unwrap();

        assert_eq!(pos, TextPosition::new(0, 0));
    }
}

fn build_completion_popup(
    cursor: TextPosition,
    completion_items: &[GpuiEditorCompletion],
    selected_index: usize,
    prefix: &str,
    can_accept_empty: bool,
) -> Option<GpuiEditorCompletionPopup> {
    let visible_items = visible_completion_items(completion_items, prefix, can_accept_empty);
    let selected_index = selected_index.min(visible_items.len().saturating_sub(1));

    (!visible_items.is_empty()).then(|| GpuiEditorCompletionPopup {
        anchor_line: cursor.line + 1,
        anchor_column: cursor.character + 1,
        left_px: completion_popup_left(cursor),
        top_px: completion_popup_top(cursor),
        selected_index,
        items: visible_items,
    })
}

fn completion_popup_left(cursor: TextPosition) -> f32 {
    GUTTER_WIDTH_PX + cursor.character as f32 * CHAR_WIDTH_PX
}

fn completion_popup_top(cursor: TextPosition) -> f32 {
    (cursor.line as f32 + 1.0) * LINE_HEIGHT_PX
}

fn visible_completion_items(
    completion_items: &[GpuiEditorCompletion],
    prefix: &str,
    can_accept_empty: bool,
) -> Vec<GpuiEditorCompletion> {
    if prefix.is_empty() {
        return can_accept_empty
            .then(|| completion_items.iter().take(6).cloned().collect())
            .unwrap_or_default();
    }

    completion_items
        .iter()
        .filter(|completion| {
            starts_with_ignore_ascii_case(&completion.insert_text, prefix)
                || starts_with_ignore_ascii_case(&completion.label, prefix)
        })
        .take(6)
        .cloned()
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SearchMatch {
    range: TextRange,
    ordinal: usize,
    total: usize,
}

fn find_next_match(
    text: &str,
    query: &str,
    cursor: TextPosition,
    selected_range: Option<TextRange>,
) -> Option<SearchMatch> {
    if query.is_empty() {
        return None;
    }

    let matches = match_start_indices(text, query);
    if matches.is_empty() {
        return None;
    }

    let start_byte = selected_range
        .map(|range| byte_index_at_position(text, range.end.max(range.start)))
        .unwrap_or_else(|| byte_index_at_position(text, cursor));
    let index = matches
        .iter()
        .position(|match_start| *match_start >= start_byte)
        .unwrap_or(0);
    build_search_match(text, query, matches[index], index + 1, matches.len())
}

fn find_previous_match(
    text: &str,
    query: &str,
    cursor: TextPosition,
    selected_range: Option<TextRange>,
) -> Option<SearchMatch> {
    if query.is_empty() {
        return None;
    }

    let matches = match_start_indices(text, query);
    if matches.is_empty() {
        return None;
    }

    let start_byte = selected_range
        .map(|range| byte_index_at_position(text, range.start.min(range.end)))
        .unwrap_or_else(|| byte_index_at_position(text, cursor));
    let index = matches
        .iter()
        .rposition(|match_start| *match_start < start_byte)
        .unwrap_or(matches.len() - 1);
    build_search_match(text, query, matches[index], index + 1, matches.len())
}

fn match_start_indices(text: &str, query: &str) -> Vec<usize> {
    text.match_indices(query)
        .map(|(byte_index, _)| byte_index)
        .collect()
}

fn parse_go_to_line_query(query: &str) -> Option<usize> {
    query.parse::<usize>().ok().filter(|line| *line > 0)
}

fn identifier_range_at_position(
    line_index: u32,
    line_text: &str,
    cursor_character: u32,
) -> Option<TextRange> {
    let chars: Vec<char> = line_text.chars().collect();
    if chars.is_empty() {
        return None;
    }

    let mut index = (cursor_character as usize).min(chars.len());
    if index == chars.len() || !is_yaml_identifier_char(chars[index]) {
        if index == 0 || !is_yaml_identifier_char(chars[index - 1]) {
            return None;
        }
        index -= 1;
    }

    let mut start = index;
    while start > 0 && is_yaml_identifier_char(chars[start - 1]) {
        start -= 1;
    }

    let mut end = index + 1;
    while end < chars.len() && is_yaml_identifier_char(chars[end]) {
        end += 1;
    }

    (start < end).then(|| {
        TextRange::new(
            TextPosition::new(line_index, start as u32),
            TextPosition::new(line_index, end as u32),
        )
    })
}

fn is_yaml_identifier_char(ch: char) -> bool {
    ch.is_alphanumeric() || matches!(ch, '_' | '-' | '.')
}

fn build_search_match(
    text: &str,
    query: &str,
    start_byte: usize,
    ordinal: usize,
    total: usize,
) -> Option<SearchMatch> {
    let end_byte = start_byte.checked_add(query.len())?;
    Some(SearchMatch {
        range: TextRange::new(
            text_position_at_byte_index(text, start_byte),
            text_position_at_byte_index(text, end_byte),
        ),
        ordinal,
        total,
    })
}

fn byte_index_at_position(text: &str, position: TextPosition) -> usize {
    let mut line = 0;
    let mut character = 0;

    for (byte_index, ch) in text.char_indices() {
        if line == position.line && character == position.character {
            return byte_index;
        }

        if ch == '\n' {
            line += 1;
            character = 0;
        } else {
            character += 1;
        }
    }

    text.len()
}

fn text_position_at_byte_index(text: &str, target_byte_index: usize) -> TextPosition {
    let mut line = 0;
    let mut character = 0;

    for (byte_index, ch) in text.char_indices() {
        if byte_index >= target_byte_index {
            break;
        }

        if ch == '\n' {
            line += 1;
            character = 0;
        } else {
            character += 1;
        }
    }

    TextPosition::new(line, character)
}

fn ordered_text_range(range: TextRange) -> TextRange {
    if range.start <= range.end {
        range
    } else {
        TextRange::new(range.end, range.start)
    }
}

fn position_after_insert(start: TextPosition, text: &str) -> TextPosition {
    let mut line = start.line;
    let mut character = start.character;

    for ch in text.chars() {
        if ch == '\n' {
            line += 1;
            character = 0;
        } else {
            character += 1;
        }
    }

    TextPosition::new(line, character)
}

fn position_after_utf16_prefix(
    start: TextPosition,
    text: &str,
    target_utf16: usize,
) -> TextPosition {
    let mut line = start.line;
    let mut character = start.character;
    let mut utf16_index = 0usize;

    for ch in text.chars() {
        let width = ch.len_utf16();
        if utf16_index + width > target_utf16 {
            break;
        }

        utf16_index += width;
        if ch == '\n' {
            line += 1;
            character = 0;
        } else {
            character += 1;
        }
    }

    TextPosition::new(line, character)
}

fn format_selection_range(range: TextRange) -> String {
    let (start, end) = if range.start <= range.end {
        (range.start, range.end)
    } else {
        (range.end, range.start)
    };
    format!(
        "{}:{} -> {}:{}",
        start.line + 1,
        start.character + 1,
        end.line + 1,
        end.character + 1
    )
}

fn severity_label(severity: DiagnosticSeverity) -> &'static str {
    match severity {
        DiagnosticSeverity::Error => "error",
        DiagnosticSeverity::Warning => "warning",
        DiagnosticSeverity::Info => "info",
        DiagnosticSeverity::Hint => "hint",
    }
}

fn format_diagnostic_summary(severity: &str, message: &str) -> String {
    format!("{severity}: {message}")
}

fn completion_kind_label(kind: CompletionKind) -> &'static str {
    match kind {
        CompletionKind::Field => "field",
        CompletionKind::EnumValue => "enum",
        CompletionKind::Reference => "reference",
        CompletionKind::Snippet => "snippet",
    }
}

fn severity_color(severity: &str) -> gpui::Hsla {
    match severity {
        "error" => rgb(0xb42318).into(),
        "warning" => rgb(0xa16207).into(),
        _ => rgb(0x2563eb).into(),
    }
}

/// Compute the character range of a selection on a given line.
/// Returns `None` when the selection does not overlap `line_index`,
/// or when the overlap is empty (e.g. collapsed selection on empty line).
fn selection_on_line(line_index: u32, line_length: u32, range: TextRange) -> Option<(u32, u32)> {
    text_range_on_line(line_index, line_length, range)
}

fn text_range_on_line(line_index: u32, line_length: u32, range: TextRange) -> Option<(u32, u32)> {
    let start = range.start.min(range.end);
    let end = range.start.max(range.end);

    if line_index < start.line || line_index > end.line {
        return None;
    }

    let sel_start = if line_index == start.line {
        start.character
    } else {
        0
    };
    let sel_end = if line_index == end.line {
        end.character
    } else {
        line_length
    };

    if sel_start >= sel_end {
        return None;
    }

    Some((sel_start, sel_end))
}
