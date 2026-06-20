# App Validation Checklist

Date: 2026-06-20

This checklist is the repeatable validation baseline for the `mocode` GPUI application shell and the shared editor core.

## Scope

Validate only the editor component boundary:

- shared `mocode-core` semantics
- `mocode` app behavior
- large YAML loading baseline
- Windows Chinese IME behavior
- focus, scroll, clipboard, save, input, and packaging observations

Do not use this checklist to expand mocode into a full Mihomo GUI. TUN control, system proxy control, subscriptions, WebDAV, tray integration, and Mihomo core process management are out of scope.

## Fixtures

| Fixture | Lines | Purpose |
| --- | ---: | --- |
| `examples/configs/large.yaml` | 5372 | Current semantic large-file baseline with generated proxies, groups, and rules. |
| `examples/configs/large-20000.yaml` | 20000 | Current 20000-line editor loading baseline. It starts from `large.yaml` and appends YAML comments as padding. |

The app no longer renders sample fixture buttons by default. Use command-line
file paths for manual checks and the fixture-backed tests for repeatable large,
invalid YAML, invalid reference, and cycle coverage.

## Automated Validation

Run these commands from the repository root.

```powershell
cargo fmt --all --check
cargo test -p mocode-core loads_twenty_thousand_line_fixture_for_validation_baseline
cargo test -p mocode loads_twenty_thousand_line_fixture_for_validation_baseline
cargo check -p mocode
cargo test --workspace
cargo build -p mocode
Test-Path target\debug\mocode.exe
```

Optional dependency-scale command:

```powershell
cargo tree -p mocode -e normal --prefix none | Measure-Object -Line
```

Optional release binary size commands:

```powershell
cargo build -p mocode --release
Get-Item target\release\mocode.exe | Select-Object Name,Length
```

Record binary size in bytes and note whether the build includes debug symbols, platform SDK differences, or local cargo profile changes.

## Manual Launch Smoke

```powershell
cargo run -p mocode
```

Record:

- OS and display scaling.
- Whether the window opens reliably.
- Whether line numbers render.
- Whether the editor surface accepts focus.
- Whether cursor movement works with Left and Right.
- Whether Shift+Left and Shift+Right extend a text selection.
- Whether Ctrl+C or Cmd+C copies the selected text to the system clipboard.
- Whether Backspace and Delete mutate text.
- Whether paste inserts clipboard text.
- Whether undo and redo work.
- Whether save writes the current file path.
- Whether saving an existing file creates a sibling `.bak` backup before overwrite.
- Whether save-as writes a new YAML file and updates the app path.
- Whether the Open button can load another YAML file.
- Whether search can be started from selected text and move to next/previous matches.
- Whether diagnostics update after editing invalid YAML.
- Whether the bottom status bar updates cursor position, YAML path, diagnostics, completion count, search state, and chain preview.
- Whether the editor keeps focus after using Open, Save, Save As, and keyboard commands.

Use command-line paths or the Open button to load `examples/configs/large.yaml`,
`examples/configs/large-20000.yaml`, `examples/configs/invalid-yaml.yaml`,
`tests/fixtures/invalid-reference.yaml`, and `tests/fixtures/dialer-cycle.yaml`.

## Daily File Workflow Script

1. Copy a real Mihomo config to a temporary path.
2. Launch `target\debug\mocode.exe path\to\copy.yaml`.
3. Edit a harmless scalar or comment.
4. Save.
5. Verify `copy.yaml.bak` contains the pre-save content.
6. Verify `copy.yaml` contains the edited content.
7. Use Save As to write a second YAML file.
8. Close and relaunch with the saved file path.
9. Confirm diagnostics and YAML path still render.

Do not run this script directly on the only copy of a production config.

## Search Script

1. Select a known token such as a proxy name.
2. Start search from the selection.
3. Move to the next match.
4. Move to the previous match.
5. Verify the status bar shows the query, ordinal, total match count, and match location.
6. Verify the current match is selected in the editor surface.

## Windows Chinese IME Script

Run on Windows with a Chinese IME enabled.

1. Focus the editor surface.
2. Move the cursor to a scalar value position, for example after `name:`.
3. Type `ceshi jiedian` through the IME and commit `测试节点`.
4. Verify the committed text lands at the cursor.
5. Verify the cursor position after commit.
6. Check whether preedit text is visible while composing.
7. Press Backspace once after commit and verify exactly one committed character is removed.
8. Paste a Chinese scalar such as `香港节点` and verify the YAML path and diagnostics refresh.

Record these fields:

| Field | Result | Notes |
| --- | --- | --- |
| Commit inserts text at cursor |  |  |
| Cursor position after commit |  |  |
| Preedit visible |  |  |
| Backspace after commit |  |  |
| Paste Chinese text |  |  |

## Scroll And Focus Script

Launch with `examples/configs/large.yaml` and `examples/configs/large-20000.yaml`.

Record:

- Initial render time by observation.
- Whether scrolling remains responsive.
- Whether row height stays stable.
- Whether line number gutter stays aligned.
- Whether cursor rendering remains aligned with text.
- Whether future completion/hover surfaces steal focus after they are reintroduced.
- Whether diagnostics remain attached to the intended lines.
- Whether CPU usage spikes persist after scrolling stops.

## Selection And Copy Script

1. Focus the editor surface.
2. Move to a scalar text position.
3. Press Shift+Right several times.
4. Verify the status bar selection summary changes from `<none>` to a range.
5. Press Ctrl+C on Windows/Linux or Cmd+C on macOS.
6. Paste into an external text field and verify the selected YAML text was copied.
7. Press Right without Shift and verify the status bar selection summary disappears.
8. Repeat Shift+Left from the same line and verify reversed selection still copies the expected text.

## Completion And Hover Script

Test with a sample config or the fixture-backed automated tests:

- Root field completion at the first line should include `mixed-port`.
- `dns.enhanced-mode` completion should include `fake-ip`.
- `proxy-groups[0].proxies` completion should include known outbounds and built-ins.
- `proxies[0].dialer-proxy` completion should include known outbounds.
- Hover over `tun.stack` should show Mihomo schema documentation.
- Hover over `proxies[].dialer-proxy` should explain outbound chaining.

Record whether the status bar completion count changes as expected. Full completion and hover popup rendering is a later hardening task; it must continue to source data from `mocode-core`.

## Diagnostic Script

Use existing automated fixtures through tests or by opening the files directly:

- `examples/configs/invalid-yaml.yaml` should produce a `yaml.syntax` diagnostic.
- `tests/fixtures/invalid-reference.yaml` should produce a missing-reference diagnostic.
- `tests/fixtures/dialer-cycle.yaml` should produce a `mihomo.dialer_proxy.cycle` diagnostic.

## Result Record Template

| Date | Commit | OS | Command or Script | Result | Notes |
| --- | --- | --- | --- | --- | --- |
|  |  |  |  | Pass / Fail / Blocked / Not run |  |

## Readiness Gate

Do not treat the app shell as production-ready until these items are recorded:

- Windows Chinese IME commit and preedit behavior.
- Interactive scroll behavior with a 5000-20000 line YAML file.
- Focus behavior around future completion and hover surfaces.
- Keyboard selection and copy ergonomics.
- Release binary size.

Until then, continue work behind the UI-independent `mocode-core` and `mocode-api` boundary. Do not move Mihomo semantics into the app crate.
