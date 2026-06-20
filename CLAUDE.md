@AGENTS.md

# Claude Code Instructions

This repository uses `CLAUDE.md` as the Claude Code project memory file.
Claude Code sessions must follow `AGENTS.md`; this file only adds handoff-specific context.

## Operating Role

- Treat Codex/user direction as the decision source for product scope.
- Preserve the architecture boundary: core semantics stay UI independent; GPUI is only the selected app shell.
- Do not use broad autonomous rewrites. Keep changes small, reviewed, tested, committed, and pushed.
- Do not accept worker output without inspecting the diff and rerunning verification.

## Current Product Direction

- Final product: a usable Rust-native Mihomo YAML editor application that is also a reusable editor component.
- Core crates must remain UI independent and reusable by future hosts.
- The app target is `mocode`; on Windows it must build to `mocode.exe`.
- Do not build a full Mihomo client, proxy core, tray app, subscription manager, TUN controller, system proxy manager, WebDAV sync, or dashboard.

## Required Workflow

- Start every session by reading `AGENTS.md`, `README.md`, and `docs/spec.md`.
- Check `git status --short --branch` and recent commits before changing files.
- For each non-trivial change, write or restate a Task Card before implementation.
- Use TDD for behavior changes: write the failing test, confirm it fails, then implement the smallest fix.
- Commit focused changes promptly with conventional commit messages.
- Push every commit.

## Rust Commands

Run these before claiming Rust behavior is complete:

```powershell
cargo fmt --all --check
cargo test --workspace
cargo build -p mocode
```

Useful targeted checks:

```powershell
cargo test -p mocode
cargo test -p mocode-core proxy_chain_preview
cargo test -p mocode-mihomo-lint
cargo check -p mocode
```

## Boundary Rules

- `mocode`: GPUI application shell and component adapter only.
- `mocode-text`: text model, ranges, edits, selection, undo/redo primitives.
- `mocode-yaml`: YAML parsing, path/scope, syntax errors, indentation/formatting.
- `mocode-mihomo-schema`: field docs, enums, snippets, schema metadata.
- `mocode-mihomo-lint`: references, diagnostics, proxy graph, risk hints.
- `mocode-core`: UI-independent editor orchestration.
- `mocode-api`: public facade for host apps.

Mihomo schema, linting, YAML path, completions, hover docs, diagnostics,
references, formatting policy, and proxy-chain semantics must never be
implemented directly in GPUI UI code.
