# Prototype Fixture Selector Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add same-spec fixture switching to the GPUI and Floem prototypes so validation can load large and diagnostic samples interactively.

**Architecture:** Each UI adapter owns a small `DemoFixture` list that maps an id, label, title, YAML text, and inspect cursor position to a `DemoDocument`. Switching fixtures rebuilds `DemoDocument` through the existing `mocode-api` path, so Mihomo schema, YAML parsing, completions, hover, and diagnostics remain in shared core crates. The selector is UI-only: it does not add file management or host-application state.

**Tech Stack:** Rust, `mocode-api`, GPUI, Floem, existing YAML fixtures under `examples/configs` and `tests/fixtures`.

---

## Task Card

- 判断: 原型验证已经有 20000 行 fixture 和自动加载测试，但交互式滚动/诊断验证仍缺少样本切换入口。补 fixture selector 是选择 GPUI/Floem 前的最小下一步。
- 范围: 给两个 demo 增加同规格内置 fixture 列表、状态切换方法、header selector UI、状态测试，并更新验证文档。
- 不做事项: 不做完整文件打开/保存，不做目录浏览，不做拖拽，不做 selection/copy，不做 completion popup，不做 Mihomo GUI 能力。
- 涉及文件: `crates/mocode-gpui-demo/src/main.rs`, `crates/mocode-floem-demo/src/main.rs`, `docs/prototype-validation-checklist.md`, `docs/gpui-vs-floem-evaluation.md`, `docs/superpowers/plans/2026-06-18-prototype-fixture-selector.md`.
- 测试: 先写 failing tests，再实现；运行 `cargo test -p mocode-gpui-demo fixture_selector_loads_large_and_diagnostic_samples`, `cargo test -p mocode-floem-demo fixture_selector_loads_large_and_diagnostic_samples`, `cargo fmt --all --check`, `cargo check -p mocode-gpui-demo`, `cargo check -p mocode-floem-demo`, `cargo test --workspace`.
- 是否需要 subagent/worktree: 不需要。范围集中，当前分支已经由用户明确要求继续推进并及时提交。
- commit message: `feat: add prototype fixture selector`

## Files

- Modify `crates/mocode-gpui-demo/src/main.rs`
  - Add `DemoFixture`.
  - Add static fixture list.
  - Add `load_fixture_by_id`.
  - Render selector buttons in the header.
  - Add adapter-state tests.
- Modify `crates/mocode-floem-demo/src/main.rs`
  - Mirror the same `DemoFixture` ids and labels.
  - Render selector buttons in the header.
  - Add adapter-state tests.
- Modify `docs/prototype-validation-checklist.md`
  - Replace the current limitation text with selector-based manual validation steps.
- Modify `docs/gpui-vs-floem-evaluation.md`
  - Record that interactive fixture selection exists, while measured IME/scroll data is still open.

## Steps

- [x] **Step 1: Write failing GPUI selector tests**

Add a test named `fixture_selector_loads_large_and_diagnostic_samples` that expects `load_fixture_by_id("large-20000")` to load at least 20000 lines and `load_fixture_by_id("invalid-yaml")` to expose a `yaml.syntax` diagnostic.

- [x] **Step 2: Run the GPUI test and confirm RED**

Run: `cargo test -p mocode-gpui-demo fixture_selector_loads_large_and_diagnostic_samples`

Expected: compile failure because `load_fixture_by_id` does not exist yet.

- [x] **Step 3: Implement the minimal GPUI fixture model and header selector**

Add `DemoFixture`, `DEMO_FIXTURES`, `load_fixture`, `load_fixture_by_id`, and a GPUI header selector that calls `MocodeGpuiDemo::select_fixture`.

- [x] **Step 4: Run the GPUI selector test and confirm GREEN**

Run: `cargo test -p mocode-gpui-demo fixture_selector_loads_large_and_diagnostic_samples`

Expected: pass.

- [x] **Step 5: Write failing Floem selector tests**

Add the same test name and assertions to the Floem demo.

- [x] **Step 6: Run the Floem test and confirm RED**

Run: `cargo test -p mocode-floem-demo fixture_selector_loads_large_and_diagnostic_samples`

Expected: compile failure because `load_fixture_by_id` does not exist yet in the Floem demo.

- [x] **Step 7: Implement the minimal Floem fixture model and header selector**

Mirror the fixture model and add a Floem selector row that updates the `RwSignal<DemoDocument>`.

- [x] **Step 8: Run the Floem selector test and confirm GREEN**

Run: `cargo test -p mocode-floem-demo fixture_selector_loads_large_and_diagnostic_samples`

Expected: pass.

- [x] **Step 9: Update validation documents**

Update the validation checklist and GPUI/Floem evaluation report to say fixture switching is now available and manual validation can load `large.yaml`, `large-20000.yaml`, invalid YAML, invalid reference, and dialer-cycle samples.

- [x] **Step 10: Full verification and publish**

Run formatting, targeted checks, full workspace tests, commit, and push.
