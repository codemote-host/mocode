# App Validation Checklist

Date: 2026-06-20

This checklist is the repeatable validation baseline for the `mocode` GPUI application shell and the shared editor core.

## Scope

Validate only the editor component boundary:

- shared `mocode-core` semantics
- `mocode` app behavior
- large YAML loading baseline
- Windows Chinese IME behavior
- focus, popup, scroll, clipboard, save, and packaging observations

Do not use this checklist to expand mocode into a full Mihomo GUI. TUN control, system proxy control, subscriptions, WebDAV, tray integration, and Mihomo core process management are out of scope.

## Fixtures

| Fixture | Lines | Purpose |
| --- | ---: | --- |
| `examples/configs/large.yaml` | 5372 | Current semantic large-file baseline with generated proxies, groups, and rules. |
| `examples/configs/large-20000.yaml` | 20000 | Current 20000-line editor loading baseline. It starts from `large.yaml` and appends YAML comments as padding. |

The built-in fixture selector exposes:

- Dialer
- Minimal
- DNS
- TUN
- Groups
- Providers
- Bad YAML
- Bad Ref
- Cycle
- Large
- 20k

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
- Whether diagnostics update after editing invalid YAML.
- Whether the completion strip updates when the cursor changes.
- Whether the completion popup anchor changes when the cursor changes.
- Whether the inspector shows current YAML path, selection summary, hover summary, diagnostics, and chain preview.
- Whether focus returns to the editor after interacting with visible panels and fixture selector buttons.

Use the fixture selector to switch to `Large`, `20k`, `Bad YAML`, `Bad Ref`, and `Cycle`. The selector is intentionally limited to built-in fixtures; it is not a general file-open UI.

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

Use the fixture selector to load `Large` and `20k`.

Record:

- Initial render time by observation.
- Whether scrolling remains responsive.
- Whether row height stays stable.
- Whether line number gutter stays aligned.
- Whether cursor rendering remains aligned with text.
- Whether completion/hover panels steal focus.
- Whether diagnostics remain attached to the intended lines.
- Whether CPU usage spikes persist after scrolling stops.

## Selection And Copy Script

1. Focus the editor surface.
2. Move to a scalar text position.
3. Press Shift+Right several times.
4. Verify the inspector selection summary changes from `<none>` to a range.
5. Press Ctrl+C on Windows/Linux or Cmd+C on macOS.
6. Paste into an external text field and verify the selected YAML text was copied.
7. Press Right without Shift and verify the selection summary returns to `<none>`.
8. Repeat Shift+Left from the same line and verify reversed selection still copies the expected text.

## Completion And Hover Script

Test with the built-in sample:

- Root field completion at the first line should include `mixed-port`.
- `dns.enhanced-mode` completion should include `fake-ip`.
- `proxy-groups[0].proxies` completion should include known outbounds and built-ins.
- `proxies[0].dialer-proxy` completion should include known outbounds.
- Hover over `tun.stack` should show Mihomo schema documentation.
- Hover over `proxies[].dialer-proxy` should explain outbound chaining.

Record whether the popup panel shows the expected `Popup @ line:column` anchor and whether the first few popup items match the completion strip.

## Diagnostic Script

Use existing automated fixtures through the selector and tests:

- `examples/configs/invalid-yaml.yaml` should produce a `yaml.syntax` diagnostic.
- `tests/fixtures/invalid-reference.yaml` should produce a missing-reference diagnostic.
- `tests/fixtures/dialer-cycle.yaml` should produce a `mihomo.dialer_proxy.cycle` diagnostic.

The selector labels for these are `Bad YAML`, `Bad Ref`, and `Cycle`.

## Result Record Template

| Date | Commit | OS | Command or Script | Result | Notes |
| --- | --- | --- | --- | --- | --- |
|  |  |  |  | Pass / Fail / Blocked / Not run |  |

## Readiness Gate

Do not treat the app shell as production-ready until these items are recorded:

- Windows Chinese IME commit and preedit behavior.
- Interactive scroll behavior with a 5000-20000 line YAML file.
- Focus behavior around completion and hover surfaces.
- Keyboard selection and copy ergonomics.
- Release binary size.

Until then, continue work behind the UI-independent `mocode-core` and `mocode-api` boundary. Do not move Mihomo semantics into the app crate.
