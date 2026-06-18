pub use mocode_core::{
    Completion, Diagnostic, EditorError, EditorLine, EditorSnapshot, Hover, HoverSummary,
    LineDiagnostic, MocodeEditor, ProxyChainPreview, ProxyChainStatus, Reference, SemanticLine,
};
pub use mocode_mihomo_lint::{DiagnosticSeverity, SemanticIndex};
pub use mocode_mihomo_schema::{CompletionKind, SchemaCatalog, ValueKind};
pub use mocode_text::{Cursor, Selection, TextBuffer, TextEdit, TextPosition, TextRange};
pub use mocode_yaml::{FormatOptions, YamlPath, YamlPathSegment, YamlSyntaxError};

pub mod prelude {
    pub use mocode_core::{MocodeEditor, ProxyChainPreview, ProxyChainStatus};
    pub use mocode_text::{TextEdit, TextPosition, TextRange};
    pub use mocode_yaml::YamlPath;
}
