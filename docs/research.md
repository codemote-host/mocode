# Research

## Summary

mocode should be built as a UI-independent editor core with a Mihomo-aware semantic layer, then rendered through the `mocode` GPUI application shell. The core technical route is:

- `ropey` for large editable text storage.
- `tree-sitter-yaml` for incremental YAML concrete syntax and syntax errors.
- A small, self-maintained Mihomo schema catalog for docs, enums, and snippets.
- A semantic index over `proxies`, `proxy-groups`, `proxy-providers`, `rule-providers`, `rules`, `listeners`, and `dialer-proxy`.
- Diagnostics and completions derived from YAML path plus semantic index.

JSON Schema can describe many fields, but it is not enough for Mihomo-specific references, rule string grammar, `dialer-proxy` graphs, or comment-preserving edits. A custom schema model is the better primary representation; JSON Schema can be generated later if host tools need it.

## Mihomo Configuration Semantics

Official docs and the current `RawConfig` source show Mihomo as one YAML document with mostly root-level keys plus several nested feature blocks. Important top-level areas are:

- General fields: `allow-lan`, `bind-address`, `mode`, `log-level`, `ipv6`, `find-process-mode`, `external-controller`, `secret`, `external-ui`, `tcp-concurrent`, `interface-name`, `routing-mark`, `geox-url`, `profile`, and `tls`.
- Inbound ports: `port`, `socks-port`, `mixed-port`, `redir-port`, and `tproxy-port`.
- `dns`: DNS mode, fake IP settings, nameservers, policies, fallback filters, and resolver routing.
- `tun`: TUN stack, routing, DNS hijack, interfaces, UID/package filters, route include/exclude fields.
- `sniffer`: domain sniffing switches and protocol-specific sniffing ports.
- `listeners`: named inbound definitions with `name`, `type`, `listen`, `port`, optional `rule`, and optional direct `proxy` routing.
- `proxies`: outbound definitions. Each item is a map keyed by `name`, `type`, protocol fields, and optional `dialer-proxy`.
- `proxy-groups`: named strategy groups. Their `proxies` entries reference proxy names, group names, and built-ins; `use` references proxy provider names.
- `proxy-providers`: named provider maps with `type`, `url`, `path`, `interval`, `filter`, `exclude-filter`, `health-check`, `override`, and optional inline `payload`.
- `rules`: routing rules as strings. The last field generally names a proxy, group, or built-in target. `RULE-SET` references `rule-providers`.
- `rule-providers`: named provider maps with `type`, `behavior`, `format`, `path`, `url`, headers, and optional inline `payload`.
- `dialer-proxy`: a proxy-level or provider override field whose value references an outbound proxy or proxy group and creates a chain.
- `external-controller` API: not part of mocode runtime control in the current scope, but fields such as `external-controller`, `external-controller-tls`, `external-controller-unix`, `external-controller-pipe`, `secret`, and CORS need docs and risk hints.

The editor should model Mihomo references explicitly:

- `proxies[*].name`
- `proxy-groups[*].name`
- `proxy-providers.<name>`
- `rule-providers.<name>`
- `rules[*]` target token
- `listeners[*].proxy`
- `listeners[*].rule`
- `proxies[*].dialer-proxy`
- `proxy-providers.<name>.override.dialer-proxy`

Built-in targets such as `DIRECT`, `REJECT`, `REJECT-DROP`, `PASS`, `COMPATIBLE`, and `GLOBAL` must be accepted even if not defined in YAML.

## YAML Editor Technology Route

### Rope

Use `ropey` in `mocode-text`. It stores UTF-8 text as a rope, supports large files, line-to-char lookup, and efficient edits. mocode positions should initially use LSP-like `{ line, character }` where `character` is a Unicode scalar index. The app may additionally map grapheme clusters for cursor painting and IME composition.

### Incremental YAML Parse

Use `tree-sitter-yaml` in `mocode-yaml`. Tree-sitter is designed for concrete syntax trees that can update as text changes and remain useful with syntax errors. It is appropriate for syntax highlighting, error ranges, indentation context, and cursor-to-node lookup.

The parser layer should expose:

- `parse(text, previous_tree, edit)`
- `syntax_errors()`
- `node_at(position)`
- `yaml_path_at(position)`
- `scope_at(position)` for completions

### Typed YAML Decoding

Use typed deserialization only for non-editing analysis that can tolerate losing comments and exact formatting. `serde_yml` is documented as deprecated/unmaintained, so it should not be a primary choice. `yaml-rust2`, `serde-saphyr`, or a direct tree-sitter walker are better candidates for semantic extraction.

### Lossless Formatting

Full YAML round-trip formatting is risky because comments, anchors, flow style, quote style, and ordering matter to users. The first formatter should be conservative:

- normalize indentation around newly inserted blocks
- format snippets before insertion
- avoid whole-document rewrite by default
- offer whole-document format only after a lossless strategy is proven

Rowan-style full-fidelity trees are attractive, but there is no ready Mihomo/YAML lossless editing stack. It is a reference idea, not a near-term dependency.

## JSON Schema vs Custom Schema

JSON Schema strengths:

- easy field/type validation for simple maps
- possible integration with external tools
- standardized enum and docs shape

JSON Schema weaknesses for mocode:

- weak for rule strings such as `DOMAIN-SUFFIX,example.com,Proxy`
- weak for cross-document semantic references
- weak for `dialer-proxy` graph cycle detection
- cannot express many context-sensitive Mihomo hints cleanly
- does not preserve comments or source locations by itself

Decision: maintain a custom Rust schema catalog first. Generate JSON Schema later if useful.

## UI Framework Direction

GPUI is the selected UI framework for the application shell. It is promising for a high-performance editor-like component because it powers a production code editor and has examples for input, key dispatch, uniform lists, windows, and text wrapping. Risks are API maturity, documentation depth, and possible dependence on Zed ecosystem patterns.

Current upstream GPUI README documents Windows support through Win32 windowing and DirectWrite text, with no Windows-specific feature flag required. The app should therefore compile the real GPUI path on Windows.

egui/eframe remains only a practical emergency fallback for quick diagnostics UIs. Its immediate-mode architecture is less suitable for a large-text editor with IME, virtualized layout, hover popups, and rich text selection.

## Risks and Unknowns

- Mihomo schema changes frequently. The schema must be versioned and source-linked.
- YAML grammar edge cases such as anchors, merge keys, block scalars, flow maps, and comments can break naive path lookup.
- `dialer-proxy` can reference groups that include providers, so chain preview must distinguish definite vs possible chains.
- Provider contents may be remote, inline YAML, URI, or base64. The local editor can validate configured provider names but cannot always know remote proxy names.
- Chinese IME behavior must be tested on the GPUI app path before treating the app shell as production-ready.
- Whole-document formatting can damage user layout; near-term work should prefer snippet and indentation formatting.
- Performance target of 5000-20000 lines requires measuring parse latency, semantic-index latency, scrolling, and UI memory separately.

## Sources

- Mihomo general config: https://wiki.metacubex.one/en/config/general/
- Mihomo inbound ports: https://wiki.metacubex.one/en/config/inbound/port/
- Mihomo DNS: https://wiki.metacubex.one/en/config/dns/
- Mihomo TUN: https://wiki.metacubex.one/en/config/inbound/tun/
- Mihomo sniffer: https://wiki.metacubex.one/en/config/sniff/
- Mihomo listeners: https://wiki.metacubex.one/en/config/inbound/listeners/
- Mihomo proxies and dialer-proxy: https://wiki.metacubex.one/en/config/proxies/dialer-proxy/
- Mihomo proxy groups: https://wiki.metacubex.one/en/config/proxy-groups/
- Mihomo rules: https://wiki.metacubex.one/en/config/rules/
- Mihomo proxy providers: https://wiki.metacubex.one/en/config/proxy-providers/
- Mihomo rule providers: https://wiki.metacubex.one/en/config/rule-providers/
- Mihomo external controller API: https://wiki.metacubex.one/en/api/
- Mihomo `RawConfig`: https://raw.githubusercontent.com/MetaCubeX/mihomo/Alpha/config/config.go
- Tree-sitter: https://tree-sitter.github.io/tree-sitter/
- tree-sitter-yaml: https://docs.rs/tree-sitter-yaml/
- Ropey: https://docs.rs/ropey/
- serde_yml status: https://doc.serdeyml.com/serde_yml/
- Rowan: https://docs.rs/rowan/
- GPUI: https://www.gpui.rs/
- GPUI upstream README: https://github.com/zed-industries/zed/blob/main/crates/gpui/README.md
- eframe: https://docs.rs/eframe/latest/eframe/
