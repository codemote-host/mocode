use mocode_api::MocodeEditor;

fn main() {
    let editor = MocodeEditor::open_text("mixed-port: 7890\n");
    println!(
        "mocode GPUI demo placeholder: {} diagnostics",
        editor.diagnostics().len()
    );
}
