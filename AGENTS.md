# Agent Guidance

## Project Stage

Cefari is pre-alpha. Breaking changes are expected and perfectly acceptable.
Backwards compatibility is not a project goal at this stage.

Do not preserve legacy behavior for its own sake. Legacy code paths, migration
shims, compatibility layers, and deprecated APIs are unacceptable unless they
are explicitly requested for a specific short-lived purpose.

Prefer continuous refinement: simplify APIs, rename concepts, remove obsolete
paths, and reshape implementation details when doing so improves the product or
the codebase. Keep changes scoped to the task, but do not let compatibility
concerns block a cleaner design during pre-alpha development.
