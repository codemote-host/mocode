mod app;
mod component;
mod fixtures;

fn main() {
    app::run();
}

#[cfg(test)]
mod tests {
    use mocode_api::TextPosition;

    use crate::{
        component::GpuiEditorDocument,
        fixtures::{SAMPLE_TITLE, default_fixture, document_by_fixture_id, document_from_fixture},
    };

    fn load_demo_document() -> GpuiEditorDocument {
        document_from_fixture(default_fixture())
    }

    fn load_fixture_by_id(id: &str) -> Option<GpuiEditorDocument> {
        document_by_fixture_id(id)
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
        assert_eq!(document.lines[0].number, 1);
        assert_eq!(document.lines[0].text, "mixed-port: 7890");
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

        assert!(document.lines.iter().any(|line| line.diagnostic_count > 0));
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
        let document =
            GpuiEditorDocument::from_text("large-20000.yaml", text, TextPosition::new(0, 0));

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
        let mut document = GpuiEditorDocument::from_text(
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
        assert_eq!(document.lines[0].number, 1);
        assert!(document.completion_labels.contains(&"exit".to_string()));
    }
}
