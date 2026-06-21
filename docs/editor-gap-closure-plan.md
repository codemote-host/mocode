# Editor Gap Closure Plan

This document is the execution backbone for taking `mocode` from the current
single-file editor shell to a daily-usable Mihomo YAML editor. It is ordered.
Agents should take the next incomplete task from this document, implement it
with tests, commit, push, then continue with the next task.

## Operating Rules

- The app target is `mocode`; on Windows it must build to `mocode.exe`.
- Every non-trivial implementation starts with a Task Card.
- Every behavior change starts with a failing test.
- Every commit is pushed immediately.
- Do not use subagents unless the user explicitly changes that rule.
- Do not add full Mihomo client features such as core process control, TUN
  management, system proxy management, subscriptions, WebDAV, tray integration,
  or dashboard pages.
- Do not put Mihomo semantic logic in `crates/mocode`; the app crate is only
  the GPUI shell and UI adapter.
- Do not bind `mocode-core` to GPUI or any concrete UI framework.
- Before each commit run:

```powershell
cargo fmt --all --check
cargo test --workspace
cargo build -p mocode
git diff --check
```

- Before each final response run the repository old-framework-name scan used by
  the controlling agent. The scan must return no matches.

## Current Baseline

The current app supports opening and saving a YAML file, backup-on-save, line
numbers, cursor movement, single selection, copy, paste, undo, redo, search,
basic find highlights, `Ctrl+G` line jump, `Ctrl+D` single-selection next match,
tab/outdent, line comments, IME commit, viewport line slicing, completion popup,
inline diagnostics, diagnostics strip, YAML path status, and proxy-chain preview.

The current core supports rope-backed text, tree-sitter YAML syntax errors,
line/position primitives, YAML path lookup for common block YAML, basic Mihomo
schema docs, root/enum/reference completions, missing reference diagnostics,
and `dialer-proxy` cycle diagnostics.

The largest gaps versus VS Code plus YAML and Zed plus YAML are general editor
polish, multi-selection editing, find/replace, syntax highlighting, YAML
outline/folding/schema coverage, formatting, and mature popup/focus behavior.

## Milestone 1: Normal Editor Surface

Goal: make the editor surface feel predictable before adding more semantic UI.

### M1.1 Syntax Highlighting

Judgment: Without syntax colors the app still feels like a text area, not an
editor.

Scope:

- Add a UI-independent token model in `mocode-core` or `mocode-yaml`.
- Use tree-sitter YAML nodes to classify comments, keys, strings, numbers,
  booleans, nulls, anchors, aliases, tags, punctuation, and errors.
- Render visible-line highlight spans in `crates/mocode`.

Files:

- `crates/mocode-yaml/src/lib.rs`
- `crates/mocode-core/src/lib.rs`
- `crates/mocode-api/src/lib.rs`
- `crates/mocode/src/component.rs`
- `crates/mocode/src/main.rs`

Acceptance:

- Comments, keys, strings, numbers, booleans, nulls, aliases, and syntax errors
  have distinct styles.
- Highlighting is requested only for visible line ranges.
- A 20000-line file still renders through viewport slices.
- Mihomo semantics remain outside the app crate.

Tests:

- Unit test token ranges in `mocode-yaml`.
- Core test for `semantic_lines_in_range` carrying highlight spans.
- App source/behavior tests for rendering highlight spans.

Commit: `feat(editor): add yaml syntax highlighting`

### M1.2 Text Coordinate Correctness

Judgment: Daily use depends on cursor, click, IME, and selection matching what
the user sees.

Scope:

- Audit `TextPosition`, UTF-16 indices, Unicode scalar indices, and painted
  character widths.
- Add grapheme-aware helper functions for UI hit testing and cursor placement
  without changing core storage prematurely.
- Fix CJK click positioning, cursor rendering, and selection segment rendering
  where tests expose mismatch.

Files:

- `crates/mocode-text/src/lib.rs`
- `crates/mocode-core/src/lib.rs`
- `crates/mocode/src/component.rs`
- `crates/mocode/src/app.rs`
- `crates/mocode/src/main.rs`

Acceptance:

- Chinese text can be inserted, clicked, selected, copied, deleted, and replaced
  without cursor drift in the tested cases.
- IME marked text does not corrupt selection or saved text.
- Existing ASCII tests continue to pass.

Tests:

- Add CJK click-to-column tests.
- Add CJK selection copy/delete/replace tests.
- Add IME preedit/commit regression tests.

Commit: `fix(editor): harden unicode cursor mapping`

### M1.3 Mouse Editing Basics

Judgment: A common editor needs mouse selection beyond click-and-drag.

Scope:

- Double-click selects current YAML identifier.
- Triple-click selects current line.
- Shift-click extends selection.
- Drag selection remains stable while scrolling is not yet implemented.

Files:

- `crates/mocode/src/component.rs`
- `crates/mocode/src/main.rs`

Acceptance:

- Single click moves cursor.
- Double click selects word-like YAML token.
- Triple click selects line content.
- Shift-click creates an ordered or reversed selection.

Tests:

- Mouse event pure function tests where possible.
- Document-level selection tests for word and line ranges.

Commit: `feat(editor): add mouse word and line selection`

### M1.4 Editor Command Cleanup

Judgment: Search, go-to-line, completion, and normal editing currently share
input paths. This must stay explicit as more commands arrive.

Scope:

- Introduce a small app command mode enum such as `Normal`, `Search`,
  `GoToLine`, and `Completion`.
- Route Backspace, Delete, Enter, Escape, text input, and IME through that mode.
- Preserve current behavior.

Files:

- `crates/mocode/src/component.rs`
- `crates/mocode/src/app.rs`
- `crates/mocode/src/main.rs`

Acceptance:

- Search input never writes into YAML.
- Go-to-line input never writes into YAML.
- Completion popup accepts and closes consistently.
- Escape priority is completion, command mode, then normal no-op.

Tests:

- Existing search and go-to-line tests remain green.
- New mode-transition tests cover Enter/Escape/Backspace.

Commit: `refactor(app): centralize editor command modes`

## Milestone 2: Daily Editing Commands

Goal: close the most painful gap with VS Code and Zed basic editing.

### M2.1 Find and Replace

Judgment: Search without replace is not enough for editing large configs.

Scope:

- Add replace query state beside search query.
- Support replace current match, replace next, and replace all for literal text.
- Keep regex out of the first implementation.

Files:

- `crates/mocode/src/component.rs`
- `crates/mocode/src/app.rs`
- `crates/mocode/src/main.rs`

Acceptance:

- `Ctrl+F` starts find.
- A replace command can be entered through a compact status-bar command state.
- Replace current preserves undo history.
- Replace all reports count and marks document dirty.

Tests:

- Replace current with and without selection.
- Replace all across multiple lines.
- Undo after replace all restores previous text.

Commit: `feat(editor): add literal find and replace`

### M2.2 Multi-Selection Text Model

Judgment: `Ctrl+D` should eventually create multiple selections, not only move
one selection.

Scope:

- Extend app selection state from one anchor to a small ordered list of ranges.
- Keep `mocode-text` edit primitives single-range, but add an app-level batch
  edit helper that applies selections from bottom to top.
- Update render path to display multiple selections and one primary cursor.

Files:

- `crates/mocode/src/component.rs`
- `crates/mocode/src/main.rs`
- `crates/mocode-text/src/lib.rs` only if shared range helpers are needed.

Acceptance:

- `Ctrl+D` adds the next occurrence to selections.
- Typing replaces all selections.
- Backspace/Delete removes all selections.
- Copy copies selected text in document order.
- Escape collapses to the primary cursor.

Tests:

- Add two selections and type replacement.
- Delete multiple selections.
- Copy multiple selections.
- Undo restores all replaced text.

Commit: `feat(editor): add multi-selection editing`

### M2.3 Selection Commands

Judgment: Once multi-selection exists, selection commands should match common
editor expectations.

Scope:

- Add select all occurrences.
- Add skip current occurrence.
- Add expand/shrink selection for YAML token, line, block.

Files:

- `crates/mocode/src/component.rs`
- `crates/mocode/src/main.rs`

Acceptance:

- `Ctrl+Shift+L` selects all occurrences of current selection.
- Skip current moves the active selection to the next occurrence.
- Expand selection moves token -> line -> YAML block where possible.

Tests:

- All occurrences selection.
- Skip current with three matches.
- Expand selection around a key/value line.

Commit: `feat(editor): add selection expansion commands`

### M2.4 Line Operations

Judgment: Config editing often means rearranging rules and providers line by
line.

Scope:

- Delete current line.
- Duplicate line.
- Move line up/down.
- Copy line up/down if it does not complicate keybindings.

Files:

- `crates/mocode-text/src/lib.rs`
- `crates/mocode-core/src/lib.rs`
- `crates/mocode/src/component.rs`
- `crates/mocode/src/main.rs`

Acceptance:

- Commands work with no selection.
- Commands work with single or multi-line selection.
- Cursor and selection remain coherent.
- Undo/redo works.

Tests:

- TextBuffer line edit tests.
- App command tests for selected and unselected lines.

Commit: `feat(editor): add line editing commands`

### M2.5 Regex Search

Judgment: Literal search covers daily use first; regex can follow once replace
is stable.

Scope:

- Add regex toggle for search and replace.
- Use Rust `regex` crate in app-level search.
- Surface invalid regex as command-state status, not a diagnostic.

Files:

- `crates/mocode/Cargo.toml`
- `crates/mocode/src/component.rs`
- `crates/mocode/src/main.rs`

Acceptance:

- Regex find next/previous works.
- Regex replace supports capture groups.
- Invalid regex does not panic or alter text.

Tests:

- Regex search across lines where valid.
- Replace with capture group.
- Invalid regex status test.

Commit: `feat(editor): add regex search and replace`

## Milestone 3: YAML Intelligence

Goal: become a strong YAML editor before adding deeper Mihomo workflow.

### M3.1 YAML Outline

Judgment: Large Mihomo files need section navigation.

Scope:

- Build a document symbol model from YAML structure.
- Include root keys, named proxies, proxy groups, providers, rule providers,
  listeners, and rules section anchors.
- Add a compact outline panel or command-state list.

Files:

- `crates/mocode-yaml/src/lib.rs`
- `crates/mocode-mihomo-lint/src/lib.rs`
- `crates/mocode-core/src/lib.rs`
- `crates/mocode-api/src/lib.rs`
- `crates/mocode/src/component.rs`
- `crates/mocode/src/main.rs`

Acceptance:

- Outline jumps to `proxies`, `proxy-groups`, `rules`, `dns`, and `tun`.
- Named proxy/group entries appear with line numbers.
- Outline data comes from core, not app parsing.

Tests:

- YAML outline unit tests.
- Mihomo semantic outline tests.
- App jump tests.

Commit: `feat(editor): add mihomo yaml outline`

### M3.2 YAML Folding

Judgment: Folding is essential for navigating large config blocks.

Scope:

- Compute fold ranges for YAML mapping and sequence blocks.
- Render collapsed rows with a clear marker.
- Keep folded state in app document state.

Files:

- `crates/mocode-yaml/src/lib.rs`
- `crates/mocode-core/src/lib.rs`
- `crates/mocode/src/component.rs`
- `crates/mocode/src/main.rs`

Acceptance:

- Root sections can fold/unfold.
- Proxy and group entries can fold/unfold.
- Diagnostics inside folded ranges are summarized on the folded row.

Tests:

- Fold range computation.
- App visible-line slicing with folded ranges.
- Diagnostic summary on folded row.

Commit: `feat(editor): add yaml folding`

### M3.3 Conservative Formatter

Judgment: Whole-document YAML format can destroy user style. Start with safe
range formatting.

Scope:

- Format only selected generated snippets and simple indentation ranges.
- Preserve comments, key order, quote style, anchors, aliases, and block scalar
  content.
- Keep whole-document format disabled until tests cover enough YAML cases.

Files:

- `crates/mocode-yaml/src/lib.rs`
- `crates/mocode-core/src/lib.rs`
- `crates/mocode/src/component.rs`
- `crates/mocode/src/main.rs`

Acceptance:

- Format selection normalizes indentation for simple block YAML.
- Comments and blank lines remain.
- Unsupported ranges return a user-visible status instead of rewriting.

Tests:

- Simple mapping/list format.
- Comments preserved.
- Block scalar rejected or preserved exactly.
- Anchors and aliases preserved.

Commit: `feat(yaml): add conservative range formatter`

### M3.4 YAML Edge Cases

Judgment: Mihomo users may use anchors, aliases, merge keys, block scalars, and
flow YAML.

Scope:

- Improve YAML path lookup for anchors, aliases, merge keys, block scalars, flow
  maps, and flow sequences where tree-sitter exposes enough structure.
- Add syntax diagnostic ranges for common broken cases.

Files:

- `crates/mocode-yaml/src/lib.rs`
- `tests/fixtures/`

Acceptance:

- Existing common block YAML behavior remains unchanged.
- Edge fixtures have stable path or explicit unsupported behavior.
- No panics on valid YAML constructs.

Tests:

- Fixtures for anchors, merge keys, block scalars, flow maps, flow sequences,
  and multi-document separators.

Commit: `fix(yaml): handle common yaml edge cases`

## Milestone 4: Mihomo Semantic Depth

Goal: make mocode meaningfully better than a generic YAML editor for Mihomo.

### M4.1 Schema Catalog Expansion

Judgment: Field completion and hover are only useful if coverage is broad and
maintainable.

Scope:

- Expand schema coverage for general, inbound ports, dns, tun, proxies,
  proxy-groups, providers, sniffer, listeners, and external controller fields.
- Add source links in schema metadata where stable.
- Add schema coverage tests by section.

Files:

- `crates/mocode-mihomo-schema/src/lib.rs`
- `docs/mihomo-schema-design.md`
- `tests/fixtures/`

Acceptance:

- Each documented Mihomo section has root and nested field completions.
- Enum completions exist for known enum fields.
- Hover summaries are concise and actionable.

Tests:

- One completion/hover test per major section.
- Snapshot-like label tests for high-value fields.

Commit: `feat(schema): expand mihomo field catalog`

### M4.2 Rules Grammar

Judgment: `rules` are the densest and most error-prone part of Mihomo configs.

Scope:

- Parse rule lines into type, payload, target, and params where applicable.
- Support common rule types first: `DOMAIN`, `DOMAIN-SUFFIX`, `DOMAIN-KEYWORD`,
  `IP-CIDR`, `IP-CIDR6`, `GEOIP`, `GEOSITE`, `MATCH`, `RULE-SET`,
  `PROCESS-NAME`, `DST-PORT`, and logical rules.
- Provide rule template completions and target completions.

Files:

- `crates/mocode-mihomo-lint/src/rules.rs`
- `crates/mocode-mihomo-lint/src/lib.rs`
- `crates/mocode-core/src/lib.rs`
- `crates/mocode-mihomo-schema/src/lib.rs`
- `crates/mocode/src/main.rs`

Acceptance:

- Invalid rule arity produces diagnostics.
- Rule target completion includes proxies, groups, providers where valid, and
  built-ins.
- `RULE-SET` validates provider references.
- Logical rule parsing handles parentheses without splitting commas inside.

Tests:

- Parser tests for each supported rule family.
- Diagnostics tests for invalid target and bad arity.
- Completion tests inside rules.

Commit: `feat(lint): parse and validate mihomo rules`

### M4.3 Provider Semantics

Judgment: Provider-heavy configs cannot be validated well with only local
proxy/group names.

Scope:

- Index `proxy-providers` and `rule-providers` names, types, paths, URLs, health
  checks, and inline payloads.
- Validate references to providers.
- Treat remote contents as unknown unless inline or supplied by a host.

Files:

- `crates/mocode-mihomo-lint/src/lib.rs`
- `crates/mocode-core/src/lib.rs`
- `tests/fixtures/providers.yaml`

Acceptance:

- Provider references complete and validate.
- Inline provider payloads contribute known names where safe.
- Remote provider contents do not produce false missing-reference errors.

Tests:

- Local file provider index.
- Inline provider payload index.
- Remote provider unknown-state behavior.

Commit: `feat(lint): improve provider reference semantics`

### M4.4 Risk Diagnostics

Judgment: The editor should catch common risky but syntactically valid Mihomo
settings.

Scope:

- Add warning/info diagnostics for TUN, DNS, sniffer, listener, and external
  controller risk patterns.
- Keep risk hints separate from definite errors.

Files:

- `crates/mocode-mihomo-lint/src/lib.rs`
- `crates/mocode-mihomo-schema/src/lib.rs`
- `tests/fixtures/`

Acceptance:

- `external-controller` exposed without `secret` warns.
- DNS `respect-rules` without compatible nameserver strategy warns.
- TUN route combinations warn for common risky combinations.
- Empty proxy groups warn unless populated by provider/use/include-all logic.

Tests:

- One fixture per risk class.
- Severity tests for warning/info/error separation.

Commit: `feat(lint): add mihomo risk diagnostics`

### M4.5 References and Rename

Judgment: The app needs go-to-reference and safe rename for proxies and groups.

Scope:

- Implement `references_at(position)` in core.
- Add go-to-definition for proxy/group/provider references.
- Add safe rename for proxy/group/provider names across known references.

Files:

- `crates/mocode-core/src/lib.rs`
- `crates/mocode-mihomo-lint/src/lib.rs`
- `crates/mocode-api/src/lib.rs`
- `crates/mocode/src/component.rs`
- `crates/mocode/src/main.rs`

Acceptance:

- From a reference, jump to definition.
- From a definition, list references.
- Rename updates definitions and references, but refuses ambiguous remote
  provider contents.

Tests:

- Reference lookup tests.
- Rename apply-edit tests.
- App jump tests.

Commit: `feat(core): add mihomo references and rename`

## Milestone 5: Completion, Hover, and Diagnostics UX

Goal: make semantic features reliable in the app surface.

### M5.1 Completion Replacement Ranges

Judgment: UI should not guess which text to replace.

Scope:

- Compute replacement ranges in core completions.
- App accepts completion by applying the supplied range.
- Avoid duplicate prefix logic in the app.

Files:

- `crates/mocode-core/src/lib.rs`
- `crates/mocode-api/src/lib.rs`
- `crates/mocode/src/component.rs`
- `crates/mocode/src/main.rs`

Acceptance:

- Field completions replace partial field names.
- Enum completions replace partial scalar values.
- Reference completions replace partial reference names.

Tests:

- Core range tests.
- App completion acceptance tests.

Commit: `refactor(completion): use core replacement ranges`

### M5.2 Completion Ranking and Filtering

Judgment: Completion lists must be short and relevant.

Scope:

- Rank by YAML path, prefix match, exact field category, built-ins, and local
  references.
- Hide invalid self-references for immediate `dialer-proxy` cycles.
- Add documentation snippets to completion details.

Files:

- `crates/mocode-core/src/lib.rs`
- `crates/mocode-mihomo-schema/src/lib.rs`
- `crates/mocode/src/component.rs`

Acceptance:

- Typing a prefix narrows completion results.
- `dialer-proxy` does not suggest itself.
- Rule target completion prioritizes local groups/proxies over built-ins.

Tests:

- Ranking tests per context.
- App popup order tests.

Commit: `feat(completion): rank mihomo completions`

### M5.3 Hover Popup

Judgment: Current hover data is mostly status/inspector text. Users need a
real hover surface.

Scope:

- Render hover popup near cursor or mouse.
- Include field docs, enum docs, diagnostics, reference target summary, and
  chain preview where relevant.
- Keep popup from stealing text input focus.

Files:

- `crates/mocode-core/src/lib.rs`
- `crates/mocode/src/component.rs`
- `crates/mocode/src/main.rs`

Acceptance:

- Hover on field key shows docs.
- Hover on diagnostic range shows diagnostic message.
- Hover on `dialer-proxy` value shows chain summary.
- Popup closes on cursor movement or Escape.

Tests:

- Core hover payload tests.
- App source/behavior tests for popup state and close behavior.

Commit: `feat(app): add editor hover popup`

### M5.4 Diagnostics Panel

Judgment: Inline diagnostics are useful, but large files need a navigable list.

Scope:

- Add diagnostics panel with severity, message, path, line, and click-to-jump.
- Allow filtering by severity.
- Keep the panel compact and collapsible.

Files:

- `crates/mocode/src/component.rs`
- `crates/mocode/src/main.rs`

Acceptance:

- Diagnostics list updates after edits.
- Clicking an item jumps to range.
- Panel can collapse without hiding inline diagnostics.

Tests:

- Panel item mapping tests.
- Jump tests.
- Collapse state tests.

Commit: `feat(app): add diagnostics panel`

## Milestone 6: App Shell Usability

Goal: make the standalone app comfortable without turning it into a full client.

### M6.1 Recent Files and Dirty Close Guard

Judgment: A daily editor needs to reopen common config files safely.

Scope:

- Store recent file paths in a small local config file.
- Add open recent command list.
- Add close/exit dirty guard where GPUI app lifecycle supports it.

Files:

- `crates/mocode/src/app.rs`
- `crates/mocode/src/component.rs`
- `crates/mocode/src/main.rs`

Acceptance:

- Recently opened paths are listed.
- Missing recent paths are handled gracefully.
- Dirty document cannot be silently discarded through open/exit paths covered by
  tests.

Tests:

- Recent file persistence tests using temp directory.
- Dirty guard tests.

Commit: `feat(app): add recent files`

### M6.2 Command Palette

Judgment: As commands grow, shortcuts alone are not discoverable.

Scope:

- Add a compact command palette for app/editor commands.
- Include open, save, save as, go to line, find, replace, outline, diagnostics,
  fold toggles, and theme toggle when available.

Files:

- `crates/mocode/src/component.rs`
- `crates/mocode/src/app.rs`
- `crates/mocode/src/main.rs`

Acceptance:

- Palette opens by shortcut.
- Typing filters commands.
- Enter runs selected command.
- Escape closes without editing YAML.

Tests:

- Command filtering tests.
- Command execution tests.
- Input routing tests.

Commit: `feat(app): add command palette`

### M6.3 Layout and Inspector Polish

Judgment: The editor should not feel like debug panels glued around text.

Scope:

- Make side inspector collapsible.
- Move secondary panels into tabs or compact sections.
- Keep editor area primary.
- Add status bar items for mode, path, diagnostics, completion count, line/col,
  and dirty state.

Files:

- `crates/mocode/src/component.rs`
- `crates/mocode/src/app.rs`
- `crates/mocode/src/main.rs`

Acceptance:

- First viewport is mostly editor text.
- Inspector can be hidden.
- Completion and hover are contextual popups, not persistent bars.

Tests:

- Source tests preventing old debug panel names.
- State tests for inspector collapse.

Commit: `refactor(app): polish editor layout`

### M6.4 Theme and Font Settings

Judgment: Long editing sessions need readable colors and fonts.

Scope:

- Add a minimal theme model: light and dark.
- Add font size setting.
- Keep styles centralized.

Files:

- `crates/mocode/src/component.rs`
- `crates/mocode/src/app.rs`
- `crates/mocode/src/main.rs`

Acceptance:

- Light/dark theme can toggle.
- Font size changes line height and hit testing consistently.
- Syntax highlight colors adapt to theme.

Tests:

- Theme state tests.
- Font size geometry tests.

Commit: `feat(app): add theme and font settings`

## Milestone 7: Performance and Reliability

Goal: keep 5000-20000 line files responsive as features accumulate.

### M7.1 Incremental YAML Parsing

Judgment: Re-parsing and re-indexing full text after each edit will become too
slow.

Scope:

- Use tree-sitter incremental parsing with edit deltas.
- Keep a safe full-parse fallback.
- Measure parse time for small and 20000-line fixtures.

Files:

- `crates/mocode-yaml/src/lib.rs`
- `crates/mocode-core/src/lib.rs`
- `crates/mocode-text/src/lib.rs`

Acceptance:

- Incremental edit updates syntax errors and path lookup.
- Full parse fallback remains correct.
- Performance tests record parse/index time without making CI flaky.

Tests:

- Incremental parse correctness tests.
- Benchmark-like ignored tests or deterministic timing smoke tests.

Commit: `feat(yaml): add incremental parsing`

### M7.2 Debounced Semantic Refresh

Judgment: Completion and diagnostics should not block every keystroke on large
files.

Scope:

- Split immediate text update from debounced diagnostics/index refresh where
  GPUI supports scheduling.
- Keep visible line rendering immediate.
- Surface stale diagnostics state in status bar if needed.

Files:

- `crates/mocode-core/src/lib.rs`
- `crates/mocode/src/component.rs`
- `crates/mocode/src/app.rs`

Acceptance:

- Typing remains responsive on 20000-line fixture.
- Diagnostics eventually refresh after edits.
- Completion at cursor can force a fresh enough context.

Tests:

- Core stale/fresh state tests if state is represented in core.
- App refresh scheduling tests where practical.

Commit: `perf(editor): debounce semantic refresh`

### M7.3 Large File Validation

Judgment: Performance must stay measurable, not anecdotal.

Scope:

- Add repeatable validation commands for large file open, scroll, search,
  completion, diagnostics, and save.
- Update manual checklist with measured fields.
- Add release build size recording.

Files:

- `docs/prototype-validation-checklist.md`
- `docs/editor-gap-closure-plan.md`
- `README.md`

Acceptance:

- Checklist has exact commands and expected observations.
- 5000-line and 20000-line fixtures are both covered.

Tests:

- Existing large fixture tests continue passing.

Commit: `docs: expand large file validation checklist`

## Milestone 8: Component API and Embedding

Goal: make the editor reusable while preserving the standalone app.

### M8.1 Public API Stabilization

Judgment: Host apps need a stable API surface, not access to app internals.

Scope:

- Define stable `mocode-api` types for document state, commands, events,
  diagnostics, completions, hover, outline, folds, and chain previews.
- Keep GPUI types out of `mocode-api`.

Files:

- `crates/mocode-api/src/lib.rs`
- `crates/mocode-core/src/lib.rs`
- `docs/spec.md`
- `docs/architecture.md`

Acceptance:

- Host-facing API compiles without GPUI.
- App crate uses the public facade where appropriate.
- API docs explain ownership and refresh model.

Tests:

- API smoke tests.
- Compile-time dependency checks through `cargo tree` where practical.

Commit: `feat(api): stabilize editor component facade`

### M8.2 Host Integration Notes

Judgment: The project is a component, so integration expectations must be
written down.

Scope:

- Document how a host app opens text, applies edits, renders visible lines,
  handles completions, receives diagnostics, and saves text.
- Document what host apps must provide for provider remote contents.

Files:

- `docs/host-integration.md`
- `README.md`

Acceptance:

- A host author can understand which crates to use and which crate to avoid.
- External Mihomo runtime responsibilities remain out of scope.

Tests:

- Documentation link checks by `rg` and manual review.

Commit: `docs: add host integration guide`

## Execution Order

Execute tasks strictly in this order unless the user explicitly reprioritizes:

1. M1.1 Syntax Highlighting
2. M1.2 Text Coordinate Correctness
3. M1.3 Mouse Editing Basics
4. M1.4 Editor Command Cleanup
5. M2.1 Find and Replace
6. M2.2 Multi-Selection Text Model
7. M2.3 Selection Commands
8. M2.4 Line Operations
9. M2.5 Regex Search
10. M3.1 YAML Outline
11. M3.2 YAML Folding
12. M3.3 Conservative Formatter
13. M3.4 YAML Edge Cases
14. M4.1 Schema Catalog Expansion
15. M4.2 Rules Grammar
16. M4.3 Provider Semantics
17. M4.4 Risk Diagnostics
18. M4.5 References and Rename
19. M5.1 Completion Replacement Ranges
20. M5.2 Completion Ranking and Filtering
21. M5.3 Hover Popup
22. M5.4 Diagnostics Panel
23. M6.1 Recent Files and Dirty Close Guard
24. M6.2 Command Palette
25. M6.3 Layout and Inspector Polish
26. M6.4 Theme and Font Settings
27. M7.1 Incremental YAML Parsing
28. M7.2 Debounced Semantic Refresh
29. M7.3 Large File Validation
30. M8.1 Public API Stabilization
31. M8.2 Host Integration Notes

## Completion Definition

mocode is daily-usable when the following are all true:

- A real Mihomo config can be opened, edited, searched, replaced, saved, and
  reopened without data loss.
- Chinese IME, CJK cursor movement, selection, copy, paste, undo, and redo are
  covered by tests and manual validation.
- YAML syntax highlighting, diagnostics, outline, folding, completions, hover,
  and conservative formatting are available.
- Mihomo references, rules, providers, `dialer-proxy` chains, and common
  TUN/DNS/controller risks are validated.
- The standalone app remains scoped to editing and does not become a Mihomo
  runtime client.
- `cargo test --workspace`, `cargo build -p mocode`, and the old-name scan pass
  before each shipped step.
