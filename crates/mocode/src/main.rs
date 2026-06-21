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

    use gpui::{Bounds, point, px};
    use mocode_api::{ProxyChainStatus, SyntaxHighlightKind, TextPosition};

    use crate::{
        app,
        component::{
            EditorCommandMode, GpuiEditorDocument, GpuiEditorSaveError, GpuiSearchHighlight,
            MouseDownSelectionPolicy, find_bar_label, go_to_line_bar_label,
            mouse_down_selection_policy,
        },
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
    fn daily_open_path_blocks_dirty_document() {
        let original_path = write_temp_yaml("dirty-open-original", "mixed-port: 7890\n");
        let target_path = write_temp_yaml("dirty-open-target", "mode: direct\n");
        let mut document = GpuiEditorDocument::from_path(&original_path).unwrap();

        document.insert_text("# unsaved\n").unwrap();
        let opened = document.open_path_if_clean(&target_path).unwrap();

        assert!(!opened);
        assert_eq!(document.path.as_deref(), Some(original_path.as_path()));
        assert_eq!(document.title, file_name(&original_path));
        assert_eq!(document.text(), "# unsaved\nmixed-port: 7890\n");
        assert!(document.dirty);
        assert!(document.save_status.contains("Unsaved changes"));

        fs::remove_file(original_path).ok();
        fs::remove_file(target_path).ok();
    }

    #[test]
    fn daily_open_path_after_save_replaces_document() {
        let original_path = write_temp_yaml("clean-open-original", "mixed-port: 7890\n");
        let target_path = write_temp_yaml("clean-open-target", "mode: direct\n");
        let mut document = GpuiEditorDocument::from_path(&original_path).unwrap();

        document.insert_text("# saved\n").unwrap();
        document.save_to_original_path().unwrap();
        let opened = document.open_path_if_clean(&target_path).unwrap();

        assert!(opened);
        assert_eq!(document.path.as_deref(), Some(target_path.as_path()));
        assert_eq!(document.title, file_name(&target_path));
        assert_eq!(document.text(), "mode: direct\n");
        assert!(!document.dirty);
        assert!(document.save_status.contains("Opened"));

        let backup_path = GpuiEditorDocument::backup_path_for(&original_path);
        fs::remove_file(original_path).ok();
        fs::remove_file(target_path).ok();
        fs::remove_file(backup_path).ok();
    }

    #[test]
    fn daily_undo_back_to_saved_text_clears_dirty() {
        let path = write_temp_yaml("undo-clean", "mixed-port: 7890\n");
        let mut document = GpuiEditorDocument::from_path(&path).unwrap();

        document.insert_text("# temporary\n").unwrap();
        assert!(document.dirty);

        document.undo().unwrap();

        assert_eq!(document.text(), "mixed-port: 7890\n");
        assert!(!document.dirty);

        fs::remove_file(path).ok();
    }

    #[test]
    fn daily_redo_after_clean_undo_marks_dirty_again() {
        let path = write_temp_yaml("redo-dirty", "mixed-port: 7890\n");
        let mut document = GpuiEditorDocument::from_path(&path).unwrap();

        document.insert_text("# temporary\n").unwrap();
        document.undo().unwrap();
        assert!(!document.dirty);

        document.redo().unwrap();

        assert_eq!(document.text(), "# temporary\nmixed-port: 7890\n");
        assert!(document.dirty);

        fs::remove_file(path).ok();
    }

    #[test]
    fn daily_save_updates_dirty_baseline() {
        let path = write_temp_yaml("save-baseline", "mixed-port: 7890\n");
        let mut document = GpuiEditorDocument::from_path(&path).unwrap();

        document.insert_text("# saved\n").unwrap();
        document.save_to_original_path().unwrap();
        assert!(!document.dirty);

        document.insert_text("# transient\n").unwrap();
        assert!(document.dirty);
        document.undo().unwrap();

        assert_eq!(document.text(), "# saved\nmixed-port: 7890\n");
        assert!(!document.dirty);

        let backup_path = GpuiEditorDocument::backup_path_for(&path);
        fs::remove_file(path).ok();
        fs::remove_file(backup_path).ok();
    }

    #[test]
    fn open_action_checks_unsaved_changes_before_file_prompt() {
        let app_source = include_str!("app.rs");

        assert!(app_source.contains("has_unsaved_changes()"));
        assert!(
            app_source.find("has_unsaved_changes()").unwrap()
                < app_source.find("prompt_for_paths").unwrap()
        );
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
    fn app_document_state_label_distinguishes_saved_unsaved_and_read_only() {
        let path = write_temp_yaml("state-label", "mixed-port: 7890\n");
        let mut saved = GpuiEditorDocument::from_path(&path).unwrap();
        let read_only = load_app_document();

        assert_eq!(app::document_state_label(&saved), "Saved");
        assert_eq!(app::document_state_label(&read_only), "Read-only");

        saved.insert_text("# unsaved\n").unwrap();

        assert_eq!(app::document_state_label(&saved), "Unsaved");

        fs::remove_file(path).ok();
    }

    #[test]
    fn app_document_activity_label_hides_redundant_modified_state() {
        let path = write_temp_yaml("activity-label", "mixed-port: 7890\n");
        let mut document = GpuiEditorDocument::from_path(&path).unwrap();

        document.insert_text("# unsaved\n").unwrap();
        assert_eq!(document.save_status, "Modified");

        assert_eq!(app::document_activity_label(&document), None);

        document.save_to_original_path().unwrap();
        assert!(
            app::document_activity_label(&document)
                .as_deref()
                .is_some_and(|label| label.contains("Saved"))
        );

        let backup_path = GpuiEditorDocument::backup_path_for(&path);
        fs::remove_file(path).ok();
        fs::remove_file(backup_path).ok();
    }

    #[test]
    fn app_header_uses_compact_document_chrome_helpers() {
        let app_source = include_str!("app.rs");

        assert!(app_source.contains("document_state_label(document)"));
        assert!(app_source.contains("document_activity_label(document)"));
        assert!(!app_source.contains("dirty { \"dirty\" } else { \"clean\" }"));
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
    fn daily_jump_to_ranged_diagnostic_moves_cursor_to_error() {
        let mut document = GpuiEditorDocument::from_text(
            "invalid-yaml.yaml",
            include_str!("../../../examples/configs/invalid-yaml.yaml"),
            TextPosition::new(0, 0),
        );
        let diagnostic_index = document
            .diagnostics
            .iter()
            .position(|diagnostic| diagnostic.line.is_some() && diagnostic.column.is_some())
            .unwrap();
        let diagnostic = document.diagnostics[diagnostic_index].clone();

        assert!(document.jump_to_diagnostic(diagnostic_index));

        assert_eq!(
            document.cursor,
            TextPosition::new(
                diagnostic.line.unwrap().saturating_sub(1),
                diagnostic.column.unwrap().saturating_sub(1)
            )
        );
        assert_eq!(document.selection_summary, "<none>");
    }

    #[test]
    fn daily_jump_to_missing_reference_diagnostic_moves_cursor_to_reference() {
        let mut document = GpuiEditorDocument::from_text(
            "invalid-reference.yaml",
            include_str!("../../../examples/configs/invalid-reference.yaml"),
            TextPosition::new(0, 0),
        );
        let diagnostic_index = document
            .diagnostics
            .iter()
            .position(|diagnostic| {
                diagnostic.code == "mihomo.reference.missing"
                    && diagnostic.message.contains("missing-dialer")
            })
            .unwrap();
        let diagnostic = document.diagnostics[diagnostic_index].clone();

        assert!(document.jump_to_diagnostic(diagnostic_index));

        assert_eq!(diagnostic.line, Some(11));
        assert_eq!(diagnostic.column, Some(19));
        assert_eq!(document.cursor, TextPosition::new(10, 18));
        assert_eq!(
            document
                .line_at(document.cursor.line as usize)
                .unwrap()
                .text,
            "    dialer-proxy: missing-dialer"
        );
    }

    #[test]
    fn daily_current_line_diagnostic_summary_shows_reference_error() {
        let document = GpuiEditorDocument::from_text(
            "invalid-reference.yaml",
            include_str!("../../../examples/configs/invalid-reference.yaml"),
            TextPosition::new(10, 18),
        );

        let summary = document.current_line_diagnostic_summary().unwrap();

        assert!(summary.starts_with("error: "));
        assert!(summary.contains("missing-dialer"));
    }

    #[test]
    fn daily_line_at_carries_diagnostic_message_for_inline_hint() {
        let document = GpuiEditorDocument::from_text(
            "invalid-reference.yaml",
            include_str!("../../../examples/configs/invalid-reference.yaml"),
            TextPosition::new(10, 18),
        );
        let line = document.line_at(10).unwrap();

        assert_eq!(line.diagnostic_count, 1);
        assert_eq!(line.diagnostic_severity.as_deref(), Some("error"));
        assert!(
            line.diagnostic_message
                .as_deref()
                .is_some_and(|message| message.contains("missing-dialer"))
        );
    }

    #[test]
    fn diagnostics_strip_is_rendered_with_click_to_jump() {
        let component_source = include_str!("component.rs");

        assert!(component_source.contains("diagnostics_strip::<T>(editor.document(), cx)"));
        assert!(component_source.contains("jump_to_diagnostic(index)"));
        assert!(component_source.contains("DiagnosticItem"));
    }

    #[test]
    fn editor_surface_renders_current_line_diagnostic_hint() {
        let component_source = include_str!("component.rs");

        assert!(component_source.contains("line_diagnostic_hint("));
        assert!(component_source.contains("current_line_diagnostic_summary()"));
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
                && item.insert_text == "fake-ip"
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
        assert_eq!(popup.left_px, 199.0);
        assert_eq!(popup.top_px, 88.0);
        assert_eq!(popup.selected_index, 0);
        assert!(popup.items.iter().any(|item| item.label == "exit"
            && item.insert_text == "exit"
            && item.kind == "reference"));
    }

    #[test]
    fn completion_popup_tracks_selected_item_for_rendering() {
        let mut document =
            GpuiEditorDocument::from_text("scratch.yaml", "mode: \n", TextPosition::new(0, 6));

        document.select_next_completion();

        let popup = document.completion_popup.as_ref().unwrap();
        assert_eq!(popup.selected_index, 1);
        assert_eq!(popup.items[popup.selected_index].label, "global");
        assert_eq!(popup.left_px, 109.0);
        assert_eq!(popup.top_px, 22.0);
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
    fn daily_accept_completion_replaces_typed_field_prefix() {
        let mut document =
            GpuiEditorDocument::from_text("scratch.yaml", "mi\n", TextPosition::new(0, 2));

        assert!(
            document
                .completion_labels
                .contains(&"mixed-port".to_string())
        );
        assert!(document.accept_completion().unwrap());

        assert_eq!(document.line_at(0).unwrap().text, "mixed-port: ");
        assert_eq!(document.cursor, TextPosition::new(0, 12));
    }

    #[test]
    fn daily_accept_completion_replaces_typed_enum_prefix() {
        let mut document = GpuiEditorDocument::from_text(
            "scratch.yaml",
            "dns:\n  enhanced-mode: fa\n",
            TextPosition::new(1, 19),
        );

        assert!(document.completion_labels.contains(&"fake-ip".to_string()));
        assert!(document.accept_completion().unwrap());

        assert_eq!(
            document.line_at(1).unwrap().text,
            "  enhanced-mode: fake-ip"
        );
        assert_eq!(document.cursor, TextPosition::new(1, 24));
    }

    #[test]
    fn daily_completion_down_selects_next_item_for_acceptance() {
        let mut document =
            GpuiEditorDocument::from_text("scratch.yaml", "mode: \n", TextPosition::new(0, 6));

        assert_eq!(
            document
                .completion_popup
                .as_ref()
                .unwrap()
                .items
                .iter()
                .map(|item| item.label.as_str())
                .collect::<Vec<_>>(),
            vec!["rule", "global", "direct"]
        );
        assert!(document.select_next_completion());
        assert_eq!(
            document.completion_popup.as_ref().unwrap().selected_index,
            1
        );
        assert!(document.accept_completion().unwrap());

        assert_eq!(document.line_at(0).unwrap().text, "mode: global");
        assert_eq!(document.cursor, TextPosition::new(0, 12));
    }

    #[test]
    fn daily_completion_click_accepts_item_by_index() {
        let mut document =
            GpuiEditorDocument::from_text("scratch.yaml", "mode: \n", TextPosition::new(0, 6));

        assert!(document.accept_completion_at(1).unwrap());

        assert_eq!(document.line_at(0).unwrap().text, "mode: global");
        assert_eq!(document.cursor, TextPosition::new(0, 12));
        assert!(document.completion_popup.is_none());
    }

    #[test]
    fn daily_completion_click_rejects_out_of_range_index() {
        let mut document =
            GpuiEditorDocument::from_text("scratch.yaml", "mode: \n", TextPosition::new(0, 6));

        assert!(!document.accept_completion_at(99).unwrap());
        assert_eq!(document.line_at(0).unwrap().text, "mode: ");
        assert_eq!(document.cursor, TextPosition::new(0, 6));
    }

    #[test]
    fn daily_completion_up_wraps_to_last_item_for_acceptance() {
        let mut document =
            GpuiEditorDocument::from_text("scratch.yaml", "mode: \n", TextPosition::new(0, 6));

        assert!(document.select_previous_completion());
        assert_eq!(
            document.completion_popup.as_ref().unwrap().selected_index,
            2
        );
        assert!(document.accept_completion().unwrap());

        assert_eq!(document.line_at(0).unwrap().text, "mode: direct");
        assert_eq!(document.cursor, TextPosition::new(0, 12));
    }

    #[test]
    fn daily_completion_selection_returns_false_without_popup() {
        let mut document = GpuiEditorDocument::from_text(
            "scratch.yaml",
            "mixed-port: 7890\n",
            TextPosition::new(0, 16),
        );

        assert!(document.completion_popup.is_none());
        assert!(!document.select_next_completion());
        assert!(!document.select_previous_completion());
    }

    #[test]
    fn daily_close_completion_popup_suppresses_same_cursor_refresh() {
        let mut document =
            GpuiEditorDocument::from_text("scratch.yaml", "mode: \n", TextPosition::new(0, 6));

        assert!(document.completion_popup.is_some());
        assert!(document.close_completion_popup());
        assert!(document.completion_popup.is_none());

        document.begin_selection_at(TextPosition::new(0, 6));
        document.finish_selection();

        assert!(document.completion_popup.is_none());
    }

    #[test]
    fn daily_typing_after_closing_completion_popup_allows_new_popup() {
        let mut document =
            GpuiEditorDocument::from_text("scratch.yaml", "mode: \n", TextPosition::new(0, 6));

        assert!(document.close_completion_popup());
        document.insert_text("g").unwrap();

        let popup = document.completion_popup.as_ref().unwrap();
        assert_eq!(popup.anchor_line, 1);
        assert_eq!(popup.anchor_column, 8);
        assert!(popup.items.iter().any(|item| item.label == "global"));
    }

    #[test]
    fn daily_closed_completion_popup_is_not_accepted_while_hidden() {
        let mut document =
            GpuiEditorDocument::from_text("scratch.yaml", "mode: \n", TextPosition::new(0, 6));

        assert!(document.close_completion_popup());

        assert!(!document.accept_completion().unwrap());
        assert_eq!(document.line_at(0).unwrap().text, "mode: ");
        assert_eq!(document.cursor, TextPosition::new(0, 6));
    }

    #[test]
    fn daily_accept_completion_ignores_plain_blank_indent() {
        let mut document =
            GpuiEditorDocument::from_text("scratch.yaml", "dns:\n  \n", TextPosition::new(1, 2));

        assert!(!document.accept_completion().unwrap());
        assert_eq!(document.line_at(1).unwrap().text, "  ");
        assert_eq!(document.cursor, TextPosition::new(1, 2));
    }

    #[test]
    fn mode_search_routes_text_backspace_enter_escape_without_editing_yaml() {
        let mut document = GpuiEditorDocument::from_text(
            "search.yaml",
            "alpha\nbeta alpha\n",
            TextPosition::new(0, 0),
        );
        let original_text = document.text();

        document.start_search_from_selection();

        assert_eq!(document.command_mode(), EditorCommandMode::Search);
        assert!(document.route_command_text_input("alpha"));
        assert_eq!(document.text(), original_text);
        assert_eq!(document.search_query, "alpha");

        assert!(document.handle_command_backspace().unwrap().handled);
        assert_eq!(document.text(), original_text);
        assert_eq!(document.search_query, "alph");

        assert!(document.handle_command_enter().unwrap().handled);
        assert_eq!(document.text(), original_text);
        assert!(document.search_active);

        assert!(document.handle_command_escape().unwrap().handled);
        assert_eq!(document.command_mode(), EditorCommandMode::Normal);
        assert_eq!(document.text(), original_text);
    }

    #[test]
    fn mode_search_delete_is_handled_without_editing_yaml() {
        let mut document =
            GpuiEditorDocument::from_text("search.yaml", "alpha\n", TextPosition::new(0, 0));
        let original_text = document.text();

        document.start_search_from_selection();

        assert_eq!(document.command_mode(), EditorCommandMode::Search);
        assert!(document.handle_command_delete().unwrap().handled);
        assert_eq!(document.text(), original_text);
        assert_eq!(document.command_mode(), EditorCommandMode::Search);
    }

    #[test]
    fn mode_search_text_input_does_not_edit_yaml_if_completion_popup_lingers() {
        let mut document = GpuiEditorDocument::from_text(
            "dns.yaml",
            "dns:\n  enhanced-mode: f\n",
            TextPosition::new(1, 19),
        );
        let original_text = document.text();
        document.search_active = true;

        assert_eq!(document.command_mode(), EditorCommandMode::Completion);
        assert!(document.route_command_text_input("alpha"));
        assert_eq!(document.text(), original_text);
        assert_eq!(document.search_query, "alpha");
    }

    #[test]
    fn mode_search_backspace_does_not_edit_yaml_if_completion_popup_lingers() {
        let mut document = GpuiEditorDocument::from_text(
            "dns.yaml",
            "dns:\n  enhanced-mode: f\n",
            TextPosition::new(1, 19),
        );
        let original_text = document.text();
        document.search_active = true;
        document.search_query = "alpha".to_string();

        assert_eq!(document.command_mode(), EditorCommandMode::Completion);
        assert!(document.handle_command_backspace().unwrap().handled);
        assert_eq!(document.text(), original_text);
        assert_eq!(document.search_query, "alph");
    }

    #[test]
    fn mode_search_enter_does_not_accept_completion_if_completion_popup_lingers() {
        let mut document = GpuiEditorDocument::from_text(
            "dns.yaml",
            "dns:\n  enhanced-mode: f\ndns:\n",
            TextPosition::new(1, 19),
        );
        let original_text = document.text();
        document.search_active = true;
        document.search_query = "dns".to_string();

        assert_eq!(document.command_mode(), EditorCommandMode::Completion);
        assert!(document.handle_command_enter().unwrap().handled);
        assert_eq!(document.text(), original_text);
        assert!(document.search_active);
        assert_eq!(document.selected_text().as_deref(), Some("dns"));
        assert_eq!(document.cursor, TextPosition::new(0, 3));
    }

    #[test]
    fn mode_search_tab_is_consumed_without_accepting_completion_if_completion_popup_lingers() {
        let mut document = GpuiEditorDocument::from_text(
            "dns.yaml",
            "dns:\n  enhanced-mode: f\n",
            TextPosition::new(1, 19),
        );
        let original_text = document.text();
        document.search_active = true;
        document.search_query = "alpha".to_string();

        assert_eq!(document.command_mode(), EditorCommandMode::Completion);
        assert!(document.handle_command_tab().unwrap().handled);
        assert_eq!(document.text(), original_text);
        assert_eq!(document.search_query, "alpha");
        assert!(document.search_active);
    }

    #[test]
    fn mode_search_escape_closes_stale_completion_before_search() {
        let mut document = GpuiEditorDocument::from_text(
            "dns.yaml",
            "dns:\n  enhanced-mode: f\n",
            TextPosition::new(1, 19),
        );
        let original_text = document.text();
        document.search_active = true;
        document.search_query = "alpha".to_string();

        assert_eq!(document.command_mode(), EditorCommandMode::Completion);
        assert!(document.handle_command_escape().unwrap().handled);
        assert_eq!(document.text(), original_text);
        assert!(document.search_active);
        assert_eq!(document.command_mode(), EditorCommandMode::Search);

        assert!(document.handle_command_escape().unwrap().handled);
        assert_eq!(document.text(), original_text);
        assert_eq!(document.command_mode(), EditorCommandMode::Normal);
    }

    #[test]
    fn mode_command_outcome_distinguishes_consumed_keys_from_cursor_reveal() {
        let mut invalid_jump = GpuiEditorDocument::from_text(
            "jump.yaml",
            "mixed-port: 7890\nmode: rule\n",
            TextPosition::new(0, 0),
        );
        invalid_jump.start_go_to_line();
        invalid_jump.append_go_to_line_input("x");

        let invalid_jump_outcome = invalid_jump.handle_command_enter().unwrap();
        assert!(invalid_jump_outcome.handled);
        assert!(!invalid_jump_outcome.reveal_cursor);

        let mut missing_search =
            GpuiEditorDocument::from_text("search.yaml", "alpha\n", TextPosition::new(0, 0));
        missing_search.set_search_query("missing");

        let missing_search_outcome = missing_search.handle_command_enter().unwrap();
        assert!(missing_search_outcome.handled);
        assert!(!missing_search_outcome.reveal_cursor);

        let mut search_delete =
            GpuiEditorDocument::from_text("search.yaml", "alpha\n", TextPosition::new(0, 0));
        search_delete.set_search_query("alpha");

        let search_delete_outcome = search_delete.handle_command_delete().unwrap();
        assert!(search_delete_outcome.handled);
        assert!(!search_delete_outcome.reveal_cursor);

        let mut go_to_line_tab =
            GpuiEditorDocument::from_text("jump.yaml", "alpha\n", TextPosition::new(0, 0));
        go_to_line_tab.start_go_to_line();

        let go_to_line_tab_outcome = go_to_line_tab.handle_command_tab().unwrap();
        assert!(go_to_line_tab_outcome.handled);
        assert!(!go_to_line_tab_outcome.reveal_cursor);
    }

    #[test]
    fn mode_go_to_line_routes_text_backspace_enter_escape_without_editing_yaml() {
        let mut document = GpuiEditorDocument::from_text(
            "jump.yaml",
            "mixed-port: 7890\nmode: rule\nlog-level: info",
            TextPosition::new(0, 0),
        );
        let original_text = document.text();

        document.start_go_to_line();

        assert_eq!(document.command_mode(), EditorCommandMode::GoToLine);
        assert!(document.route_command_text_input("3x"));
        assert_eq!(document.text(), original_text);
        assert_eq!(document.go_to_line_query, "3");

        assert!(document.handle_command_backspace().unwrap().handled);
        assert_eq!(document.text(), original_text);
        assert_eq!(document.go_to_line_query, "");

        assert!(document.route_command_text_input("2"));
        assert!(document.handle_command_enter().unwrap().handled);
        assert_eq!(document.text(), original_text);
        assert_eq!(document.cursor, TextPosition::new(1, 0));
        assert_eq!(document.command_mode(), EditorCommandMode::Normal);

        document.start_go_to_line();
        assert!(document.handle_command_escape().unwrap().handled);
        assert_eq!(document.command_mode(), EditorCommandMode::Normal);
        assert_eq!(document.text(), original_text);
    }

    #[test]
    fn mode_go_to_line_delete_is_handled_without_editing_yaml() {
        let mut document =
            GpuiEditorDocument::from_text("jump.yaml", "a\nb\n", TextPosition::new(0, 0));
        let original_text = document.text();

        document.start_go_to_line();

        assert_eq!(document.command_mode(), EditorCommandMode::GoToLine);
        assert!(document.handle_command_delete().unwrap().handled);
        assert_eq!(document.text(), original_text);
        assert_eq!(document.command_mode(), EditorCommandMode::GoToLine);
    }

    #[test]
    fn mode_completion_accepts_enter_tab_and_escape_closes_before_search() {
        let mut document = GpuiEditorDocument::from_text(
            "dns.yaml",
            "dns:\n  enhanced-mode: f\n",
            TextPosition::new(1, 19),
        );

        assert_eq!(document.command_mode(), EditorCommandMode::Completion);
        assert!(document.handle_command_enter().unwrap().handled);
        assert_eq!(
            document.line_at(1).unwrap().text,
            "  enhanced-mode: fake-ip"
        );
        assert_eq!(document.command_mode(), EditorCommandMode::Normal);

        let mut tab_document = GpuiEditorDocument::from_text(
            "dns.yaml",
            "dns:\n  enhanced-mode: f\n",
            TextPosition::new(1, 19),
        );

        assert_eq!(tab_document.command_mode(), EditorCommandMode::Completion);
        assert!(tab_document.handle_command_tab().unwrap().handled);
        assert_eq!(
            tab_document.line_at(1).unwrap().text,
            "  enhanced-mode: fake-ip"
        );
        assert_eq!(tab_document.command_mode(), EditorCommandMode::Normal);

        let mut escape_document = GpuiEditorDocument::from_text(
            "dns.yaml",
            "dns:\n  enhanced-mode: f\n",
            TextPosition::new(1, 19),
        );
        escape_document.search_active = true;

        assert_eq!(
            escape_document.command_mode(),
            EditorCommandMode::Completion
        );
        assert!(escape_document.handle_command_escape().unwrap().handled);
        assert!(escape_document.search_active);
        assert_eq!(escape_document.command_mode(), EditorCommandMode::Search);
        assert!(escape_document.handle_command_escape().unwrap().handled);
        assert_eq!(escape_document.command_mode(), EditorCommandMode::Normal);
    }

    #[test]
    fn completion_acceptance_is_wired_before_tab_and_enter_fallbacks() {
        let component_source = include_str!("component.rs");

        assert!(component_source.contains("handle_command_tab()"));
        assert!(component_source.contains("handle_command_enter()"));
        assert!(component_source.contains("accept_completion()?"));
        assert!(component_source.contains("select_next_completion"));
        assert!(component_source.contains("select_previous_completion"));
        assert!(component_source.contains("EditorCommandMode::Completion"));
        assert!(component_source.contains("EditorCommandMode::Normal"));
        assert!(component_source.contains("else if this.editor_component_mut().move_down()"));
        assert!(component_source.contains("else if this.editor_component_mut().move_up()"));
    }

    #[test]
    fn escape_action_closes_completion_popup_before_search_fallback() {
        let component_source = include_str!("component.rs");

        assert!(component_source.contains("close_completion_popup()"));
        assert!(
            component_source.find("close_completion_popup()").unwrap()
                < component_source.find("close_search()").unwrap()
        );
    }

    #[test]
    fn editor_surface_renders_inline_completion_popup() {
        let component_source = include_str!("component.rs");

        assert!(component_source.contains("completion_popup::<T>("));
        assert!(component_source.contains("CompletionItemSelected"));
        assert!(component_source.contains("popup.left_px"));
        assert!(component_source.contains("popup.top_px"));
        assert!(component_source.contains("accept_completion_at(index)"));
        assert!(component_source.contains("MouseButton::Left"));
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
    fn enter_inherits_current_line_indent() {
        let mut document = GpuiEditorDocument::from_text(
            "scratch.yaml",
            "dns:\n  enable: true\n",
            TextPosition::new(1, 14),
        );

        document.insert_newline().unwrap();

        assert_eq!(document.line_at(2).unwrap().text, "  ");
        assert_eq!(document.cursor, TextPosition::new(2, 2));
    }

    #[test]
    fn enter_indents_after_mapping_key() {
        let mut document =
            GpuiEditorDocument::from_text("scratch.yaml", "dns:\n", TextPosition::new(0, 4));

        document.insert_newline().unwrap();

        assert_eq!(document.line_at(1).unwrap().text, "  ");
        assert_eq!(document.cursor, TextPosition::new(1, 2));
    }

    #[test]
    fn enter_indents_after_list_item() {
        let mut document = GpuiEditorDocument::from_text(
            "scratch.yaml",
            "proxies:\n  - \n",
            TextPosition::new(1, 4),
        );

        document.insert_newline().unwrap();

        assert_eq!(document.line_at(2).unwrap().text, "    ");
        assert_eq!(document.cursor, TextPosition::new(2, 4));
    }

    #[test]
    fn enter_action_uses_auto_indent_path() {
        let component_source = include_str!("component.rs");

        assert!(component_source.contains("insert_newline"));
        assert!(!component_source.contains("insert_text(\"\\n\")"));
    }

    #[test]
    fn paste_multiline_normalizes_following_lines_to_current_indent() {
        let mut document = GpuiEditorDocument::from_text(
            "scratch.yaml",
            "dns:\n  nameserver:\n    \n",
            TextPosition::new(2, 4),
        );

        document.insert_pasted_text("- 1.1.1.1\n- 8.8.8.8").unwrap();

        assert_eq!(document.line_at(2).unwrap().text, "    - 1.1.1.1");
        assert_eq!(document.line_at(3).unwrap().text, "    - 8.8.8.8");
        assert_eq!(document.cursor, TextPosition::new(3, 13));
    }

    #[test]
    fn paste_multiline_keeps_first_line_as_pasted() {
        let mut document = GpuiEditorDocument::from_text(
            "scratch.yaml",
            "rules:\n  - \n",
            TextPosition::new(1, 4),
        );

        document
            .insert_pasted_text("DOMAIN,example.com,Proxy\nDOMAIN-SUFFIX,example.org,Proxy")
            .unwrap();

        assert_eq!(
            document.line_at(1).unwrap().text,
            "  - DOMAIN,example.com,Proxy"
        );
        assert_eq!(
            document.line_at(2).unwrap().text,
            "  DOMAIN-SUFFIX,example.org,Proxy"
        );
    }

    #[test]
    fn paste_single_line_keeps_text_unchanged() {
        let mut document = GpuiEditorDocument::from_text(
            "scratch.yaml",
            "dns:\n  enhanced-mode: \n",
            TextPosition::new(1, 17),
        );

        document.insert_pasted_text("fake-ip").unwrap();

        assert_eq!(
            document.line_at(1).unwrap().text,
            "  enhanced-mode: fake-ip"
        );
        assert_eq!(document.cursor, TextPosition::new(1, 24));
    }

    #[test]
    fn paste_action_uses_paste_specific_indent_path() {
        let component_source = include_str!("component.rs");

        assert!(component_source.contains("insert_pasted_text(&text)"));
    }

    #[test]
    fn daily_toggle_comment_comments_current_line_after_indent() {
        let mut document = GpuiEditorDocument::from_text(
            "scratch.yaml",
            "dns:\n  enable: true\n",
            TextPosition::new(1, 4),
        );

        document.toggle_line_comment().unwrap();

        assert_eq!(document.line_at(1).unwrap().text, "  # enable: true");
        assert_eq!(document.cursor, TextPosition::new(1, 6));
    }

    #[test]
    fn daily_toggle_comment_uncomments_current_line() {
        let mut document = GpuiEditorDocument::from_text(
            "scratch.yaml",
            "dns:\n  # enable: true\n",
            TextPosition::new(1, 6),
        );

        document.toggle_line_comment().unwrap();

        assert_eq!(document.line_at(1).unwrap().text, "  enable: true");
        assert_eq!(document.cursor, TextPosition::new(1, 4));
    }

    #[test]
    fn daily_toggle_comment_comments_selected_lines() {
        let mut document = GpuiEditorDocument::from_text(
            "scratch.yaml",
            "dns:\n  enable: true\n  enhanced-mode: fake-ip\n",
            TextPosition::new(1, 2),
        );
        document.select_down().unwrap();

        document.toggle_line_comment().unwrap();

        assert_eq!(document.line_at(1).unwrap().text, "  # enable: true");
        assert_eq!(
            document.line_at(2).unwrap().text,
            "  # enhanced-mode: fake-ip"
        );
        assert!(document.selected_text().is_none());
    }

    #[test]
    fn daily_toggle_comment_uncomments_selected_lines_when_all_are_commented() {
        let mut document = GpuiEditorDocument::from_text(
            "scratch.yaml",
            "dns:\n  # enable: true\n  # enhanced-mode: fake-ip\n",
            TextPosition::new(1, 2),
        );
        document.select_down().unwrap();

        document.toggle_line_comment().unwrap();

        assert_eq!(document.line_at(1).unwrap().text, "  enable: true");
        assert_eq!(
            document.line_at(2).unwrap().text,
            "  enhanced-mode: fake-ip"
        );
        assert!(document.selected_text().is_none());
    }

    #[test]
    fn toggle_comment_action_is_bound_to_common_shortcut() {
        let component_source = include_str!("component.rs");

        assert!(component_source.contains("ToggleComment"));
        assert!(component_source.contains("KeyBinding::new(\"ctrl-/\""));
        assert!(component_source.contains("KeyBinding::new(\"cmd-/\""));
        assert!(component_source.contains("toggle_line_comment().is_ok()"));
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
    fn cjk_selection_copy_delete_and_replace_uses_character_columns() {
        let mut document = GpuiEditorDocument::from_text(
            "scratch.yaml",
            "proxies:\n  - name: 香港节点\n",
            TextPosition::new(1, 10),
        );

        for _ in 0.."香港节点".chars().count() {
            document.select_right().unwrap();
        }

        assert_eq!(document.selected_text().unwrap(), "香港节点");
        assert_eq!(document.copy_selection_text().unwrap(), "香港节点");
        assert_eq!(document.selection_summary, "2:11 -> 2:15");

        document.delete().unwrap();
        assert_eq!(document.line_at(1).unwrap().text, "  - name: ");
        assert_eq!(document.cursor, TextPosition::new(1, 10));

        document.insert_text("台湾节点").unwrap();
        assert_eq!(document.line_at(1).unwrap().text, "  - name: 台湾节点");
        assert_eq!(document.cursor, TextPosition::new(1, 14));
    }

    #[test]
    fn ime_preedit_commit_replaces_cjk_selection_without_leaking_marked_text() {
        let mut document = GpuiEditorDocument::from_text(
            "scratch.yaml",
            "proxies:\n  - name: 旧节点\n",
            TextPosition::new(1, 10),
        );

        for _ in 0.."旧节点".chars().count() {
            document.select_right().unwrap();
        }
        let (selected_utf16, _) = document.selected_utf16_range();

        document
            .replace_and_mark_utf16_range(Some(selected_utf16), "xianggang", Some(9..9))
            .unwrap();
        assert_eq!(document.line_at(1).unwrap().text, "  - name: xianggang");
        assert_eq!(document.marked_utf16_range(), Some(19..28));
        assert!(document.selected_text().is_none());

        document.replace_utf16_range(None, "香港节点").unwrap();

        assert_eq!(document.line_at(1).unwrap().text, "  - name: 香港节点");
        assert_eq!(document.cursor, TextPosition::new(1, 14));
        assert_eq!(document.marked_utf16_range(), None);
        assert!(document.selected_text().is_none());
    }

    #[test]
    fn cjk_bounds_for_utf16_range_use_painted_character_widths() {
        let document =
            GpuiEditorDocument::from_text("scratch.yaml", "a中b\n", TextPosition::new(0, 2));
        let bounds = Bounds::from_corners(point(px(0.0), px(0.0)), point(px(400.0), px(200.0)));

        let range_bounds = document.bounds_for_utf16_range(2..2, bounds).unwrap();

        assert_eq!(f32::from(range_bounds.left()), 64.0 + 7.5 + 15.0);
    }

    #[test]
    fn grapheme_bounds_for_utf16_range_do_not_split_combining_mark() {
        let document =
            GpuiEditorDocument::from_text("scratch.yaml", "e\u{0301}x\n", TextPosition::new(0, 1));
        let bounds = Bounds::from_corners(point(px(0.0), px(0.0)), point(px(400.0), px(200.0)));

        let range_bounds = document.bounds_for_utf16_range(1..1, bounds).unwrap();

        assert_eq!(f32::from(range_bounds.left()), 64.0);
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
    fn mouse_double_click_selects_yaml_identifier_token() {
        let mut document = GpuiEditorDocument::from_text(
            "scratch.yaml",
            "proxy-groups:\n  - name: fallback.group\n",
            TextPosition::new(0, 0),
        );

        assert!(document.select_yaml_identifier_at(TextPosition::new(1, 13)));

        assert_eq!(document.selected_text().unwrap(), "fallback.group");
        assert_eq!(document.cursor, TextPosition::new(1, 24));
        assert_eq!(document.selection_summary, "2:11 -> 2:25");
    }

    #[test]
    fn mouse_double_click_selects_decomposed_grapheme_identifier() {
        let mut document = GpuiEditorDocument::from_text(
            "scratch.yaml",
            "name: e\u{0301}x\n",
            TextPosition::new(0, 0),
        );

        assert!(document.select_yaml_identifier_at(TextPosition::new(0, 6)));

        assert_eq!(document.selected_text().unwrap(), "e\u{0301}x");
        assert_eq!(document.cursor, TextPosition::new(0, 9));
        assert_eq!(document.selection_summary, "1:7 -> 1:10");
    }

    #[test]
    fn mouse_double_click_selects_cjk_identifier() {
        let mut document = GpuiEditorDocument::from_text(
            "scratch.yaml",
            "name: 香港节点\n",
            TextPosition::new(0, 0),
        );

        assert!(document.select_yaml_identifier_at(TextPosition::new(0, 7)));

        assert_eq!(document.selected_text().unwrap(), "香港节点");
        assert_eq!(document.cursor, TextPosition::new(0, 10));
        assert_eq!(document.selection_summary, "1:7 -> 1:11");
    }

    #[test]
    fn mouse_double_click_punctuation_falls_back_to_single_click_cursor() {
        let mut document =
            GpuiEditorDocument::from_text("scratch.yaml", "name: value\n", TextPosition::new(0, 0));

        document.apply_mouse_down_selection(TextPosition::new(0, 4), 2, false);
        document.finish_selection();

        assert_eq!(document.cursor, TextPosition::new(0, 4));
        assert!(document.selected_text().is_none());
        assert_eq!(document.selection_summary, "<none>");
    }

    #[test]
    fn mouse_triple_click_selects_line_content_without_newline() {
        let mut document = GpuiEditorDocument::from_text(
            "scratch.yaml",
            "dns:\n  enhanced-mode: fake-ip\nmode: rule\n",
            TextPosition::new(0, 0),
        );

        assert!(document.select_line_content_at(TextPosition::new(1, 5)));

        assert_eq!(
            document.selected_text().unwrap(),
            "  enhanced-mode: fake-ip"
        );
        assert_eq!(document.cursor, TextPosition::new(1, 24));
        assert_eq!(document.selection_summary, "2:1 -> 2:25");
    }

    #[test]
    fn mouse_triple_click_empty_line_moves_cursor_without_selection() {
        let mut document = GpuiEditorDocument::from_text(
            "scratch.yaml",
            "dns:\n\nmode: rule\n",
            TextPosition::new(0, 0),
        );

        document.apply_mouse_down_selection(TextPosition::new(1, 0), 3, false);
        document.finish_selection();

        assert_eq!(document.cursor, TextPosition::new(1, 0));
        assert!(document.selected_text().is_none());
        assert_eq!(document.selection_summary, "<none>");
    }

    #[test]
    fn shift_click_extends_selection_from_existing_anchor_forward() {
        let mut document = GpuiEditorDocument::from_text(
            "scratch.yaml",
            "dns:\n  enhanced-mode: fake-ip\n",
            TextPosition::new(1, 2),
        );

        document.extend_selection_to(TextPosition::new(1, 15));

        assert_eq!(document.selected_text().unwrap(), "enhanced-mode");
        assert_eq!(document.cursor, TextPosition::new(1, 15));
        assert_eq!(document.selection_summary, "2:3 -> 2:16");
    }

    #[test]
    fn shift_click_extends_selection_from_existing_anchor_reversed() {
        let mut document = GpuiEditorDocument::from_text(
            "scratch.yaml",
            "dns:\n  enhanced-mode: fake-ip\n",
            TextPosition::new(1, 15),
        );

        document.extend_selection_to(TextPosition::new(1, 2));

        assert_eq!(document.selected_text().unwrap(), "enhanced-mode");
        assert_eq!(document.cursor, TextPosition::new(1, 2));
        assert_eq!(document.selection_summary, "2:3 -> 2:16");
    }

    #[test]
    fn mouse_down_selection_policy_maps_click_count_and_shift() {
        assert_eq!(
            mouse_down_selection_policy(1, false),
            MouseDownSelectionPolicy::Single
        );
        assert_eq!(
            mouse_down_selection_policy(2, false),
            MouseDownSelectionPolicy::Word
        );
        assert_eq!(
            mouse_down_selection_policy(3, false),
            MouseDownSelectionPolicy::Line
        );
        assert_eq!(
            mouse_down_selection_policy(2, true),
            MouseDownSelectionPolicy::Extend
        );
    }

    #[test]
    fn editor_surface_wires_mouse_drag_selection_handlers() {
        let component_source = include_str!("component.rs");

        assert!(component_source.contains(".on_mouse_down("));
        assert!(component_source.contains(".on_mouse_move("));
        assert!(component_source.contains(".on_mouse_up("));
        assert!(component_source.contains("apply_mouse_down_selection"));
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
        assert!(!document.dirty);
        assert_eq!(document.selection_summary, "<none>");

        document.redo().unwrap();
        assert_eq!(document.cursor, TextPosition::new(1, 24));
        assert_eq!(
            document.line_at(1).unwrap().text,
            "  enhanced-mode: fake-ip"
        );
        assert!(document.dirty);
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
    fn daily_select_next_match_selects_identifier_at_cursor() {
        let mut document = GpuiEditorDocument::from_text(
            "select.yaml",
            "mode: rule\nmode: global",
            TextPosition::new(0, 1),
        );

        assert!(document.select_next_match());

        assert_eq!(document.selected_text().unwrap(), "mode");
        assert_eq!(document.cursor, TextPosition::new(0, 4));
        assert_eq!(document.selection_summary, "1:1 -> 1:5");
    }

    #[test]
    fn daily_select_next_match_moves_to_next_occurrence() {
        let mut document = GpuiEditorDocument::from_text(
            "select.yaml",
            "mode: rule\nmode: global",
            TextPosition::new(0, 1),
        );

        assert!(document.select_next_match());
        assert!(document.select_next_match());

        assert_eq!(document.selected_text().unwrap(), "mode");
        assert_eq!(document.cursor, TextPosition::new(1, 4));
        assert_eq!(document.selection_summary, "2:1 -> 2:5");
    }

    #[test]
    fn daily_select_next_match_keeps_manual_selection_text() {
        let mut document = GpuiEditorDocument::from_text(
            "select.yaml",
            "proxy one\nproxy two\nproxy one",
            TextPosition::new(0, 0),
        );
        for _ in 0..9 {
            document.select_right().unwrap();
        }

        assert_eq!(document.selected_text().unwrap(), "proxy one");

        assert!(document.select_next_match());

        assert_eq!(document.selected_text().unwrap(), "proxy one");
        assert_eq!(document.cursor, TextPosition::new(2, 9));
        assert_eq!(document.selection_summary, "3:1 -> 3:10");
    }

    #[test]
    fn daily_select_next_match_returns_false_without_identifier() {
        let mut document =
            GpuiEditorDocument::from_text("select.yaml", "mode: rule", TextPosition::new(0, 5));

        assert!(!document.select_next_match());
        assert!(document.selected_text().is_none());
        assert_eq!(document.cursor, TextPosition::new(0, 5));
    }

    #[test]
    fn select_next_match_action_is_bound_to_common_shortcut() {
        let component_source = include_str!("component.rs");

        assert!(component_source.contains("SelectNextMatch"));
        assert!(component_source.contains("KeyBinding::new(\"ctrl-d\", SelectNextMatch"));
        assert!(component_source.contains("KeyBinding::new(\"cmd-d\", SelectNextMatch"));
        assert!(component_source.contains("select_next_match()"));
    }

    #[test]
    fn daily_go_to_line_jumps_to_requested_line() {
        let mut document = GpuiEditorDocument::from_text(
            "jump.yaml",
            "mixed-port: 7890\nmode: rule\nlog-level: info",
            TextPosition::new(0, 0),
        );

        document.start_go_to_line();
        document.append_go_to_line_input("3");

        assert!(document.submit_go_to_line());
        assert_eq!(document.cursor, TextPosition::new(2, 0));
        assert!(!document.go_to_line_active);
        assert_eq!(document.go_to_line_summary, "Line 3");
        assert!(document.selected_text().is_none());
    }

    #[test]
    fn daily_go_to_line_clamps_to_last_line() {
        let mut document = GpuiEditorDocument::from_text(
            "jump.yaml",
            "mixed-port: 7890\nmode: rule\nlog-level: info",
            TextPosition::new(0, 0),
        );

        document.start_go_to_line();
        document.append_go_to_line_input("99");

        assert!(document.submit_go_to_line());
        assert_eq!(document.cursor, TextPosition::new(2, 0));
        assert_eq!(document.go_to_line_summary, "Line 3");
    }

    #[test]
    fn daily_go_to_line_filters_digits_and_supports_backspace() {
        let mut document = GpuiEditorDocument::from_text(
            "jump.yaml",
            "mixed-port: 7890\nmode: rule\nlog-level: info",
            TextPosition::new(0, 0),
        );

        document.start_go_to_line();
        document.append_go_to_line_input("1a2");
        document.go_to_line_backspace();

        assert_eq!(document.go_to_line_query, "1");
        assert_eq!(document.go_to_line_summary, "Go to line 1");
    }

    #[test]
    fn go_to_line_bar_tracks_active_query() {
        let mut document = GpuiEditorDocument::from_text(
            "jump.yaml",
            "mixed-port: 7890\nmode: rule\nlog-level: info",
            TextPosition::new(0, 0),
        );

        assert_eq!(go_to_line_bar_label(&document), None);

        document.start_go_to_line();
        assert_eq!(
            go_to_line_bar_label(&document),
            Some("Go to line 1-3".to_string())
        );

        document.append_go_to_line_input("2");
        assert_eq!(
            go_to_line_bar_label(&document),
            Some("Go to line 2".to_string())
        );

        document.close_go_to_line();
        assert_eq!(go_to_line_bar_label(&document), None);
    }

    #[test]
    fn go_to_line_action_is_bound_to_ctrl_g_and_enter_escape() {
        let component_source = include_str!("component.rs");

        assert!(component_source.contains("GoToLine"));
        assert!(component_source.contains("KeyBinding::new(\"ctrl-g\", GoToLine"));
        assert!(component_source.contains("start_go_to_line()"));
        assert!(component_source.contains("submit_go_to_line()"));
        assert!(component_source.contains("close_go_to_line()"));
    }

    #[test]
    fn text_input_routes_to_go_to_line_before_search_or_buffer() {
        let app_source = include_str!("app.rs");
        let command_route_index = app_source.find("route_command_text_input").unwrap();
        let replace_index = app_source.find("replace_utf16_range").unwrap();

        assert!(command_route_index < replace_index);
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
    fn find_bar_hidden_when_search_is_inactive() {
        let document =
            GpuiEditorDocument::from_text("search.yaml", "alpha\n", TextPosition::new(0, 0));

        assert_eq!(find_bar_label(&document), None);
    }

    #[test]
    fn find_bar_shows_query_and_match_summary_when_active() {
        let mut document = GpuiEditorDocument::from_text(
            "search.yaml",
            "alpha\nbeta alpha\n",
            TextPosition::new(0, 0),
        );

        document.set_search_query("alpha");
        document.find_next();

        assert_eq!(
            find_bar_label(&document),
            Some("Find: alpha - 1/2 at 1:1".to_string())
        );
    }

    #[test]
    fn find_bar_disappears_after_escape_closes_search() {
        let mut document =
            GpuiEditorDocument::from_text("search.yaml", "alpha\n", TextPosition::new(0, 0));

        document.set_search_query("alpha");
        document.find_next();
        assert!(find_bar_label(&document).is_some());

        document.close_search();

        assert_eq!(find_bar_label(&document), None);
    }

    #[test]
    fn status_bar_renders_inline_find_bar_without_search_panel() {
        let component_source = include_str!("component.rs");

        assert!(component_source.contains("find_bar_label(document)"));
        assert!(component_source.contains("fn find_bar("));
        assert!(component_source.contains("status_bar(editor.document())"));
        assert!(!component_source.contains("search_panel("));
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
    fn editor_document_exposes_syntax_highlights_from_visible_core_slice() {
        let document = GpuiEditorDocument::from_text(
            "syntax.yaml",
            "# hidden\nmixed-port: 7890\n",
            TextPosition::new(1, 0),
        );

        let line = document.line_at(1).unwrap();

        assert!(line.syntax_highlights.iter().any(|highlight| {
            highlight.start == 0
                && highlight.end == 10
                && highlight.kind == SyntaxHighlightKind::Key
        }));
        assert!(line.syntax_highlights.iter().any(|highlight| {
            highlight.start == 12
                && highlight.end == 16
                && highlight.kind == SyntaxHighlightKind::Number
        }));
    }

    #[test]
    fn editor_lines_in_range_carries_distinct_syntax_highlight_kinds() {
        let document = GpuiEditorDocument::from_text(
            "syntax.yaml",
            "# comment\nmixed-port: 7890\nallow-lan: true\n",
            TextPosition::new(1, 0),
        );

        let lines = document.lines_in_range(0, 3);

        assert!(
            lines[0]
                .syntax_highlights
                .iter()
                .any(|highlight| highlight.kind == SyntaxHighlightKind::Comment)
        );
        assert!(
            lines[1]
                .syntax_highlights
                .iter()
                .any(|highlight| highlight.kind == SyntaxHighlightKind::Key)
        );
        assert!(
            lines[1]
                .syntax_highlights
                .iter()
                .any(|highlight| highlight.kind == SyntaxHighlightKind::Number)
        );
        assert!(
            lines[2]
                .syntax_highlights
                .iter()
                .any(|highlight| highlight.kind == SyntaxHighlightKind::Boolean)
        );
    }

    #[test]
    fn editor_surface_wires_syntax_highlights_into_line_rendering() {
        let component_source = include_str!("component.rs");

        assert!(component_source.contains("syntax_highlights: Vec<GpuiSyntaxHighlight>"));
        assert!(component_source.contains("line.syntax_highlights"));
        assert!(component_source.contains("syntax_highlights_for_line"));
        assert!(component_source.contains("SyntaxHighlightKind::Error"));
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
