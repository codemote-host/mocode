# Floem Semantic Demo Parity Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Bring `mocode-floem-demo` from placeholder to the same semantic display baseline as the GPUI demo.

**Architecture:** `mocode-floem-demo` owns only UI state and rendering. It consumes `MocodeEditor` through `mocode-api`, maps core completions/hover/diagnostics into local view DTOs, and does not implement Mihomo schema, lint, or YAML path logic.

**Tech Stack:** Rust, Floem 0.2.0, mocode-api, mocode-core semantic view APIs, tree-sitter-yaml, yaml-rust2.

---

## Task Card

Judgment: implement Floem parity now so the project can compare GPUI and Floem on the same mocode-core surface.

Scope:
- Add a `DemoDocument` state model to `mocode-floem-demo`.
- Reuse `MocodeEditor::semantic_lines`, `hover_summary_at`, `completions_at`, `diagnostics`, and `current_yaml_path`.
- Render a restrained utility UI with an editor surface, line numbers, diagnostics marker, completion strip, and inspector.
- Load `examples/configs/dialer-proxy.yaml` by default.
- Test loading `examples/configs/large.yaml` as the 5000+ line baseline.
- Update README so Floem is no longer documented as a placeholder.

Non-goals:
- No full text editing in Floem yet.
- No completion accept/apply workflow.
- No syntax highlighting.
- No Mihomo semantic logic in the Floem UI layer.
- No changes to `mocode-core` unless a missing UI-neutral API is proven by tests.
- No GPUI changes.

Visual thesis:
- Dense operational editor surface, low chrome, neutral background, one red/yellow/blue diagnostic accent inherited from severity.

Content plan:
- Header: prototype name, loaded file, line count.
- Completion strip: first core completion items near the editor surface.
- Workspace: left editor-like line list, right semantic inspector.
- Inspector: YAML path, cursor, hover summary, diagnostics.

Interaction thesis:
- This stage is static/read-only for Floem except normal window rendering. Interaction work starts after parity proves Floem can carry the same semantic data.

Files:
- `docs/superpowers/plans/2026-06-18-floem-semantic-demo-parity.md`
- `crates/mocode-floem-demo/Cargo.toml`
- `crates/mocode-floem-demo/src/main.rs`
- `README.md`

Tests:
- `cargo test -p mocode-floem-demo`
- `cargo check -p mocode-floem-demo`
- `cargo fmt --all --check`
- `cargo test --workspace`

Commits:
- `feat(floem): add semantic demo parity`

## Task 1: Floem Demo State

- [ ] Write failing tests for `DemoDocument` building from `MocodeEditor`.
- [ ] Assert current YAML path is `proxies[0].dialer-proxy` for `examples/configs/dialer-proxy.yaml`.
- [ ] Assert completion items include `exit`.
- [ ] Assert hover title/body are populated from core.
- [ ] Assert invalid YAML produces a ranged `yaml.syntax` diagnostic and a line marker.
- [ ] Assert `examples/configs/large.yaml` loads at 5000+ lines.
- [ ] Run `cargo test -p mocode-floem-demo` and confirm failures are missing state fields/types.
- [ ] Implement the minimal state model and mappings.
- [ ] Run `cargo test -p mocode-floem-demo`.

## Task 2: Floem Rendering

- [ ] Add `floem = "0.2.0"` to `crates/mocode-floem-demo/Cargo.toml`.
- [ ] Render the header with title and line count.
- [ ] Render the completion strip from core completions.
- [ ] Render line numbers, line text, and diagnostic marker for each line.
- [ ] Render the inspector with YAML path, cursor, hover, and diagnostics.
- [ ] Run `cargo check -p mocode-floem-demo`.

## Task 3: Docs and Verification

- [ ] Update README to describe both demos as implemented prototypes.
- [ ] Run `cargo fmt --all --check`.
- [ ] Run `cargo check -p mocode-floem-demo`.
- [ ] Run `cargo test --workspace`.
- [ ] Commit and push to `origin/master`.
