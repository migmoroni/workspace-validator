# Outcome Fixtures

This isolated workspace demonstrates every check status rendered by
`workspace-validator`:

- `PASS`: a Rust test succeeds;
- `FAIL`: a Rust test contains an intentional failing assertion;
- `SKIPPED`: a check depends on the intentional failure;
- `BLOCKED`: a check requires an intentionally unavailable executable.

The fixture configuration lives below this directory in
`.validation/config.json`. Configuration discovery never traverses child
directories, so validation started from the repository root continues to use
the repository configuration and does not execute these fixtures. The nested
Cargo workspace is independent from the root package, so regular root checks do
not execute its intentional outcomes.

Run every outcome explicitly from the repository root:

```sh
cargo run --quiet --locked -- validate \
  --config fixtures/outcomes/.validation/config.json
```

Run one outcome group:

```sh
cargo run --quiet --locked -- validate outcomes.pass \
  --config fixtures/outcomes/.validation/config.json

cargo run --quiet --locked -- validate outcomes.fail \
  --config fixtures/outcomes/.validation/config.json

cargo run --quiet --locked -- validate outcomes.blocked \
  --config fixtures/outcomes/.validation/config.json
```

All regular human-presentation options remain available. This command is useful
for inspecting every status with both visual accessibility axes enabled:

```sh
cargo run --quiet --locked -- validate \
  --color=high-contrast --presentation=low-vision \
  --config fixtures/outcomes/.validation/config.json
```

The expected process codes are `0` for `outcomes.pass`, `1` for
`outcomes.fail` and `outcomes.all`, and `2` for `outcomes.blocked`. Non-zero
codes in those demonstrations are the expected fixture behavior.

Run the fixture assertions themselves with the ignored integration-test target:

```sh
cargo test --test outcome_fixtures -- --ignored
```

These tests use Rust's explicit `#[ignore]` boundary, so the regular workspace
test gate compiles the harness but never executes the intentional outcomes.

Alternatively, enter this directory and omit `--config`; upward discovery
selects the local fixture configuration:

```sh
cd fixtures/outcomes
cargo run --quiet --locked --manifest-path ../../Cargo.toml -- validate
```

The local `.gitignore` excludes only nested Cargo build output. Source files,
the fixture configuration, and expected outcomes remain versioned and
reviewable.
