# Editor Features

## Auto Indentation

Indentation belongs in `mocode-yaml`, not UI code. The first version should use YAML context from tree-sitter plus current line text.

Rules:

- Pressing Enter after `key:` indents one level.
- Pressing Enter after `- item:` aligns nested fields under the item.
- New list items align with previous `-`.
- Pasting multiple lines preserves relative indentation and shifts the block to the current context.
- Tabs should be converted to spaces.
- Default indent width is 2 spaces.

## Syntax Error Marking

`tree-sitter-yaml` exposes error nodes and missing nodes. `mocode-yaml` should convert them into source ranges with messages.

UI adapters render diagnostics as:

- gutter marker
- underline
- hover message
- diagnostics list entry

## Formatting

Formatter priorities:

1. Preserve comments.
2. Preserve key order.
3. Avoid quote-style churn.
4. Avoid block scalar changes.
5. Prefer range formatting and snippet formatting before whole-document formatting.

Initial formatter actions:

- normalize inserted snippet indentation
- normalize pasted block indentation
- optionally trim trailing whitespace
- optionally ensure final newline

Whole-document formatting is deferred until a reliable lossless approach is proven.

## Code Completion

Completion flow:

```text
position -> YAML path/scope -> schema context -> semantic index -> completion list
```

Completion categories:

- fields
- enum values
- proxy/group/provider names
- built-ins
- rule templates
- snippets

Completions must include replacement ranges so UI adapters do not guess token boundaries.

## Documentation Hints

Documentation hint sources:

- schema field docs
- enum value docs
- rule token docs
- semantic reference target summary
- risk hints for sensitive fields

Hover payload is markdown-compatible but must not contain UI toolkit types.

## Hover

Hover resolution order:

1. If cursor is over a field key, show field docs.
2. If cursor is over an enum value, show enum docs.
3. If cursor is over a reference value, show target summary.
4. If cursor is inside a rule string, parse the active token and show rule docs.
5. Otherwise return `None`.

## Diagnostics

Diagnostic sources:

- YAML syntax layer
- schema shape layer
- semantic reference layer
- `dialer-proxy` graph layer
- risk-hint layer

Severity mapping:

- `Error`: invalid YAML or definitely broken references.
- `Warning`: risky but runnable configuration.
- `Info`: helpful context and platform caveats.
- `Hint`: style or maintainability suggestions.

## References and Navigation

`references_at(position)` should return:

- definition range for a referenced proxy/group/provider
- all use sites for a definition
- group membership references
- rules target references
- `dialer-proxy` graph references

For providers, remote proxy contents are not navigable unless inline or supplied by a host application.

## Large File Performance

Targets:

- 5000-20000 line YAML loads without UI stalls.
- Text edits avoid whole-string copies.
- YAML parse is incremental after phase 1.
- Semantic index rebuild is whole-file in phase 1 but measured; later it should use changed ranges.
- UI adapters virtualize visible lines and diagnostics.

Design constraints:

- `mocode-text` owns the rope and line/char mapping.
- UI adapters should request visible line slices.
- Syntax highlighting should be range-based.
- Diagnostics should be stored by text range and updated after edits.
- Completion and hover should be computed on demand.

