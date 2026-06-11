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
| `cargo test -p cefari-core` | Core crate tests | Passed |
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

- `crates/cefari-core/src/config.rs`: config serialization, defaults, unknown-field rejection, and save/load error mapping reviewed with no findings.
- `crates/cefari-core/src/ipc.rs` and `crates/cefari-core/bindings/ipc.ts`: IPC command/result/event contracts and generated TypeScript binding currency reviewed with no findings.
- `crates/cefari-core/src/logging.rs`: runtime log config and rotated log pruning reviewed with no findings.
- `crates/cefari-core/src/paths.rs`: platform project directory resolution reviewed with no findings.
- `crates/cefari-core/src/resources.rs`: packaged resource path validation and existence checks reviewed with no findings.
- `crates/cefari-core/src/services.rs`: service-manager wrappers, default levels, Windows `sc.exe` status fallback, and tests reviewed with no findings.
- `crates/cefari-core/src/updates.rs`: updater configuration preparation, unconfigured-state handling, and update result modeling reviewed with no findings.

## Validation Summary

- `cargo test -p cefari-core`: passed. This ran 31 unit tests and doc tests successfully; the `native_service_lifecycle_smoke` integration test remains ignored by design because it installs and starts a native OS service.

## Skipped Or Limited Checks

- `crates/cefari-core/tests/service_lifecycle.rs::native_service_lifecycle_smoke` was not run because it is explicitly ignored and requires a disposable host for native service installation/start/stop verification.
