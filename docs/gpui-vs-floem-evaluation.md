# GPUI vs Floem Evaluation

Date: 2026-06-18

This is the first repository-backed evaluation of GPUI and Floem as UI carriers for mocode. It compares the current prototypes only. It is not a final framework selection.

## Executive Summary

Both prototypes now use the same `mocode-api` and keep Mihomo schema, lint, YAML path, hover, completion, and text mutation logic outside the UI layer. That boundary is the most important result so far: neither GPUI nor Floem forced Mihomo business logic into the adapter.

Current state:

- GPUI is slightly ahead on editor-like event structure because it has explicit key bindings, focus handles, actions, and a `uniform_list` path that maps naturally to editor rows.
- Floem is slightly ahead on reactive semantic display and IME committed text because the demo now stores `DemoDocument` in an `RwSignal`, uses `virtual_stack`, and handles `ImeCommit`.
- Neither prototype is ready for final selection. The missing decisive data is real manual validation: Chinese IME preedit and commit behavior, smooth scrolling on 5000-20000 lines, focus/popup reliability, selection/copy ergonomics, and packaged binary size.

Provisional recommendation:

Continue one more validation slice before selecting a primary UI framework. If forced to choose only from current repository facts, keep both candidates active: GPUI remains the higher-confidence editor-shell candidate; Floem remains the simpler reactive adapter candidate with a smaller observed dependency tree.

## Evidence Snapshot

Local versions:

| Item | GPUI Demo | Floem Demo |
| --- | --- | --- |
| Crate | `mocode-gpui-demo` | `mocode-floem-demo` |
| UI dependency | `gpui = "0.2.2"` | `floem = "0.2.0"` |
| Extra direct UI data dependency | none | `im = "15.1"` for `virtual_stack` data |
| Shared component API | `mocode-api` | `mocode-api` |
| Source size | 691 lines | 708 lines |
| Current core size | `mocode-core` is 487 lines | same |
| Approximate normal dependency tree lines | 1012 | 506 |

The dependency tree numbers come from local `cargo tree -e normal --prefix none` output and are only a dependency-scale signal. They are not binary size measurements.

Verification used for this report:

- `cargo check -p mocode-gpui-demo`
- `cargo check -p mocode-floem-demo`
- `cargo test --workspace`
- Source inspection of both demo adapters

Validation harness status:

- [prototype-validation-checklist.md](prototype-validation-checklist.md) defines the repeatable validation procedure.
- `examples/configs/large-20000.yaml` is available as a 20000-line editor loading baseline.
- `mocode-core`, `mocode-gpui-demo`, and `mocode-floem-demo` all have automated 20000-line fixture loading tests.
- Manual Windows IME, interactive large-file scrolling, focus, popup, and release-size measurements are still open.

## Acceptance Matrix

| Requirement | GPUI Current Status | Floem Current Status | Notes |
| --- | --- | --- | --- |
| Load 5000-20000 line YAML | Partial | Partial | Both have automated adapter-state load tests for `examples/configs/large.yaml` with 5372 lines and `examples/configs/large-20000.yaml` with 20000 lines. Interactive 20000-line scroll testing still needs file-open UI or a sample switcher. |
| Smooth scrolling | Needs manual validation | Needs manual validation | GPUI uses `uniform_list`; Floem uses `virtual_stack`. No frame timing or screenshot verification yet. |
| Line numbers | Implemented | Implemented | Both render line gutters. |
| Cursor movement | Implemented | Implemented | Left/right movement delegates to `mocode-core`. |
| Text selection | Not implemented | Not implemented | Required before final acceptance. |
| Copy | Not implemented | Not implemented | Paste exists; copy requires selection first. |
| Paste | Implemented | Implemented | GPUI uses app clipboard read; Floem uses `Clipboard::get_contents()`. |
| Chinese IME test | Needs manual validation | Partial | Floem handles `ImeCommit`; neither demo renders IME preedit. GPUI does not yet have explicit IME commit handling. |
| YAML syntax error rendering | Implemented | Implemented | Diagnostics are displayed from core; row markers exist for ranged diagnostics. |
| Hover docs | Implemented | Implemented | Both display hover summary from `mocode-core`. |
| Field-name completion | Implemented | Implemented | Both render current completion items from core. |
| `proxy-groups.proxies` completion | Core implemented, UI path-dependent | Core implemented, UI path-dependent | Core tests cover reference completions. UI needs manual cursor-position checks beyond default sample. |
| `dialer-proxy` completion | Implemented | Implemented | Default demo position shows outbound reference completions. |
| Missing reference diagnostic | Implemented | Implemented | Both display diagnostics from core. |
| `dialer-proxy` cycle diagnostic | Core implemented, UI generic display | Core implemented, UI generic display | UI can display diagnostics, but default sample is not the cycle fixture. |
| Current YAML path panel | Implemented | Implemented | Both show current cursor path. |
| Chain preview panel | Not implemented | Not implemented | `MocodeEditor::proxy_chain_preview_at` still returns `None`. |
| No copied Mihomo business logic | Pass | Pass | UI adapters map core data to display DTOs only. |

## GPUI Prototype Notes

Strengths:

- The adapter shape is close to an editor shell: `MocodeGpuiDemo` owns document state, focus handle, key context, command actions, and a row virtualizer.
- `uniform_list` is a direct fit for editor rows and keeps the row rendering model simple.
- Key handling is explicit through GPUI actions for Backspace, Delete, Left, Right, and Paste, plus text insertion through key-down events.
- Existing README/research context records upstream Windows support through Win32 windowing and DirectWrite text.

Risks:

- The observed dependency tree is larger than Floem's in this workspace.
- IME commit/preedit handling is not wired explicitly in the demo.
- The UI adapter uses more framework-specific concepts: `Context`, `Window`, `FocusHandle`, actions, listeners, key contexts, and `cx.notify()`.
- Manual validation is still required for focus, popup layering, clipboard behavior, and smooth scrolling.

Implementation details:

- State methods delegate to `MocodeEditor`: `insert_text`, `backspace`, `delete`, `move_left`, `move_right`.
- Rendering uses a three-area layout: header, completion strip, editor surface, and inspector.
- The line row displays cursor by splitting text at the current `TextPosition`.

## Floem Prototype Notes

Strengths:

- The reactive model is concise for semantic display: one `RwSignal<DemoDocument>` drives header, completions, rows, cursor, inspector, and diagnostics.
- `virtual_stack` is a good fit for large YAML row virtualization.
- Floem exposes `ImeCommit`, and the demo inserts committed IME text.
- The observed dependency tree is smaller than GPUI's in this workspace.

Risks:

- Input handling is more manually assembled: the demo translates `EventListener::KeyDown`, `ImeCommit`, modifiers, clipboard, and focus requests into document actions.
- `virtual_stack` required an additional direct `im` dependency for data.
- IME preedit rendering is not implemented.
- Selection/copy and popup behavior are still untested.
- API stability and maintenance pace remain open questions from the research phase.

Implementation details:

- `DemoDocument` matches the GPUI state boundary and delegates edits to `MocodeEditor`.
- `DocumentSignal = RwSignal<DemoDocument>` is the only UI state carrier.
- Line rows use `DemoVisibleLine` to combine immutable line data and cursor position for rendering.

## Shared Core Boundary

The current architecture is holding:

- `mocode-text` owns text storage and primitive edit behavior.
- `mocode-yaml` owns YAML syntax errors and path lookup.
- `mocode-mihomo-schema` owns field docs and completion metadata.
- `mocode-mihomo-lint` owns semantic index validation.
- `mocode-core` exposes UI-neutral editor operations and derived semantic data.
- GPUI and Floem only translate framework events and render core-derived state.

This means framework selection can stay focused on rendering, input, focus, performance, packaging, and maintainability instead of re-litigating Mihomo semantics.

## Current Decision Pressure

Do not select a final framework yet.

Reasons:

- Both demos lack selection/copy.
- Chain preview is not implemented in core.
- Neither demo has measured smooth scrolling.
- Chinese IME is not fully tested; Floem has committed text handling, but preedit display is missing, and GPUI needs explicit IME wiring.
- Package size is not measured.
- Popup behavior for completion/hover has not been implemented in either framework.

## Next Validation Checklist

Before choosing GPUI or Floem:

1. Execute [prototype-validation-checklist.md](prototype-validation-checklist.md) on Windows.
2. Record Chinese IME commit and preedit behavior for both demos.
3. Add a file-open UI or sample switcher so `large-20000.yaml` can be tested interactively.
4. Add an automated startup/load timing command for `large.yaml` and `large-20000.yaml`.
5. Add screenshot-based smoke tests for both demos if the environment supports GUI capture.
6. Implement selection and copy in both demos.
7. Implement completion popup positioning in both demos.
8. Measure packaged binary size for release builds.
9. Record focus behavior when switching between editor, completion panel, and inspector.

## Provisional Scorecard

| Dimension | Current Lean | Reason |
| --- | --- | --- |
| Editor-shell ergonomics | GPUI | Actions, focus handle, key context, and `uniform_list` map well to editor UI. |
| Reactive semantic display | Floem | `RwSignal<DemoDocument>` keeps derived UI refresh straightforward. |
| IME committed text path | Floem | `ImeCommit` is wired in the current demo. |
| Dependency scale signal | Floem | Local normal dependency tree is smaller. |
| Core boundary preservation | Tie | Both adapters keep Mihomo logic out of UI code. |
| Large row virtualization | Tie | GPUI `uniform_list`; Floem `virtual_stack`. Manual performance testing still needed. |
| Current code size | Tie | 691 vs 708 lines is not meaningfully different. |
| Final selection readiness | Neither | Missing manual validation and acceptance items. |

## Recommended Next Step

Execute the validation harness and record measured data in this report. The next implementation slice should add a file-open UI or sample switcher before final scroll validation, then selection/copy parity.
