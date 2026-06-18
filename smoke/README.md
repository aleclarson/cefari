# Cefari Smoke

This project is a local smoke fixture for `cefari dev`. It uses the built-in
Vite frontend server and a real Deno daemon stream.

Run the smoke from the repository root:

```bash
smoke/run.sh
```

The runner starts `cefari dev ./smoke` with `CEFARI_SMOKE_BACKGROUND=1`, an
isolated `HOME`, and a watchdog. The desktop window is created hidden and
unfocused, so the smoke run should not interrupt the active app.

The smoke fixture does not call external services and does not mock Cefari
behavior. It verifies a byte round trip through `cefari.daemon.connect()`; the
daemon keeps stdout reserved for protocol bytes and writes logs to stderr.
Background smoke mode only adds Chromium command-line switches that keep macOS
credential and permission dialogs out of the run.
