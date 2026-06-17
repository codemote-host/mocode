#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValueKind {
    Bool,
    Integer,
    String,
    Sequence,
    Mapping,
    RuleString,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompletionKind {
    Field,
    EnumValue,
    Reference,
    Snippet,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldDoc {
    pub path: &'static str,
    pub kind: ValueKind,
    pub summary: &'static str,
    pub details: &'static str,
    pub enum_values: &'static [&'static str],
    pub source_url: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaCompletion {
    pub label: String,
    pub insert_text: String,
    pub kind: CompletionKind,
    pub documentation: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SchemaCatalog {
    fields: &'static [FieldDoc],
}

impl Default for SchemaCatalog {
    fn default() -> Self {
        Self {
            fields: DEFAULT_FIELDS,
        }
    }
}

impl SchemaCatalog {
    pub fn default_catalog() -> Self {
        Self::default()
    }

    pub fn field_doc(&self, path: &str) -> Option<&FieldDoc> {
        self.fields.iter().find(|field| field.path == path)
    }

    pub fn root_field_completions(&self) -> Vec<SchemaCompletion> {
        self.fields
            .iter()
            .filter_map(|field| field.path.split_once('.').is_none().then_some(field))
            .map(|field| SchemaCompletion {
                label: field.path.to_string(),
                insert_text: format!("{}: ", field.path),
                kind: CompletionKind::Field,
                documentation: Some(field.summary.to_string()),
            })
            .collect()
    }

    pub fn enum_completions(&self, path: &str) -> Vec<SchemaCompletion> {
        self.field_doc(path)
            .map(|field| {
                field
                    .enum_values
                    .iter()
                    .map(|value| SchemaCompletion {
                        label: (*value).to_string(),
                        insert_text: (*value).to_string(),
                        kind: CompletionKind::EnumValue,
                        documentation: Some(field.summary.to_string()),
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
}

pub const BUILTIN_OUTBOUNDS: &[&str] = &[
    "DIRECT",
    "REJECT",
    "REJECT-DROP",
    "PASS",
    "COMPATIBLE",
    "GLOBAL",
];

pub const DEFAULT_FIELDS: &[FieldDoc] = &[
    FieldDoc {
        path: "mixed-port",
        kind: ValueKind::Integer,
        summary: "HTTP and SOCKS mixed inbound port.",
        details: "The mixed port accepts both HTTP(S) and SOCKS clients.",
        enum_values: &[],
        source_url: "https://wiki.metacubex.one/en/config/inbound/port/",
    },
    FieldDoc {
        path: "mode",
        kind: ValueKind::String,
        summary: "Mihomo operation mode.",
        details: "Common values are rule, global, and direct.",
        enum_values: &["rule", "global", "direct"],
        source_url: "https://wiki.metacubex.one/en/config/general/",
    },
    FieldDoc {
        path: "log-level",
        kind: ValueKind::String,
        summary: "Runtime log verbosity.",
        details: "Controls core log output.",
        enum_values: &["silent", "error", "warning", "info", "debug"],
        source_url: "https://wiki.metacubex.one/en/config/general/",
    },
    FieldDoc {
        path: "dns",
        kind: ValueKind::Mapping,
        summary: "DNS resolver configuration.",
        details: "Configures resolver mode, nameservers, fake IP, and policies.",
        enum_values: &[],
        source_url: "https://wiki.metacubex.one/en/config/dns/",
    },
    FieldDoc {
        path: "dns.enhanced-mode",
        kind: ValueKind::String,
        summary: "DNS enhanced mode.",
        details: "Controls DNS mapping behavior.",
        enum_values: &["normal", "fake-ip", "redir-host"],
        source_url: "https://wiki.metacubex.one/en/config/dns/",
    },
    FieldDoc {
        path: "tun",
        kind: ValueKind::Mapping,
        summary: "TUN inbound configuration.",
        details: "Configures stack, routing, DNS hijack, and platform filters.",
        enum_values: &[],
        source_url: "https://wiki.metacubex.one/en/config/inbound/tun/",
    },
    FieldDoc {
        path: "tun.stack",
        kind: ValueKind::String,
        summary: "TUN network stack.",
        details: "Typical values are system, gvisor, and mixed.",
        enum_values: &["system", "gvisor", "mixed"],
        source_url: "https://wiki.metacubex.one/en/config/inbound/tun/",
    },
    FieldDoc {
        path: "proxies",
        kind: ValueKind::Sequence,
        summary: "Outbound proxy definitions.",
        details: "Each item defines a named outbound proxy.",
        enum_values: &[],
        source_url: "https://wiki.metacubex.one/en/config/proxies/",
    },
    FieldDoc {
        path: "proxy-groups",
        kind: ValueKind::Sequence,
        summary: "Named strategy groups.",
        details: "Groups reference proxies, other groups, built-ins, and providers.",
        enum_values: &[],
        source_url: "https://wiki.metacubex.one/en/config/proxy-groups/",
    },
    FieldDoc {
        path: "rules",
        kind: ValueKind::Sequence,
        summary: "Routing rules.",
        details: "Rules are scalar strings whose target usually names an outbound.",
        enum_values: &[],
        source_url: "https://wiki.metacubex.one/en/config/rules/",
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_enum_completions() {
        let catalog = SchemaCatalog::default_catalog();
        let labels: Vec<_> = catalog
            .enum_completions("mode")
            .into_iter()
            .map(|item| item.label)
            .collect();

        assert_eq!(labels, vec!["rule", "global", "direct"]);
    }
}
