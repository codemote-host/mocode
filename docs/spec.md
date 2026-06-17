# Specification

## Product Definition

mocode is a reusable Mihomo Config Editor Component written in Rust. It provides a UI-independent editing core and Mihomo semantic services for YAML configuration files.

The primary users are:

- Mihomo GUI authors who need an embeddable config editor.
- Advanced Mihomo users who edit YAML directly but want semantic assistance.
- Future mocode host apps that need a lightweight editor without owning parser, lint, and completion logic.

## Functional Scope

Phase 1 core scope:

- Open text into a rope-backed document.
- Apply text edits.
- Parse YAML and report syntax errors.
- Compute current YAML path.
- Provide Mihomo field docs and enum completions.
- Build a semantic index for proxies, groups, providers, rule providers, rules, and dialer-proxy references.
- Report missing references and obvious cycle diagnostics.

Prototype UI scope:

- line numbers, cursor, selection, copy/paste, undo/redo baseline
- syntax highlighting and diagnostics rendering
- completion popup and hover popup
- right-side path and proxy-chain panels
- 5000-20000 line file loading and scrolling evaluation
- Chinese IME evaluation

## Non-goals

- No Mihomo core embedding.
- No TUN/device management.
- No system proxy management.
- No subscription manager.
- No WebDAV or sync.
- No dashboard replacement.
- No generic IDE feature set.
- No direct dependency on Zed editor internals.

## Core API Draft

```rust
impl MocodeEditor {
    pub fn open_text(text: impl Into<String>) -> Self;
    pub fn apply_edit(&mut self, edit: TextEdit) -> Result<(), EditorError>;
    pub fn format(&self) -> Result<String, EditorError>;
    pub fn diagnostics(&self) -> Vec<Diagnostic>;
    pub fn completions_at(&self, position: TextPosition) -> Vec<Completion>;
    pub fn hover_at(&self, position: TextPosition) -> Option<Hover>;
    pub fn current_yaml_path(&self, position: TextPosition) -> Option<YamlPath>;
    pub fn semantic_index(&self) -> &SemanticIndex;
    pub fn proxy_chain_preview_at(&self, position: TextPosition) -> Option<ProxyChainPreview>;
    pub fn references_at(&self, position: TextPosition) -> Vec<Reference>;
    pub fn validate(&self) -> Vec<Diagnostic>;
}
```

The API must remain UI-independent. UI adapters pass text edits and positions into `mocode-core`; they do not parse Mihomo semantics.

## Semantic Model

`SemanticIndex` contains:

- named proxy nodes from `proxies[*].name`
- named proxy groups from `proxy-groups[*].name`
- proxy provider names from `proxy-providers` map keys
- rule provider names from `rule-providers` map keys
- listener names and listener routing references
- rule target references
- group member references
- provider `use` references
- `dialer-proxy` graph edges

Every indexed entity should carry a source range when tree-sitter path extraction can provide one.

## Completion Model

Completion sources are layered:

- YAML path and schema context decide which category is valid.
- Schema catalog provides field and enum completions.
- Semantic index provides proxy, group, provider, rule-provider, and built-in target completions.
- Rule templates provide snippets for common Mihomo rule forms.

Examples:

- At root: complete `mixed-port`, `dns`, `tun`, `proxies`, `proxy-groups`, `rules`.
- At `dns.enhanced-mode`: complete `normal`, `fake-ip`, `redir-host` if supported by the active schema version.
- At `proxy-groups[0].proxies[2]`: complete proxies, proxy groups, and built-ins.
- At `proxies[3].dialer-proxy`: complete proxies and proxy groups, excluding the current proxy when possible.
- At `rules[10]`: complete rule templates and known targets.

## Hover Documentation Model

Hover is resolved from:

- YAML path, for fields and enum values.
- Semantic index, for references.
- Rule parser, for tokens inside rule strings.

Hover payload:

- title
- short summary
- detailed markdown
- source URL
- optional examples
- optional risk level

## Diagnostic Model

Diagnostics have:

- severity: error, warning, info, hint
- code
- message
- source range
- optional fix suggestions

Initial diagnostics:

- YAML syntax error
- unknown root field
- missing proxy/group reference
- missing provider reference
- `RULE-SET` points to missing rule provider
- `dialer-proxy` points to missing outbound
- `dialer-proxy` cycle
- empty proxy group without `use` or include-all behavior
- TUN risk hints such as `strict-route` or `auto-route`
- DNS risk hints such as `respect-rules` without `proxy-server-nameserver`
- external controller exposed without secret

## Formatter Model

Formatting is conservative:

- Preserve comments and key order.
- Format generated snippets before insertion.
- Auto-indent on newline and paste.
- Normalize simple indentation only when a range can be rewritten safely.
- Whole-document format is opt-in and should remain unavailable until a lossless strategy is proven.

## Prototype Acceptance

Both GPUI and Floem demos must use the same `mocode-core` API and meet the same checklist:

1. Load 5000-20000 line Mihomo YAML.
2. Smooth scrolling.
3. Line numbers.
4. Cursor movement, text selection, copy, and paste.
5. Chinese IME test.
6. YAML syntax error rendering.
7. Hover docs on Mihomo fields.
8. Field-name completion.
9. Completion in `proxy-groups.proxies`.
10. Completion in `dialer-proxy`.
11. Missing reference diagnostics.
12. `dialer-proxy` cycle diagnostics.
13. Right-side current YAML path panel.
14. Right-side chain preview panel: local -> entry node -> intermediate node -> exit node -> target.
15. No copied business logic between demos.

