use std::{
    ops::Range,
    path::{Path, PathBuf},
};

use crate::{
    component::{
        self, GpuiEditorComponent, GpuiEditorDocument, GpuiEditorHost, render_editor_component,
    },
    fixtures::default_document,
};
use gpui::{
    App, Application, Bounds, Context, EntityInputHandler, FocusHandle, Focusable, IntoElement,
    MouseButton, MouseDownEvent, PathPromptOptions, Pixels, Point, Render, UTF16Selection, Window,
    WindowBounds, WindowOptions, div, prelude::*, px, rgb, size,
};

pub(crate) fn run() {
    let startup_path = std::env::args_os().nth(1).map(PathBuf::from);
    run_with_startup_path(startup_path);
}

fn run_with_startup_path(startup_path: Option<PathBuf>) {
    Application::new().run(move |cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(1120.0), px(720.0)), cx);
        component::bind_editor_keys(cx);
        let startup_path = startup_path.clone();
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            move |window, cx| {
                let focus_handle = cx.focus_handle().tab_stop(true);
                focus_handle.focus(window);
                let document = initial_document_from_startup_path(startup_path.as_deref());
                cx.new(|_| MocodeApp {
                    editor: GpuiEditorComponent::new(document, focus_handle),
                })
            },
        )
        .unwrap();
        cx.activate(true);
    });
}

pub(crate) fn initial_document_from_startup_path(path: Option<&Path>) -> GpuiEditorDocument {
    match path {
        Some(path) => match GpuiEditorDocument::from_path(path) {
            Ok(document) => document,
            Err(error) => {
                let mut document = default_document();
                document.save_status = format!(
                    "Failed to open {}: {error}; showing default fixture",
                    path.display()
                );
                document
            }
        },
        None => default_document(),
    }
}

struct MocodeApp {
    editor: GpuiEditorComponent,
}

impl MocodeApp {
    fn suggested_save_name(&self) -> String {
        self.editor
            .document()
            .path
            .as_ref()
            .and_then(|path| path.file_name())
            .and_then(|name| name.to_str())
            .map(str::to_string)
            .or_else(|| {
                Path::new(&self.editor.document().title)
                    .file_name()
                    .and_then(|name| name.to_str())
                    .map(str::to_string)
            })
            .unwrap_or_else(|| "config.yaml".to_string())
    }

    fn suggested_save_directory(&self) -> PathBuf {
        self.editor
            .document()
            .path
            .as_ref()
            .and_then(|path| path.parent())
            .map(Path::to_path_buf)
            .or_else(|| std::env::current_dir().ok())
            .unwrap_or_else(|| PathBuf::from("."))
    }
}

impl GpuiEditorHost for MocodeApp {
    fn editor_component(&self) -> &GpuiEditorComponent {
        &self.editor
    }

    fn editor_component_mut(&mut self) -> &mut GpuiEditorComponent {
        &mut self.editor
    }

    fn open_document(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.editor.document_mut().save_status = "Opening file...".to_string();
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Open YAML".into()),
        });

        cx.spawn(async move |this, cx| {
            let result = receiver.await;
            let _ = this.update(cx, |this, cx| {
                match result {
                    Ok(Ok(Some(paths))) => {
                        if let Some(path) = paths.into_iter().next() {
                            match this.editor.open_path(&path) {
                                Ok(()) => {
                                    this.editor.document_mut().save_status =
                                        format!("Opened {}", path.display());
                                }
                                Err(error) => {
                                    this.editor.document_mut().save_status =
                                        format!("Failed to open {}: {error}", path.display());
                                }
                            }
                        }
                    }
                    Ok(Ok(None)) => {
                        this.editor.document_mut().save_status = "Open canceled".to_string();
                    }
                    Ok(Err(error)) => {
                        this.editor.document_mut().save_status =
                            format!("Open dialog failed: {error}");
                    }
                    Err(error) => {
                        this.editor.document_mut().save_status =
                            format!("Open dialog canceled: {error}");
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn save_document(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.editor.document().path.is_none() {
            self.save_document_as(window, cx);
            return;
        }

        let _ = self.editor.document_mut().save_to_original_path();
    }

    fn save_document_as(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.editor.document_mut().save_status = "Choosing save path...".to_string();
        let directory = self.suggested_save_directory();
        let suggested_name = self.suggested_save_name();
        let receiver = cx.prompt_for_new_path(&directory, Some(&suggested_name));

        cx.spawn(async move |this, cx| {
            let result = receiver.await;
            let _ = this.update(cx, |this, cx| {
                match result {
                    Ok(Ok(Some(path))) => {
                        let _ = this.editor.document_mut().save_as(&path);
                    }
                    Ok(Ok(None)) => {
                        this.editor.document_mut().save_status = "Save as canceled".to_string();
                    }
                    Ok(Err(error)) => {
                        this.editor.document_mut().save_status =
                            format!("Save dialog failed: {error}");
                    }
                    Err(error) => {
                        this.editor.document_mut().save_status =
                            format!("Save dialog canceled: {error}");
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }
}

impl EntityInputHandler for MocodeApp {
    fn text_for_range(
        &mut self,
        range: Range<usize>,
        adjusted_range: &mut Option<Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        self.editor
            .document()
            .text_for_utf16_range(range, adjusted_range)
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        let (range, reversed) = self.editor.document().selected_utf16_range();
        Some(UTF16Selection { range, reversed })
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        self.editor.document().marked_utf16_range()
    }

    fn unmark_text(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.editor.document_mut().unmark_ime_text();
        cx.notify();
    }

    fn replace_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        text: &str,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.editor.document().search_active {
            self.editor.document_mut().append_search_input(text);
            cx.notify();
            return;
        }

        if self.editor.replace_utf16_range(range, text).is_ok() {
            cx.notify();
        }
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        new_text: &str,
        new_selected_range: Option<Range<usize>>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.editor.document().search_active {
            self.editor.document_mut().append_search_input(new_text);
            cx.notify();
            return;
        }

        if self
            .editor
            .replace_and_mark_utf16_range(range, new_text, new_selected_range)
            .is_ok()
        {
            cx.notify();
        }
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        element_bounds: Bounds<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        self.editor
            .document()
            .bounds_for_utf16_range(range_utf16, element_bounds)
    }

    fn character_index_for_point(
        &mut self,
        point: Point<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        let bounds = self.editor.line_list_bounds()?;
        self.editor.document().utf16_index_for_point(point, bounds)
    }
}

impl Focusable for MocodeApp {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.editor.focus_handle().clone()
    }
}

impl Render for MocodeApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
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
                    .child(render_editor_component(&self.editor, window, cx)),
            )
    }
}

fn header(editor: &GpuiEditorComponent, cx: &mut Context<'_, MocodeApp>) -> impl IntoElement {
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
            div().flex().flex_col().gap_1().child("mocode").child(
                div()
                    .text_color(rgb(0x5f6b7a))
                    .text_size(px(12.0))
                    .child(document.title.clone()),
            ),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .text_color(rgb(0x5f6b7a))
                .text_size(px(12.0))
                .child(document.path_display.clone())
                .child(format!(
                    "{} - {}",
                    if document.dirty { "dirty" } else { "clean" },
                    document.save_status
                )),
        )
        .child(command_buttons(cx))
        .child(
            div()
                .text_color(rgb(0x5f6b7a))
                .child(format!("{} lines", document.line_count)),
        )
}

fn command_buttons(cx: &mut Context<'_, MocodeApp>) -> impl IntoElement {
    div()
        .flex()
        .flex_row()
        .gap_1()
        .child(command_button("Open", cx, |this, window, cx| {
            this.open_document(window, cx);
        }))
        .child(command_button("Save", cx, |this, window, cx| {
            this.save_document(window, cx);
        }))
        .child(command_button("Save As", cx, |this, window, cx| {
            this.save_document_as(window, cx);
        }))
}

fn command_button(
    label: &'static str,
    cx: &mut Context<'_, MocodeApp>,
    handler: fn(&mut MocodeApp, &mut Window, &mut Context<MocodeApp>),
) -> impl IntoElement {
    div()
        .px_2()
        .py_1()
        .bg(rgb(0xeff6ff))
        .border_1()
        .border_color(rgb(0xbfdbfe))
        .text_color(rgb(0x1d4ed8))
        .text_size(px(11.0))
        .child(label)
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, _: &MouseDownEvent, window: &mut Window, cx| {
                handler(this, window, cx);
            }),
        )
}
