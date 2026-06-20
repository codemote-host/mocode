mod app;
mod component;
mod fixtures;

fn main() {
    app::run();
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    use mocode_api::{ProxyChainStatus, TextPosition};

    use crate::{
        app,
        component::{GpuiEditorDocument, GpuiEditorSaveError, GpuiSearchHighlight},
        fixtures::{SAMPLE_TITLE, default_fixture, document_by_fixture_id, document_from_fixture},
    };

    fn load_app_document() -> GpuiEditorDocument {
        document_from_fixture(default_fixture())
    }

    fn load_fixture_by_id(id: &str) -> Option<GpuiEditorDocument> {
        document_by_fixture_id(id)
    }

    fn unique_temp_yaml_path(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "mocode-{label}-{}-{nanos}.yaml",
            std::process::id()
        ))
    }

    fn write_temp_yaml(label: &str, text: &str) -> PathBuf {
        let path = unique_temp_yaml_path(label);
        fs::write(&path, text).expect("test yaml should be writable");
        path
    }

    fn file_name(path: &Path) -> String {
        path.file_name()
            .and_then(|name| name.to_str())
            .expect("test path should have utf-8 file name")
            .to_string()
    }

    #[test]
    fn opens_document_from_startup_path() {
        let path = write_temp_yaml("startup-open", "mixed-port: 7890\n");

        let document = app::initial_document_from_startup_path(Some(path.as_path()));

        assert_eq!(document.title, file_name(&path));
        assert_eq!(document.path.as_deref(), Some(path.as_path()));
        assert_eq!(document.path_display, path.display().to_string());
        assert_eq!(document.text(), "mixed-port: 7890\n");
        assert!(!document.dirty);
        assert!(document.save_status.contains("Opened"));

        fs::remove_file(path).ok();
    }

    #[test]
    fn falls_back_to_default_fixture_when_startup_path_missing() {
        let missing_path = unique_temp_yaml_path("missing-startup");

        let document = app::initial_document_from_startup_path(Some(missing_path.as_path()));

        assert_eq!(document.title, SAMPLE_TITLE);
        assert!(document.path.is_none());
        assert!(!document.dirty);
        assert!(document.save_status.contains("Failed to open"));
        assert!(
            document
                .save_status
                .contains(&missing_path.display().to_string())
        );
    }

    #[test]
    fn editing_marks_opened_document_dirty() {
        let path = write_temp_yaml("dirty", "mixed-port: 7890\n");
        let mut document = app::initial_document_from_startup_path(Some(path.as_path()));

        document.insert_text("# edited\n").unwrap();

        assert!(document.dirty);
        assert!(document.save_status.contains("Modified"));

        fs::remove_file(path).ok();
    }

    #[test]
    fn saves_opened_document_to_disk() {
        let path = write_temp_yaml("save", "mixed-port: 7890\n");
        let mut document = app::initial_document_from_startup_path(Some(path.as_path()));

        document.insert_text("# saved\n").unwrap();
        document.save_to_original_path().unwrap();

        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "# saved\nmixed-port: 7890\n"
        );
        assert!(!document.dirty);
        assert!(document.save_status.contains("Saved"));

        fs::remove_file(path).ok();
    }

    #[test]
    fn daily_save_creates_backup_before_overwrite() {
        let path = write_temp_yaml("backup-save", "mixed-port: 7890\n");
        let backup_path = GpuiEditorDocument::backup_path_for(&path);
        let mut document = app::initial_document_from_startup_path(Some(path.as_path()));

        document.insert_text("# changed\n").unwrap();
        document.save_to_original_path().unwrap();

        assert_eq!(
            fs::read_to_string(&backup_path).unwrap(),
            "mixed-port: 7890\n"
        );
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "# changed\nmixed-port: 7890\n"
        );
        assert!(!document.dirty);
        assert!(document.save_status.contains("Backup"));

        fs::remove_file(path).ok();
        fs::remove_file(backup_path).ok();
    }

    #[test]
    fn daily_save_as_updates_document_path_and_title() {
        let path = unique_temp_yaml_path("save-as");
        let mut document = GpuiEditorDocument::from_text(
            "scratch.yaml",
            "mixed-port: 7890\n",
            TextPosition::new(0, 0),
        );

        document.insert_text("# saved as\n").unwrap();
        document.save_as(&path).unwrap();

        assert_eq!(document.path.as_deref(), Some(path.as_path()));
        assert_eq!(document.title, file_name(&path));
        assert_eq!(document.path_display, path.display().to_string());
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "# saved as\nmixed-port: 7890\n"
        );
        assert!(!document.dirty);
        assert!(document.save_status.contains("Saved as"));

        fs::remove_file(path).ok();
    }

    #[test]
    fn saving_fixture_without_path_reports_unsaved_state() {
        let mut document = load_app_document();

        let result = document.save_to_original_path();

        assert!(matches!(result, Err(GpuiEditorSaveError::MissingPath)));
        assert!(document.path.is_none());
        assert!(!document.dirty);
        assert!(document.save_status.contains("not saveable"));
    }

    #[test]
    fn component_source_stays_fixture_agnostic() {
        let component_source = include_str!("component.rs");

        assert!(!component_source.contains("crate::fixtures"));
        assert!(!component_source.contains("AppFixture"));
        assert!(!component_source.contains("from_fixture"));
    }

    #[test]
    fn app_header_does_not_render_fixture_buttons() {
        let app_source = include_str!("app.rs");

        assert!(!app_source.contains("fixtures["));
        assert!(!app_source.contains("all_fixtures().iter()"));
        assert!(!app_source.contains("fixture_selector"));
    }

    #[test]
    fn normal_editor_shell_does_not_stack_debug_panels() {
        let component_source = include_str!("component.rs");
        let app_source = include_str!("app.rs");

        assert!(!component_source.contains("search_panel("));
        assert!(!component_source.contains("completion_panel("));
        assert!(!component_source.contains("completion_popup_panel("));
        assert!(!component_source.contains("Popup @"));
        assert!(!app_source.contains("fixture_selector("));
        assert!(component_source.contains("status_bar("));
    }

    #[test]
    fn ime_input_handler_is_registered_from_paint_path() {
        let component_source = include_str!("component.rs");
        let prepaint_index = component_source
            .find(".on_children_prepainted")
            .expect("editor should capture child bounds during prepaint");
        let prepaint_tail = &component_source[prepaint_index..];
        let prepaint_block = prepaint_tail
            .split(".child(")
            .next()
            .expect("prepaint block should end before child layout");

        assert!(!prepaint_block.contains("handle_input("));
        assert!(component_source.contains("canvas("));
        assert!(component_source.contains("ElementInputHandler::new"));
    }

    #[test]
    fn builds_app_document_from_core_snapshot() {
        let document = load_app_document();

        assert_eq!(document.title, SAMPLE_TITLE);
        assert!(document.line_count > 10);
        let line0 = document.line_at(0).unwrap();
        assert_eq!(line0.number, 1);
        assert_eq!(line0.text, "mixed-port: 7890");
        assert_eq!(document.current_yaml_path, "proxies[0].dialer-proxy");
        assert!(document.completion_labels.contains(&"exit".to_string()));
    }

    #[test]
    fn carries_core_diagnostics_without_reimplementing_lints() {
        let document = GpuiEditorDocument::from_text(
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
        let document = GpuiEditorDocument::from_text(
            "invalid-yaml.yaml",
            include_str!("../../../examples/configs/invalid-yaml.yaml"),
            TextPosition::new(2, 0),
        );

        assert!(
            document
                .lines_in_range(0, document.line_count)
                .iter()
                .any(|line| line.diagnostic_count > 0)
        );
        assert!(document.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "yaml.syntax"
                && diagnostic.line.is_some()
                && diagnostic.column.is_some()
        }));
    }

    #[test]
    fn carries_hover_summary_for_current_position() {
        let document = GpuiEditorDocument::from_text(
            "tun.yaml",
            "tun:\n  stack: system\n",
            TextPosition::new(1, 4),
        );

        assert_eq!(document.hover_title, "tun.stack");
        assert!(document.hover_body.contains("TUN network stack"));
    }

    #[test]
    fn carries_completion_item_details_for_panel() {
        let document = GpuiEditorDocument::from_text(
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
        let document = GpuiEditorDocument::from_text(
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
        let document = GpuiEditorDocument::from_text("large.yaml", text, TextPosition::new(0, 0));

        assert!(text.lines().count() >= 5_000);
        assert!(document.line_count >= 5_000);
        assert_eq!(document.line_at(0).unwrap().text, "mixed-port: 7890");
        assert!(
            document
                .completion_labels
                .contains(&"mixed-port".to_string())
        );
    }

    #[test]
    fn loads_twenty_thousand_line_fixture_for_validation_baseline() {
        let text = include_str!("../../../examples/configs/large-20000.yaml");
        let document =
            GpuiEditorDocument::from_text("large-20000.yaml", text, TextPosition::new(0, 0));

        assert!(text.lines().count() >= 20_000);
        assert!(document.line_count >= 20_000);
        assert_eq!(document.line_at(0).unwrap().text, "mixed-port: 7890");
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
        let mut document = GpuiEditorDocument::from_text(
            "scratch.yaml",
            "dns:\n  enhanced-mode: \n",
            TextPosition::new(1, 17),
        );

        document.insert_text("fake-ip").unwrap();

        assert_eq!(document.cursor, TextPosition::new(1, 24));
        assert_eq!(
            document.line_at(1).unwrap().text,
            "  enhanced-mode: fake-ip"
        );
        assert_eq!(document.current_yaml_path, "dns.enhanced-mode");
        assert!(document.completion_labels.contains(&"fake-ip".to_string()));
    }

    #[test]
    fn daily_tab_inserts_two_spaces_at_cursor() {
        let mut document = GpuiEditorDocument::from_text(
            "scratch.yaml",
            "dns:\nmode: fake-ip\n",
            TextPosition::new(1, 0),
        );

        document.insert_tab().unwrap();

        assert_eq!(document.line_at(1).unwrap().text, "  mode: fake-ip");
        assert_eq!(document.cursor, TextPosition::new(1, 2));
    }

    #[test]
    fn daily_shift_tab_outdents_current_line() {
        let mut document = GpuiEditorDocument::from_text(
            "scratch.yaml",
            "dns:\n  mode: fake-ip\n",
            TextPosition::new(1, 7),
        );

        document.outdent_current_line().unwrap();

        assert_eq!(document.line_at(1).unwrap().text, "mode: fake-ip");
        assert_eq!(document.cursor, TextPosition::new(1, 5));
    }

    #[test]
    fn daily_ime_commit_inserts_unicode_text() {
        let mut document = GpuiEditorDocument::from_text(
            "scratch.yaml",
            "proxies:\n  - name: \n",
            TextPosition::new(1, 10),
        );

        document.commit_text("香港节点").unwrap();

        assert_eq!(document.line_at(1).unwrap().text, "  - name: 香港节点");
        assert_eq!(document.cursor, TextPosition::new(1, 14));
    }

    #[test]
    fn selection_insert_replaces_selected_text() {
        let mut document = GpuiEditorDocument::from_text(
            "scratch.yaml",
            "dns:\n  enhanced-mode: redir-host\n",
            TextPosition::new(1, 17),
        );

        for _ in 0.."redir-host".chars().count() {
            document.select_right().unwrap();
        }
        assert_eq!(document.selected_text().unwrap(), "redir-host");

        document.insert_text("fake-ip").unwrap();

        assert_eq!(
            document.line_at(1).unwrap().text,
            "  enhanced-mode: fake-ip"
        );
        assert_eq!(document.cursor, TextPosition::new(1, 24));
        assert!(document.selected_text().is_none());
        assert_eq!(document.selection_summary, "<none>");
    }

    #[test]
    fn selection_commit_replaces_selected_text_with_unicode() {
        let mut document = GpuiEditorDocument::from_text(
            "scratch.yaml",
            "proxies:\n  - name: old-name\n",
            TextPosition::new(1, 10),
        );

        for _ in 0.."old-name".chars().count() {
            document.select_right().unwrap();
        }

        document.commit_text("香港节点").unwrap();

        assert_eq!(document.line_at(1).unwrap().text, "  - name: 香港节点");
        assert_eq!(document.cursor, TextPosition::new(1, 14));
        assert!(document.selected_text().is_none());
    }

    #[test]
    fn selection_backspace_deletes_selected_text() {
        let mut document = GpuiEditorDocument::from_text(
            "scratch.yaml",
            "dns:\n  enhanced-mode: fake-ip\n",
            TextPosition::new(1, 17),
        );

        for _ in 0.."fake-ip".chars().count() {
            document.select_right().unwrap();
        }

        document.backspace().unwrap();

        assert_eq!(document.line_at(1).unwrap().text, "  enhanced-mode: ");
        assert_eq!(document.cursor, TextPosition::new(1, 17));
        assert!(document.selected_text().is_none());
    }

    #[test]
    fn selection_delete_deletes_reversed_selected_text() {
        let mut document = GpuiEditorDocument::from_text(
            "scratch.yaml",
            "dns:\n  enhanced-mode: fake-ip\n",
            TextPosition::new(1, 24),
        );

        for _ in 0.."fake-ip".chars().count() {
            document.select_left().unwrap();
        }
        assert_eq!(document.selected_text().unwrap(), "fake-ip");

        document.delete().unwrap();

        assert_eq!(document.line_at(1).unwrap().text, "  enhanced-mode: ");
        assert_eq!(document.cursor, TextPosition::new(1, 17));
        assert!(document.selected_text().is_none());
    }

    #[test]
    fn selection_paste_path_replaces_selected_text() {
        let mut document = GpuiEditorDocument::from_text(
            "scratch.yaml",
            "dns:\n  enhanced-mode: redir-host\n",
            TextPosition::new(1, 17),
        );

        for _ in 0.."redir-host".chars().count() {
            document.select_right().unwrap();
        }

        document.insert_text("fake-ip").unwrap();

        assert_eq!(
            document.line_at(1).unwrap().text,
            "  enhanced-mode: fake-ip"
        );
        assert_eq!(document.cursor, TextPosition::new(1, 24));
    }

    #[test]
    fn select_all_selects_entire_document() {
        let mut document = GpuiEditorDocument::from_text(
            "scratch.yaml",
            "mixed-port: 7890\ndns:\n  enable: true\n",
            TextPosition::new(1, 2),
        );

        document.select_all().unwrap();

        assert_eq!(
            document.selected_text().unwrap(),
            "mixed-port: 7890\ndns:\n  enable: true\n"
        );
        assert_eq!(document.cursor, TextPosition::new(3, 0));
        assert_eq!(document.selection_summary, "1:1 -> 4:1");
    }

    #[test]
    fn shift_home_selects_to_line_start() {
        let mut document = GpuiEditorDocument::from_text(
            "scratch.yaml",
            "dns:\n  enable: true\n",
            TextPosition::new(1, 8),
        );

        document.select_line_start().unwrap();

        assert_eq!(document.selected_text().unwrap(), "  enable");
        assert_eq!(document.cursor, TextPosition::new(1, 0));
        assert_eq!(document.selection_summary, "2:1 -> 2:9");
    }

    #[test]
    fn shift_end_selects_to_line_end() {
        let mut document = GpuiEditorDocument::from_text(
            "scratch.yaml",
            "dns:\n  enable: true\n",
            TextPosition::new(1, 2),
        );

        document.select_line_end().unwrap();

        assert_eq!(document.selected_text().unwrap(), "enable: true");
        assert_eq!(document.cursor, TextPosition::new(1, 14));
        assert_eq!(document.selection_summary, "2:3 -> 2:15");
    }

    #[test]
    fn mouse_drag_selection_selects_forward_text() {
        let mut document = GpuiEditorDocument::from_text(
            "scratch.yaml",
            "dns:\n  enhanced-mode: fake-ip\n",
            TextPosition::new(0, 0),
        );

        document.begin_selection_at(TextPosition::new(1, 2));
        document.select_to(TextPosition::new(1, 15));
        document.finish_selection();

        assert_eq!(document.selected_text().unwrap(), "enhanced-mode");
        assert_eq!(document.cursor, TextPosition::new(1, 15));
        assert_eq!(document.selection_summary, "2:3 -> 2:16");
    }

    #[test]
    fn mouse_drag_selection_selects_reversed_text() {
        let mut document = GpuiEditorDocument::from_text(
            "scratch.yaml",
            "dns:\n  enhanced-mode: fake-ip\n",
            TextPosition::new(0, 0),
        );

        document.begin_selection_at(TextPosition::new(1, 15));
        document.select_to(TextPosition::new(1, 2));
        document.finish_selection();

        assert_eq!(document.selected_text().unwrap(), "enhanced-mode");
        assert_eq!(document.cursor, TextPosition::new(1, 2));
        assert_eq!(document.selection_summary, "2:3 -> 2:16");
    }

    #[test]
    fn mouse_click_without_drag_clears_previous_selection() {
        let mut document = GpuiEditorDocument::from_text(
            "scratch.yaml",
            "dns:\n  enhanced-mode: fake-ip\n",
            TextPosition::new(1, 2),
        );

        for _ in 0.."enhanced-mode".chars().count() {
            document.select_right().unwrap();
        }
        assert!(document.selected_text().is_some());

        document.begin_selection_at(TextPosition::new(0, 3));
        document.finish_selection();

        assert_eq!(document.cursor, TextPosition::new(0, 3));
        assert!(document.selected_text().is_none());
        assert_eq!(document.selection_summary, "<none>");
    }

    #[test]
    fn editor_surface_wires_mouse_drag_selection_handlers() {
        let component_source = include_str!("component.rs");

        assert!(component_source.contains(".on_mouse_down("));
        assert!(component_source.contains(".on_mouse_move("));
        assert!(component_source.contains(".on_mouse_up("));
        assert!(component_source.contains("begin_mouse_selection"));
        assert!(component_source.contains("update_mouse_selection"));
        assert!(component_source.contains("finish_mouse_selection"));
    }

    #[test]
    fn editor_surface_tracks_cursor_reveal_scroll_handle() {
        let component_source = include_str!("component.rs");

        assert!(component_source.contains("UniformListScrollHandle"));
        assert!(component_source.contains("reveal_cursor"));
        assert!(component_source.contains(".track_scroll(scroll_handle)"));
    }

    #[test]
    fn editor_surface_wires_common_selection_shortcuts() {
        let component_source = include_str!("component.rs");

        assert!(component_source.contains("SelectAll"));
        assert!(component_source.contains("SelectLineStart"));
        assert!(component_source.contains("SelectLineEnd"));
        assert!(component_source.contains("ctrl-a"));
        assert!(component_source.contains("shift-home"));
        assert!(component_source.contains("shift-end"));
    }

    #[test]
    fn editor_surface_allows_horizontal_scroll_for_long_lines() {
        let component_source = include_str!("component.rs");

        assert!(component_source.contains("ListHorizontalSizingBehavior::Unconstrained"));
        assert!(component_source.contains(".with_horizontal_sizing_behavior("));
    }

    #[test]
    fn editor_line_text_is_not_truncated() {
        let component_source = include_str!("component.rs");
        let line_text_source = component_source
            .split("fn render_line_text")
            .nth(1)
            .and_then(|source| source.split("fn char_to_byte_index").next())
            .expect("render_line_text source should be present");

        assert!(!line_text_source.contains(".text_ellipsis()"));
        assert!(!line_text_source.contains(".overflow_hidden()"));
        assert!(!line_text_source.contains(".w(px(756.0))"));
    }

    #[test]
    fn document_text_returns_current_core_text() {
        let mut document = GpuiEditorDocument::from_text(
            "scratch.yaml",
            "dns:\n  enhanced-mode: \n",
            TextPosition::new(1, 17),
        );

        assert_eq!(document.text(), "dns:\n  enhanced-mode: \n");

        document.insert_text("fake-ip").unwrap();

        assert_eq!(document.text(), "dns:\n  enhanced-mode: fake-ip\n");
    }

    #[test]
    fn backspaces_deletes_and_moves_cursor_in_app_state() {
        let mut document = GpuiEditorDocument::from_text(
            "scratch.yaml",
            "dns:\n  enable: true\n",
            TextPosition::new(1, 2),
        );

        document.backspace().unwrap();
        assert_eq!(document.cursor, TextPosition::new(1, 1));
        assert_eq!(document.line_at(1).unwrap().text, " enable: true");

        document.move_left().unwrap();
        assert_eq!(document.cursor, TextPosition::new(1, 0));

        document.move_right().unwrap();
        assert_eq!(document.cursor, TextPosition::new(1, 1));

        document.delete().unwrap();
        assert_eq!(document.cursor, TextPosition::new(1, 1));
        assert_eq!(document.line_at(1).unwrap().text, " nable: true");
    }

    #[test]
    fn undo_redo_updates_app_state_and_clears_selection() {
        let mut document = GpuiEditorDocument::from_text(
            "scratch.yaml",
            "dns:\n  enhanced-mode: \n",
            TextPosition::new(1, 17),
        );

        document.insert_text("fake-ip").unwrap();
        assert_eq!(document.cursor, TextPosition::new(1, 24));
        assert_eq!(
            document.line_at(1).unwrap().text,
            "  enhanced-mode: fake-ip"
        );
        assert!(document.dirty);
        assert_eq!(document.selection_summary, "<none>");

        document.undo().unwrap();
        assert_eq!(document.cursor, TextPosition::new(1, 17));
        assert_eq!(document.line_at(1).unwrap().text, "  enhanced-mode: ");
        assert!(document.dirty);
        assert_eq!(document.selection_summary, "<none>");

        document.redo().unwrap();
        assert_eq!(document.cursor, TextPosition::new(1, 24));
        assert_eq!(
            document.line_at(1).unwrap().text,
            "  enhanced-mode: fake-ip"
        );
    }

    #[test]
    fn selection_copy_uses_shared_core_range() {
        let mut document = GpuiEditorDocument::from_text(
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

    #[test]
    fn daily_search_next_selects_match_and_wraps() {
        let mut document = GpuiEditorDocument::from_text(
            "search.yaml",
            "alpha\nbeta alpha\n",
            TextPosition::new(0, 0),
        );

        document.set_search_query("alpha");

        assert!(document.find_next());
        assert_eq!(document.cursor, TextPosition::new(0, 5));
        assert_eq!(document.selected_text().unwrap(), "alpha");
        assert_eq!(document.search_summary, "alpha - 1/2 at 1:1");

        assert!(document.find_next());
        assert_eq!(document.cursor, TextPosition::new(1, 10));
        assert_eq!(document.selected_text().unwrap(), "alpha");
        assert_eq!(document.search_summary, "alpha - 2/2 at 2:6");

        assert!(document.find_next());
        assert_eq!(document.cursor, TextPosition::new(0, 5));
        assert_eq!(document.search_summary, "alpha - 1/2 at 1:1");
    }

    #[test]
    fn daily_search_previous_wraps_from_first_match() {
        let mut document = GpuiEditorDocument::from_text(
            "search.yaml",
            "alpha\nbeta alpha\n",
            TextPosition::new(0, 0),
        );

        document.set_search_query("alpha");

        assert!(document.find_previous());
        assert_eq!(document.cursor, TextPosition::new(1, 10));
        assert_eq!(document.selected_text().unwrap(), "alpha");
        assert_eq!(document.search_summary, "alpha - 2/2 at 2:6");
    }

    #[test]
    fn daily_search_query_can_start_from_selection() {
        let mut document = GpuiEditorDocument::from_text(
            "search.yaml",
            "dns:\n  enhanced-mode: fake-ip\n",
            TextPosition::new(1, 2),
        );

        for _ in 0..13 {
            document.select_right().unwrap();
        }

        document.start_search_from_selection();

        assert!(document.search_active);
        assert_eq!(document.search_query, "enhanced-mode");
        assert_eq!(document.search_summary, "enhanced-mode - 1/1 at 2:3");
    }

    #[test]
    fn search_highlights_cover_visible_matches_and_current_match() {
        let mut document = GpuiEditorDocument::from_text(
            "search.yaml",
            "alpha\nbeta alpha\nalpha beta\n",
            TextPosition::new(0, 0),
        );

        document.set_search_query("alpha");
        document.find_next();

        let highlights = document.search_highlights_in_range(0, 3);

        assert_eq!(
            highlights.get(&0).unwrap(),
            &vec![GpuiSearchHighlight {
                start: 0,
                end: 5,
                active: true,
            }]
        );
        assert_eq!(
            highlights.get(&1).unwrap(),
            &vec![GpuiSearchHighlight {
                start: 5,
                end: 10,
                active: false,
            }]
        );
        assert_eq!(
            highlights.get(&2).unwrap(),
            &vec![GpuiSearchHighlight {
                start: 0,
                end: 5,
                active: false,
            }]
        );
    }

    #[test]
    fn editor_surface_wires_search_highlights_into_line_rendering() {
        let component_source = include_str!("component.rs");

        assert!(component_source.contains("search_highlights_in_range"));
        assert!(component_source.contains("line_search_highlights"));
        assert!(component_source.contains(".child(render_line_text("));
        assert!(component_source.contains("search_highlights: Vec<GpuiSearchHighlight>"));
    }

    #[test]
    fn fixture_boundary_builds_reusable_gpui_editor_document() {
        let document = document_by_fixture_id("dialer-proxy").expect("dialer fixture should exist");

        assert_eq!(document.title, SAMPLE_TITLE);
        assert_eq!(document.cursor, TextPosition::new(10, 17));
        assert!(document.line_count > 0);
        assert_eq!(document.line_at(0).unwrap().number, 1);
        assert!(document.completion_labels.contains(&"exit".to_string()));
    }

    // ── chain preview tests ──

    #[test]
    fn chain_preview_shows_steps_for_dialer_proxy_position() {
        let document = GpuiEditorDocument::from_text(
            "chain.yaml",
            "proxies:\n  - name: entry\n    type: ss\n    dialer-proxy: mid\n  - name: mid\n    type: ss\n    dialer-proxy: exit\n  - name: exit\n    type: ss\n",
            TextPosition::new(3, 20),
        );

        let preview = document.chain_preview.as_ref().unwrap();
        assert_eq!(
            preview.steps,
            vec!["Local", "entry", "mid", "exit", "Target"]
        );
        assert_eq!(preview.status, ProxyChainStatus::Complete);
        assert!(preview.is_definite);
    }

    #[test]
    fn chain_preview_shows_missing_reference_for_bad_target() {
        let document = GpuiEditorDocument::from_text(
            "missing.yaml",
            "proxies:\n  - name: entry\n    type: ss\n    dialer-proxy: missing\n",
            TextPosition::new(3, 22),
        );

        let preview = document.chain_preview.as_ref().unwrap();
        assert_eq!(preview.status, ProxyChainStatus::MissingReference);
        assert!(
            preview
                .message
                .as_deref()
                .is_some_and(|msg| msg.contains("missing"))
        );
        assert!(!preview.is_definite);
    }

    #[test]
    fn chain_preview_shows_cycle_for_cycle_fixture() {
        let document = GpuiEditorDocument::from_text(
            "cycle.yaml",
            include_str!("../../../tests/fixtures/dialer-cycle.yaml"),
            TextPosition::new(10, 20),
        );

        let preview = document.chain_preview.as_ref().unwrap();
        assert_eq!(preview.status, ProxyChainStatus::Cycle);
        assert!(
            preview
                .message
                .as_deref()
                .is_some_and(|msg| msg.contains("cycle"))
        );
        assert!(!preview.is_definite);
    }

    #[test]
    fn chain_preview_is_none_for_non_dialer_proxy_position() {
        let document = GpuiEditorDocument::from_text(
            "not-chain.yaml",
            "mixed-port: 7890\n",
            TextPosition::new(0, 0),
        );

        assert!(document.chain_preview.is_none());
    }

    // ── vertical navigation tests ──

    #[test]
    fn move_up_down_clears_selection() {
        let mut document = GpuiEditorDocument::from_text(
            "scratch.yaml",
            "line one\nline two\nline three\n",
            TextPosition::new(1, 4),
        );

        // Extend selection with shift-right
        document.select_right().unwrap();
        assert!(document.selected_text().is_some());
        assert_eq!(document.cursor, TextPosition::new(1, 5));

        // move_up clears selection, preserves column (5 fits on "line one"=8 chars)
        document.move_up().unwrap();
        assert!(document.selected_text().is_none());
        assert_eq!(document.cursor, TextPosition::new(0, 5));
    }

    #[test]
    fn move_down_clamps_column_to_shorter_line() {
        let mut document = GpuiEditorDocument::from_text(
            "test.yaml",
            "abc\nvery long line here\n",
            TextPosition::new(0, 2),
        );

        document.move_down().unwrap();
        // Column preserved on longer line
        assert_eq!(document.cursor, TextPosition::new(1, 2));

        document.move_up().unwrap();
        // Column still fits on "abc" (len 3)
        assert_eq!(document.cursor, TextPosition::new(0, 2));
    }

    #[test]
    fn move_up_clamps_from_long_to_short_line() {
        let mut document = GpuiEditorDocument::from_text(
            "test.yaml",
            "abc\nvery long line here\n",
            TextPosition::new(1, 10),
        );

        document.move_up().unwrap();
        assert_eq!(document.cursor, TextPosition::new(0, 3)); // clamped to "abc".len()
    }

    #[test]
    fn select_up_and_down_extend_selection() {
        let mut document = GpuiEditorDocument::from_text(
            "test.yaml",
            "alpha\nbeta\ngamma\n",
            TextPosition::new(1, 2),
        );

        document.select_up().unwrap();
        assert_eq!(document.cursor, TextPosition::new(0, 2));
        // Selection spans from anchor (1,2) to cursor (0,2), crossing lines
        let sel = document.selected_text().unwrap();
        assert!(!sel.is_empty(), "selection should be non-empty: {sel:?}");

        // select_down returns to original position — now anchor == cursor
        document.select_down().unwrap();
        assert_eq!(document.cursor, TextPosition::new(1, 2));
        assert!(
            document.selected_text().is_none(),
            "selection should clear when anchor==cursor"
        );
    }

    #[test]
    fn home_end_navigation_clears_selection() {
        let mut document =
            GpuiEditorDocument::from_text("test.yaml", "alpha\nbeta\n", TextPosition::new(1, 2));

        document.move_line_start().unwrap();
        assert_eq!(document.cursor, TextPosition::new(1, 0));
        assert!(document.selected_text().is_none());

        document.move_line_end().unwrap();
        assert_eq!(document.cursor, TextPosition::new(1, 4)); // "beta" is 4 chars
        assert!(document.selected_text().is_none());
    }

    #[test]
    fn page_up_down_navigation() {
        let text: String = (0..30).map(|i| format!("line {i}\n")).collect();
        let mut document =
            GpuiEditorDocument::from_text("tall.yaml", &text, TextPosition::new(20, 5));

        document.page_up().unwrap();
        // 20 - PAGE_LINES(25) = 0 (saturated)
        assert_eq!(document.cursor.line, 0);
        assert!(document.selected_text().is_none());

        document.page_down().unwrap();
        assert_eq!(document.cursor.line, 25);
        assert!(document.selected_text().is_none());
    }

    #[test]
    fn selection_preserved_after_vertical_navigation() {
        let mut document = GpuiEditorDocument::from_text(
            "scratch.yaml",
            "dns:\n  enable: true\n",
            TextPosition::new(1, 2),
        );

        // Select right 6 chars on line 1 (move from col 2 to col 8)
        for _ in 0..6 {
            document.select_right().unwrap();
        }
        assert_eq!(document.cursor, TextPosition::new(1, 8));
        assert_eq!(document.selected_text().unwrap(), "enable");

        // Move up — selection clears, column 8 clamped to "dns:" len (4)
        document.move_up().unwrap();
        assert!(document.selected_text().is_none());
        assert_eq!(document.cursor, TextPosition::new(0, 4));
    }
}
