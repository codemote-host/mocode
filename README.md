# mocode

mocode is a Rust-native Mihomo Config Editor Component. It is a lightweight YAML editor that understands Mihomo configuration semantics: fields, references, rules, proxy chains, diagnostics, hover docs, completions, and conservative formatting policy.

mocode is not a general IDE, not a Zed editor extraction, not a full Mihomo GUI, and not a proxy core. The current application shell uses GPUI, while all Mihomo semantics remain in UI-independent crates.

## Workspace

```text
mocode/
  crates/
    mocode/                 # GPUI application shell and component adapter
    mocode-text/            # Rope text model, positions, ranges, edits
    mocode-yaml/            # YAML parsing/path/error/formatting boundary
    mocode-mihomo-schema/   # Mihomo field docs, enums, snippets
    mocode-mihomo-lint/     # semantic index and lint diagnostics
    mocode-core/            # editor orchestration API
    mocode-api/             # public component facade
  docs/
  examples/configs/
  tests/fixtures/
```

## Quick Start

```powershell
cargo test --workspace
cargo run -p mocode
```

Build the Windows executable:

```powershell
cargo build -p mocode
target\debug\mocode.exe
```

Open a real config directly:

```powershell
target\debug\mocode.exe path\to\config.yaml
```

The `mocode` crate uses the shared `mocode-api` facade and keeps Mihomo semantics out of the UI layer. It currently supports a daily editing loop: open a YAML file from the command line or app button, load built-in fixtures, show line numbers, move the cursor, select text, copy, paste, undo, redo, save, save as, search, and refresh YAML path, completion, diagnostics, and proxy-chain panels from core state.

When saving over an existing file, mocode writes a sibling backup named like `config.yaml.bak` before replacing the file. Built-in fixtures are not overwritten; saving them opens the save-as flow.

`examples/configs/large.yaml` is a generated 5000+ line Mihomo sample. `examples/configs/large-20000.yaml` is the current 20000-line loading baseline used by core and app tests.

## Development Roadmap

1. Phase 0: research, specs, docs, workspace skeleton.
2. Phase 1: UI-independent `mocode-text`, `mocode-yaml`, `mocode-mihomo-schema`, `mocode-mihomo-lint`, and `mocode-core`.
3. Phase 2: GPUI application shell and component adapter.
4. Phase 3: editor component hardening: IME, viewport rendering, save/open ergonomics, diagnostics, completions, hover, and chain preview.
5. Phase 4: reusable component API and host integration notes.

See [docs/roadmap.md](docs/roadmap.md), [docs/spec.md](docs/spec.md), and [docs/prototype-validation-checklist.md](docs/prototype-validation-checklist.md).
