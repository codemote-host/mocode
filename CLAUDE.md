@AGENTS.md

# Claude Code Instructions

This repository uses `CLAUDE.md` as the Claude Code project memory file.
Claude Code official guidance says project instructions should be concrete,
concise, and kept in the repository root; this file imports `AGENTS.md` so
future Claude sessions start with the same project contract.

## Operating Role

- Act as the main engineering agent for `mocode`.
- Preserve the existing architecture boundary: core semantics stay UI
  independent; GPUI is only the selected UI shell.
- Use DeepSeek or other subagents as narrow implementation workers when useful.
- The main agent should plan, assign, review, verify, commit, and push. Do not
  accept subagent output without inspection.

## Current Product Direction

- Final product: a usable Rust-native Mihomo YAML editor application that is
  also a reusable GPUI editor component.
- Core crates must remain UI independent and reusable by future hosts.
- GPUI is the primary UI framework. Floem is frozen as a reference prototype.
- Do not build a full Mihomo client, proxy core, tray app, subscription manager,
  TUN controller, system proxy manager, WebDAV sync, or dashboard.

## Required Workflow

- Start every session by reading `AGENTS.md`, `README.md`, and
  `docs/ui-framework-decision.md`.
- Check `git status --short --branch` and recent commits before changing files.
- For each non-trivial change, write or restate a Task Card before implementation.
- Use TDD for behavior changes: write the failing test, confirm it fails, then
  implement the smallest fix.
- Keep worker tasks small with disjoint file ownership.
- After each worker task, run a spec review and a code-quality review before
  treating the task as complete.
- Commit focused changes promptly with conventional commit messages.
- Push only reviewed work. If a commit has review findings, fix it before pushing.

## Rust Commands

Run these before claiming Rust behavior is complete:

```powershell
cargo fmt --all --check
cargo test --workspace
```

Useful targeted checks:

```powershell
cargo test -p mocode-gpui-demo
cargo test -p mocode-core proxy_chain_preview
cargo test -p mocode-mihomo-lint
cargo check -p mocode-gpui-demo
```

## Boundary Rules

- `mocode-text`: text model, ranges, edits, selection, undo/redo primitives.
- `mocode-yaml`: YAML parsing, path/scope, syntax errors, indentation/formatting.
- `mocode-mihomo-schema`: field docs, enums, snippets, schema metadata.
- `mocode-mihomo-lint`: references, diagnostics, proxy graph, risk hints.
- `mocode-core`: UI-independent editor orchestration.
- `mocode-api`: public facade for host apps.
- `mocode-gpui-demo`: GPUI app/component adapter only.
- `mocode-floem-demo`: frozen reference adapter only.

Mihomo schema, linting, YAML path, completions, hover docs, diagnostics,
references, formatting policy, and proxy-chain semantics must never be
implemented directly in GPUI UI code.
