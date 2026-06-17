# Phase 2 GPUI Demo Plan

## Task Card

Judgment: start the first GUI carrier prototype with GPUI, but keep business behavior inside `mocode-core` and expose only read-only adapter data to the demo.

Scope:
- Add a small core snapshot API suitable for UI adapters.
- Add a testable GPUI demo view model that loads Mihomo YAML from the existing examples.
- Replace the GPUI placeholder with a native GPUI window that renders a three-pane editor shell: line numbers, YAML text, and semantic inspector.

Non-goals:
- Do not implement the full text editor surface.
- Do not implement Floem in this phase.
- Do not add Mihomo core, TUN management, subscription management, system proxy management, or external-controller client features.
- Do not put Mihomo semantic logic in the GPUI UI layer.

Files:
- `crates/mocode-text/src/lib.rs`
- `crates/mocode-core/src/lib.rs`
- `crates/mocode-api/src/lib.rs`
- `crates/mocode-gpui-demo/Cargo.toml`
- `crates/mocode-gpui-demo/src/main.rs`

Tests:
- Unit tests for text line access.
- Unit tests for editor snapshot data.
- Unit tests for GPUI demo view-model data.
- `cargo fmt --all --check`
- `cargo test --workspace`
- `cargo check -p mocode-gpui-demo`

Subagent/worktree:
- No subagent needed; scope is narrow.
- Work in the current repository because the user explicitly asked to start and the baseline is clean.

Commit messages:
- `feat(core): expose editor snapshot helpers`
- `feat(gpui): add initial editor demo shell`

## Steps

1. Add line-level read APIs to `mocode-text`.
2. Add `EditorLine`, `EditorSnapshot`, and `snapshot` helpers to `mocode-core`.
3. Re-export snapshot types from `mocode-api`.
4. Build a `DemoDocument` view model inside `mocode-gpui-demo` and cover it with unit tests.
5. Add `gpui = "0.2.2"` and implement a read-only GPUI shell.
6. Run formatting, tests, and GPUI package check.
7. Commit after each verified milestone and push at the end.
