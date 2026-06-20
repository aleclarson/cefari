# Proposal: Production Log Routing

## Summary

Cefari should treat logging as a routed event stream, not as "write everything
to SQLite, then ask the user to export it."

The local SQLite log database remains valuable for development, local
inspection, and durable diagnostics. It should not be assumed to be the
production source of truth. In production, Cefari should be able to stream log
events automatically to configured exporters such as Sentry, with local storage
enabled, disabled, or reduced according to the app's configuration.

## Goals

- Give app developers one Cefari logging boundary for app, daemon, worker, and
  platform logs.
- Route each captured log event to one or more configured sinks.
- Make local SQLite storage a sink, not the whole logging architecture.
- Support automatic streaming export in production.
- Use Sentry as the first proving exporter.
- Keep source code free of separate Sentry hooks for app, daemon, worker, and
  platform logs.
- Preserve local `cefari logs` inspection when local storage is enabled.

## Non-Goals

- Do not reintroduce `cefari logs export sentry` as the production path.
- Do not implement production streaming export in this proposal.
- Do not require SQLite to be enabled in production.
- Do not remove the existing local SQLite log store in this proposal.
- Do not decide a final config schema here.

## Model

Every log source emits a Cefari log event:

- platform runtime logs
- frontend app logs from `cefari.logs`
- daemon logs
- worker logs

Cefari passes each event through a log router. The router fans out to configured
sinks:

- local SQLite storage
- streaming exporters such as Sentry
- future sinks such as OTLP or file output

SQLite is a local storage sink. It can support inspection, retention, and
backfill, but streaming exporters should not depend on a manual SQLite export
command.

## Local Storage

Local SQLite storage should remain the default development experience because
it makes logs inspectable without a third-party account:

- `cefari logs path`
- `cefari logs page`
- `cefari logs tail`
- `cefari logs expand`

Production should be configurable:

- local storage enabled for support-heavy apps
- local storage disabled for privacy-sensitive or high-volume apps
- local storage retained only as a bounded fallback buffer

The current implementation always persists to SQLite. A later implementation
must add the actual local-storage policy before docs claim production SQLite
can be disabled.

## Streaming Exporters

A streaming exporter receives log events as they are captured or shortly after
they are queued. It should be automatic once configured. It should not require
a human to run a CLI command after deployment.

The Sentry exporter should map Cefari log events to Sentry logs with:

- Cefari `log` level mapped to Sentry `info`
- structured properties preserved as Sentry attributes
- stable Cefari attributes for source scope, log row ID when one exists, and
  process ID
- environment and release from config or environment variables
- Sentry filtering hooks such as `beforeSendLog` where appropriate

The existing Sentry-shaped mapping and adapter can inform this sink, but the
production path should be driven by the router.

## Buffering And Failure

Streaming export needs explicit backpressure and failure policy:

- batch size
- flush interval
- retry strategy
- maximum in-memory queue
- optional durable fallback
- shutdown flush timeout
- behavior when the exporter is unavailable

SQLite can be one possible durable buffer, but it should not be the only design
option. If SQLite is disabled in production, Cefari still needs a clear answer
for temporary exporter outages and crash-time diagnostics.

## Configuration Direction

A future config might separate storage from exporters:

```ts
export default defineConfig({
  logs: {
    local: {
      enabled: "development",
      retention: "7d",
    },
    exporters: {
      sentry: {
        enabled: "production",
        dsnEnv: "SENTRY_DSN",
        environment: "production",
        release: "1.2.3",
        level: "info",
      },
    },
  },
});
```

This is not a committed schema. It is a shape for the next implementation
sprint to review.

## Runtime Ownership

The exporter should be owned by Cefari-managed runtime infrastructure, not app
frontend code.

Open design questions:

- Should the router live in the platform runtime, daemon, or a dedicated helper
  process?
- How are worker and daemon stderr events routed before local storage?
- How does packaged startup initialize exporters without delaying app launch?
- How does shutdown flush interact with OS quit behavior?
- How are secrets such as Sentry DSNs passed to packaged apps?

## Later Implementation Work

Before production streaming is supported, Cefari needs:

- a config contract for log storage and exporters
- a router abstraction shared by app, daemon, worker, and platform logs
- a Sentry streaming sink wired into that router
- tests for routing, filtering, batching, retry, and shutdown flush
- a local storage policy that can differ between development and production
- docs that describe implemented production behavior without asking users to
  run a manual export command

## Review Questions

- Should Cefari's production logging model be a router with optional SQLite and
  streaming exporters?
- Should Sentry be the first official streaming sink?
- Should local SQLite default to development-only, production-enabled, or
  configurable per app?
- What durability should Cefari guarantee when streaming export is configured
  but temporarily unavailable?
