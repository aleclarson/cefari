# Logs Commands

Use `cefari logs` commands to inspect the local Cefari SQLite log database.
These commands show local storage only. In projects that disable
`logs.local.enabled` for the active mode, production logs may still stream to
configured exporters such as Sentry, but there may be no local rows to inspect.

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

## `cefari logs expand`

Print a collapsed large value:

```bash
cefari logs expand str_01K00000000000000000000000
```

Use this for values shown by formatted log rows as `{str_...}`, `{arr_...}`, or
`{obj_...}`.
