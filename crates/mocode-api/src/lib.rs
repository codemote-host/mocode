pub use mocode_core::{
    Completion, Diagnostic, EditorError, Hover, MocodeEditor, ProxyChainPreview, Reference,
};
pub use mocode_mihomo_lint::{DiagnosticSeverity, SemanticIndex};
pub use mocode_mihomo_schema::{CompletionKind, SchemaCatalog, ValueKind};
pub use mocode_text::{Cursor, Selection, TextBuffer, TextEdit, TextPosition, TextRange};
pub use mocode_yaml::{FormatOptions, YamlPath, YamlPathSegment, YamlSyntaxError};

pub mod prelude {
    pub use mocode_core::{MocodeEditor, ProxyChainPreview};
    pub use mocode_text::{TextEdit, TextPosition, TextRange};
    pub use mocode_yaml::YamlPath;
}
