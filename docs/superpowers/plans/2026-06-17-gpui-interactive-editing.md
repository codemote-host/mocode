# GPUI Interactive Editing Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move the GPUI prototype from read-only rendering to a minimal editable YAML surface backed by the shared UI-independent core.

**Architecture:** `mocode-text` owns position math and primitive edits, `mocode-core` owns document state refresh after edits, and `mocode-gpui-demo` adapts keyboard/text input into core edits. GPUI remains a thin adapter and does not perform Mihomo semantic validation itself.

**Tech Stack:** Rust, ropey, tree-sitter-yaml, yaml-rust2, gpui 0.2.2.

---

## Task Card

Judgment: implement only the smallest editing loop needed to compare GPUI as a carrier. Keep rich editor work such as syntax highlighting, hover popups, and real completion UI for later prototype slices.

Scope:
- Add `TextBuffer` helpers for end-of-line lookup, insertion, backspace, delete, and simple cursor movement.
- Add `MocodeEditor` wrapper methods that apply those edits and refresh YAML/semantic state.
- Add a testable `DemoDocument` state model that owns `MocodeEditor`, cursor position, and derived view data.
- Wire GPUI focus, text insertion, backspace/delete, arrow movement, and paste into the demo state.

Non-goals:
- No full editor implementation.
- No Floem work.
- No Mihomo GUI, proxy runtime, TUN management, subscriptions, or external-controller API client.
- No Mihomo semantic logic in GPUI code.
- No complex selection, syntax highlighting, or completion popup yet.

Files:
- `docs/superpowers/plans/2026-06-17-gpui-interactive-editing.md`
- `crates/mocode-text/src/lib.rs`
- `crates/mocode-core/src/lib.rs`
- `crates/mocode-api/src/lib.rs`
- `crates/mocode-gpui-demo/src/main.rs`

Tests:
- `cargo test -p mocode-text`
- `cargo test -p mocode-core`
- `cargo test -p mocode-gpui-demo`
- `cargo fmt --all --check`
- `cargo check -p mocode-gpui-demo`
- `cargo test --workspace`

Commits:
- `feat(text): add cursor edit primitives`
- `feat(core): expose interactive edit helpers`
- `feat(gpui): wire basic editor input`

## Task 1: Text Edit Primitives

- [ ] Write failing tests for `line_end_position`, `insert_text_at`, `backspace_at`, `delete_at`, `move_left`, and `move_right`.
- [ ] Run `cargo test -p mocode-text` and confirm failures are from missing APIs.
- [ ] Implement the minimal `TextBuffer` helpers.
- [ ] Run `cargo test -p mocode-text` and confirm pass.

## Task 2: Core Edit Helpers

- [ ] Write failing tests for `MocodeEditor::insert_text_at`, `backspace_at`, `delete_at`, and `move_*` helpers.
- [ ] Run `cargo test -p mocode-core` and confirm failures are from missing APIs.
- [ ] Implement core wrappers by delegating to `TextBuffer` and reparsing through existing `apply_edit`.
- [ ] Re-export helper-facing types through `mocode-api` if needed.
- [ ] Run `cargo test -p mocode-core -p mocode-api`.

## Task 3: GPUI Demo State Model

- [ ] Write failing tests for `DemoDocument::insert_text`, `backspace`, `delete`, `move_left`, and inspector refresh after edit.
- [ ] Run `cargo test -p mocode-gpui-demo` and confirm failures are from missing state APIs.
- [ ] Refactor `DemoDocument` to own `MocodeEditor`, cursor position, and derived line/path/diagnostic/completion data.
- [ ] Run `cargo test -p mocode-gpui-demo`.

## Task 4: GPUI Input Wiring

- [ ] Add a focused GPUI editor surface with key actions for backspace, delete, left, right, paste, and simple text insertion.
- [ ] Render a cursor marker on the active line using the state model.
- [ ] Keep the right inspector derived from core state.
- [ ] Run `cargo check -p mocode-gpui-demo`.

## Task 5: Verification and Publish

- [ ] Run `cargo fmt --all --check`.
- [ ] Run `cargo check -p mocode-gpui-demo`.
- [ ] Run `cargo test --workspace`.
- [ ] Commit each verified milestone.
- [ ] Push `master` to `origin/master`.
