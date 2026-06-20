# Roadmap

## Phase 0: Research and Documentation

Status: complete baseline.

Deliverables:

- research summary
- product spec
- architecture doc
- Mihomo schema design
- editor feature design
- AGENTS.md
- README.md
- workspace skeleton
- sample configs and fixtures

Exit criteria:

- docs cover core boundaries and first app acceptance
- `cargo test --workspace` passes for skeleton
- no UI-specific Mihomo logic exists

## Phase 1: UI-independent Core

Crates:

- `mocode-text`
- `mocode-yaml`
- `mocode-mihomo-schema`
- `mocode-mihomo-lint`
- `mocode-core`

Deliverables:

- rope-backed text buffer
- text edit application
- undo/redo primitives
- tree-sitter YAML parser
- current YAML path lookup
- basic Mihomo schema docs
- field hover
- field and enum completions
- semantic index for proxies/groups/providers/rules
- missing reference diagnostics
- basic `dialer-proxy` cycle detection
- unit tests and fixtures

## Phase 2: GPUI Application Shell

Deliverables:

- `mocode` app crate over `mocode-core`
- file load/save path
- line numbers, cursor, selection, clipboard
- undo/redo keyboard path
- diagnostics rendering
- hover docs
- completions
- bottom status bar for YAML path, diagnostics, completions, search, and chain preview
- IME and performance notes

## Phase 3: Editor Hardening

Deliverables:

- robust IME commit and preedit handling
- viewport-only rendering for 20000-line files
- click/selection coordinate correctness
- completion and hover popup focus plus keyboard navigation
- syntax highlighting pass
- search status and navigation
- paste indentation policy
- screenshot/manual validation notes

## Phase 4: Reusable Component API

Deliverables:

- component API over UI-independent `mocode-core`
- host application integration notes
- packaging strategy
- fixture suite expansion
- compatibility matrix
- app/component split where it improves reuse without leaking semantics into UI

## Current Direction

The app target is `mocode`; on Windows it must build to `mocode.exe`. New feature work should improve the GPUI app shell and shared core without adding full Mihomo client responsibilities.
