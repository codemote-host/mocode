# Completion Popup Positioning Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add same-spec completion popup state and rendering to the GPUI and Floem prototypes.

**Architecture:** Completion semantics remain in `mocode-core`; each UI adapter derives a small `DemoCompletionPopup` from the current cursor and core completion items. The popup records line and column anchors so positioning is testable today, while the first rendering remains a compact anchored panel instead of a full pixel-accurate floating layer.

**Tech Stack:** Rust, existing `mocode-api` completions, GPUI, Floem.

---

## Task Card

- 判断: 当前两个 demo 只有顶部 completion strip，评估报告仍缺 completion popup positioning。先实现带 cursor anchor 的 popup 状态和同规格渲染，后续手工验证再决定是否升级为像素级浮层。
- 范围: GPUI/Floem adapter 新增 `DemoCompletionPopup`、锚点测试、popup 面板渲染、文档更新。
- 不做事项: 不改 `mocode-core` 补全逻辑，不实现补全选择/提交，不实现滚动偏移追踪，不做复杂 z-index/layer manager，不做真实编辑器级 popup placement engine。
- 涉及文件: `crates/mocode-gpui-demo/src/main.rs`, `crates/mocode-floem-demo/src/main.rs`, `docs/prototype-validation-checklist.md`, `docs/gpui-vs-floem-evaluation.md`, `README.md`, `docs/superpowers/plans/2026-06-18-completion-popup-positioning.md`.
- 测试: 先写 failing tests，再实现；运行 `cargo test -p mocode-gpui-demo completion_popup_tracks_cursor_anchor_and_items`, `cargo test -p mocode-floem-demo completion_popup_tracks_cursor_anchor_and_items`, `cargo fmt --all --check`, `cargo check -p mocode-gpui-demo`, `cargo check -p mocode-floem-demo`, `cargo test --workspace`.
- 是否需要 subagent/worktree: 不需要。范围集中，用户要求当前仓库连续推进并及时提交。
- commit message: `feat: add completion popup positioning`

## Steps

- [x] **Step 1: Write failing GPUI popup test**

Add `completion_popup_tracks_cursor_anchor_and_items` to the GPUI demo. It should build a document at `proxies[0].dialer-proxy`, assert popup anchor line/column, and assert `exit` is in popup item labels.

- [x] **Step 2: Confirm GPUI RED**

Run: `cargo test -p mocode-gpui-demo completion_popup_tracks_cursor_anchor_and_items`

Expected: compile failure because `completion_popup` does not exist yet.

- [x] **Step 3: Implement GPUI popup state and panel**

Add `DemoCompletionPopup`, derive it from `completion_items`, render a compact popup panel labelled with `line:column`, and keep the old strip as a fallback for manual comparison.

- [x] **Step 4: Confirm GPUI GREEN**

Run: `cargo test -p mocode-gpui-demo completion_popup_tracks_cursor_anchor_and_items`

Expected: pass.

- [x] **Step 5: Write failing Floem popup test**

Add the same test name and assertions to the Floem demo.

- [x] **Step 6: Confirm Floem RED**

Run: `cargo test -p mocode-floem-demo completion_popup_tracks_cursor_anchor_and_items`

Expected: compile failure because `completion_popup` does not exist yet.

- [x] **Step 7: Implement Floem popup state and panel**

Mirror the same `DemoCompletionPopup` data and render a compact popup panel below the completion strip.

- [x] **Step 8: Confirm Floem GREEN**

Run: `cargo test -p mocode-floem-demo completion_popup_tracks_cursor_anchor_and_items`

Expected: pass.

- [x] **Step 9: Update docs**

Update README, validation checklist, and evaluation report to say anchor-aware popup positioning exists and still needs manual focus/layer validation.

- [x] **Step 10: Full verification and publish**

Run full verification, commit, and push.
