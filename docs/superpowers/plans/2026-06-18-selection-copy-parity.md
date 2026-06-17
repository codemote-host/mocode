# Selection Copy Parity Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add the first same-spec keyboard selection and copy path to the GPUI and Floem prototypes.

**Architecture:** `mocode-text` owns ordered text range extraction from the rope, and `mocode-core` exposes it through `text_in_range`. Each UI adapter owns only transient selection state (`anchor`, `active`, selected text summary) and delegates actual text extraction to the shared core. GPUI and Floem use their native clipboard APIs, but the selected range semantics stay identical.

**Tech Stack:** Rust, ropey-backed `mocode-text`, `mocode-core`, GPUI actions/clipboard, Floem keyboard events/clipboard.

---

## Task Card

- 判断: 当前两个 demo 都能编辑、粘贴和切换 fixture，但验收矩阵里 selection/copy 仍是共同缺口。先做键盘选择和复制，能支撑后续手工验证。
- 范围: 添加 ordered range 文本提取 API、core facade、两个 demo 的 Shift+Left/Right 选择、Ctrl/Cmd+C 复制、基础选区可视状态和测试。
- 不做事项: 不做鼠标拖选、不做 Shift+Up/Down、不做 Select All、不做剪切、不做多光标、不做复杂高亮跨行绘制。
- 涉及文件: `crates/mocode-text/src/lib.rs`, `crates/mocode-core/src/lib.rs`, `crates/mocode-api/src/lib.rs`, `crates/mocode-gpui-demo/src/main.rs`, `crates/mocode-floem-demo/src/main.rs`, `docs/prototype-validation-checklist.md`, `docs/gpui-vs-floem-evaluation.md`, `docs/superpowers/plans/2026-06-18-selection-copy-parity.md`.
- 测试: 先写 failing tests，再实现；运行 targeted text/core/demo tests、`cargo fmt --all --check`, `cargo check -p mocode-gpui-demo`, `cargo check -p mocode-floem-demo`, `cargo test --workspace`.
- 是否需要 subagent/worktree: 不需要。范围集中，当前分支由用户明确要求继续推进并及时提交。
- commit message: `feat: add selection copy parity`

## Steps

- [x] **Step 1: Write failing text/core tests**

Add tests proving `TextBuffer::text_in_range` and `MocodeEditor::text_in_range` extract single-line and multi-line ranges, and normalize reversed ranges.

- [x] **Step 2: Run targeted tests and confirm RED**

Run: `cargo test -p mocode-text text_in_range` and `cargo test -p mocode-core text_in_range`.

Expected: compile failure because the APIs do not exist yet.

- [x] **Step 3: Implement shared range extraction**

Add `TextBuffer::text_in_range`, `TextBuffer::ordered_range`, and `MocodeEditor::text_in_range`.

- [x] **Step 4: Confirm shared extraction GREEN**

Run: `cargo test -p mocode-text text_in_range` and `cargo test -p mocode-core text_in_range`.

Expected: pass.

- [x] **Step 5: Write failing GPUI selection tests**

Add a test that extends selection with `select_right`, verifies `selected_text`, copies it with `copy_selection_text`, and verifies normal cursor movement clears selection.

- [x] **Step 6: Run GPUI test and confirm RED**

Run: `cargo test -p mocode-gpui-demo selection_copy_uses_shared_core_range`.

Expected: compile failure because selection APIs do not exist yet.

- [x] **Step 7: Implement GPUI keyboard selection and copy**

Add selection state to `DemoDocument`, Shift+Left/Right actions, Ctrl/Cmd+C action, and a small inspector/header selection summary.

- [x] **Step 8: Confirm GPUI GREEN**

Run: `cargo test -p mocode-gpui-demo selection_copy_uses_shared_core_range`.

Expected: pass.

- [x] **Step 9: Write failing Floem selection tests**

Add the same test name and assertions to Floem.

- [x] **Step 10: Run Floem test and confirm RED**

Run: `cargo test -p mocode-floem-demo selection_copy_uses_shared_core_range`.

Expected: compile failure because selection APIs do not exist yet in the Floem demo.

- [x] **Step 11: Implement Floem keyboard selection and copy**

Mirror the `DemoDocument` selection behavior, handle Shift+Left/Right and Ctrl/Cmd+C in `handle_key_down`, and surface the selection summary.

- [x] **Step 12: Confirm Floem GREEN**

Run: `cargo test -p mocode-floem-demo selection_copy_uses_shared_core_range`.

Expected: pass.

- [x] **Step 13: Update docs**

Update the validation checklist and evaluation report to record keyboard selection/copy as implemented but still needing manual clipboard validation.

- [x] **Step 14: Full verification, commit, push**

Run all verification commands, commit, and push to `origin/master`.
