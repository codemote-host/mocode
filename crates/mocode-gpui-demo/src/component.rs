use mocode_api::{
    CompletionKind, DiagnosticSeverity, EditorError, MocodeEditor, TextPosition, TextRange,
};

use crate::fixtures::DemoFixture;
use gpui::{
    App, ClipboardItem, Context, FocusHandle, IntoElement, KeyBinding, KeyDownEvent, MouseButton,
    MouseDownEvent, Window, actions, div, prelude::*, px, rgb, uniform_list,
};

actions!(
    mocode_editor,
    [
        Backspace,
        Delete,
        Left,
        Right,
        SelectLeft,
        SelectRight,
        Paste,
        Copy
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

#[derive(Debug, Clone)]
pub(crate) struct GpuiEditorDocument {
    pub(crate) title: String,
    editor: MocodeEditor,
    pub(crate) cursor: TextPosition,
    pub(crate) line_count: usize,
    pub(crate) lines: Vec<GpuiEditorLine>,
    pub(crate) current_yaml_path: String,
    pub(crate) diagnostics: Vec<GpuiEditorDiagnostic>,
    pub(crate) completion_labels: Vec<String>,
    pub(crate) completion_items: Vec<GpuiEditorCompletion>,
    pub(crate) completion_popup: Option<GpuiEditorCompletionPopup>,
    pub(crate) hover_title: String,
    pub(crate) hover_body: String,
    selection_anchor: Option<TextPosition>,
    pub(crate) selection_summary: String,
}

impl GpuiEditorDocument {
    pub(crate) fn from_text(
        title: impl Into<String>,
        text: &str,
        inspect_position: TextPosition,
    ) -> Self {
        let editor = MocodeEditor::open_text(text);
        let mut document = Self {
            title: title.into(),
            editor,
            cursor: inspect_position,
            line_count: 0,
            lines: Vec::new(),
            current_yaml_path: String::new(),
            diagnostics: Vec::new(),
            completion_labels: Vec::new(),
            completion_items: Vec::new(),
            completion_popup: None,
            hover_title: String::new(),
            hover_body: String::new(),
            selection_anchor: None,
            selection_summary: String::new(),
        };
        document.refresh_derived();
        document
    }

    pub(crate) fn from_fixture(fixture: &DemoFixture) -> Self {
        Self::from_text(fixture.title, fixture.text, fixture.inspect_position)
    }

    pub(crate) fn insert_text(&mut self, text: &str) -> Result<(), EditorError> {
        self.cursor = self.editor.insert_text_at(self.cursor, text)?;
        self.clear_selection();
        self.refresh_derived();
        Ok(())
    }

    pub(crate) fn backspace(&mut self) -> Result<(), EditorError> {
        self.cursor = self.editor.backspace_at(self.cursor)?;
        self.clear_selection();
        self.refresh_derived();
        Ok(())
    }

    pub(crate) fn delete(&mut self) -> Result<(), EditorError> {
        self.cursor = self.editor.delete_at(self.cursor)?;
        self.clear_selection();
        self.refresh_derived();
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

    pub(crate) fn selected_text(&self) -> Option<String> {
        let range = self.selected_range()?;
        self.editor.text_in_range(range).ok()
    }

    pub(crate) fn copy_selection_text(&self) -> Option<String> {
        self.selected_text()
    }

    fn refresh_derived(&mut self) {
        let snapshot = self.editor.snapshot();
        let semantic_lines = self.editor.semantic_lines();
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
        self.diagnostics = snapshot
            .diagnostics
            .into_iter()
            .map(|diagnostic| GpuiEditorDiagnostic {
                severity: severity_label(diagnostic.severity).to_string(),
                code: diagnostic.code,
                message: diagnostic.message,
                line: diagnostic.range.map(|range| range.start.line + 1),
                column: diagnostic.range.map(|range| range.start.character + 1),
            })
            .collect();
        self.lines = semantic_lines
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
            .collect();
        self.line_count = self.lines.len();
        self.selection_summary = self
            .selected_range()
            .map(format_selection_range)
            .unwrap_or_else(|| "<none>".to_string());
    }

    fn ensure_selection_anchor(&mut self) {
        if self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.cursor);
        }
    }

    fn clear_selection(&mut self) {
        self.selection_anchor = None;
    }

    fn selected_range(&self) -> Option<TextRange> {
        let anchor = self.selection_anchor?;
        (anchor != self.cursor).then(|| TextRange::new(anchor, self.cursor))
    }
}

pub(crate) struct GpuiEditorComponent {
    document: GpuiEditorDocument,
    focus_handle: FocusHandle,
}

impl GpuiEditorComponent {
    pub(crate) fn new(document: GpuiEditorDocument, focus_handle: FocusHandle) -> Self {
        Self {
            document,
            focus_handle,
        }
    }

    pub(crate) fn document(&self) -> &GpuiEditorDocument {
        &self.document
    }

    pub(crate) fn replace_document(&mut self, document: GpuiEditorDocument) {
        self.document = document;
    }

    pub(crate) fn focus_handle(&self) -> &FocusHandle {
        &self.focus_handle
    }

    pub(crate) fn focus(&self, window: &mut Window) {
        self.focus_handle.focus(window);
    }

    fn insert_text(&mut self, text: &str) -> Result<(), EditorError> {
        self.document.insert_text(text)
    }

    fn backspace(&mut self) -> Result<(), EditorError> {
        self.document.backspace()
    }

    fn delete(&mut self) -> Result<(), EditorError> {
        self.document.delete()
    }

    fn move_left(&mut self) -> Result<(), EditorError> {
        self.document.move_left()
    }

    fn move_right(&mut self) -> Result<(), EditorError> {
        self.document.move_right()
    }

    fn select_left(&mut self) -> Result<(), EditorError> {
        self.document.select_left()
    }

    fn select_right(&mut self) -> Result<(), EditorError> {
        self.document.select_right()
    }

    fn copy_selection_text(&self) -> Option<String> {
        self.document.copy_selection_text()
    }
}

pub(crate) trait GpuiEditorHost {
    fn editor_component(&self) -> &GpuiEditorComponent;
    fn editor_component_mut(&mut self) -> &mut GpuiEditorComponent;
}

pub(crate) fn bind_editor_keys(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("backspace", Backspace, Some("MocodeEditor")),
        KeyBinding::new("delete", Delete, Some("MocodeEditor")),
        KeyBinding::new("left", Left, Some("MocodeEditor")),
        KeyBinding::new("right", Right, Some("MocodeEditor")),
        KeyBinding::new("shift-left", SelectLeft, Some("MocodeEditor")),
        KeyBinding::new("shift-right", SelectRight, Some("MocodeEditor")),
        KeyBinding::new("cmd-v", Paste, Some("MocodeEditor")),
        KeyBinding::new("ctrl-v", Paste, Some("MocodeEditor")),
        KeyBinding::new("cmd-c", Copy, Some("MocodeEditor")),
        KeyBinding::new("ctrl-c", Copy, Some("MocodeEditor")),
    ]);
}

pub(crate) fn render_editor_component<T>(
    editor: &GpuiEditorComponent,
    cx: &mut Context<'_, T>,
) -> impl IntoElement
where
    T: GpuiEditorHost + 'static,
{
    div()
        .flex()
        .flex_col()
        .h_full()
        .child(completion_panel(editor.document()))
        .child(completion_popup_panel(editor.document()))
        .child(
            div()
                .flex()
                .flex_row()
                .h_full()
                .child(editor_surface(editor, cx))
                .child(inspector(editor.document())),
        )
}

fn editor_surface<T>(editor: &GpuiEditorComponent, cx: &mut Context<'_, T>) -> impl IntoElement
where
    T: GpuiEditorHost + 'static,
{
    let line_count = editor.document().lines.len();
    div()
        .w(px(820.0))
        .h_full()
        .bg(rgb(0xffffff))
        .track_focus(editor.focus_handle())
        .key_context("MocodeEditor")
        .on_action(
            cx.listener(|this: &mut T, _: &Backspace, _: &mut Window, cx| {
                if this.editor_component_mut().backspace().is_ok() {
                    cx.notify();
                }
            }),
        )
        .on_action(cx.listener(|this: &mut T, _: &Delete, _: &mut Window, cx| {
            if this.editor_component_mut().delete().is_ok() {
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
        .on_key_down(
            cx.listener(|this: &mut T, event: &KeyDownEvent, _: &mut Window, cx| {
                let modifiers = &event.keystroke.modifiers;
                if modifiers.control || modifiers.alt || modifiers.platform || modifiers.function {
                    return;
                }

                let Some(text) = event.keystroke.key_char.as_deref() else {
                    return;
                };

                if is_insertable_text(text) && this.editor_component_mut().insert_text(text).is_ok()
                {
                    cx.stop_propagation();
                    cx.notify();
                }
            }),
        )
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(
                |this: &mut T, _: &MouseDownEvent, window: &mut Window, _: &mut Context<T>| {
                    this.editor_component().focus(window);
                },
            ),
        )
        .child(
            uniform_list(
                "mocode-lines",
                line_count,
                cx.processor(|this: &mut T, range, _window, _cx| {
                    let document = this.editor_component().document();
                    let mut rows = Vec::new();
                    for index in range {
                        let line = &document.lines[index];
                        let cursor = (document.cursor.line as usize == index)
                            .then_some(document.cursor.character);
                        rows.push(line_row(
                            index,
                            line.number,
                            line.text.clone(),
                            line.diagnostic_count,
                            line.diagnostic_severity.clone(),
                            cursor,
                        ));
                    }
                    rows
                }),
            )
            .h_full(),
        )
}

fn completion_panel(document: &GpuiEditorDocument) -> impl IntoElement {
    div()
        .flex()
        .flex_row()
        .items_center()
        .gap_2()
        .px_4()
        .py_2()
        .bg(rgb(0xf8fafc))
        .border_b_1()
        .border_color(rgb(0xd9e2ec))
        .child(
            div()
                .w(px(88.0))
                .text_color(rgb(0x64748b))
                .text_size(px(11.0))
                .child("Completions"),
        )
        .when(document.completion_items.is_empty(), |this| {
            this.child(div().text_color(rgb(0x64748b)).child("<none>"))
        })
        .children(
            document
                .completion_items
                .iter()
                .take(6)
                .map(completion_item),
        )
}

fn completion_popup_panel(document: &GpuiEditorDocument) -> impl IntoElement {
    let anchor = document
        .completion_popup
        .as_ref()
        .map(|popup| format!("{}:{}", popup.anchor_line, popup.anchor_column))
        .unwrap_or_else(|| "<none>".to_string());

    let items = document
        .completion_popup
        .as_ref()
        .map(|popup| popup.items.iter().take(4).collect::<Vec<_>>())
        .unwrap_or_default();

    div()
        .flex()
        .flex_row()
        .items_center()
        .gap_2()
        .px_4()
        .py_2()
        .bg(rgb(0xffffff))
        .border_b_1()
        .border_color(rgb(0xd9e2ec))
        .child(
            div()
                .w(px(132.0))
                .text_color(rgb(0x64748b))
                .text_size(px(11.0))
                .child(format!("Popup @ {anchor}")),
        )
        .when(items.is_empty(), |this| {
            this.child(div().text_color(rgb(0x64748b)).child("<none>"))
        })
        .children(items.into_iter().map(completion_item))
}

fn inspector(document: &GpuiEditorDocument) -> impl IntoElement {
    div()
        .w(px(300.0))
        .h_full()
        .px_4()
        .py_4()
        .bg(rgb(0xf2f5f9))
        .border_l_1()
        .border_color(rgb(0xd9e2ec))
        .child(section("YAML path", document.current_yaml_path.clone()))
        .child(section(
            "Cursor",
            format!(
                "{}:{}",
                document.cursor.line + 1,
                document.cursor.character + 1
            ),
        ))
        .child(section("Selection", document.selection_summary.clone()))
        .child(section(
            "Completions",
            if document.completion_labels.is_empty() {
                "<none>".to_string()
            } else {
                document.completion_labels.join(", ")
            },
        ))
        .child(section(
            "Hover",
            if document.hover_body.is_empty() {
                document.hover_title.clone()
            } else {
                format!("{}\n{}", document.hover_title, document.hover_body)
            },
        ))
        .child(
            div()
                .flex()
                .flex_col()
                .gap_2()
                .mt_4()
                .child(label("Diagnostics"))
                .children(document.diagnostics.iter().map(diagnostic_row)),
        )
}

fn line_row(
    index: usize,
    number: u32,
    text: String,
    diagnostic_count: usize,
    diagnostic_severity: Option<String>,
    cursor: Option<u32>,
) -> impl IntoElement {
    let (before_cursor, after_cursor) = cursor
        .map(|character| split_at_character(&text, character))
        .unwrap_or_else(|| (text, String::new()));
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
        .child(
            div()
                .w(px(756.0))
                .px_3()
                .flex()
                .flex_row()
                .items_center()
                .text_color(rgb(0x0f172a))
                .whitespace_nowrap()
                .overflow_hidden()
                .text_ellipsis()
                .child(before_cursor)
                .when(cursor.is_some(), |this| {
                    this.child(div().w(px(1.0)).h(px(16.0)).bg(rgb(0x2563eb)))
                })
                .child(after_cursor),
        )
}

fn section(title: &'static str, value: String) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .mb_4()
        .child(label(title))
        .child(
            div()
                .text_color(rgb(0x1f2937))
                .line_height(px(18.0))
                .child(value),
        )
}

fn label(text: &'static str) -> impl IntoElement {
    div()
        .text_color(rgb(0x64748b))
        .text_size(px(11.0))
        .child(text)
}

fn diagnostic_row(diagnostic: &GpuiEditorDiagnostic) -> impl IntoElement {
    let location = match (diagnostic.line, diagnostic.column) {
        (Some(line), Some(column)) => format!(" at {line}:{column}"),
        _ => String::new(),
    };
    div()
        .flex()
        .flex_col()
        .gap_1()
        .p_2()
        .bg(rgb(0xffffff))
        .border_1()
        .border_color(rgb(0xd9e2ec))
        .child(
            div()
                .text_color(severity_color(&diagnostic.severity))
                .child(format!(
                    "{} {}{}",
                    diagnostic.severity, diagnostic.code, location
                )),
        )
        .child(
            div()
                .text_color(rgb(0x334155))
                .line_height(px(18.0))
                .child(diagnostic.message.clone()),
        )
}

fn completion_item(completion: &GpuiEditorCompletion) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .px_2()
        .py_1()
        .bg(rgb(0xffffff))
        .border_1()
        .border_color(rgb(0xd9e2ec))
        .child(
            div()
                .text_color(rgb(0x0f172a))
                .child(format!("{} {}", completion.kind, completion.label)),
        )
        .child(
            div()
                .max_w(px(180.0))
                .text_color(rgb(0x64748b))
                .text_size(px(11.0))
                .line_height(px(14.0))
                .whitespace_nowrap()
                .overflow_hidden()
                .text_ellipsis()
                .child(
                    completion
                        .documentation
                        .clone()
                        .unwrap_or_else(|| "<no docs>".to_string()),
                ),
        )
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

fn split_at_character(text: &str, character: u32) -> (String, String) {
    let split_at = text
        .char_indices()
        .nth(character as usize)
        .map(|(index, _)| index)
        .unwrap_or(text.len());
    (text[..split_at].to_string(), text[split_at..].to_string())
}

fn is_insertable_text(text: &str) -> bool {
    !text.is_empty()
        && text
            .chars()
            .all(|ch| ch == '\n' || ch == '\t' || !ch.is_control())
}
