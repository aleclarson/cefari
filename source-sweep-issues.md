# Source Sweep Issues

Full source sweep report for sprint `full-source-sweep`.

## Baseline

- Base: detached HEAD `63d288aa3d43a1e1319d121573392cd2daf2f0b0` (`63d288a`)
- Sprint branch: `sprint/full-source-sweep/review`
- Initial repo state: clean working tree, 151 tracked files
- Sweep scope: Rust crates, Deno TypeScript package, Vite React template, scripts, documentation, mirrored Cefari skill files, GitHub actions/workflows, generated TypeScript IPC bindings
- Exclusions: third-party dependencies, build outputs, VCS internals, and generated cache/output directories such as `target`, `node_modules`, `dist`, and `build`

## Finding Schema

Each finding uses:

- `ID`: stable source-sweep identifier
- `Severity`: `Critical`, `High`, `Medium`, `Low`, or `Info`
- `Status`: `Confirmed`, `Risk`, `Test Gap`, or `Documentation Drift`
- `Confidence`: `High`, `Medium`, or `Low`
- `Area`: affected source area
- `Files`: file references
- `Evidence`: source or command evidence
- `Impact`: potential user, developer, release, security, or maintenance impact
- `Suggested next step`: focused follow-up action
- `Verification notes`: commands, reproduction, or remaining manual verification

## Planned Validation Commands

| Command | Purpose | Result |
| --- | --- | --- |
| `cargo fmt --all --check` | Rust formatting | Pending |
| `cargo clippy --workspace --all-targets -- -D warnings` | Rust linting | Pending |
| `cargo test --workspace` | Rust test suite | Pending |
| `cargo test -p cefari-core` | Core crate tests | Pending |
| `cargo test -p cefari-desktop` | Desktop crate tests | Pending |
| `cargo test -p cefari-cli` | CLI crate tests | Pending |
| `deno task --cwd packages/cefari-app check` | TypeScript package type check | Pending |
| `deno task --cwd packages/cefari-app test` | TypeScript package tests | Pending |
| `deno task --cwd templates/vite-react-basic/frontend check` | Template frontend type check | Pending |
| `deno task --cwd templates/vite-react-basic/frontend build` | Template frontend build | Pending |
| `actionlint .github/workflows/*.yml templates/vite-react-basic/.github/workflows/*.yml docs/examples/cefari-release-workflow.yml` | Workflow syntax | Pending |
| `shellcheck scripts/extract-native-package-payload.sh .github/actions/cefari-release/release.sh` | Shell script diagnostics | Pending |
| `ruby -c scripts/sync-cefari-skill-docs.rb` | Ruby syntax check | Pending |
| `ruby -c scripts/verify-native-package-payload.rb` | Ruby syntax check | Pending |

## Findings

No findings recorded yet.

## Reviewed Areas With No Findings

None yet.

## Validation Summary

Pending.

## Skipped Or Limited Checks

Pending.
