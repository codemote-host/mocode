use mocode_api::{DiagnosticSeverity, EditorError, MocodeEditor, TextPosition};

const SAMPLE_TITLE: &str = "examples/configs/dialer-proxy.yaml";
const SAMPLE_TEXT: &str = include_str!("../../../examples/configs/dialer-proxy.yaml");
const INSPECT_POSITION: TextPosition = TextPosition::new(10, 17);

#[derive(Debug, Clone, PartialEq, Eq)]
struct DemoLine {
    number: u32,
    text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DemoDiagnostic {
    severity: String,
    code: String,
    message: String,
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
        };
        document.refresh_derived();
        document
    }

    fn insert_text(&mut self, text: &str) -> Result<(), EditorError> {
        self.cursor = self.editor.insert_text_at(self.cursor, text)?;
        self.refresh_derived();
        Ok(())
    }

    fn backspace(&mut self) -> Result<(), EditorError> {
        self.cursor = self.editor.backspace_at(self.cursor)?;
        self.refresh_derived();
        Ok(())
    }

    fn delete(&mut self) -> Result<(), EditorError> {
        self.cursor = self.editor.delete_at(self.cursor)?;
        self.refresh_derived();
        Ok(())
    }

    fn move_left(&mut self) -> Result<(), EditorError> {
        self.cursor = self.editor.move_left(self.cursor)?;
        self.refresh_derived();
        Ok(())
    }

    fn move_right(&mut self) -> Result<(), EditorError> {
        self.cursor = self.editor.move_right(self.cursor)?;
        self.refresh_derived();
        Ok(())
    }

    fn refresh_derived(&mut self) {
        let snapshot = self.editor.snapshot();
        self.current_yaml_path = self
            .editor
            .current_yaml_path(self.cursor)
            .map(|path| path.to_string())
            .unwrap_or_else(|| "<none>".to_string());
        self.completion_labels = self
            .editor
            .completions_at(self.cursor)
            .into_iter()
            .map(|completion| completion.label)
            .collect();
        self.diagnostics = snapshot
            .diagnostics
            .into_iter()
            .map(|diagnostic| DemoDiagnostic {
                severity: severity_label(diagnostic.severity).to_string(),
                code: diagnostic.code,
                message: diagnostic.message,
            })
            .collect();
        self.lines = snapshot
            .lines
            .into_iter()
            .map(|line| DemoLine {
                number: line.number,
                text: line.text,
            })
            .collect();
        self.line_count = self.lines.len();
    }
}

fn load_demo_document() -> DemoDocument {
    DemoDocument::from_text(SAMPLE_TITLE, SAMPLE_TEXT, INSPECT_POSITION)
}

fn severity_label(severity: DiagnosticSeverity) -> &'static str {
    match severity {
        DiagnosticSeverity::Error => "error",
        DiagnosticSeverity::Warning => "warning",
        DiagnosticSeverity::Info => "info",
        DiagnosticSeverity::Hint => "hint",
    }
}

mod gpui_app {
    use super::{DemoDiagnostic, DemoDocument, load_demo_document};
    use gpui::{
        App, Application, Bounds, Context, FocusHandle, Focusable, IntoElement, KeyBinding,
        KeyDownEvent, MouseButton, MouseDownEvent, Window, WindowBounds, WindowOptions, actions,
        div, prelude::*, px, rgb, size, uniform_list,
    };

    actions!(mocode_editor, [Backspace, Delete, Left, Right, Paste]);

    pub fn run() {
        Application::new().run(|cx: &mut App| {
            let bounds = Bounds::centered(None, size(px(1120.0), px(720.0)), cx);
            cx.bind_keys([
                KeyBinding::new("backspace", Backspace, Some("MocodeEditor")),
                KeyBinding::new("delete", Delete, Some("MocodeEditor")),
                KeyBinding::new("left", Left, Some("MocodeEditor")),
                KeyBinding::new("right", Right, Some("MocodeEditor")),
                KeyBinding::new("cmd-v", Paste, Some("MocodeEditor")),
                KeyBinding::new("ctrl-v", Paste, Some("MocodeEditor")),
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
                        .child(header(&self.document))
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

    fn header(document: &DemoDocument) -> impl IntoElement {
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
            .child(
                div()
                    .text_color(rgb(0x5f6b7a))
                    .child(format!("{} lines", document.line_count)),
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
            .on_action(cx.listener(MocodeGpuiDemo::paste))
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
                            rows.push(line_row(index, line.number, line.text.clone(), cursor));
                        }
                        rows
                    }),
                )
                .h_full(),
            )
    }

    fn line_row(index: usize, number: u32, text: String, cursor: Option<u32>) -> impl IntoElement {
        let (before_cursor, after_cursor) = cursor
            .map(|character| split_at_character(&text, character))
            .unwrap_or_else(|| (text, String::new()));

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
                    .px_2()
                    .text_color(rgb(0x94a3b8))
                    .bg(rgb(0xf8fafc))
                    .child(format!("{number:>4}")),
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
            .child(section(
                "Completions",
                if document.completion_labels.is_empty() {
                    "<none>".to_string()
                } else {
                    document.completion_labels.join(", ")
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
                    .child(format!("{} {}", diagnostic.severity, diagnostic.code)),
            )
            .child(
                div()
                    .text_color(rgb(0x334155))
                    .line_height(px(18.0))
                    .child(diagnostic.message.clone()),
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
}
