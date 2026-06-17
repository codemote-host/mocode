# Roadmap

## Phase 0: Research and Documentation

Status: current phase.

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

- docs cover core boundaries and first prototype acceptance
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
- tree-sitter YAML parser
- current YAML path lookup
- basic Mihomo schema docs
- field hover
- field and enum completions
- semantic index for proxies/groups/providers/rules
- missing reference diagnostics
- basic `dialer-proxy` cycle detection
- unit tests and fixtures

## Phase 2: GPUI Prototype

Deliverables:

- GPUI adapter over `mocode-core`
- file load/save path
- line numbers, cursor, selection, clipboard
- diagnostics rendering
- hover docs
- completions
- right-side YAML path and chain preview panels
- IME and performance notes

## Phase 3: Floem Prototype

Deliverables:

- Floem adapter over `mocode-core`
- same feature checklist as GPUI prototype
- no copied Mihomo business logic
- IME and performance notes

## Phase 4: Comparison Evaluation

Deliverable: [gpui-vs-floem-evaluation.md](gpui-vs-floem-evaluation.md)

Status: initial report and [prototype-validation-checklist.md](prototype-validation-checklist.md) available. The current report compares repository-backed prototype facts and keeps final framework selection open until manual IME, interactive large-file scroll, focus, popup, and package-size validation is complete.

Evaluation dimensions:

- Chinese IME behavior
- scroll and layout performance on 5000-20000 lines
- popup and focus reliability
- text selection and clipboard ergonomics
- rendering complexity
- package size
- API stability
- code complexity
- maintenance risk

## Phase 5: UI Framework Selection and Componentization

Deliverables:

- chosen UI framework
- production component API
- packaging strategy
- host application integration notes
- full fixture suite
- compatibility matrix
