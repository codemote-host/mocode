use mocode_api::{CompletionKind, DiagnosticSeverity, EditorError, MocodeEditor, TextPosition};

const SAMPLE_TITLE: &str = "examples/configs/dialer-proxy.yaml";
const SAMPLE_TEXT: &str = include_str!("../../../examples/configs/dialer-proxy.yaml");
const INSPECT_POSITION: TextPosition = TextPosition::new(10, 17);

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

#[derive(Debug, Clone)]
struct DemoDocument {
    title: String,
    editor: MocodeEditor,
    cursor: TextPosition,
    line_count: usize,
    lines: Vec<DemoLine>,
    current_yaml_path: String,
    diagnostics: Vec<DemoDiagnostic>,
    completion_items: Vec<DemoCompletion>,
    hover_title: String,
    hover_body: String,
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
            completion_items: Vec::new(),
            hover_title: String::new(),
            hover_body: String::new(),
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
            .map(|diagnostic| DemoDiagnostic {
                severity: severity_label(diagnostic.severity).to_string(),
                code: diagnostic.code,
                message: diagnostic.message,
                line: diagnostic.range.map(|range| range.start.line + 1),
                column: diagnostic.range.map(|range| range.start.character + 1),
            })
            .collect();

        self.lines = self
            .editor
            .semantic_lines()
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
    }
}

fn load_demo_document() -> DemoDocument {
    DemoDocument::from_text(SAMPLE_TITLE, SAMPLE_TEXT, INSPECT_POSITION)
}

fn completion_kind_label(kind: CompletionKind) -> &'static str {
    match kind {
        CompletionKind::Field => "field",
        CompletionKind::EnumValue => "enum",
        CompletionKind::Reference => "reference",
        CompletionKind::Snippet => "snippet",
    }
}

fn severity_label(severity: DiagnosticSeverity) -> &'static str {
    match severity {
        DiagnosticSeverity::Error => "error",
        DiagnosticSeverity::Warning => "warning",
        DiagnosticSeverity::Info => "info",
        DiagnosticSeverity::Hint => "hint",
    }
}

fn main() {
    let document = load_demo_document();
    floem::launch(move || app_view(document));
}

fn app_view(document: DemoDocument) -> impl floem::IntoView {
    use floem::prelude::*;

    let title = document.title.clone();
    let line_count = document.line_count;
    let completions = document.completion_items.clone();
    let lines = document.lines.clone();

    v_stack((
        header(title, line_count),
        completion_strip(completions),
        h_stack((editor_surface(lines), inspector(document))).style(|style| {
            style
                .flex()
                .flex_row()
                .height_full()
                .background(color(0xf7, 0xf9, 0xfc))
        }),
    ))
    .style(|style| {
        style
            .size_full()
            .flex_col()
            .font_size(13.0)
            .color(color(0x1f, 0x29, 0x37))
            .background(color(0xf7, 0xf9, 0xfc))
    })
}

fn header(title: String, line_count: usize) -> impl floem::IntoView {
    use floem::prelude::*;

    h_stack((
        v_stack((
            text_label("mocode Floem prototype"),
            text_label(title).style(|style| style.font_size(12.0).color(color(0x5f, 0x6b, 0x7a))),
        ))
        .style(|style| style.flex_col().gap(4.0)),
        text_label(format!("{line_count} lines"))
            .style(|style| style.color(color(0x5f, 0x6b, 0x7a))),
    ))
    .style(|style| {
        style
            .flex()
            .flex_row()
            .justify_between()
            .items_center()
            .padding_horiz(16.0)
            .padding_vert(12.0)
            .background(floem::prelude::Color::WHITE)
            .border_bottom(1.0)
            .border_color(color(0xd9, 0xe2, 0xec))
    })
}

fn completion_strip(items: Vec<DemoCompletion>) -> impl floem::IntoView {
    use floem::prelude::*;

    h_stack((
        text_label("Completions").style(|style| {
            style
                .width(88.0)
                .font_size(11.0)
                .color(color(0x64, 0x74, 0x8b))
        }),
        h_stack_from_iter(
            items
                .into_iter()
                .take(6)
                .map(completion_item)
                .collect::<Vec<_>>(),
        )
        .style(|style| style.gap(8.0)),
    ))
    .style(|style| {
        style
            .flex()
            .flex_row()
            .items_center()
            .gap(8.0)
            .padding_horiz(16.0)
            .padding_vert(8.0)
            .background(color(0xf8, 0xfa, 0xfc))
            .border_bottom(1.0)
            .border_color(color(0xd9, 0xe2, 0xec))
    })
}

fn editor_surface(lines: Vec<DemoLine>) -> impl floem::IntoView {
    use floem::prelude::*;
    use floem::views::{VirtualDirection, VirtualItemSize, scroll, virtual_stack};

    let lines: im::Vector<DemoLine> = lines.into_iter().collect();
    let lines = create_rw_signal(lines);

    scroll(
        virtual_stack(
            VirtualDirection::Vertical,
            VirtualItemSize::Fixed(Box::new(|| 22.0)),
            move || lines.get(),
            |line| line.number,
            line_row,
        )
        .style(|style| style.flex_col().width_full()),
    )
    .style(|style| {
        style
            .width(820.0)
            .height_full()
            .background(floem::prelude::Color::WHITE)
    })
}

fn line_row(line: DemoLine) -> impl floem::IntoView {
    use floem::prelude::*;

    let marker_color = line
        .diagnostic_severity
        .as_deref()
        .map(severity_color)
        .unwrap_or_else(|| color(0xf8, 0xfa, 0xfc));
    let line_number = if line.diagnostic_count == 0 {
        format!("{:>4}", line.number)
    } else {
        format!("{:>3}!", line.number)
    };

    h_stack((
        h_stack((
            floem::views::empty()
                .style(move |style| style.width(4.0).height_full().background(marker_color)),
            text_label(line_number).style(|style| style.width(60.0).padding_horiz(8.0)),
        ))
        .style(|style| {
            style
                .width(64.0)
                .height_full()
                .flex()
                .flex_row()
                .background(color(0xf8, 0xfa, 0xfc))
                .color(color(0x94, 0xa3, 0xb8))
        }),
        text_label(line.text).style(|style| {
            style
                .width(756.0)
                .padding_horiz(12.0)
                .color(color(0x0f, 0x17, 0x2a))
                .text_ellipsis()
        }),
    ))
    .style(|style| {
        style
            .height(22.0)
            .line_height(1.0)
            .flex()
            .flex_row()
            .items_center()
            .border_bottom(1.0)
            .border_color(color(0xf1, 0xf5, 0xf9))
    })
}

fn inspector(document: DemoDocument) -> impl floem::IntoView {
    use floem::prelude::*;
    use floem::views::{scroll, v_stack_from_iter};

    let hover = if document.hover_body.is_empty() {
        document.hover_title.clone()
    } else {
        format!("{}\n{}", document.hover_title, document.hover_body)
    };

    v_stack((
        info_section("YAML path", document.current_yaml_path.clone()),
        info_section(
            "Cursor",
            format!(
                "{}:{}",
                document.cursor.line + 1,
                document.cursor.character + 1
            ),
        ),
        info_section("Hover", hover),
        label_text("Diagnostics"),
        scroll(
            v_stack_from_iter(
                document
                    .diagnostics
                    .clone()
                    .into_iter()
                    .map(diagnostic_row)
                    .collect::<Vec<_>>(),
            )
            .style(|style| style.flex_col().gap(8.0)),
        )
        .style(|style| style.height_full()),
    ))
    .style(|style| {
        style
            .width(300.0)
            .height_full()
            .flex_col()
            .padding(16.0)
            .background(color(0xf2, 0xf5, 0xf9))
            .border_left(1.0)
            .border_color(color(0xd9, 0xe2, 0xec))
    })
}

fn info_section(title: &'static str, value: String) -> impl floem::IntoView {
    use floem::prelude::*;

    v_stack((
        label_text(title),
        text_label(value).style(|style| style.color(color(0x1f, 0x29, 0x37)).line_height(1.35)),
    ))
    .style(|style| style.flex_col().gap(4.0).margin_bottom(16.0))
}

fn completion_item(item: DemoCompletion) -> impl floem::IntoView {
    use floem::prelude::*;

    let documentation = item
        .documentation
        .unwrap_or_else(|| "<no docs>".to_string());
    v_stack((
        text_label(format!("{} {}", item.kind, item.label))
            .style(|style| style.color(color(0x0f, 0x17, 0x2a))),
        text_label(documentation).style(|style| {
            style
                .width(180.0)
                .font_size(11.0)
                .line_height(1.2)
                .color(color(0x64, 0x74, 0x8b))
                .text_ellipsis()
        }),
    ))
    .style(|style| {
        style
            .flex_col()
            .gap(4.0)
            .padding_horiz(8.0)
            .padding_vert(4.0)
            .background(floem::prelude::Color::WHITE)
            .border(1.0)
            .border_color(color(0xd9, 0xe2, 0xec))
    })
}

fn diagnostic_row(diagnostic: DemoDiagnostic) -> impl floem::IntoView {
    use floem::prelude::*;

    let location = match (diagnostic.line, diagnostic.column) {
        (Some(line), Some(column)) => format!(" at {line}:{column}"),
        _ => String::new(),
    };
    let severity = diagnostic.severity.clone();

    v_stack((
        text_label(format!(
            "{} {}{}",
            diagnostic.severity, diagnostic.code, location
        ))
        .style(move |style| style.color(severity_color(&severity))),
        text_label(diagnostic.message)
            .style(|style| style.color(color(0x33, 0x41, 0x55)).line_height(1.35)),
    ))
    .style(|style| {
        style
            .flex_col()
            .gap(4.0)
            .padding(8.0)
            .background(floem::prelude::Color::WHITE)
            .border(1.0)
            .border_color(color(0xd9, 0xe2, 0xec))
    })
}

fn label_text(content: &'static str) -> impl floem::IntoView {
    use floem::prelude::*;

    text_label(content).style(|style| style.font_size(11.0).color(color(0x64, 0x74, 0x8b)))
}

fn text_label(content: impl Into<String>) -> impl floem::IntoView {
    use floem::prelude::*;

    let content: String = content.into();
    label(move || content.clone())
}

fn severity_color(severity: &str) -> floem::prelude::Color {
    match severity {
        "error" => color(0xb4, 0x23, 0x18),
        "warning" => color(0xa1, 0x62, 0x07),
        _ => color(0x25, 0x63, 0xeb),
    }
}

fn color(red: u8, green: u8, blue: u8) -> floem::prelude::Color {
    floem::prelude::Color::rgb8(red, green, blue)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mocode_api::TextPosition;

    #[test]
    fn builds_demo_document_from_core_semantics() {
        let document = DemoDocument::from_text(
            "dialer-proxy.yaml",
            include_str!("../../../examples/configs/dialer-proxy.yaml"),
            TextPosition::new(10, 17),
        );

        assert_eq!(document.title, "dialer-proxy.yaml");
        assert!(document.line_count > 10);
        assert_eq!(document.lines[0].number, 1);
        assert_eq!(document.lines[0].text, "mixed-port: 7890");
        assert_eq!(document.current_yaml_path, "proxies[0].dialer-proxy");
        assert!(
            document
                .completion_items
                .iter()
                .any(|item| { item.label == "exit" && item.kind == "reference" })
        );
        assert_eq!(document.hover_title, "proxies[].dialer-proxy");
        assert!(document.hover_body.contains("Outbound used"));
    }

    #[test]
    fn marks_yaml_syntax_diagnostics_on_lines() {
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
    fn loads_large_fixture_for_floem_baseline() {
        let text = include_str!("../../../examples/configs/large.yaml");
        let document = DemoDocument::from_text("large.yaml", text, TextPosition::new(0, 0));

        assert!(text.lines().count() >= 5_000);
        assert!(document.line_count >= 5_000);
        assert_eq!(document.lines[0].text, "mixed-port: 7890");
        assert!(
            document
                .completion_items
                .iter()
                .any(|item| { item.label == "mixed-port" && item.kind == "field" })
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
        assert!(
            document
                .completion_items
                .iter()
                .any(|item| item.label == "fake-ip")
        );
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
