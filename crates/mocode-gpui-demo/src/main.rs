use mocode_api::{DiagnosticSeverity, MocodeEditor, TextPosition};

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

#[derive(Debug, Clone, PartialEq, Eq)]
struct DemoDocument {
    title: String,
    line_count: usize,
    lines: Vec<DemoLine>,
    current_yaml_path: String,
    diagnostics: Vec<DemoDiagnostic>,
    completion_labels: Vec<String>,
}

impl DemoDocument {
    fn from_text(title: impl Into<String>, text: &str, inspect_position: TextPosition) -> Self {
        let editor = MocodeEditor::open_text(text);
        let snapshot = editor.snapshot();
        let current_yaml_path = editor
            .current_yaml_path(inspect_position)
            .map(|path| path.to_string())
            .unwrap_or_else(|| "<none>".to_string());
        let completion_labels = editor
            .completions_at(inspect_position)
            .into_iter()
            .map(|completion| completion.label)
            .collect();
        let diagnostics = snapshot
            .diagnostics
            .into_iter()
            .map(|diagnostic| DemoDiagnostic {
                severity: severity_label(diagnostic.severity).to_string(),
                code: diagnostic.code,
                message: diagnostic.message,
            })
            .collect();
        let lines: Vec<DemoLine> = snapshot
            .lines
            .into_iter()
            .map(|line| DemoLine {
                number: line.number,
                text: line.text,
            })
            .collect();

        Self {
            title: title.into(),
            line_count: lines.len(),
            lines,
            current_yaml_path,
            diagnostics,
            completion_labels,
        }
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
        App, Application, Bounds, Context, IntoElement, Window, WindowBounds, WindowOptions, div,
        prelude::*, px, rgb, size, uniform_list,
    };

    pub fn run() {
        Application::new().run(|cx: &mut App| {
            let bounds = Bounds::centered(None, size(px(1120.0), px(720.0)), cx);
            cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    ..Default::default()
                },
                |_, cx| {
                    cx.new(|_| MocodeGpuiDemo {
                        document: load_demo_document(),
                    })
                },
            )
            .unwrap();
            cx.activate(true);
        });
    }

    struct MocodeGpuiDemo {
        document: DemoDocument,
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
                                .child(editor_surface(&self.document, cx))
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
        cx: &mut Context<'_, MocodeGpuiDemo>,
    ) -> impl IntoElement {
        let line_count = document.lines.len();
        div().w(px(820.0)).h_full().bg(rgb(0xffffff)).child(
            uniform_list(
                "mocode-lines",
                line_count,
                cx.processor(|this, range, _window, _cx| {
                    let mut rows = Vec::new();
                    for index in range {
                        let line = &this.document.lines[index];
                        rows.push(line_row(index, line.number, line.text.clone()));
                    }
                    rows
                }),
            )
            .h_full(),
        )
    }

    fn line_row(index: usize, number: u32, text: String) -> impl IntoElement {
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
                    .text_color(rgb(0x0f172a))
                    .whitespace_nowrap()
                    .overflow_hidden()
                    .text_ellipsis()
                    .child(text),
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
}
