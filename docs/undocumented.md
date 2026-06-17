# Undocumented Behavior

This file tracks user-facing behavior that exists but has not yet been placed in
the supported feature inventory or another documentation page.

## Pending Placement

- Multi-window native runtime support is planned but not yet supported. The
  current contract proposal lives in
  [`docs/proposals/multi-window-support.md`](proposals/multi-window-support.md).
  The IPC schema and TypeScript facade reserve `windowCurrent`, `windowList`,
  `windowCreate`, target-aware window controls, and window-scoped lifecycle
  events, but secondary native window creation is still pending.
