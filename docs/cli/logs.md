# Logs Commands

Use `cefari logs` commands to inspect the local Cefari SQLite log database.

## `cefari logs path`

Print the active log database path:

```bash
cefari logs path
```

## `cefari logs page`

Print a page of log rows:

```bash
cefari logs page
cefari logs page --json --level debug
cefari logs page --scope daemon --grep startup
```

Options:

- `--json`: print JSON rows.
- `--level LEVEL`: minimum level. Defaults to `info`.
- `--scope SCOPE`: filter to one scope, such as `app`, `daemon`, `cefari`, or
  `worker:thumbnailer`.
- `--grep TEXT`: search messages and serialized properties.
- `--regex PATTERN`: search messages and properties with a JavaScript regular
  expression.
- `--property KEY=VALUE`: filter by a structured property. Repeat for multiple
  properties.
- `--since VALUE`: filter after an ISO timestamp or duration such as `10m`,
  `2h`, or `1d`.
- `--after-id ID`: return rows after a cursor.
- `--before-id ID`: return rows before a cursor.
- `--limit COUNT`: limit rows.
- `--debug-scope PREFIX`: filter debug rows by debug scope prefix.

## `cefari logs tail`

Follow new log rows:

```bash
cefari logs tail
cefari logs tail --scope app --level debug
```

`tail` accepts the same filters as `page`. Use `--once` for one polling pass in
scripts and tests.

## `cefari logs export sentry`

Export rows from the local SQLite log database to Sentry:

```bash
cefari logs export sentry --dsn "$SENTRY_DSN" --environment production --release "my-app@1.2.3" --once
```

Preview the exact Sentry-shaped records without sending or requiring a DSN:

```bash
cefari logs export sentry --dry-run
cefari logs export sentry --dry-run --level warn --scope worker:thumbnailer
```

Options:

- `--dsn DSN`: Sentry DSN. Defaults to `SENTRY_DSN`.
- `--environment NAME`: Sentry environment. Defaults to `SENTRY_ENVIRONMENT`.
- `--release VERSION`: Sentry release. Defaults to `SENTRY_RELEASE`.
- `--cursor NAME`: export cursor name. Defaults to `sentry`.
- `--batch-size COUNT`: maximum rows to export per batch.
- `--level LEVEL`: minimum level to export.
- `--scope SCOPE`: export only one scope.
- `--sample-rate RATE`: Sentry SDK sample rate.
- `--once`: export one polling pass and exit.
- `--poll-ms MS`: polling interval for long-running export.
- `--dry-run`: print mapped records without importing Sentry, sending network
  requests, or advancing the cursor.

Cursor behavior:

- Cefari reads from the named cursor and sends rows after the last acknowledged
  log row ID.
- The cursor advances only after Sentry export and flush both succeed.
- If export fails, rerunning the command retries the same rows.
- Filtered exports use the same named cursor semantics. Use a separate
  `--cursor` value for each independent filtered export stream.

Human-run Sentry check:

```bash
export SENTRY_DSN="https://..."
export SENTRY_ENVIRONMENT="production"
export SENTRY_RELEASE="my-app@1.2.3"
cefari logs export sentry --once --level debug
```

After the command exits, confirm in Sentry Logs that records include
`cefari.scope`, `cefari.log_id`, `cefari.pid`, and any structured properties
written by the app, daemon, worker, or Cefari runtime.

## `cefari logs expand`

Print a collapsed large value:

```bash
cefari logs expand str_01K00000000000000000000000
```

Use this for values shown by formatted log rows as `{str_...}`, `{arr_...}`, or
`{obj_...}`.
