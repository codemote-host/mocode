# Prototype Validation Checklist Plan

Date: 2026-06-18

## Goal

Create the first repeatable validation baseline for choosing between the GPUI and Floem mocode prototypes.

This step does not add new editor behavior. It adds the fixture, checklist, commands, and test coverage needed to validate the behavior that already exists and expose the remaining gaps.

## Task Card

- 判断: 当前 GPUI/Floem 对比报告缺少 20000 行 fixture、Windows 中文 IME 手工脚本、release build size 命令和可重复 smoke commands。先补验证基线比继续堆功能更有价值。
- 范围: 新增原型验证清单、生成 20000 行 Mihomo YAML fixture、添加 core 层加载基线测试，并把 README/roadmap/evaluation report 链接到该清单。
- 不做事项: 不实现选择/复制、completion popup、chain preview、文件打开 UI、TUN/系统代理/订阅管理，也不改变 core 与 UI adapter 边界。
- 涉及文件: `docs/prototype-validation-checklist.md`, `examples/configs/large-20000.yaml`, `crates/mocode-core/src/lib.rs`, `crates/mocode-gpui-demo/src/main.rs`, `crates/mocode-floem-demo/src/main.rs`, `docs/gpui-vs-floem-evaluation.md`, `docs/roadmap.md`, `README.md`.
- 测试: 先添加 `mocode-core` 的 20000 行 fixture 加载测试并观察失败，再生成 fixture 后运行 core/GPUI/Floem 的 20000 行目标测试、`cargo fmt --all --check`, `cargo test -p mocode-core`, `cargo check -p mocode-gpui-demo`, `cargo check -p mocode-floem-demo`, `cargo test --workspace`.
- 是否需要 subagent/worktree: 不需要。变更集中且当前 worktree 干净。
- commit message: `docs: add prototype validation checklist`

## Steps

1. Add a failing core test that includes `examples/configs/large-20000.yaml`, checks it has at least 20000 lines, opens it through `MocodeEditor`, and verifies no diagnostics.
2. Generate a deterministic 20000-line YAML fixture from the existing valid large Mihomo sample plus comment padding, so the baseline measures editor loading without triggering the current tree-sitter-yaml oversized-sequence behavior.
3. Add equivalent 20000-line adapter-state tests for the GPUI and Floem demos.
4. Add `docs/prototype-validation-checklist.md` with automated commands, manual Windows Chinese IME checks, scroll/focus/popup observations, and result recording template.
5. Update the evaluation report, README, and roadmap to reference the new checklist and fixture.
6. Run formatting, targeted tests, demo checks, and full workspace tests.
7. Commit and push the finished validation baseline.

## Success Criteria

- `examples/configs/large-20000.yaml` is committed and valid for the current core lint rules.
- `mocode-core`, `mocode-gpui-demo`, and `mocode-floem-demo` have automated load tests for the 20000-line fixture.
- The validation checklist can be executed manually without reading previous chat context.
- The evaluation report clearly says the 20000-line fixture exists, while manual performance/IME data remains open.
- The branch is committed and pushed to `origin/master`.
