# Architecture

## Workspace Structure

```text
crates/
  mocode-text/
  mocode-yaml/
  mocode-mihomo-schema/
  mocode-mihomo-lint/
  mocode-core/
  mocode-api/
  mocode-gpui-demo/
  mocode-floem-demo/
examples/configs/
tests/fixtures/
docs/
```

## Crate Design

### mocode-text

Owns text primitives:

- `TextBuffer`
- `TextPosition`
- `TextRange`
- `TextEdit`
- cursor and selection primitives
- future undo/redo operation log

It may depend on `ropey`. It must not depend on YAML, Mihomo schema, or UI crates.

### mocode-yaml

Owns YAML parsing and source mapping:

- tree-sitter parser integration
- syntax error ranges
- cursor position to YAML node
- cursor position to YAML path
- indentation context
- conservative formatting helpers

It depends on `mocode-text`, not on Mihomo schema or UI crates.

### mocode-mihomo-schema

Owns static Mihomo knowledge:

- field catalog
- enum values
- doc strings
- snippets
- source links
- schema version metadata

It should remain data-oriented and independently testable.

### mocode-mihomo-lint

Owns semantic indexing and validation:

- extract named entities
- resolve references
- build `dialer-proxy` graph
- detect cycles
- create diagnostics
- surface risk hints

It may depend on `mocode-yaml`, `mocode-text`, and `mocode-mihomo-schema`.

### mocode-core

Owns editor orchestration:

- text buffer
- YAML snapshot
- schema catalog
- semantic index
- completion, hover, diagnostics, references, format, and chain preview APIs

It must not depend on any UI crate.

### mocode-api

Public facade for host applications. It re-exports stable API types and hides crate layout churn.

### mocode-gpui-demo and mocode-floem-demo

UI adapters only. They can own rendering, input, focus, scrolling, popups, side panels, and platform integration. They must not own Mihomo semantic rules.

## Data Flow

```text
User input
  -> UI adapter
  -> TextEdit
  -> mocode-core.apply_edit
  -> mocode-text updates Rope
  -> mocode-yaml incremental parse
  -> mocode-mihomo-lint rebuilds affected semantic index
  -> mocode-core publishes snapshot
  -> UI adapter renders text, diagnostics, hover, completions, path, chain
```

## Edit Event Flow

1. UI adapter converts keypress, paste, IME commit, or command into `TextEdit`.
2. `mocode-core` validates and applies the edit to `TextBuffer`.
3. `mocode-yaml` receives the edit range and reparses incrementally.
4. Semantic index is rebuilt incrementally later; phase 1 may rebuild whole-file for simplicity.
5. Diagnostics and completion caches are invalidated.
6. UI adapter requests the updated viewport and semantic overlays.

## YAML Parse Flow

Phase 1:

```text
text -> tree-sitter-yaml -> syntax tree -> YAML path walker -> syntax errors
```

Later:

```text
old tree + InputEdit + edited text -> incremental tree -> changed ranges -> partial semantic refresh
```

The path walker must handle:

- block maps
- block sequences
- flow maps and sequences
- nested scalar keys
- comments and blank lines
- incomplete documents
- syntax errors near cursor

## Semantic Index Flow

```text
YAML tree
  -> top-level section scanner
  -> entity extractor
  -> reference extractor
  -> dialer graph builder
  -> diagnostics
```

The index stores source ranges for names and references. Provider remote contents are represented as unknown until loaded by a host application or inline payload parser.

## UI Adapter Design

The UI adapter receives a plain API:

- current text snapshot
- visible line slices
- diagnostics
- completion list
- hover payload
- YAML path
- proxy chain preview

The adapter owns:

- text layout and painting
- scrolling and virtualization
- IME composition
- focus and keyboard dispatch
- clipboard
- popup placement
- platform windows

The adapter does not:

- parse YAML
- know Mihomo field rules
- resolve references
- detect `dialer-proxy` cycles

