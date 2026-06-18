# mocode

mocode is a Rust-native Mihomo Config Editor Component. It is a lightweight YAML editor core that understands Mihomo configuration semantics: fields, references, rules, proxy chains, diagnostics, hover docs, completions, and formatting policy.

mocode is not a general IDE, not a Zed editor extraction, not a full Mihomo GUI, and not a proxy core. GPUI is the selected primary UI framework for the editor component shell. Floem is retained as a frozen reference prototype over the same UI-independent `mocode-core`.

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

The current implementation includes the first UI-independent editor core plus a minimally editable GPUI prototype. The Floem prototype is still buildable as a frozen reference, but new product work targets GPUI first.

```powershell
cargo test --workspace
cargo run -p mocode-gpui-demo
cargo run -p mocode-floem-demo
```

`mocode-gpui-demo` uses the shared `mocode-api` facade and keeps Mihomo semantics out of the UI layer. It currently supports the first editable loop: focus the editor surface, type simple text, use Backspace/Delete, move left/right, use Shift+Left/Right keyboard selection, copy selection with Ctrl/Cmd+C, paste text, switch built-in fixtures from the header, and watch the YAML path/completion popup/diagnostic inspector refresh from core state. Current upstream GPUI README documents Windows support through Win32 windowing and DirectWrite text, so the demo builds the same GPUI adapter on Windows, macOS, and Linux.

`mocode-floem-demo` uses the same `mocode-api` facade and remains as a reference adapter. It should stay buildable while useful, but it is no longer a required parity target for new mocode features.

`examples/configs/large.yaml` is a generated 5000+ line Mihomo sample for prototype loading and scrolling baselines. `examples/configs/large-20000.yaml` is the current 20000-line loading baseline used by core, GPUI adapter, and Floem adapter tests.

## Development Roadmap

1. Phase 0: research, specs, docs, workspace skeleton.
2. Phase 1: UI-independent `mocode-text`, `mocode-yaml`, `mocode-mihomo-schema`, `mocode-mihomo-lint`, and `mocode-core`.
3. Phase 2: GPUI prototype and component shell.
4. Phase 3: Floem reference prototype freeze.
5. Phase 4: GPUI selection decision and validation report.
6. Phase 5: GPUI componentization.

See [docs/roadmap.md](docs/roadmap.md).

The UI framework decision is recorded in [docs/ui-framework-decision.md](docs/ui-framework-decision.md). The GPUI/Floem comparison remains available at [docs/gpui-vs-floem-evaluation.md](docs/gpui-vs-floem-evaluation.md), and the repeatable validation checklist is available at [docs/prototype-validation-checklist.md](docs/prototype-validation-checklist.md).
