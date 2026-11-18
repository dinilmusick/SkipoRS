# SkipoRS

Runtime-neutral Rust counterparts of Skipo application framework contracts.

## Implemented slice

Initial Rust environment governance and deterministic project scaffolding, aligned with `dinilmusick/Skipo@25a9921b051e7c6b2601fe33f0f163e432a56d2a` (published package version 1.3.0):

- `plan_env` validates the four canonical app environment names: required `SYSTEM_DATABASE_URL` and `APP_URL`, optional `SYSTEM_DATABASE_URL_2` / `SYSTEM_DATABASE_URL_3`.
- Recognizes one legacy alias, `DEPM_GATEWAY_DOMAIN` → `APP_URL`; rejects unknown keys, duplicate declarations, malformed syntax, missing required keys and simultaneous old/new values without modifying the source.
- `verify_env` rejects remaining legacy aliases; subsequent plans of a repaired file are idempotent.
- `plan_rust_scaffold` produces deterministic minimal Rust application files for a validated slug. `write_rust_scaffold` creates only a new directory, refuses overwrites or unsafe parent paths and cleans up failed writes.
- Native Rust scaffold is deliberately a **shell**, not a functional provider/auth/storage implementation. Integrations must use canonical ULTRST provider contracts rather than inline duplicates.

## Verification

```bash
cargo fmt --all -- --check
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
```

CI executes these on the exact PR head. See `tests/scaffold_contract.rs`.

## Outstanding parity

Not yet implemented: Skipo Next.js/Vite generation, canonical Floks/Luth auth and workspace wiring, graph tabs, dependency migration scripts, transactional multisource repair, source-level environment scanners for YAML/TS/Python, published package compatibility, actual Rust native GUI integration, and end-to-end application deployment. No claim of complete Skipo parity or live application certification is made.
