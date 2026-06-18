# UI Framework Decision

Date: 2026-06-18

## Decision

mocode will continue development with GPUI as the primary UI framework.

Floem is frozen as a reference prototype. It should remain buildable while it is useful for comparison, but new mocode product work no longer needs GPUI/Floem parity.

## Why GPUI

GPUI is the better fit for the editor shell mocode is building:

- Explicit actions, key bindings, focus handles, and key contexts map cleanly to editor input.
- `uniform_list` is a direct fit for virtualized editor rows.
- The current GPUI adapter keeps Mihomo semantics outside the UI layer.
- GPUI's editor-oriented API shape is closer to the long-term component shell than Floem's reactive demo shape.

This is a product direction decision, not a claim that every validation item is already complete. Manual Windows IME, scroll, popup, focus, and packaging checks still matter, but they should now be performed against the GPUI path first.

## Floem Status

Floem remains in the repository as:

- a historical comparison prototype
- a reference for adapter-boundary checks
- a fallback source of design ideas

Do not continue implementing new mocode features in Floem unless a later explicit decision reopens framework comparison.

## Architecture Boundary

Choosing GPUI does not make mocode's core GPUI-specific.

These crates must stay UI independent:

- `mocode-text`
- `mocode-yaml`
- `mocode-mihomo-schema`
- `mocode-mihomo-lint`
- `mocode-core`
- `mocode-api`

Mihomo schema, lint, YAML path, completion, hover, diagnostics, formatting, reference validation, and proxy-chain semantics must remain outside `mocode-gpui-demo` and any future GPUI component crate.

## Next Direction

The next implementation work should:

1. Rename or evolve `mocode-gpui-demo` toward a real GPUI component shell.
2. Keep Floem compiling, but stop parity work.
3. Implement missing shared-core features such as `proxy_chain_preview_at`.
4. Run the prototype validation checklist on the GPUI path and record measured results.
5. Update roadmap and specs as GPUI componentization replaces framework comparison.
