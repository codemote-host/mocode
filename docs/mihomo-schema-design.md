# Mihomo Schema Design

## Schema Maintenance

mocode should maintain a versioned Rust schema catalog. Each field entry should include:

- path pattern, such as `dns.enhanced-mode` or `proxy-groups[].proxies[]`
- value kind
- enum values when applicable
- short summary
- detailed markdown docs
- examples or snippets
- source URL
- support status: stable, deprecated, experimental, unknown
- schema version and last review date

The catalog should start with high-value fields instead of complete coverage:

1. root fields
2. `dns`
3. `tun`
4. `proxies` common fields
5. `proxy-groups`
6. `rules`
7. providers
8. `sniffer`
9. `listeners`
10. external controller

## Field Documentation

Docs should be stored close to schema metadata, not in UI code. The UI receives `Hover` payloads from `mocode-core`.

Recommended source shape:

```rust
FieldDoc {
    path: "dns.respect-rules",
    kind: ValueKind::Bool,
    summary: "Route DNS connections according to rules.",
    details: "...",
    source_url: "https://wiki.metacubex.one/en/config/dns/",
    risk: Some(RiskLevel::Warning),
}
```

## Field Completion

Completion generation is context-based:

- Root map context completes root fields.
- Nested map context completes fields for that section.
- Sequence item map context completes item fields.
- Scalar value context completes enums, names, or snippets.

Schema completions should include:

- label
- insert text
- kind
- docs summary
- replacement range
- sort priority

## Enum Completion

High-value initial enums:

- `mode`: `rule`, `global`, `direct`
- `log-level`: `silent`, `error`, `warning`, `info`, `debug`
- `find-process-mode`: `always`, `strict`, `off`
- `dns.enhanced-mode`: `normal`, `fake-ip`, `redir-host` where valid
- `proxy-groups[].type`: `select`, `url-test`, `fallback`, `load-balance`
- `rule-providers.<name>.type`: `http`, `file`, `inline`
- `rule-providers.<name>.behavior`: `classical`, `domain`, `ipcidr`
- listener `type`: `http`, `socks`, `mixed`, `redir`, `tproxy`, `tun`, `tunnel`
- `tun.stack`: `system`, `gvisor`, `mixed`

## Reference Completion

Reference completions are driven by the semantic index:

- `proxy-groups[].proxies[]`: proxy names, group names, built-ins
- `proxy-groups[].use[]`: proxy provider names
- `rules[]` target token: proxy names, group names, built-ins
- `RULE-SET` provider token: rule provider names
- `listeners[].proxy`: proxy names, group names, built-ins
- `listeners[].rule`: sub-rule names
- `proxies[].dialer-proxy`: proxy names and group names
- `proxy-providers.<name>.override.dialer-proxy`: proxy names and group names

The current proxy should be deprioritized or excluded in `dialer-proxy` completion because it creates an immediate cycle.

## Rule Template Completion

Rules are scalar strings, so mocode needs a small rule grammar and template engine.

Initial templates:

- `DOMAIN-SUFFIX,example.com,<target>`
- `DOMAIN,example.com,<target>`
- `DOMAIN-KEYWORD,keyword,<target>`
- `IP-CIDR,1.1.1.0/24,<target>,no-resolve`
- `GEOIP,CN,<target>`
- `GEOSITE,category,<target>`
- `RULE-SET,<provider>,<target>`
- `MATCH,<target>`

The rule editor should parse comma-separated tokens while respecting logical rules that contain parentheses.

## TUN and DNS Risk Hints

Risk hints are diagnostics with severity `Info` or `Warning` unless a configuration is definitely invalid.

Examples:

- `tun.strict-route: true`: warn that it may prevent local devices from reaching the machine.
- `tun.auto-route: true` with platform-specific include/exclude fields: show platform caveats.
- `tun.auto-redirect: true` without `auto-route`: warn that `auto-route` is required.
- `dns.respect-rules: true` without `proxy-server-nameserver`: warn about resolver routing requirements.
- `dns.prefer-h3: true` with `respect-rules: true`: warn because official docs strongly discourage this combination.
- `external-controller: 0.0.0.0:...` with empty `secret`: warn about exposed control API.
- `external-controller-unix` or `external-controller-pipe`: warn that secret verification is not used by those access paths according to docs.

## dialer-proxy Chain Checks

The lint layer builds a directed graph:

```text
proxy_or_provider_generated_proxy -> dialer_proxy_target
```

Checks:

- missing target
- target exists but is not an outbound or group
- immediate self-reference
- cycle across multiple proxies
- possible cycle through a group when the group contains the source proxy

Chain preview should distinguish:

- definite chain: all targets are concrete proxies
- group branch: target is a group, so preview shows possible next hops
- provider branch: target group uses remote providers, so preview marks unknown provider members

Display shape:

```text
local -> entry proxy -> dialer target/group -> exit proxy -> target
```

