use mocode_api::{
    CompletionKind, DiagnosticSeverity, EditorError, MocodeEditor, TextPosition, TextRange,
};

const SAMPLE_TITLE: &str = "examples/configs/dialer-proxy.yaml";
const SAMPLE_TEXT: &str = include_str!("../../../examples/configs/dialer-proxy.yaml");
const INSPECT_POSITION: TextPosition = TextPosition::new(10, 17);
const DEFAULT_FIXTURE_ID: &str = "dialer-proxy";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DemoFixture {
    id: &'static str,
    label: &'static str,
    title: &'static str,
    text: &'static str,
    inspect_position: TextPosition,
}

const DEMO_FIXTURES: &[DemoFixture] = &[
    DemoFixture {
        id: "dialer-proxy",
        label: "Dialer",
        title: SAMPLE_TITLE,
        text: SAMPLE_TEXT,
        inspect_position: INSPECT_POSITION,
    },
    DemoFixture {
        id: "minimal",
        label: "Minimal",
        title: "examples/configs/minimal.yaml",
        text: include_str!("../../../examples/configs/minimal.yaml"),
        inspect_position: TextPosition::new(0, 0),
    },
    DemoFixture {
        id: "dns",
        label: "DNS",
        title: "examples/configs/dns.yaml",
        text: include_str!("../../../examples/configs/dns.yaml"),
        inspect_position: TextPosition::new(2, 16),
    },
    DemoFixture {
        id: "tun",
        label: "TUN",
        title: "examples/configs/tun.yaml",
        text: include_str!("../../../examples/configs/tun.yaml"),
        inspect_position: TextPosition::new(2, 4),
    },
    DemoFixture {
        id: "proxy-groups",
        label: "Groups",
        title: "examples/configs/proxy-groups.yaml",
        text: include_str!("../../../examples/configs/proxy-groups.yaml"),
        inspect_position: TextPosition::new(7, 8),
    },
    DemoFixture {
        id: "providers",
        label: "Providers",
        title: "examples/configs/providers.yaml",
        text: include_str!("../../../examples/configs/providers.yaml"),
        inspect_position: TextPosition::new(0, 0),
    },
    DemoFixture {
        id: "invalid-yaml",
        label: "Bad YAML",
        title: "examples/configs/invalid-yaml.yaml",
        text: include_str!("../../../examples/configs/invalid-yaml.yaml"),
        inspect_position: TextPosition::new(2, 0),
    },
    DemoFixture {
        id: "invalid-reference",
        label: "Bad Ref",
        title: "examples/configs/invalid-reference.yaml",
        text: include_str!("../../../examples/configs/invalid-reference.yaml"),
        inspect_position: TextPosition::new(0, 0),
    },
    DemoFixture {
        id: "dialer-cycle",
        label: "Cycle",
        title: "tests/fixtures/dialer-cycle.yaml",
        text: include_str!("../../../tests/fixtures/dialer-cycle.yaml"),
        inspect_position: TextPosition::new(0, 0),
    },
    DemoFixture {
        id: "large",
        label: "Large",
        title: "examples/configs/large.yaml",
        text: include_str!("../../../examples/configs/large.yaml"),
        inspect_position: TextPosition::new(0, 0),
    },
    DemoFixture {
        id: "large-20000",
        label: "20k",
        title: "examples/configs/large-20000.yaml",
        text: include_str!("../../../examples/configs/large-20000.yaml"),
        inspect_position: TextPosition::new(0, 0),
    },
];

#[derive(Debug, Clone, PartialEq, Eq)]
struct DemoLine {
    number: u32,
    text: String,
    diagnostic_count: usize,
    diagnostic_severity: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DemoDiagnostic {
    severity: String,
    code: String,
    message: String,
    line: Option<u32>,
    column: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DemoCompletion {
    label: String,
    kind: String,
    documentation: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DemoCompletionPopup {
    anchor_line: u32,
    anchor_column: u32,
    items: Vec<DemoCompletion>,
}

#[derive(Debug, Clone)]
struct DemoDocument {
    title: String,
    editor: MocodeEditor,
    cursor: TextPosition,
    line_count: usize,
    lines: Vec<DemoLine>,
    current_yaml_path: String,
    diagnostics: Vec<DemoDiagnostic>,
    completion_labels: Vec<String>,
    completion_items: Vec<DemoCompletion>,
    completion_popup: Option<DemoCompletionPopup>,
    hover_title: String,
    hover_body: String,
    selection_anchor: Option<TextPosition>,
    selection_summary: String,
}

impl DemoDocument {
    fn from_text(title: impl Into<String>, text: &str, inspect_position: TextPosition) -> Self {
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

    fn insert_text(&mut self, text: &str) -> Result<(), EditorError> {
        self.cursor = self.editor.insert_text_at(self.cursor, text)?;
        self.clear_selection();
        self.refresh_derived();
        Ok(())
    }

    fn backspace(&mut self) -> Result<(), EditorError> {
        self.cursor = self.editor.backspace_at(self.cursor)?;
        self.clear_selection();
        self.refresh_derived();
        Ok(())
    }

    fn delete(&mut self) -> Result<(), EditorError> {
        self.cursor = self.editor.delete_at(self.cursor)?;
        self.clear_selection();
        self.refresh_derived();
        Ok(())
    }

    fn move_left(&mut self) -> Result<(), EditorError> {
        self.cursor = self.editor.move_left(self.cursor)?;
        self.clear_selection();
        self.refresh_derived();
        Ok(())
    }

    fn move_right(&mut self) -> Result<(), EditorError> {
        self.cursor = self.editor.move_right(self.cursor)?;
        self.clear_selection();
        self.refresh_derived();
        Ok(())
    }

    fn select_left(&mut self) -> Result<(), EditorError> {
        self.ensure_selection_anchor();
        self.cursor = self.editor.move_left(self.cursor)?;
        self.refresh_derived();
        Ok(())
    }

    fn select_right(&mut self) -> Result<(), EditorError> {
        self.ensure_selection_anchor();
        self.cursor = self.editor.move_right(self.cursor)?;
        self.refresh_derived();
        Ok(())
    }

    fn selected_text(&self) -> Option<String> {
        let range = self.selected_range()?;
        self.editor.text_in_range(range).ok()
    }

    fn copy_selection_text(&self) -> Option<String> {
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
            .map(|completion| DemoCompletion {
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
            .map(|diagnostic| DemoDiagnostic {
                severity: severity_label(diagnostic.severity).to_string(),
                code: diagnostic.code,
                message: diagnostic.message,
                line: diagnostic.range.map(|range| range.start.line + 1),
                column: diagnostic.range.map(|range| range.start.character + 1),
            })
            .collect();
        self.lines = semantic_lines
            .into_iter()
            .map(|line| DemoLine {
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

fn build_completion_popup(
    cursor: TextPosition,
    completion_items: &[DemoCompletion],
) -> Option<DemoCompletionPopup> {
    (!completion_items.is_empty()).then(|| DemoCompletionPopup {
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

fn load_demo_document() -> DemoDocument {
    load_fixture_by_id(DEFAULT_FIXTURE_ID).expect("default fixture must exist")
}

fn load_fixture_by_id(id: &str) -> Option<DemoDocument> {
    DEMO_FIXTURES
        .iter()
        .find(|fixture| fixture.id == id)
        .map(load_fixture)
}

fn load_fixture(fixture: &DemoFixture) -> DemoDocument {
    DemoDocument::from_text(fixture.title, fixture.text, fixture.inspect_position)
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

mod gpui_app {
    use super::{
        DEMO_FIXTURES, DemoCompletion, DemoDiagnostic, DemoDocument, DemoFixture,
        load_demo_document, load_fixture_by_id,
    };
    use gpui::{
        App, Application, Bounds, ClipboardItem, Context, FocusHandle, Focusable, IntoElement,
        KeyBinding, KeyDownEvent, MouseButton, MouseDownEvent, Window, WindowBounds, WindowOptions,
        actions, div, prelude::*, px, rgb, size, uniform_list,
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

    pub fn run() {
        Application::new().run(|cx: &mut App| {
            let bounds = Bounds::centered(None, size(px(1120.0), px(720.0)), cx);
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
            cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    ..Default::default()
                },
                |window, cx| {
                    let focus_handle = cx.focus_handle().tab_stop(true);
                    focus_handle.focus(window);
                    cx.new(|_| MocodeGpuiDemo {
                        document: load_demo_document(),
                        focus_handle,
                    })
                },
            )
            .unwrap();
            cx.activate(true);
        });
    }

    struct MocodeGpuiDemo {
        document: DemoDocument,
        focus_handle: FocusHandle,
    }

    impl MocodeGpuiDemo {
        fn backspace(&mut self, _: &Backspace, _: &mut Window, cx: &mut Context<Self>) {
            if self.document.backspace().is_ok() {
                cx.notify();
            }
        }

        fn delete(&mut self, _: &Delete, _: &mut Window, cx: &mut Context<Self>) {
            if self.document.delete().is_ok() {
                cx.notify();
            }
        }

        fn left(&mut self, _: &Left, _: &mut Window, cx: &mut Context<Self>) {
            if self.document.move_left().is_ok() {
                cx.notify();
            }
        }

        fn right(&mut self, _: &Right, _: &mut Window, cx: &mut Context<Self>) {
            if self.document.move_right().is_ok() {
                cx.notify();
            }
        }

        fn select_left(&mut self, _: &SelectLeft, _: &mut Window, cx: &mut Context<Self>) {
            if self.document.select_left().is_ok() {
                cx.notify();
            }
        }

        fn select_right(&mut self, _: &SelectRight, _: &mut Window, cx: &mut Context<Self>) {
            if self.document.select_right().is_ok() {
                cx.notify();
            }
        }

        fn copy(&mut self, _: &Copy, _: &mut Window, cx: &mut Context<Self>) {
            if let Some(text) = self.document.copy_selection_text() {
                cx.write_to_clipboard(ClipboardItem::new_string(text));
            }
        }

        fn paste(&mut self, _: &Paste, _: &mut Window, cx: &mut Context<Self>) {
            if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text())
                && self.document.insert_text(&text).is_ok()
            {
                cx.notify();
            }
        }

        fn on_key_down(&mut self, event: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
            let modifiers = &event.keystroke.modifiers;
            if modifiers.control || modifiers.alt || modifiers.platform || modifiers.function {
                return;
            }

            let Some(text) = event.keystroke.key_char.as_deref() else {
                return;
            };

            if is_insertable_text(text) && self.document.insert_text(text).is_ok() {
                cx.stop_propagation();
                cx.notify();
            }
        }

        fn focus_editor(&mut self, _: &MouseDownEvent, window: &mut Window, _: &mut Context<Self>) {
            self.focus_handle.focus(window);
        }

        fn select_fixture(
            &mut self,
            id: &'static str,
            window: &mut Window,
            cx: &mut Context<Self>,
        ) {
            if let Some(document) = load_fixture_by_id(id) {
                self.document = document;
                self.focus_handle.focus(window);
                cx.notify();
            }
        }
    }

    impl Focusable for MocodeGpuiDemo {
        fn focus_handle(&self, _: &App) -> FocusHandle {
            self.focus_handle.clone()
        }
    }

    impl Render for MocodeGpuiDemo {
        fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
            div()
                .size_full()
                .bg(rgb(0xf7f9fc))
                .text_color(rgb(0x1f2937))
                .text_size(px(13.0))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .size_full()
                        .child(header(&self.document, cx))
                        .child(completion_panel(&self.document))
                        .child(completion_popup_panel(&self.document))
                        .child(
                            div()
                                .flex()
                                .flex_row()
                                .h_full()
                                .child(editor_surface(&self.document, &self.focus_handle, cx))
                                .child(inspector(&self.document)),
                        ),
                )
        }
    }

    fn header(document: &DemoDocument, cx: &mut Context<'_, MocodeGpuiDemo>) -> impl IntoElement {
        div()
            .flex()
            .flex_row()
            .justify_between()
            .items_center()
            .px_4()
            .py_3()
            .bg(rgb(0xffffff))
            .border_b_1()
            .border_color(rgb(0xd9e2ec))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child("mocode GPUI prototype")
                    .child(
                        div()
                            .text_color(rgb(0x5f6b7a))
                            .text_size(px(12.0))
                            .child(document.title.clone()),
                    ),
            )
            .child(fixture_selector(cx))
            .child(
                div()
                    .text_color(rgb(0x5f6b7a))
                    .child(format!("{} lines", document.line_count)),
            )
    }

    fn fixture_selector(cx: &mut Context<'_, MocodeGpuiDemo>) -> impl IntoElement {
        div()
            .flex()
            .flex_row()
            .gap_1()
            .child(fixture_button(&DEMO_FIXTURES[0], cx))
            .child(fixture_button(&DEMO_FIXTURES[1], cx))
            .child(fixture_button(&DEMO_FIXTURES[2], cx))
            .child(fixture_button(&DEMO_FIXTURES[3], cx))
            .child(fixture_button(&DEMO_FIXTURES[4], cx))
            .child(fixture_button(&DEMO_FIXTURES[5], cx))
            .child(fixture_button(&DEMO_FIXTURES[6], cx))
            .child(fixture_button(&DEMO_FIXTURES[7], cx))
            .child(fixture_button(&DEMO_FIXTURES[8], cx))
            .child(fixture_button(&DEMO_FIXTURES[9], cx))
            .child(fixture_button(&DEMO_FIXTURES[10], cx))
    }

    fn fixture_button(
        fixture: &'static DemoFixture,
        cx: &mut Context<'_, MocodeGpuiDemo>,
    ) -> impl IntoElement {
        let fixture_id = fixture.id;
        div()
            .px_2()
            .py_1()
            .bg(rgb(0xf8fafc))
            .border_1()
            .border_color(rgb(0xd9e2ec))
            .text_color(rgb(0x334155))
            .text_size(px(11.0))
            .child(fixture.label)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _: &MouseDownEvent, window: &mut Window, cx| {
                    this.select_fixture(fixture_id, window, cx);
                }),
            )
    }

    fn editor_surface(
        document: &DemoDocument,
        focus_handle: &FocusHandle,
        cx: &mut Context<'_, MocodeGpuiDemo>,
    ) -> impl IntoElement {
        let line_count = document.lines.len();
        div()
            .w(px(820.0))
            .h_full()
            .bg(rgb(0xffffff))
            .track_focus(focus_handle)
            .key_context("MocodeEditor")
            .on_action(cx.listener(MocodeGpuiDemo::backspace))
            .on_action(cx.listener(MocodeGpuiDemo::delete))
            .on_action(cx.listener(MocodeGpuiDemo::left))
            .on_action(cx.listener(MocodeGpuiDemo::right))
            .on_action(cx.listener(MocodeGpuiDemo::select_left))
            .on_action(cx.listener(MocodeGpuiDemo::select_right))
            .on_action(cx.listener(MocodeGpuiDemo::paste))
            .on_action(cx.listener(MocodeGpuiDemo::copy))
            .on_key_down(cx.listener(MocodeGpuiDemo::on_key_down))
            .on_mouse_down(MouseButton::Left, cx.listener(MocodeGpuiDemo::focus_editor))
            .child(
                uniform_list(
                    "mocode-lines",
                    line_count,
                    cx.processor(|this, range, _window, _cx| {
                        let mut rows = Vec::new();
                        for index in range {
                            let line = &this.document.lines[index];
                            let cursor = (this.document.cursor.line as usize == index)
                                .then_some(this.document.cursor.character);
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

    fn inspector(document: &DemoDocument) -> impl IntoElement {
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

    fn completion_panel(document: &DemoDocument) -> impl IntoElement {
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

    fn completion_popup_panel(document: &DemoDocument) -> impl IntoElement {
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

    fn diagnostic_row(diagnostic: &DemoDiagnostic) -> impl IntoElement {
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

    fn completion_item(completion: &DemoCompletion) -> impl IntoElement {
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
}

fn main() {
    gpui_app::run();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_demo_document_from_core_snapshot() {
        let document = load_demo_document();

        assert_eq!(document.title, SAMPLE_TITLE);
        assert!(document.line_count > 10);
        assert_eq!(document.lines[0].number, 1);
        assert_eq!(document.lines[0].text, "mixed-port: 7890");
        assert_eq!(document.current_yaml_path, "proxies[0].dialer-proxy");
        assert!(document.completion_labels.contains(&"exit".to_string()));
    }

    #[test]
    fn carries_core_diagnostics_without_reimplementing_lints() {
        let document = DemoDocument::from_text(
            "invalid-reference.yaml",
            include_str!("../../../examples/configs/invalid-reference.yaml"),
            TextPosition::new(10, 18),
        );

        assert!(document.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "mihomo.reference.missing"
                && diagnostic.message.contains("missing-dialer")
        }));
    }

    #[test]
    fn marks_lines_with_ranged_diagnostics_from_core() {
        let document = DemoDocument::from_text(
            "invalid-yaml.yaml",
            include_str!("../../../examples/configs/invalid-yaml.yaml"),
            TextPosition::new(2, 0),
        );

        assert!(document.lines.iter().any(|line| line.diagnostic_count > 0));
        assert!(document.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "yaml.syntax"
                && diagnostic.line.is_some()
                && diagnostic.column.is_some()
        }));
    }

    #[test]
    fn carries_hover_summary_for_current_position() {
        let document = DemoDocument::from_text(
            "tun.yaml",
            "tun:\n  stack: system\n",
            TextPosition::new(1, 4),
        );

        assert_eq!(document.hover_title, "tun.stack");
        assert!(document.hover_body.contains("TUN network stack"));
    }

    #[test]
    fn carries_completion_item_details_for_panel() {
        let document = DemoDocument::from_text(
            "dns.yaml",
            "dns:\n  enhanced-mode: \n",
            TextPosition::new(1, 17),
        );

        assert!(document.completion_items.iter().any(|item| {
            item.label == "fake-ip"
                && item.kind == "enum"
                && item
                    .documentation
                    .as_deref()
                    .is_some_and(|text| !text.is_empty())
        }));
    }

    #[test]
    fn completion_popup_tracks_cursor_anchor_and_items() {
        let document = DemoDocument::from_text(
            "dialer.yaml",
            "proxies:\n  - name: entry\n    type: ss\n    dialer-proxy: \n  - name: exit\n    type: ss\n",
            TextPosition::new(3, 18),
        );

        let popup = document.completion_popup.as_ref().unwrap();
        assert_eq!(popup.anchor_line, 4);
        assert_eq!(popup.anchor_column, 19);
        assert!(
            popup
                .items
                .iter()
                .any(|item| item.label == "exit" && item.kind == "reference")
        );
    }

    #[test]
    fn loads_large_fixture_for_scroll_baseline() {
        let text = include_str!("../../../examples/configs/large.yaml");
        let document = DemoDocument::from_text("large.yaml", text, TextPosition::new(0, 0));

        assert!(text.lines().count() >= 5_000);
        assert!(document.line_count >= 5_000);
        assert_eq!(document.lines[0].text, "mixed-port: 7890");
        assert!(
            document
                .completion_labels
                .contains(&"mixed-port".to_string())
        );
    }

    #[test]
    fn loads_twenty_thousand_line_fixture_for_validation_baseline() {
        let text = include_str!("../../../examples/configs/large-20000.yaml");
        let document = DemoDocument::from_text("large-20000.yaml", text, TextPosition::new(0, 0));

        assert!(text.lines().count() >= 20_000);
        assert!(document.line_count >= 20_000);
        assert_eq!(document.lines[0].text, "mixed-port: 7890");
        assert!(document.diagnostics.is_empty());
    }

    #[test]
    fn fixture_selector_loads_large_and_diagnostic_samples() {
        let large = load_fixture_by_id("large-20000").unwrap();
        assert_eq!(large.title, "examples/configs/large-20000.yaml");
        assert!(large.line_count >= 20_000);
        assert!(large.diagnostics.is_empty());

        let invalid_yaml = load_fixture_by_id("invalid-yaml").unwrap();
        assert_eq!(invalid_yaml.title, "examples/configs/invalid-yaml.yaml");
        assert!(
            invalid_yaml
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "yaml.syntax")
        );
    }

    #[test]
    fn edits_document_through_shared_core() {
        let mut document = DemoDocument::from_text(
            "scratch.yaml",
            "dns:\n  enhanced-mode: \n",
            TextPosition::new(1, 17),
        );

        document.insert_text("fake-ip").unwrap();

        assert_eq!(document.cursor, TextPosition::new(1, 24));
        assert_eq!(document.lines[1].text, "  enhanced-mode: fake-ip");
        assert_eq!(document.current_yaml_path, "dns.enhanced-mode");
        assert!(document.completion_labels.contains(&"fake-ip".to_string()));
    }

    #[test]
    fn backspaces_deletes_and_moves_cursor_in_demo_state() {
        let mut document = DemoDocument::from_text(
            "scratch.yaml",
            "dns:\n  enable: true\n",
            TextPosition::new(1, 2),
        );

        document.backspace().unwrap();
        assert_eq!(document.cursor, TextPosition::new(1, 1));
        assert_eq!(document.lines[1].text, " enable: true");

        document.move_left().unwrap();
        assert_eq!(document.cursor, TextPosition::new(1, 0));

        document.move_right().unwrap();
        assert_eq!(document.cursor, TextPosition::new(1, 1));

        document.delete().unwrap();
        assert_eq!(document.cursor, TextPosition::new(1, 1));
        assert_eq!(document.lines[1].text, " nable: true");
    }

    #[test]
    fn selection_copy_uses_shared_core_range() {
        let mut document = DemoDocument::from_text(
            "scratch.yaml",
            "dns:\n  enable: true\n",
            TextPosition::new(1, 2),
        );

        for _ in 0..6 {
            document.select_right().unwrap();
        }

        assert_eq!(document.cursor, TextPosition::new(1, 8));
        assert_eq!(document.selected_text().unwrap(), "enable");
        assert_eq!(document.copy_selection_text().unwrap(), "enable");
        assert_eq!(document.selection_summary, "2:3 -> 2:9");

        document.move_right().unwrap();

        assert!(document.selected_text().is_none());
        assert_eq!(document.selection_summary, "<none>");
    }
}
