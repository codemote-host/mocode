# GPUI Semantic Interactions Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add the first semantic interaction layer to the GPUI prototype: line diagnostics, hover documentation, a basic completion panel, and a large YAML fixture for scrolling checks.

**Architecture:** `mocode-core` exposes UI-neutral semantic view data derived from existing diagnostics, hover, and completion APIs. `mocode-gpui-demo` stores only display state and renders semantic data from the shared core; it does not implement Mihomo schema or lint logic.

**Tech Stack:** Rust, mocode-core, mocode-api, GPUI 0.2.2, tree-sitter-yaml, yaml-rust2.

---

## Task Card

Judgment: continue the GPUI prototype until it demonstrates Mihomo-aware editing value. Do not start Floem until the GPUI baseline includes semantic diagnostics, hover docs, and completions.

Scope:
- Add UI-neutral semantic line data in `mocode-core`.
- Add hover and completion summaries to the GPUI demo state model.
- Render diagnostic markers on editor rows.
- Render hover documentation and diagnostic details in the right inspector.
- Render a basic completion panel near the editor surface.
- Add a generated large Mihomo YAML fixture for later scrolling checks.

Non-goals:
- No full completion accept/apply workflow yet.
- No rich syntax highlighting.
- No multi-cursor or complex selection.
- No Floem implementation.
- No Mihomo runtime GUI features.
- No Mihomo semantic logic in `mocode-gpui-demo`.

Files:
- `docs/superpowers/plans/2026-06-18-gpui-semantic-interactions.md`
- `crates/mocode-core/src/lib.rs`
- `crates/mocode-api/src/lib.rs`
- `crates/mocode-gpui-demo/src/main.rs`
- `examples/configs/large.yaml`
- `README.md`

Tests:
- `cargo test -p mocode-core`
- `cargo test -p mocode-gpui-demo`
- `cargo fmt --all --check`
- `cargo check -p mocode-gpui-demo`
- `cargo test --workspace`

Commits:
- `feat(core): expose semantic line view data`
- `feat(gpui): show semantic diagnostics and completions`
- `test(fixtures): add large mihomo config sample`

## Task 1: Core Semantic View Data

- [ ] Write failing tests for `MocodeEditor::semantic_lines()` returning line text plus line diagnostics.
- [ ] Write failing tests for `MocodeEditor::hover_summary_at()` returning title and first documentation paragraph.
- [ ] Run `cargo test -p mocode-core` and confirm failures are missing APIs.
- [ ] Implement minimal UI-neutral structs in `mocode-core`.
- [ ] Re-export new structs through `mocode-api`.
- [ ] Run `cargo test -p mocode-core -p mocode-api`.

## Task 2: GPUI Demo Semantic State

- [ ] Write failing tests for `DemoDocument` carrying line diagnostic counts/severities, hover title/body, and completion items.
- [ ] Run `cargo test -p mocode-gpui-demo` and confirm failures are missing state fields.
- [ ] Extend `DemoDocument::refresh_derived()` to use core semantic data.
- [ ] Run `cargo test -p mocode-gpui-demo`.

## Task 3: GPUI Semantic Rendering

- [ ] Render line diagnostic markers in the gutter.
- [ ] Render a hover documentation block in the inspector.
- [ ] Render diagnostic details with line/column text.
- [ ] Render a compact completion panel below the header.
- [ ] Run `cargo check -p mocode-gpui-demo`.

## Task 4: Large Fixture

- [ ] Add `examples/configs/large.yaml` with 5000+ lines based on repeated realistic Mihomo sections.
- [ ] Update README with the fixture purpose and GPUI demo scope.
- [ ] Run `cargo test --workspace`.

## Task 5: Verification and Publish

- [ ] Run `cargo fmt --all --check`.
- [ ] Run `cargo check -p mocode-gpui-demo`.
- [ ] Run `cargo test --workspace`.
- [ ] Commit verified milestones.
- [ ] Push `master` to `origin/master`.
