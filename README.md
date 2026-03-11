# tracey (Forked)

Changes in this fork:
- Support requirements definition in Markdown headings
- `tracey generate` to export `tracey web` site as static HTML.
- Support `impl.` / `verify.` requirement prefixes across coverage, dashboard, and tooling
- Do not treat Markdown link text as requirement definitions
- Allow underscores in requirement IDs

## Support requirements definition in Markdown headings

This fork allows requirement definitions directly in Markdown headings.

- A heading that starts with `r[...]` is treated as a requirement definition.
- Example marker: `### r[feature.login]`
- Requirement content starts on the next line and continues until the next heading
  at the same level or a higher level (fewer `#`).
- This means requirement bodies can contain multiple lines, multiple paragraphs,
  lists, code blocks, and deeper subheadings.

In short: headings can now act as requirement markers, while still preserving
normal Markdown structure.

## Definition Samples

```markdown
# Product Spec

## Authentication

### r[auth.login]
The system must allow users to sign in with email and password.

If credentials are invalid, the system must return a user-safe error message.

#### Notes
- Lock the account for 5 minutes after 5 failed attempts.
- Do not reveal whether the email exists.

### r[auth.logout]
The system must terminate the current session immediately.

After logout, protected pages must require re-authentication.

## API

### r[api.token.refresh]
The API must provide a refresh-token endpoint.

The endpoint must rotate refresh tokens and invalidate the old token.

#### Error handling
- Expired refresh token -> `401 Unauthorized`
- Revoked refresh token -> `401 Unauthorized`

## Audit

### r[audit.login.event]
Each successful login must be recorded in the audit log.
Include user ID, timestamp, and source IP.
```

```markdown
# Service Behavior

## r[service.startup]
The service must become ready within 10 seconds after startup.

### Health checks included in this requirement body
- Dependency connectivity check
- Configuration validation check

## r[service.shutdown]
The service must stop gracefully on SIGTERM.

It must finish in-flight requests before exit.
```


## `tracey generate` to export `tracey web` site as static HTML.

Default output directory: `docs/generate`

```
tracey generate
```

To specify output directory, you can use `-o` or `--output`.

```
tracey generate -o docs/my-site
tracey generate --output /tmp/tracey-site
```

## Support `impl.` / `verify.` requirement prefixes across coverage, dashboard, and tooling

This fork adds special handling for requirement IDs that start with `impl.` or
`verify.`.

- `impl.*` requirements only require implementation coverage.
- `verify.*` requirements only require verification coverage.
- The non-required side is treated as `Not Needed` in the dashboard.
- Coverage aggregation, uncovered/untested queries, static export, and tooling
  use the same rule.

Examples:

- `r[impl.gateway.start]` requires implementation, but not verification.
- `r[verify.gateway.start]` requires verification, but not implementation.

## Do not treat Markdown link text as requirement definitions

This fork no longer treats requirement-like text inside Markdown links as a new
requirement definition.

That means links like the following are safe and do not create duplicate
requirement IDs:

```markdown
[r[req.gateway.start]](#rreqgatewaystart)
```

The link text is preserved as normal Markdown content, while the actual
requirement definition must still appear outside the link.

## Allow underscores in requirement IDs

This fork allows underscores in requirement IDs.

- `r[req.gateway_start]` is valid.
- `r[req.server_list.load]` is valid.
- Hyphens are still valid too, so both `-` and `_` can be used inside each ID
  segment.

The first character of each segment must still start with a lowercase letter.
