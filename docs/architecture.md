# Architecture

## Workspace Structure

```text
crates/
  mocode/
  mocode-text/
  mocode-yaml/
  mocode-mihomo-schema/
  mocode-mihomo-lint/
  mocode-core/
  mocode-api/
examples/configs/
tests/fixtures/
docs/
```

## Crate Design

### mocode

GPUI application shell and component adapter. It owns rendering, keyboard input, focus, clipboard, scrolling, popups, side panels, open/save UX, and app-window integration. It must not own Mihomo semantic rules.

### mocode-text

Owns text primitives:

- `TextBuffer`
- `TextPosition`
- `TextRange`
- `TextEdit`
- cursor and selection primitives
- undo/redo operation history

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

## Data Flow

```text
User input
  -> mocode app adapter
  -> TextEdit
  -> mocode-core.apply_edit
  -> mocode-text updates Rope
  -> mocode-yaml parses YAML
  -> mocode-mihomo-lint rebuilds affected semantic index
  -> mocode-core publishes derived state
  -> mocode app renders text, diagnostics, hover, completions, path, chain
```

## Edit Event Flow

1. The app converts keypress, paste, IME commit, or command into `TextEdit`.
2. `mocode-core` validates and applies the edit to `TextBuffer`.
3. `mocode-yaml` receives the current text and parses it.
4. Semantic index is rebuilt whole-file for now; later work can make this incremental.
5. Diagnostics and completion caches are refreshed.
6. The app requests updated viewport lines and semantic overlays.

## YAML Parse Flow

Current:

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

The app receives a plain API:

- current text snapshot
- visible line slices
- diagnostics
- completion list
- hover payload
- YAML path
- proxy chain preview

The app owns:

- text layout and painting
- scrolling and virtualization
- IME composition
- focus and keyboard dispatch
- clipboard
- popup placement
- platform windows

The app does not:

- parse YAML
- know Mihomo field rules
- resolve references
- detect `dialer-proxy` cycles
