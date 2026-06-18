use floem::prelude::{SignalUpdate, SignalWith};
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
struct DemoVisibleLine {
    line: DemoLine,
    cursor: Option<u32>,
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
    completion_items: Vec<DemoCompletion>,
    completion_popup: Option<DemoCompletionPopup>,
    hover_title: String,
    hover_body: String,
    selection_anchor: Option<TextPosition>,
    selection_summary: String,
}

type DocumentSignal = floem::reactive::RwSignal<DemoDocument>;

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

    let document = create_rw_signal(document);

    v_stack((
        header(document),
        completion_strip(document),
        completion_popup_panel(document),
        h_stack((editor_surface(document), inspector(document))).style(|style| {
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

fn completion_popup_panel(document: DocumentSignal) -> impl floem::IntoView {
    use floem::prelude::*;

    h_stack((
        dynamic_text_label(move || {
            document
                .with(|document| {
                    document.completion_popup.as_ref().map(|popup| {
                        format!("Popup @ {}:{}", popup.anchor_line, popup.anchor_column)
                    })
                })
                .unwrap_or_else(|| "Popup @ <none>".to_string())
        })
        .style(|style| {
            style
                .width(132.0)
                .font_size(11.0)
                .color(color(0x64, 0x74, 0x8b))
        }),
        dyn_stack(
            move || {
                document
                    .with(|document| {
                        document
                            .completion_popup
                            .as_ref()
                            .map(|popup| popup.items.iter().take(4).cloned().collect::<Vec<_>>())
                    })
                    .unwrap_or_default()
            },
            |item| (item.kind.clone(), item.label.clone()),
            completion_item,
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
            .background(floem::prelude::Color::WHITE)
            .border_bottom(1.0)
            .border_color(color(0xd9, 0xe2, 0xec))
    })
}

fn header(document: DocumentSignal) -> impl floem::IntoView {
    use floem::prelude::*;

    h_stack((
        v_stack((
            text_label("mocode Floem prototype"),
            dynamic_text_label(move || document.with(|document| document.title.clone()))
                .style(|style| style.font_size(12.0).color(color(0x5f, 0x6b, 0x7a))),
        ))
        .style(|style| style.flex_col().gap(4.0)),
        fixture_selector(document),
        dynamic_text_label(move || {
            document.with(|document| format!("{} lines", document.line_count))
        })
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

fn fixture_selector(document: DocumentSignal) -> impl floem::IntoView {
    use floem::prelude::*;

    dyn_stack(
        || DEMO_FIXTURES.to_vec(),
        |fixture| fixture.id,
        move |fixture| fixture_button(document, fixture),
    )
    .style(|style| style.flex_row().gap(4.0))
}

fn fixture_button(document: DocumentSignal, fixture: DemoFixture) -> impl floem::IntoView {
    use floem::prelude::*;

    text_label(fixture.label)
        .style(|style| {
            style
                .padding_horiz(8.0)
                .padding_vert(4.0)
                .font_size(11.0)
                .background(color(0xf8, 0xfa, 0xfc))
                .border(1.0)
                .border_color(color(0xd9, 0xe2, 0xec))
                .color(color(0x33, 0x41, 0x55))
        })
        .on_click_stop(move |_| {
            if let Some(next) = load_fixture_by_id(fixture.id) {
                document.set(next);
            }
        })
}

fn completion_strip(document: DocumentSignal) -> impl floem::IntoView {
    use floem::prelude::*;

    h_stack((
        text_label("Completions").style(|style| {
            style
                .width(88.0)
                .font_size(11.0)
                .color(color(0x64, 0x74, 0x8b))
        }),
        dyn_stack(
            move || {
                document.with(|document| {
                    document
                        .completion_items
                        .iter()
                        .take(6)
                        .cloned()
                        .collect::<Vec<_>>()
                })
            },
            |item| (item.kind.clone(), item.label.clone()),
            completion_item,
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

fn editor_surface(document: DocumentSignal) -> impl floem::IntoView {
    use floem::prelude::*;
    use floem::{
        event::{EventListener, EventPropagation},
        views::{VirtualDirection, VirtualItemSize, scroll, virtual_stack},
    };

    let focus_request = create_rw_signal(0_u64);
    let focus_for_click = focus_request;

    scroll(
        virtual_stack(
            VirtualDirection::Vertical,
            VirtualItemSize::Fixed(Box::new(|| 22.0)),
            move || visible_lines(document),
            |visible_line| {
                (
                    visible_line.line.number,
                    visible_line.line.text.clone(),
                    visible_line.line.diagnostic_count,
                    visible_line.line.diagnostic_severity.clone(),
                    visible_line.cursor,
                )
            },
            line_row,
        )
        .style(|style| style.flex_col().width_full()),
    )
    .keyboard_navigable()
    .request_focus(move || {
        focus_request.get();
    })
    .on_event(EventListener::PointerDown, move |_| {
        focus_for_click.update(|value| *value += 1);
        EventPropagation::Stop
    })
    .on_event(EventListener::KeyDown, move |event| {
        if handle_key_down(document, event) {
            EventPropagation::Stop
        } else {
            EventPropagation::Continue
        }
    })
    .on_event(EventListener::ImeCommit, move |event| {
        if handle_ime_commit(document, event) {
            EventPropagation::Stop
        } else {
            EventPropagation::Continue
        }
    })
    .style(|style| {
        style
            .width(820.0)
            .height_full()
            .background(floem::prelude::Color::WHITE)
    })
}

fn line_row(visible_line: DemoVisibleLine) -> impl floem::IntoView {
    use floem::prelude::*;

    let line = visible_line.line;
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
    let (before_cursor, after_cursor) = visible_line
        .cursor
        .map(|character| split_at_character(&line.text, character))
        .unwrap_or_else(|| (line.text, String::new()));

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
        h_stack((
            text_label(before_cursor),
            cursor_bar(visible_line.cursor.is_some()),
            text_label(after_cursor),
        ))
        .style(|style| {
            style
                .width(756.0)
                .padding_horiz(12.0)
                .flex_row()
                .items_center()
                .color(color(0x0f, 0x17, 0x2a))
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

fn inspector(document: DocumentSignal) -> impl floem::IntoView {
    use floem::prelude::*;
    use floem::views::scroll;

    v_stack((
        info_section("YAML path", move || {
            document.with(|document| document.current_yaml_path.clone())
        }),
        info_section("Cursor", move || {
            document.with(|document| {
                format!(
                    "{}:{}",
                    document.cursor.line + 1,
                    document.cursor.character + 1
                )
            })
        }),
        info_section("Selection", move || {
            document.with(|document| document.selection_summary.clone())
        }),
        info_section("Hover", move || {
            document.with(|document| {
                if document.hover_body.is_empty() {
                    document.hover_title.clone()
                } else {
                    format!("{}\n{}", document.hover_title, document.hover_body)
                }
            })
        }),
        label_text("Diagnostics"),
        scroll(
            dyn_stack(
                move || document.with(|document| document.diagnostics.clone()),
                |diagnostic| {
                    (
                        diagnostic.severity.clone(),
                        diagnostic.code.clone(),
                        diagnostic.line,
                        diagnostic.column,
                        diagnostic.message.clone(),
                    )
                },
                diagnostic_row,
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

fn info_section(
    value_title: &'static str,
    value: impl Fn() -> String + 'static,
) -> impl floem::IntoView {
    use floem::prelude::*;

    v_stack((
        label_text(value_title),
        dynamic_text_label(value)
            .style(|style| style.color(color(0x1f, 0x29, 0x37)).line_height(1.35)),
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

fn dynamic_text_label(content: impl Fn() -> String + 'static) -> impl floem::IntoView {
    use floem::prelude::*;

    label(content)
}

fn text_label(content: impl Into<String>) -> impl floem::IntoView {
    use floem::prelude::*;

    let content: String = content.into();
    label(move || content.clone())
}

fn cursor_bar(visible: bool) -> impl floem::IntoView {
    use floem::prelude::*;

    floem::views::empty().style(move |style| {
        style
            .width(if visible { 1.0 } else { 0.0 })
            .height(16.0)
            .background(color(0x25, 0x63, 0xeb))
    })
}

fn visible_lines(document: DocumentSignal) -> im::Vector<DemoVisibleLine> {
    document.with(|document| {
        let cursor_line = document.cursor.line as usize;
        document
            .lines
            .iter()
            .enumerate()
            .map(|(index, line)| DemoVisibleLine {
                line: line.clone(),
                cursor: (index == cursor_line).then_some(document.cursor.character),
            })
            .collect()
    })
}

fn update_document(
    document: DocumentSignal,
    action: impl FnOnce(&mut DemoDocument) -> Result<(), EditorError>,
) {
    document.update(|document| {
        let _ = action(document);
    });
}

fn handle_key_down(document: DocumentSignal, event: &floem::event::Event) -> bool {
    use floem::{
        event::Event,
        keyboard::{Key, NamedKey},
    };

    let Event::KeyDown(event) = event else {
        return false;
    };

    let modifiers = event.modifiers;
    match &event.key.logical_key {
        Key::Named(NamedKey::Backspace) => {
            update_document(document, DemoDocument::backspace);
            true
        }
        Key::Named(NamedKey::Delete) => {
            update_document(document, DemoDocument::delete);
            true
        }
        Key::Named(NamedKey::ArrowLeft) if modifiers.shift() => {
            update_document(document, DemoDocument::select_left);
            true
        }
        Key::Named(NamedKey::ArrowLeft) => {
            update_document(document, DemoDocument::move_left);
            true
        }
        Key::Named(NamedKey::ArrowRight) if modifiers.shift() => {
            update_document(document, DemoDocument::select_right);
            true
        }
        Key::Named(NamedKey::ArrowRight) => {
            update_document(document, DemoDocument::move_right);
            true
        }
        Key::Named(NamedKey::Enter) if !has_command_modifier(modifiers) => {
            update_document(document, |document| document.insert_text("\n"));
            true
        }
        Key::Named(NamedKey::Tab) if !has_command_modifier(modifiers) => {
            update_document(document, |document| document.insert_text("\t"));
            true
        }
        Key::Character(character) if is_paste_shortcut(character, modifiers) => {
            if let Ok(text) = floem::Clipboard::get_contents() {
                update_document(document, |document| document.insert_text(&text));
            }
            true
        }
        Key::Character(character) if is_copy_shortcut(character, modifiers) => {
            if let Some(text) = document.with(DemoDocument::copy_selection_text) {
                let _ = floem::Clipboard::set_contents(text);
            }
            true
        }
        Key::Character(character)
            if !has_command_modifier(modifiers) && is_insertable_text(&character.to_string()) =>
        {
            let text = character.to_string();
            update_document(document, |document| document.insert_text(&text));
            true
        }
        _ => false,
    }
}

fn handle_ime_commit(document: DocumentSignal, event: &floem::event::Event) -> bool {
    let floem::event::Event::ImeCommit(text) = event else {
        return false;
    };

    if !is_insertable_text(text) {
        return false;
    }

    update_document(document, |document| document.insert_text(text));
    true
}

fn has_command_modifier(modifiers: floem::keyboard::Modifiers) -> bool {
    modifiers.control() || modifiers.meta() || modifiers.alt()
}

fn is_paste_shortcut(character: &str, modifiers: floem::keyboard::Modifiers) -> bool {
    (modifiers.control() || modifiers.meta()) && character.eq_ignore_ascii_case("v")
}

fn is_copy_shortcut(character: &str, modifiers: floem::keyboard::Modifiers) -> bool {
    (modifiers.control() || modifiers.meta()) && character.eq_ignore_ascii_case("c")
}

fn is_insertable_text(text: &str) -> bool {
    !text.is_empty()
        && text
            .chars()
            .all(|character| character == '\n' || character == '\t' || !character.is_control())
}

fn split_at_character(text: &str, character: u32) -> (String, String) {
    let split_at = text
        .char_indices()
        .nth(character as usize)
        .map(|(index, _)| index)
        .unwrap_or(text.len());
    (text[..split_at].to_string(), text[split_at..].to_string())
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
