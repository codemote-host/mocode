# AGENTS.md

This repository is for `mocode`, a Mihomo Config Editor Component. Agents must preserve the separation between editor core, Mihomo semantics, and UI adapters.

## Ground Rules

- Do not turn this project into a full Mihomo GUI.
- Do not implement Mihomo core management, TUN management, system proxy management, subscriptions, WebDAV, tray, updater, or dashboard integration unless a later task explicitly scopes it.
- Do not copy large third-party editor code. Zed, Lapce, and other projects are references only.
- Do not put Mihomo semantic logic inside `mocode-gpui-demo` or `mocode-floem-demo`.
- Do not bind `mocode-core` to GPUI, Floem, Tauri, egui, or any concrete UI toolkit.
- Do not make large UI/core boundary changes without a Task Card and an explicit rationale.
- Do not chase 100% Mihomo field coverage in one pass. Add schema coverage incrementally with tests and fixtures.

## Branch Strategy

- Use short-lived feature branches once this directory is a Git repository.
- Prefer one focused branch per feature area: text model, YAML parser, schema catalog, lint, GPUI demo, Floem demo.
- Keep docs and implementation in the same branch when the implementation changes public behavior.

## Commit Style

Use conventional, scoped commits:

- `docs: add phase 0 research`
- `feat(text): add rope-backed buffer`
- `feat(yaml): compute yaml path at cursor`
- `test(lint): cover missing proxy reference`
- `chore(workspace): add demo crates`

## Testing Requirements

- Run `cargo fmt --all --check` before claiming Rust changes are complete.
- Run `cargo test --workspace` before claiming behavior is complete.
- Add fixtures under `tests/fixtures/` or `examples/configs/` for Mihomo-specific behavior.
- Diagnostics, completions, YAML path lookup, and chain validation need direct unit tests before UI work depends on them.
- GPUI and Floem demos must be tested against the same acceptance checklist.

## Task Card Requirement

Every non-trivial development suggestion must include a Task Card:

```markdown
### Task Card: <title>

- Judgment:
- Scope:
- Non-goals:
- Files:
- Tests:
- Subagent/worktree:
- Commit message:
```

## Crate Ownership

- `mocode-text`: text storage, positions, ranges, edits, cursor/selection primitives, undo/redo model.
- `mocode-yaml`: YAML parse tree, YAML path lookup, syntax errors, indentation, formatting boundary.
- `mocode-mihomo-schema`: Mihomo schema catalog, docs, enums, snippets, completion sources.
- `mocode-mihomo-lint`: reference validation, proxy graph checks, risk diagnostics.
- `mocode-core`: UI-independent orchestration API.
- `mocode-api`: public API facade for host applications.
- `mocode-gpui-demo`: GPUI adapter only.
- `mocode-floem-demo`: Floem adapter only.

## UI Adapter Discipline

UI crates may render:

- text viewport
- line numbers
- cursor and selection
- completion popup
- hover popup
- diagnostics gutter/underline
- right-side YAML path and proxy chain panel

UI crates must request all semantic information from `mocode-core`.

