use crate::{
    component::{
        self, GpuiEditorComponent, GpuiEditorDocument, GpuiEditorHost, render_editor_component,
    },
    fixtures::{DemoFixture, all_fixtures, default_fixture, fixture_by_id},
};
use gpui::{
    App, Application, Bounds, Context, FocusHandle, Focusable, IntoElement, MouseButton,
    MouseDownEvent, Render, Window, WindowBounds, WindowOptions, div, prelude::*, px, rgb, size,
};

pub(crate) fn run() {
    Application::new().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(1120.0), px(720.0)), cx);
        component::bind_editor_keys(cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |window, cx| {
                let focus_handle = cx.focus_handle().tab_stop(true);
                focus_handle.focus(window);
                cx.new(|_| MocodeGpuiDemo {
                    editor: GpuiEditorComponent::new(
                        GpuiEditorDocument::from_fixture(default_fixture()),
                        focus_handle,
                    ),
                })
            },
        )
        .unwrap();
        cx.activate(true);
    });
}

struct MocodeGpuiDemo {
    editor: GpuiEditorComponent,
}

impl MocodeGpuiDemo {
    fn select_fixture(&mut self, id: &'static str, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(fixture) = fixture_by_id(id) {
            self.editor
                .replace_document(GpuiEditorDocument::from_fixture(fixture));
            self.editor.focus(window);
            cx.notify();
        }
    }
}

impl GpuiEditorHost for MocodeGpuiDemo {
    fn editor_component(&self) -> &GpuiEditorComponent {
        &self.editor
    }

    fn editor_component_mut(&mut self) -> &mut GpuiEditorComponent {
        &mut self.editor
    }
}

impl Focusable for MocodeGpuiDemo {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.editor.focus_handle().clone()
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
                    .child(header(&self.editor, cx))
                    .child(render_editor_component(&self.editor, cx)),
            )
    }
}

fn header(editor: &GpuiEditorComponent, cx: &mut Context<'_, MocodeGpuiDemo>) -> impl IntoElement {
    let document = editor.document();
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
    let fixtures = all_fixtures();
    div()
        .flex()
        .flex_row()
        .gap_1()
        .child(fixture_button(&fixtures[0], cx))
        .child(fixture_button(&fixtures[1], cx))
        .child(fixture_button(&fixtures[2], cx))
        .child(fixture_button(&fixtures[3], cx))
        .child(fixture_button(&fixtures[4], cx))
        .child(fixture_button(&fixtures[5], cx))
        .child(fixture_button(&fixtures[6], cx))
        .child(fixture_button(&fixtures[7], cx))
        .child(fixture_button(&fixtures[8], cx))
        .child(fixture_button(&fixtures[9], cx))
        .child(fixture_button(&fixtures[10], cx))
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
