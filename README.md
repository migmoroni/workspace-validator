# Workspace Validator

`workspace-validator` runs declarative validation graphs for heterogeneous
workspaces. It validates the complete configuration before starting a process,
checks tool versions, executes argument vectors without an implicit shell,
applies timeouts, preserves deterministic ordering, detects Git-visible
mutations, and emits accessible human output or a versioned JSON report.

The same pipeline is available as a CLI and as a Rust library.

## Installation

Install the published binary with Cargo:

```sh
cargo install workspace-validator --locked
```

Rust 1.87 or newer is required.

## Quick Start

Create `.validation/config.json` at the workspace boundary:

```json
{
  "$schema": "./config.schema.json",
  "schemaVersion": 6,
  "workspaceRoot": "..",
  "defaultGroup": "all",
  "outputLimitBytes": 1048576,
  "tools": [
    {
      "id": "cargo",
      "program": "cargo",
      "requiresTools": [],
      "versionArgs": ["--version"],
      "versionParser": "firstSemver",
      "versionRequirement": ">=1.87"
    }
  ],
  "checks": [
    {
      "id": "rust.cargo.check",
      "label": "Cargo check",
      "description": "Checks selected Rust targets.",
      "toolId": "cargo",
      "args": ["check", "{target}"],
      "requiresTools": [],
      "timeoutSeconds": 600
    }
  ],
  "suites": [
    {
      "id": "rust",
      "label": "Rust",
      "description": "Checks the Rust workspace.",
      "workingDirectory": ".",
      "checks": [
        {
          "checkId": "rust.cargo.check",
          "parameters": {
            "target": ["--workspace", "--all-targets"]
          },
          "dependsOn": []
        }
      ]
    }
  ],
  "groups": [
    {
      "id": "all",
      "label": "All validations",
      "description": "Runs the complete workspace gate.",
      "members": [
        { "kind": "suite", "id": "rust" }
      ]
    }
  ]
}
```

Generate the local schemas and run the default group:

```sh
workspace-validator schema config > .validation/config.schema.json
workspace-validator schema report > .validation/report.schema.json
workspace-validator config validate
workspace-validator validate
```

Use `--format=json` when a machine-readable report is required:

```sh
workspace-validator validate --format=json
```

## Agent Skill

The source distribution includes one versioned, agent-neutral skill at
[`skills/workspace-validator`](skills/workspace-validator). Its `SKILL.md` is a
small router that checks CLI compatibility and loads only the requested
workflow reference: execution, triage, configuration, or coverage auditing.

Copy the complete directory from the source release that matches the installed
CLI into the skill directory recognized by the consumer workspace. For an
agent that discovers workspace skills under `.agents/skills`, run this from a
checkout of `workspace-validator`:

```sh
workspace_root=/path/to/workspace
mkdir -p "$workspace_root/.agents/skills"
cp -R skills/workspace-validator "$workspace_root/.agents/skills/"
```

The source path is not tied to a specific model or agent. Use a different
destination when the consumer uses another skill discovery convention.
Installing the Cargo binary does not register the skill automatically.

Copy the whole directory so `manifest.json` and the `references` remain on the
same version. Compare `workspace-validator --version` with the source crate
version recorded at the top of the copied skill. The router warns about a stale
but compatible copy and stops workflows outside its declared compatibility
range. Replace the copied directory from the matching release to update it;
keep project-specific commands and approval rules in a separate local skill or
agent policy.

## Configuration Model

The version 6 configuration has four ownership layers:

```text
tool -> check -> suite -> group
```

- A **tool** declares executable discovery and version validation.
- A **check** declares one reusable program and argument template.
- A **suite** binds parameters, dependencies, and one working directory to
  check invocations.
- A **group** composes suites and other groups into an ordered DAG.

An argument formed entirely as `{name}` is a structured placeholder. A suite
parameter string emits one argument, an array emits multiple arguments, and an
unbound placeholder emits no argument. Values are never split, recursively
expanded, interpolated inside another string, or interpreted by a shell.

Only suites own `workingDirectory`. Paths are resolved below `workspaceRoot`;
absolute paths, parent traversal, missing directories, files, and symlink
escapes are rejected before preflight. Group references form a DAG, and shared
groups or suites execute once per validation run.

The normative ownership and extension rules are documented in
[`CONFIG_DESIGN.md`](CONFIG_DESIGN.md).

## Commands

```text
workspace-validator config validate [--config <path>]
workspace-validator validate [group-or-suite] [--config <path>] [--format=human|json]
workspace-validator check <check-id> [--config <path>] [--format=human|json]
workspace-validator list [--tree] [--config <path>]
workspace-validator explain group <group-id> [--config <path>]
workspace-validator explain suite <suite-id> [--config <path>]
workspace-validator explain check <check-id> [--config <path>]
workspace-validator schema config
workspace-validator schema report
```

Without `--config`, the CLI discovers the nearest
`.validation/config.json` by walking from the current directory upward.
`validate` without a target selects `defaultGroup`. `config validate`, `list`,
and `explain` inspect configuration without running preflight or checks.

Exit codes are stable CLI behavior:

| Code | Meaning |
| ---: | --- |
| `0` | Every counted result passed |
| `1` | At least one check or repository gate failed |
| `2` | At least one result was blocked or skipped, with no failure |
| `3` | Invalid CLI usage or configuration |
| `4` | Internal executor or reporting failure |
| `130` | Validation was interrupted |

## Human And JSON Output

Human output is plain by default. `--color` selects the standard palette, and
`--color=<palette>` selects `high-contrast`, `protanopia`, `deuteranopia`,
`tritanopia`, or `achromatopsia`. Layout is independent:
`--presentation=low-vision` increases spacing and removes dim styling. Color
never carries the only indication of status, node type, hierarchy, or errors.

```sh
workspace-validator validate --color=high-contrast
workspace-validator validate --color=deuteranopia --presentation=low-vision
workspace-validator validate --presentation=low-vision
```

`--format=json` emits only the version 4 `ValidationReport`; visual options are
therefore rejected in JSON mode. Reports keep tools, groups, suites, concrete
checks, and the optional repository gate structurally distinct. Captured output
is bounded by `outputLimitBytes` for each stream.

## Library Usage

Loading and planning establish invariants before execution. Their resulting
types expose read-only inspection rather than public constructors or mutable
fields.

```rust,no_run
use std::{
    path::Path,
    sync::{atomic::AtomicBool, Arc},
};
use workspace_validator::{config, execution, planning, reporting};

# fn main() -> Result<(), Box<dyn std::error::Error>> {
let current = std::env::current_dir()?;
let validated = config::load(Some(Path::new(".validation/config.json")), &current)?;
let plan = planning::target(&validated, None)?;
let outcome = execution::run(&validated, &plan, Arc::new(AtomicBool::new(false)));
let json = reporting::result::json::render(&outcome.report)?;
println!("{json}");
# Ok(())
# }
```

`execution::run_with_progress` accepts an implementation of
`execution::progress::ProgressReporter` for typed lifecycle events. The
checked-in [`examples/inspect.rs`](examples/inspect.rs) demonstrates
non-destructive configuration and plan inspection.

The Rust API, CLI commands and exit codes, schema shapes, JSON report, execution
semantics, palette names, and presentation names are observable contracts.
While the crate is `0.x`, incompatible public API changes raise the minor
version and compatible fixes raise the patch version. Cargo versions and JSON
schema versions evolve independently.

## Security And Trust Model

Configuration is trusted executable input. It selects programs and arguments
that run with the permissions of the process invoking `workspace-validator`.
Execution without an implicit shell prevents accidental shell interpretation;
it does not make an untrusted configuration safe.

Review `.validation/config.json` before validating a third-party repository.
The validator does not install dependencies, modify source intentionally, or
apply fixes. Optional Git-integrity detection reports Git-visible mutations,
but it is not an operating-system sandbox and cannot observe every external
side effect.

Private vulnerability reports follow [`SECURITY.md`](SECURITY.md).

## Compatibility And Platforms

The supported platforms are Linux, macOS, and Windows. Functional behavior
includes executable resolution, suite working directories, bounded stream
capture, exit codes, timeout, cancellation, descendant-process termination,
human reports, JSON reports, and Git repository integrity.

The minimum supported Rust version is 1.87. Configuration schema version 6 and
report schema version 4 are exact contracts; unsupported schema versions are
rejected rather than inferred or converted.

## Schemas, Design, And License

Published package contents include:

- [`schemas/config.schema.json`](schemas/config.schema.json);
- [`schemas/report.schema.json`](schemas/report.schema.json);
- [`CONFIG_DESIGN.md`](CONFIG_DESIGN.md);
- [`CHANGELOG.md`](CHANGELOG.md);
- [`SECURITY.md`](SECURITY.md);
- [`LICENSE`](LICENSE).

The crate is distributed under the MIT License.

## Repository Development

From the source repository, replace the installed binary in examples with:

```sh
cargo run --locked -- <command>
```

CI treats the committed `Cargo.lock` as the reproducible dependency graph for
the CLI. `cargo audit` rejects known RustSec advisories, while `cargo deny`
applies the license, source, and duplicate-dependency policy in
[`deny.toml`](deny.toml). The package job verifies the isolated crate archive
and smoke-tests the packaged binary. The CI workflow never publishes a package.
