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
Cargo workspace is also covered by the repository Cargo workspace `exclude`
contract, so root `--workspace` checks do not compile or run it.

Run every outcome explicitly from the repository root:

```sh
cargo run --quiet --locked -p workspace-validator -- validate \
  --config tools/workspace-validator/fixtures/outcomes/.validation/config.json
```

Run one outcome group:

```sh
cargo run --quiet --locked -p workspace-validator -- validate outcomes.pass \
  --config tools/workspace-validator/fixtures/outcomes/.validation/config.json

cargo run --quiet --locked -p workspace-validator -- validate outcomes.fail \
  --config tools/workspace-validator/fixtures/outcomes/.validation/config.json

cargo run --quiet --locked -p workspace-validator -- validate outcomes.blocked \
  --config tools/workspace-validator/fixtures/outcomes/.validation/config.json
```

All regular human-presentation options remain available. This command is useful
for inspecting every status with both visual accessibility axes enabled:

```sh
cargo run --quiet --locked -p workspace-validator -- validate \
  --color=high-contrast --presentation=low-vision \
  --config tools/workspace-validator/fixtures/outcomes/.validation/config.json
```

The expected process codes are `0` for `outcomes.pass`, `1` for
`outcomes.fail` and `outcomes.all`, and `2` for `outcomes.blocked`. Non-zero
codes in those demonstrations are the expected fixture behavior.

Run the fixture assertions themselves with the ignored integration-test target:

```sh
cargo test -p workspace-validator --test outcome_fixtures -- --ignored
```

These tests use Rust's explicit `#[ignore]` boundary, so the regular workspace
test gate compiles the harness but never executes the intentional outcomes.

Alternatively, enter this directory and omit `--config`; upward discovery
selects the local fixture configuration:

```sh
cd tools/workspace-validator/fixtures/outcomes
cargo run --quiet --locked --manifest-path ../../Cargo.toml -- validate
```

The local `.gitignore` excludes only nested Cargo build output. Source files,
the fixture configuration, and expected outcomes remain versioned and
reviewable.
