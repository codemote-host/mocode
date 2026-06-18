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
        component::{GpuiEditorDocument, GpuiEditorSaveError},
        fixtures::{SAMPLE_TITLE, default_fixture, document_by_fixture_id, document_from_fixture},
    };

    fn load_demo_document() -> GpuiEditorDocument {
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
            "mocode-gpui-demo-{label}-{}-{nanos}.yaml",
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
    fn saving_fixture_without_path_reports_unsaved_state() {
        let mut document = load_demo_document();

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
        assert!(!component_source.contains("DemoFixture"));
        assert!(!component_source.contains("from_fixture"));
    }

    #[test]
    fn fixture_selector_iterates_over_all_registered_fixtures() {
        let app_source = include_str!("app.rs");

        assert!(!app_source.contains("fixtures["));
        assert!(app_source.contains("all_fixtures().iter()"));
    }

    #[test]
    fn builds_demo_document_from_core_snapshot() {
        let document = load_demo_document();

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
    fn backspaces_deletes_and_moves_cursor_in_demo_state() {
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
}
