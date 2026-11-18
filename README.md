# SkipoRS

Portable Rust counterpart of the ULTRST Skipo application composition framework.

## Implemented: environment-governance v2 slice

- Pinned source: `dinilmusick/Skipo@25a9921b051e7c6b2601fe33f0f163e432a56d2a`
- Original canonical contract: `architecture/environment/contract.json` (schema 2, contract 2.0.0)
- A byte-identical snapshot of that contract is in `fixtures/skipo_environment_contract.json`
- Exports: `skipo_rs::environment::{classify,plan_env_file,canonical_env_example}`
- Deterministic fail-closed `.env` assignment parsing, canonical four-name classification, exact alias replacement while preserving original values/line endings, unknown-variable and duplicate detection, alias value conflict detection, required-name checks, idempotent plans, and secret-free diagnostic metadata.
- No environment file is mutated by this API. The caller must review `EnvironmentPlan.proposed` and perform its own safe commit if authorized.
- Tests cross-check all recognized runtime/tooling/alias names directly against the pinned upstream JSON and exercise negative/failure paths.

## Verification

```sh
cargo fmt --all -- --check
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
```

Public CI runs the complete matrix on Linux, macOS and Windows.

## Explicit remaining parity gaps

This is a **partial implementation**, not full Skipo parity. The current Rust crate does **not** implement the current Next.js/Vite project generator, framework graph composer, capability ownership registry, application identity, dependency resolution/migration, transactional repair journal, full workspace/source-reference scanner, browser/runtime conformance engine or downstream integration. Unknown configuration is rejected rather than guessed. Continue against the pinned canonical upstream interfaces and add separate cross-language fixtures as features are implemented. ATAX schedules no agents; SkipoRS does not spawn any agents.
