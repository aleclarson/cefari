# Template Authoring

Use this reference when changing files under `templates/`.

## Template Rules

- Every checked-in template should be runnable with the local Cefari build.
- Prefer commands that work from the repository root.
- Keep template config aligned with `cefari init` output unless the template intentionally demonstrates a different setup.
- For Deno workspaces, keep root, frontend, and daemon tasks coherent.

## Frontend

- Frontend dev servers should be declared through `cefari.config.ts` using
  `frontend.devCommand` and `frontend.devPort`.
- Frontend build output should match `frontend.dist`.

## Release Workflows

- Template release workflows should call the shared Cefari release action.
- Do not duplicate build/package/sign/update logic in template workflow YAML.
- Document triggers, secrets, variables, and artifacts in the template README.

## Verification

- Run template commands from the repository root.
- Validate workflow YAML with `actionlint` when available.
- Confirm template README commands still match actual paths.
