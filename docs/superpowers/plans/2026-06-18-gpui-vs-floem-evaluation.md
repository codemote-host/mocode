# GPUI vs Floem Evaluation Report Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Produce the first repository-backed comparison report for GPUI and Floem as carriers for the mocode Mihomo YAML editor component.

**Architecture:** The report is a documentation deliverable only. It compares the two existing UI adapters against the same acceptance checklist, using local repository facts, Cargo metadata, current tests, and explicit open validation items instead of changing editor behavior.

**Tech Stack:** Markdown, Cargo workspace metadata, existing Rust tests, GPUI 0.2.2 demo, Floem 0.2.0 demo.

---

## Task Card

Judgment: produce the Phase 4 evaluation report now because both GPUI and Floem have the same minimum semantic display and basic input loop backed by `mocode-core`.

Scope:
- Create `docs/gpui-vs-floem-evaluation.md`.
- Compare current feature coverage, shared-core boundary, input handling, IME status, large-file baseline, diagnostics/hover/completion display, dependency scale, and code complexity.
- Record open manual validation tasks for Chinese IME, scroll smoothness, focus behavior, popup behavior, and packaging size.
- Update README and roadmap to link the report.

Non-goals:
- No final UI framework selection.
- No benchmark automation in this task.
- No changes to `mocode-core`, GPUI demo, or Floem demo behavior.
- No claims about upstream framework status beyond current local dependencies and prior research docs.

Files:
- `docs/superpowers/plans/2026-06-18-gpui-vs-floem-evaluation.md`
- `docs/gpui-vs-floem-evaluation.md`
- `README.md`
- `docs/roadmap.md`

Evidence commands:
- `git status --short --branch`
- `cargo tree -p mocode-gpui-demo -e normal --depth 1`
- `cargo tree -p mocode-floem-demo -e normal --depth 1`
- `cargo tree -p mocode-gpui-demo -e normal --prefix none`
- `cargo tree -p mocode-floem-demo -e normal --prefix none`
- line counts for `crates/mocode-gpui-demo/src/main.rs`, `crates/mocode-floem-demo/src/main.rs`, and `crates/mocode-core/src/lib.rs`

Verification:
- `cargo fmt --all --check`
- `cargo check -p mocode-gpui-demo`
- `cargo check -p mocode-floem-demo`
- `cargo test --workspace`

Commit:
- `docs: add gpui vs floem evaluation`

## Task 1: Evaluation Report

- [ ] Write `docs/gpui-vs-floem-evaluation.md`.
- [ ] Include an executive summary with a provisional recommendation.
- [ ] Include a feature matrix covering the current acceptance checklist.
- [ ] Include separate sections for GPUI and Floem strengths, risks, and implementation notes.
- [ ] Include evidence and caveats so future measurements can replace qualitative notes.
- [ ] Include next validation checklist.

## Task 2: Documentation Links

- [ ] Update `README.md` with a link to the evaluation report.
- [ ] Update `docs/roadmap.md` Phase 4 from pending deliverable to initial report available, while keeping manual validation open.

## Task 3: Verification and Publish

- [ ] Run `cargo fmt --all --check`.
- [ ] Run `cargo check -p mocode-gpui-demo`.
- [ ] Run `cargo check -p mocode-floem-demo`.
- [ ] Run `cargo test --workspace`.
- [ ] Commit and push to `origin/master`.
