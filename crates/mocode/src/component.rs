use std::{
    cell::RefCell,
    fs, io,
    ops::Range,
    path::{Path, PathBuf},
    rc::Rc,
};

use mocode_api::{
    CompletionKind, DiagnosticSeverity, EditorError, MocodeEditor, ProxyChainPreview,
    ProxyChainStatus, TextEdit, TextPosition, TextRange,
};

use gpui::{
    App, Bounds, ClipboardItem, Context, ElementInputHandler, EntityInputHandler, FocusHandle,
    IntoElement, KeyBinding, ListHorizontalSizingBehavior, MouseButton, MouseDownEvent,
    MouseMoveEvent, MouseUpEvent, Pixels, Point, ScrollStrategy, UniformListScrollHandle, Window,
    actions, canvas, div, point, prelude::*, px, rgb, uniform_list,
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
        Left,
        Right,
        SelectLeft,
        SelectRight,
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
    pub(crate) kind: String,
    pub(crate) documentation: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GpuiEditorCompletionPopup {
    pub(crate) anchor_line: u32,
    pub(crate) anchor_column: u32,
    pub(crate) items: Vec<GpuiEditorCompletion>,
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
    editor: MocodeEditor,
    pub(crate) cursor: TextPosition,
    pub(crate) line_count: usize,
    pub(crate) current_yaml_path: String,
    pub(crate) diagnostics: Vec<GpuiEditorDiagnostic>,
    pub(crate) completion_labels: Vec<String>,
    pub(crate) completion_items: Vec<GpuiEditorCompletion>,
    pub(crate) completion_popup: Option<GpuiEditorCompletionPopup>,
    pub(crate) hover_title: String,
    pub(crate) hover_body: String,
    pub(crate) chain_preview: Option<ProxyChainPreview>,
    selection_anchor: Option<TextPosition>,
    pub(crate) selection_summary: String,
    pub(crate) search_active: bool,
    pub(crate) search_query: String,
    pub(crate) search_summary: String,
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
            editor,
            cursor: inspect_position,
            line_count: 0,
            current_yaml_path: String::new(),
            diagnostics: Vec::new(),
            completion_labels: Vec::new(),
            completion_items: Vec::new(),
            completion_popup: None,
            hover_title: String::new(),
            hover_body: String::new(),
            chain_preview: None,
            selection_anchor: None,
            selection_summary: String::new(),
            search_active: false,
            search_query: String::new(),
            search_summary: "<none>".to_string(),
            ime_marked_range: None,
        };
        document.refresh_derived();
        document
    }

    pub(crate) fn insert_text(&mut self, text: &str) -> Result<(), EditorError> {
        self.replace_utf16_range(None, text)
    }

    pub(crate) fn commit_text(&mut self, text: &str) -> Result<(), EditorError> {
        self.replace_utf16_range(None, text)
    }

    pub(crate) fn insert_tab(&mut self) -> Result<(), EditorError> {
        self.commit_text(TAB_WIDTH)
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
            })
            .collect()
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

        if let Err(error) = fs::write(path, self.text()) {
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

    fn mark_dirty(&mut self) {
        self.dirty = true;
        self.save_status = "Modified".to_string();
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
                kind: completion_kind_label(completion.kind).to_string(),
                documentation: completion.documentation,
            })
            .collect();
        self.completion_labels = self
            .completion_items
            .iter()
            .map(|completion| completion.label.clone())
            .collect();
        self.completion_popup = build_completion_popup(self.cursor, &self.completion_items);
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

    fn insert_text(&mut self, text: &str) -> Result<(), EditorError> {
        let result = self.document.insert_text(text);
        self.reveal_if_ok(result)
    }

    fn insert_tab(&mut self) -> Result<(), EditorError> {
        let result = self.document.insert_tab();
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
        self.document = GpuiEditorDocument::from_path(path)?;
        self.reveal_cursor();
        Ok(())
    }

    fn start_search_from_selection(&mut self) {
        self.document.start_search_from_selection();
    }

    fn search_backspace(&mut self) {
        self.document.search_backspace();
    }

    fn close_search(&mut self) {
        self.document.close_search();
    }

    fn search_active(&self) -> bool {
        self.document.search_active
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
        KeyBinding::new("left", Left, Some("MocodeEditor")),
        KeyBinding::new("right", Right, Some("MocodeEditor")),
        KeyBinding::new("shift-left", SelectLeft, Some("MocodeEditor")),
        KeyBinding::new("shift-right", SelectRight, Some("MocodeEditor")),
        KeyBinding::new("up", Up, Some("MocodeEditor")),
        KeyBinding::new("down", Down, Some("MocodeEditor")),
        KeyBinding::new("shift-up", SelectUp, Some("MocodeEditor")),
        KeyBinding::new("shift-down", SelectDown, Some("MocodeEditor")),
        KeyBinding::new("home", Home, Some("MocodeEditor")),
        KeyBinding::new("end", End, Some("MocodeEditor")),
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
        KeyBinding::new("cmd-g", FindNext, Some("MocodeEditor")),
        KeyBinding::new("ctrl-g", FindNext, Some("MocodeEditor")),
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
    let line_count = editor.document().line_count;
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
                if this.editor_component().search_active() {
                    this.editor_component_mut().search_backspace();
                    cx.notify();
                } else if this.editor_component_mut().backspace().is_ok() {
                    cx.notify();
                }
            }),
        )
        .on_action(cx.listener(|this: &mut T, _: &Delete, _: &mut Window, cx| {
            if this.editor_component_mut().delete().is_ok() {
                cx.notify();
            }
        }))
        .on_action(cx.listener(|this: &mut T, _: &Tab, _: &mut Window, cx| {
            if this.editor_component_mut().insert_tab().is_ok() {
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
                this.editor_component_mut().close_search();
                cx.notify();
            }),
        )
        .on_action(cx.listener(|this: &mut T, _: &Enter, _: &mut Window, cx| {
            if this.editor_component().search_active() {
                this.editor_component_mut().find_next();
                cx.notify();
            } else if this.editor_component_mut().insert_text("\n").is_ok() {
                cx.notify();
            }
        }))
        .on_action(cx.listener(|this: &mut T, _: &Up, _: &mut Window, cx| {
            if this.editor_component_mut().move_up().is_ok() {
                cx.notify();
            }
        }))
        .on_action(cx.listener(|this: &mut T, _: &Down, _: &mut Window, cx| {
            if this.editor_component_mut().move_down().is_ok() {
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
                && this.editor_component_mut().insert_text(&text).is_ok()
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
                        let mut rows = Vec::new();
                        for (offset, line) in slice.into_iter().enumerate() {
                            let index = range.start + offset;
                            let index_u32 = index as u32;
                            let cursor = (document.cursor.line as usize == index)
                                .then_some(document.cursor.character);
                            let line_selection = selection_range.and_then(|r| {
                                selection_on_line(index_u32, line.text.chars().count() as u32, r)
                            });
                            rows.push(line_row(
                                index,
                                line.number,
                                line.text,
                                line.diagnostic_count,
                                line.diagnostic_severity,
                                cursor,
                                line_selection,
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
}

fn status_bar(document: &GpuiEditorDocument) -> impl IntoElement {
    let cursor = format!(
        "Ln {}, Col {}",
        document.cursor.line + 1,
        document.cursor.character + 1
    );
    let selection = (document.selection_summary != "<none>")
        .then(|| format!("Sel {}", document.selection_summary));
    let search = document
        .search_active
        .then(|| format!("Find {}", document.search_summary));

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
        .when_some(search, |this, search| this.child(status_item(search)))
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
    cursor: Option<u32>,
    selection: Option<(u32, u32)>,
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
        .child(render_line_text(text, cursor, selection))
}

fn render_line_text(
    text: String,
    cursor: Option<u32>,
    selection: Option<(u32, u32)>,
) -> impl IntoElement {
    if let Some((sel_start, sel_end)) = selection {
        let sel_start_byte = char_to_byte_index(&text, sel_start);
        let sel_end_byte = char_to_byte_index(&text, sel_end);

        let before = text[..sel_start_byte].to_string();
        let highlighted = text[sel_start_byte..sel_end_byte].to_string();
        let after = text[sel_end_byte..].to_string();

        let cursor_at_start = cursor == Some(sel_start);
        let cursor_at_end = cursor == Some(sel_end);

        div()
            .px_3()
            .flex()
            .flex_row()
            .items_center()
            .text_color(rgb(0x0f172a))
            .whitespace_nowrap()
            .child(before)
            .when(cursor_at_start, |this| {
                this.child(div().w(px(1.0)).h(px(16.0)).bg(rgb(0x2563eb)))
            })
            .child(div().bg(rgb(0xdbeafe)).child(highlighted))
            .when(cursor_at_end, |this| {
                this.child(div().w(px(1.0)).h(px(16.0)).bg(rgb(0x2563eb)))
            })
            .child(after)
    } else {
        let (before_cursor, after_cursor) = cursor
            .map(|c| split_at_character(&text, c))
            .unwrap_or_else(|| (text, String::new()));

        div()
            .px_3()
            .flex()
            .flex_row()
            .items_center()
            .text_color(rgb(0x0f172a))
            .whitespace_nowrap()
            .child(before_cursor)
            .when(cursor.is_some(), |this| {
                this.child(div().w(px(1.0)).h(px(16.0)).bg(rgb(0x2563eb)))
            })
            .child(after_cursor)
    }
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
) -> Option<GpuiEditorCompletionPopup> {
    (!completion_items.is_empty()).then(|| GpuiEditorCompletionPopup {
        anchor_line: cursor.line + 1,
        anchor_column: cursor.character + 1,
        items: completion_items.iter().take(6).cloned().collect(),
    })
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

fn split_at_character(text: &str, character: u32) -> (String, String) {
    let split_at = text
        .char_indices()
        .nth(character as usize)
        .map(|(index, _)| index)
        .unwrap_or(text.len());
    (text[..split_at].to_string(), text[split_at..].to_string())
}
