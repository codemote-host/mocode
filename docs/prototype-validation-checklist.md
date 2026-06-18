# Prototype Validation Checklist

Date: 2026-06-18

This checklist is the repeatable validation baseline for the mocode UI prototypes. After the GPUI decision, GPUI is the primary validation target. Floem checks are optional reference checks unless a regression affects shared core boundaries.

## Scope

Validate only the editor component prototype boundary:

- shared `mocode-core` semantics
- GPUI adapter behavior as the primary product path
- Floem adapter behavior as a frozen reference path
- large YAML loading baseline
- Windows Chinese IME behavior
- focus, popup, scroll, clipboard, and packaging observations

Do not use this checklist to expand mocode into a full Mihomo GUI. TUN control, system proxy control, subscriptions, WebDAV, tray integration, and Mihomo core process management are out of scope.

## Fixtures

| Fixture | Lines | Purpose |
| --- | ---: | --- |
| `examples/configs/large.yaml` | 5372 | Current semantic large-file baseline with generated proxies, groups, and rules. |
| `examples/configs/large-20000.yaml` | 20000 | Current 20000-line editor loading baseline. It starts from `large.yaml` and appends YAML comments as padding. |

The 20000-line fixture intentionally avoids a single oversized YAML sequence. A first generated variant with very large continuous sequences triggered a tree-sitter-yaml syntax error around line 16383, which would make UI validation measure parser behavior instead of editor loading and scrolling. A separate semantic-scale parser benchmark can be added later.

Both demos expose the same built-in fixture selector:

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
cargo test -p mocode-gpui-demo loads_twenty_thousand_line_fixture_for_validation_baseline
cargo test -p mocode-floem-demo loads_twenty_thousand_line_fixture_for_validation_baseline
cargo check -p mocode-gpui-demo
cargo check -p mocode-floem-demo
cargo test --workspace
```

Optional dependency-scale commands:

```powershell
cargo tree -p mocode-gpui-demo -e normal --prefix none | Measure-Object -Line
cargo tree -p mocode-floem-demo -e normal --prefix none | Measure-Object -Line
```

Optional release binary size commands:

```powershell
cargo build -p mocode-gpui-demo --release
cargo build -p mocode-floem-demo --release
Get-Item target\release\mocode-gpui-demo.exe,target\release\mocode-floem-demo.exe | Select-Object Name,Length
```

Record binary size in bytes and note whether the build includes debug symbols, platform SDK differences, or local cargo profile changes.

## Manual Launch Smoke

Run the GPUI demo first. Floem is optional reference validation.

```powershell
cargo run -p mocode-gpui-demo
cargo run -p mocode-floem-demo
```

For each demo that is run, record:

- OS and display scaling.
- Whether the window opens reliably.
- Whether line numbers render.
- Whether the editor surface accepts focus.
- Whether cursor movement works with Left and Right.
- Whether Shift+Left and Shift+Right extend a text selection.
- Whether Ctrl+C or Cmd+C copies the selected text to the system clipboard.
- Whether Backspace and Delete mutate text.
- Whether paste inserts clipboard text.
- Whether diagnostics update after editing invalid YAML.
- Whether the completion strip updates when the cursor changes.
- Whether the completion popup anchor changes when the cursor changes.
- Whether the inspector shows current YAML path, selection summary, hover summary, and diagnostics.
- Whether focus returns to the editor after interacting with visible panels and fixture selector buttons.

Use the fixture selector to switch to `Large`, `20k`, `Bad YAML`, `Bad Ref`, and `Cycle`. The selector is intentionally limited to built-in fixtures; it is not a general file-open UI.

## Windows Chinese IME Script

Run on Windows with a Chinese IME enabled. Test GPUI first; test Floem only when reference data is useful.

1. Focus the editor surface.
2. Move the cursor to a scalar value position, for example after `name:`.
3. Type `ceshi jiedian` through the IME and commit `测试节点`.
4. Verify the committed text lands at the cursor.
5. Verify the cursor position after commit.
6. Check whether preedit text is visible while composing.
7. Press Backspace once after commit and verify exactly one committed character is removed.
8. Paste a Chinese scalar such as `香港节点` and verify the YAML path and diagnostics refresh.

Record these fields:

| Field | GPUI | Floem |
| --- | --- | --- |
| Commit inserts text at cursor |  |  |
| Cursor position after commit |  |  |
| Preedit visible |  |  |
| Backspace after commit |  |  |
| Paste Chinese text |  |  |
| Notes |  |  |

## Scroll And Focus Script

Use the fixture selector to load `Large` and `20k` in the GPUI demo first. Floem is optional reference validation.

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

Run on the GPUI demo first. Floem is optional reference validation.

1. Focus the editor surface.
2. Move to a scalar text position.
3. Press Shift+Right several times.
4. Verify the inspector selection summary changes from `<none>` to a range.
5. Press Ctrl+C on Windows/Linux or Cmd+C on macOS.
6. Paste into an external text field and verify the selected YAML text was copied.
7. Press Right without Shift and verify the selection summary returns to `<none>`.
8. Repeat Shift+Left from the same line and verify reversed selection still copies the expected text.

## Completion And Hover Script

Test the GPUI demo with the built-in sample. Floem is optional reference validation:

- Root field completion at the first line should include `mixed-port`.
- `dns.enhanced-mode` completion should include `fake-ip`.
- `proxy-groups[0].proxies` completion should include known outbounds and built-ins.
- `proxies[0].dialer-proxy` completion should include known outbounds.
- Hover over `tun.stack` should show Mihomo schema documentation.
- Hover over `proxies[].dialer-proxy` should explain outbound chaining.

Record whether the popup panel shows the expected `Popup @ line:column` anchor and whether the first few popup items match the completion strip. This is not yet a pixel-accurate floating layer tied to scroll offset.

## Diagnostic Script

Use existing automated fixtures through the selector and tests:

- `examples/configs/invalid-yaml.yaml` should produce a `yaml.syntax` diagnostic.
- `tests/fixtures/invalid-reference.yaml` should produce a missing-reference diagnostic.
- `tests/fixtures/dialer-cycle.yaml` should produce a `mihomo.dialer_proxy.cycle` diagnostic.

The selector labels for these are `Bad YAML`, `Bad Ref`, and `Cycle`.

## Result Record Template

| Date | Commit | OS | Demo | Command or Script | Result | Notes |
| --- | --- | --- | --- | --- | --- | --- |
|  |  |  | GPUI |  | Pass / Fail / Blocked / Not run |  |
|  |  |  | Floem |  | Pass / Fail / Blocked / Not run |  |

## GPUI Readiness Gate

GPUI is already selected as the primary UI framework. Do not treat the GPUI prototype as a production component shell until these items are recorded:

- GPUI Windows Chinese IME commit and preedit behavior.
- GPUI interactive scroll behavior with a 5000-20000 line YAML file.
- GPUI focus behavior around completion and hover surfaces.
- GPUI keyboard selection and copy ergonomics.
- GPUI release binary size.
- Updated evidence in `docs/gpui-vs-floem-evaluation.md`.

Until then, continue GPUI work behind the UI-independent `mocode-core` and `mocode-api` boundary. Do not move Mihomo semantics into the GPUI adapter.
