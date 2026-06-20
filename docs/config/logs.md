# `logs`

`logs` controls Cefari's runtime log routing.

```ts
export default defineConfig({
  logs: {
    local: {
      enabled: "development",
      retention: "14d",
    },
    exporters: {
      sentry: {
        enabled: "production",
        dsnEnv: "SENTRY_DSN",
        environment: "production",
        release: "my-app@1.2.3",
        level: "info",
        sampleRate: 1,
      },
    },
  },
});
```

## Local Storage

`logs.local.enabled` controls the local SQLite sink:

- `true`: always write local SQLite logs.
- `false`: never write local SQLite logs.
- `"development"`: write local SQLite logs only in Cefari dev mode.
- `"production"`: write local SQLite logs only outside Cefari dev mode.

The default is `true`, so existing projects keep local `cefari logs`
inspection. `retention` is accepted as configuration text but is not enforced
yet.

## Sentry

`logs.exporters.sentry.enabled` uses the same enabled values as local storage.
When enabled, Cefari streams routed app, daemon, worker, and platform logs to
Sentry automatically. It does not require `cefari logs export`.

Use `dsnEnv` for production secrets:

```ts
logs: {
  exporters: {
    sentry: {
      enabled: "production",
      dsnEnv: "SENTRY_DSN",
    },
  },
}
```

Fields:

- `dsnEnv`: environment variable that contains the Sentry DSN.
- `dsn`: literal Sentry DSN. Prefer `dsnEnv` for production.
- `environment`: Sentry environment attribute.
- `release`: Sentry release attribute.
- `level`: minimum routed level: `debug`, `info`, `log`, `warn`, or `error`.
- `sampleRate`: number from `0` to `1`.

Cefari maps its `log` level to Sentry `info`. Structured properties are sent as
Sentry log attributes with Cefari attributes for scope, process ID, and log ID.
