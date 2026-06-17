# Floem Basic Input Parity Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Bring `mocode-floem-demo` from read-only semantic display to the same basic edit loop already present in the GPUI prototype.

**Architecture:** `DemoDocument` remains the UI adapter state boundary and delegates all text mutation to `MocodeEditor` through `mocode-api`. The Floem view tree stores `DemoDocument` in a reactive signal, translates keyboard/IME/clipboard events into state method calls, and re-renders derived semantic data from shared core state.

**Tech Stack:** Rust, Floem 0.2.0, mocode-api, mocode-core text edit helpers, tree-sitter-yaml, yaml-rust2.

---

## Task Card

Judgment: implement Floem basic input parity now because semantic display parity is already landed, and framework comparison needs the same minimum editing surface on both prototypes.

Scope:
- Add `DemoDocument` edit methods: `insert_text`, `backspace`, `delete`, `move_left`, `move_right`.
- Render a visible cursor in Floem rows.
- Store `DemoDocument` in `RwSignal<DemoDocument>` for reactive rendering.
- Handle focused editor key events for Backspace, Delete, ArrowLeft, ArrowRight, printable characters, and Ctrl/Cmd+V paste.
- Handle `ImeCommit` for composed text input.
- Keep all schema, lint, YAML path, hover, and completion logic inside `mocode-core`.
- Update README to describe Floem as a basic editable prototype.

Non-goals:
- No selection model in Floem.
- No copy command in Floem yet.
- No completion accept/apply flow.
- No syntax highlighting.
- No complete IME preedit rendering; only committed text is inserted.
- No Mihomo GUI features.

Visual thesis:
- Same dense operational editor as the previous Floem semantic demo, with one thin blue cursor bar as the only new interaction affordance.

Content plan:
- Header: unchanged file and line count.
- Completion strip: driven by current cursor path.
- Workspace: virtualized line list with diagnostic marker and cursor.
- Inspector: YAML path, cursor position, hover, diagnostics.

Interaction thesis:
- Clicking the editor requests focus.
- Keyboard input mutates core text through `DemoDocument`.
- Every edit refreshes YAML path, completions, hover, diagnostics, and visible lines from core.

Files:
- `docs/superpowers/plans/2026-06-18-floem-basic-input-parity.md`
- `crates/mocode-floem-demo/src/main.rs`
- `README.md`

Tests:
- `cargo test -p mocode-floem-demo`
- `cargo check -p mocode-floem-demo`
- `cargo fmt --all --check`
- `cargo test --workspace`

Commit:
- `feat(floem): wire basic editor input`

## Task 1: Floem Demo State Editing

- [ ] Write failing tests in `crates/mocode-floem-demo/src/main.rs`:

```rust
#[test]
fn edits_document_through_shared_core() {
    let mut document = DemoDocument::from_text(
        "scratch.yaml",
        "dns:\n  enhanced-mode: \n",
        TextPosition::new(1, 17),
    );

    document.insert_text("fake-ip").unwrap();

    assert_eq!(document.cursor, TextPosition::new(1, 24));
    assert_eq!(document.lines[1].text, "  enhanced-mode: fake-ip");
    assert_eq!(document.current_yaml_path, "dns.enhanced-mode");
    assert!(document.completion_items.iter().any(|item| item.label == "fake-ip"));
}

#[test]
fn backspaces_deletes_and_moves_cursor_in_demo_state() {
    let mut document = DemoDocument::from_text(
        "scratch.yaml",
        "dns:\n  enable: true\n",
        TextPosition::new(1, 2),
    );

    document.backspace().unwrap();
    assert_eq!(document.cursor, TextPosition::new(1, 1));
    assert_eq!(document.lines[1].text, " enable: true");

    document.move_left().unwrap();
    assert_eq!(document.cursor, TextPosition::new(1, 0));

    document.move_right().unwrap();
    assert_eq!(document.cursor, TextPosition::new(1, 1));

    document.delete().unwrap();
    assert_eq!(document.cursor, TextPosition::new(1, 1));
    assert_eq!(document.lines[1].text, " nable: true");
}
```

- [ ] Run `cargo test -p mocode-floem-demo edits_document_through_shared_core backspaces_deletes_and_moves_cursor_in_demo_state`.
- [ ] Expected: FAIL because edit methods are not defined on `DemoDocument`.
- [ ] Implement the five methods by delegating to `MocodeEditor` and calling `refresh_derived`.
- [ ] Run `cargo test -p mocode-floem-demo`.

## Task 2: Reactive Floem Rendering

- [ ] Change `app_view` to create `RwSignal<DemoDocument>`.
- [ ] Change header, completion strip, editor surface, and inspector to read from the document signal.
- [ ] Render the cursor in `line_row` by splitting line text at the cursor character.
- [ ] Keep `virtual_stack` for YAML lines.
- [ ] Run `cargo check -p mocode-floem-demo`.

## Task 3: Floem Input Events

- [ ] Add event handlers to the editor surface:
  - `PointerDown`: request focus.
  - `KeyDown`: handle Backspace, Delete, ArrowLeft, ArrowRight, printable `Key::Character`, Ctrl/Cmd+V.
  - `ImeCommit`: insert committed IME text.
- [ ] Add helper functions:
  - `handle_key_down(document, event)`.
  - `is_insertable_text(text)`.
  - `split_at_character(text, character)`.
- [ ] Run `cargo check -p mocode-floem-demo`.
- [ ] Run `cargo test -p mocode-floem-demo`.

## Task 4: Docs and Publish

- [ ] Update `README.md` to say Floem supports the first editable loop and committed IME text.
- [ ] Run `cargo fmt --all --check`.
- [ ] Run `cargo check -p mocode-floem-demo`.
- [ ] Run `cargo test --workspace`.
- [ ] Commit with `feat(floem): wire basic editor input`.
- [ ] Push `master` to `origin/master`.
