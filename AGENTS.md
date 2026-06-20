# AGENTS.md

This repository is for `mocode`, a Mihomo Config Editor Component. Agents must preserve the separation between editor core, Mihomo semantics, and the GPUI application shell.

## Ground Rules

- Do not turn this project into a full Mihomo GUI.
- Do not implement Mihomo core management, TUN management, system proxy management, subscriptions, WebDAV, tray, updater, or dashboard integration unless a later task explicitly scopes it.
- Do not copy large third-party editor code. Zed, Lapce, and other projects are references only.
- Do not put Mihomo semantic logic inside `crates/mocode`.
- Do not bind `mocode-core` to GPUI, Tauri, egui, or any concrete UI toolkit.
- Do not make large UI/core boundary changes without a Task Card and an explicit rationale.
- Do not chase 100% Mihomo field coverage in one pass. Add schema coverage incrementally with tests and fixtures.
- Every commit must be pushed after verification.

## Branch Strategy

- Use short-lived feature branches for larger feature areas.
- Prefer one focused branch per feature area: text model, YAML parser, schema catalog, lint, app shell, or public API.
- Keep docs and implementation in the same branch when the implementation changes public behavior.

## Commit Style

Use conventional, scoped commits:

- `docs: update app direction`
- `feat(text): add rope-backed buffer`
- `feat(yaml): compute yaml path at cursor`
- `test(lint): cover missing proxy reference`
- `refactor(app): keep semantics out of ui`

## Testing Requirements

- Run `cargo fmt --all --check` before claiming Rust changes are complete.
- Run `cargo test --workspace` before claiming behavior is complete.
- Run `cargo build -p mocode` before claiming the app target builds.
- Add fixtures under `tests/fixtures/` or `examples/configs/` for Mihomo-specific behavior.
- Diagnostics, completions, YAML path lookup, and chain validation need direct unit tests before UI work depends on them.

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

- `mocode`: GPUI application shell and component adapter only.
- `mocode-text`: text storage, positions, ranges, edits, cursor/selection primitives, undo/redo model.
- `mocode-yaml`: YAML parse tree, YAML path lookup, syntax errors, indentation, formatting boundary.
- `mocode-mihomo-schema`: Mihomo schema catalog, docs, enums, snippets, completion sources.
- `mocode-mihomo-lint`: reference validation, proxy graph checks, risk diagnostics.
- `mocode-core`: UI-independent orchestration API.
- `mocode-api`: public API facade for host applications.

## UI Adapter Discipline

The app crate may render:

- text viewport
- line numbers
- cursor and selection
- completion popup
- hover popup
- diagnostics gutter/underline
- right-side YAML path and proxy chain panel

The app crate must request all semantic information from `mocode-core`.
