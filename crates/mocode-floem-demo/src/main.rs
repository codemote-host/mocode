use mocode_api::MocodeEditor;

fn main() {
    let editor = MocodeEditor::open_text("mixed-port: 7890\n");
    println!(
        "mocode Floem demo placeholder: {} diagnostics",
        editor.diagnostics().len()
    );
}
