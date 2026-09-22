# Coding Agent Instructions

These instructions apply to the entire repository.

## Product State

- Treat the current code, schemas, and documented contracts as the source of
  truth.
- Implement only the requested current behavior. Do not add compatibility
  layers, deprecated aliases, migrations, or fallback behavior unless the task
  explicitly requires them.
- Keep the validator project-agnostic. Repository-specific validation belongs
  in consumer configuration, not in the validator runtime.

## Compatibility And Contracts

- Preserve the minimum supported Rust version declared by `rust-version` in
  `Cargo.toml`.
- Treat configuration and report schema versions as exact public contracts.
  Intentional contract changes must update Rust types, checked-in JSON Schemas,
  tests, and documentation together.
- Keep process execution shell-free and preserve the platform-specific process
  tree termination guarantees.
- Document public Rust APIs with Rustdoc. Write code comments and documentation
  in English, and use regular comments to explain non-obvious implementation
  reasons rather than restating the code.

## Validation

- Run `cargo fmt --all -- --check`.
- Run `cargo clippy --all-targets --locked -- -D warnings`.
- Run `cargo test --all-targets --locked` and `cargo test --doc --locked`.
- Run the ignored outcome fixtures when behavior related to execution states or
  reporting changes:
  `cargo test --test outcome_fixtures -- --ignored`.
- Validate the MSRV for dependency or language-feature changes.
- Do not install tools or dependencies without user approval.

## Repository Hygiene

- Keep the root `Cargo.lock` committed because this repository ships a CLI.
- Do not commit Cargo build output or generated package archives.
- Preserve unrelated user changes and avoid destructive Git operations.
