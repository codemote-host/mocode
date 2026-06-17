# mocode

mocode is a Rust-native Mihomo Config Editor Component. It is a lightweight YAML editor core that understands Mihomo configuration semantics: fields, references, rules, proxy chains, diagnostics, hover docs, completions, and formatting policy.

mocode is not a general IDE, not a Zed editor extraction, not a full Mihomo GUI, and not a proxy core. GPUI and Floem demos are planned only as UI adapters over the same UI-independent `mocode-core`.

## Workspace

```text
mocode/
  crates/
    mocode-text/            # Rope text model, positions, ranges, edits
    mocode-yaml/            # YAML parsing/path/error/formatting boundary
    mocode-mihomo-schema/   # Mihomo field docs, enums, snippets
    mocode-mihomo-lint/     # semantic index and lint diagnostics
    mocode-core/            # editor orchestration API
    mocode-api/             # public component facade
    mocode-gpui-demo/       # GPUI adapter prototype, no Mihomo logic
    mocode-floem-demo/      # Floem adapter prototype, no Mihomo logic
  docs/
  examples/configs/
  tests/fixtures/
```

## Quick Start

The current implementation includes the first UI-independent editor core and an initial GPUI adapter shell.

```powershell
cargo test --workspace
cargo run -p mocode-gpui-demo
cargo run -p mocode-floem-demo
```

`mocode-gpui-demo` uses the shared `mocode-api` facade and keeps Mihomo semantics out of the UI layer. Current upstream GPUI README documents Windows support through Win32 windowing and DirectWrite text, so the demo builds the same GPUI adapter on Windows, macOS, and Linux.

`mocode-floem-demo` is still a placeholder until the Floem prototype phase.

## Development Roadmap

1. Phase 0: research, specs, docs, workspace skeleton.
2. Phase 1: UI-independent `mocode-text`, `mocode-yaml`, `mocode-mihomo-schema`, `mocode-mihomo-lint`, and `mocode-core`.
3. Phase 2: GPUI prototype with the same acceptance checklist.
4. Phase 3: Floem prototype with the same acceptance checklist.
5. Phase 4: evaluation report.
6. Phase 5: choose UI framework and continue componentization.

See [docs/roadmap.md](docs/roadmap.md).
