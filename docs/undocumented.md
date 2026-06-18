# Undocumented Behavior

This file tracks user-facing behavior that exists but has not yet been placed in
the supported feature inventory or another documentation page.

## Pending Placement

- `cefari.workers.spawn()` does not yet expose a first-class promise for the
  worker's returned output value. The runtime currently emits that returned
  value as a final `worker.message` event.
