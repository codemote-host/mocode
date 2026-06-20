# UI Framework Decision

Date: 2026-06-20

## Decision

mocode will continue development with GPUI as the application shell and component UI framework. The application package is `mocode`, and the Windows build target is `mocode.exe`.

## Why GPUI

GPUI is the best current fit for the editor shell mocode is building:

- Explicit actions, key bindings, focus handles, and key contexts map cleanly to editor input.
- `uniform_list` is a direct fit for virtualized editor rows.
- The current app adapter keeps Mihomo semantics outside the UI layer.
- The upstream GPUI README documents Windows support through Win32 windowing and DirectWrite text.

This is a product direction decision, not a claim that every validation item is complete. Manual Windows IME, scroll, popup, focus, and packaging checks still matter, but they should be performed against the `mocode` app path.

## Architecture Boundary

Choosing GPUI does not make mocode's core GPUI-specific.

These crates must stay UI independent:

- `mocode-text`
- `mocode-yaml`
- `mocode-mihomo-schema`
- `mocode-mihomo-lint`
- `mocode-core`
- `mocode-api`

Mihomo schema, lint, YAML path, completion, hover, diagnostics, formatting, reference validation, and proxy-chain semantics must remain outside `crates/mocode`.

## Next Direction

The next implementation work should:

1. Harden the `mocode` app target and keep it building as `mocode.exe` on Windows.
2. Improve viewport rendering, IME handling, popup focus, save/open UX, and syntax highlighting.
3. Implement missing shared-core features before exposing them in the app shell.
4. Run the app validation checklist and record measured results.
5. Keep the UI-independent component boundary intact while improving the reusable API.
