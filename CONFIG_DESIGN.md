# Configuration Design

This document defines the design contract for
`.validation/config.json`. It complements the usage-oriented
[`README.md`](README.md) and the machine-readable
[`config.schema.json`](schemas/config.schema.json).

The configuration is a declarative domain-specific language for validation
graphs. It is configuration as data: the document describes available
capabilities, reusable validation operations, contextual executions, and their
composition. It does not contain imperative control flow.

## Design Goals

The configuration is designed to be:

- explicit: execution behavior is visible in the document;
- deterministic: the same valid document produces the same execution plan;
- reusable: command definitions do not encode the project context that invokes
  them;
- strict: unknown fields, invalid references, cycles, and unsafe paths are
  rejected before any process starts;
- inspectable: commands and execution graphs can be explained without running
  them;
- portable: commands are represented as executable and argument vectors rather
  than shell expressions;
- versioned: structural meaning belongs to one exact `schemaVersion`.

The configuration is not intended to be a shell language, a package manager, a
source-file discovery mechanism, or a general-purpose build system.

## Conceptual Model

The model has four layers:

```text
tool -> check -> suite -> group
```

Each layer owns one kind of decision:

| Layer | Responsibility | Must not decide |
| --- | --- | --- |
| `tool` | How an executable is found and versioned | What validation it performs |
| `check` | The reusable validation command and its argument template | Which project context invokes it |
| `suite` | Where checks execute and which placeholder values contextualize them | How broader validation contexts are composed |
| `group` | Which suites and groups form an ordered validation context | Command arguments or working directories |

This ownership rule is normative. A new field belongs to the lowest layer that
can own it without learning about a higher-level context. The same concern must
not be represented in multiple layers.

### Tools

A tool is an executable prerequisite. It defines:

- a stable configuration ID;
- the executable program;
- tool prerequisites;
- the arguments and parser used to discover its version;
- an optional semantic-version requirement.

A tool does not define validation behavior. Multiple checks may use the same
tool, and more than one tool ID may use the same program when their capability
or version probes differ.

### Checks

A check is the smallest reusable validation operation. It defines:

- identity, label, and description;
- the primary `toolId`;
- an argument template;
- additional tool prerequisites;
- a timeout.

A check must be meaningful without knowing a suite. Literal arguments describe
the stable part of a command, while exact `{name}` arguments reserve optional
positions for contextual argument sequences.

A check does not own `workingDirectory`, package selection, application scope,
or dependencies on checks in an unrelated context. Direct `check <id>`
execution therefore omits every placeholder and runs the resulting general
command at `workspaceRoot`. Every argument template must consequently remain a
valid general command after all placeholders are removed. Flags that require a
parameter value belong inside that parameter's argument sequence rather than
beside its placeholder in the template.

### Suites

A suite is one executable validation context. It defines:

- one optional `workingDirectory` shared by its checks;
- one or more ordered check invocations;
- invocation-specific parameter bindings;
- dependencies among invocations in that suite.

For an invocation, the final process arguments are exactly:

```text
expand(check.args, suiteCheck.parameters)
```

Expansion walks `check.args` in order. An argument formed entirely as `{name}`
is a placeholder when `name` follows the configuration ID grammar:

- a bound string emits one argument;
- a bound string array emits zero or more arguments;
- an unbound placeholder emits no arguments;
- every other argument remains literal.

Parameter bindings are inferred from the placeholders; checks do not maintain
a second declaration list. Supplied names that do not occur in the check are
invalid. Placeholders may occur more than once, but values are never split on
whitespace, expanded recursively, substituted inside a larger string, or
interpreted by a shell.

`dependsOn` is local to the suite. It may reference only a check invocation
that appears earlier in the same suite. A check ID may occur at most once in a
suite, making that ID the invocation identity within the suite.

The same check may appear in different suites. Each occurrence is an
independent contextual execution because its working directory and parameter
bindings may differ.

### Groups

A group is an ordered composition of typed `group` and `suite` references. It
expresses a validation context such as `frontend`, `rust`, `knowledge`, or the
general workspace gate.

Groups form a directed acyclic graph. Convergent references are valid: a suite
or group reached through multiple paths executes once, and later references
reuse its result. Member order remains deterministic.

Groups do not contain commands, arguments, working directories, or checks
directly. Group and suite IDs share one selection namespace and therefore must
not collide.

## Command Model

Commands use an exec-style model:

```text
program, argv[], cwd
```

They are never assembled into a shell command string. Consequently:

- every resolved argument is one explicit JSON string;
- quoting and escaping are not delegated to a shell;
- pipes, redirections, substitutions, and shell operators are not interpreted;
- the selected suite supplies `cwd` without changing global process state.

If shell behavior is genuinely required, it must be represented by an explicit
tool whose program is the chosen shell. It must not be inferred from argument
contents.

## Working Directories

`workspaceRoot` establishes the filesystem boundary for the document. A suite
may select a directory relative to that root; omission means the root itself.

Resolved working directories must:

- be relative paths contained by `workspaceRoot`;
- exist and be directories;
- reject `..`, absolute paths, and symlink escapes.

Checks and groups never own or inherit working directories. This keeps one
concrete execution context in one suite and prevents hidden path composition.

## Dependencies And Ordering

The configuration contains two distinct graph relationships:

1. group membership composes groups and suites into a DAG;
2. suite-local `dependsOn` constrains ordered check invocations.

These relationships must remain separate. Group membership expresses
composition and reuse. `dependsOn` expresses execution eligibility inside one
suite: when a prerequisite does not pass, its dependent check is skipped.

Array order is meaningful and deterministic. Dependencies may restrict that
order but must not introduce an alternative global check graph.

## Repository Integrity

`repository` is a run-level integrity gate, not a regular check. It captures
repository state around the selected execution and detects mutations caused by
validation commands.

Keeping it separate prevents repository integrity from being duplicated in
groups, suites, or check counts. Its provider and tool are explicit, and the
gate is absent when `repository` is not configured.

## Validation Boundary

Configuration health is established before preflight or execution. Validation
has two complementary layers:

1. JSON Schema validates document shape, primitive constraints, required
   properties, and unknown fields;
2. semantic validation resolves IDs, paths, tool requirements, dependencies,
   namespace collisions, and graph invariants.

A document is executable only when both layers accept the complete
configuration. Validation does not stop after checking only the selected group
or suite, because an invalid unselected definition would make the document an
unreliable source of truth.

## Identity And Naming

IDs are stable machine identifiers. Labels and descriptions are human-facing
metadata and never participate in resolution or execution.

IDs use lowercase namespaced segments separated by `.`, `_`, or `-`. Dotted
segments should communicate ownership from broad to specific:

```text
repository.git.diff
frontend.svelte.static
rust.cargo.tests
knowledge.domain.validate
```

Recommended meanings are:

- tool IDs name an executable capability;
- check IDs name a domain, implementation, and action when applicable;
- suite IDs name the concrete validation context;
- group IDs name the broader context being composed.

Renaming a label must not require changing references. Renaming an ID is a
contract change and requires updating every current reference atomically.

## Schema Evolution

`schemaVersion` identifies the exact configuration contract accepted by the
binary. Structural or semantic changes to that contract require coordinated
updates to:

- Rust configuration types;
- the checked-in JSON Schema;
- semantic validation;
- planning and execution when affected;
- inspection commands and documentation;
- configuration fixtures and tests;
- the canonical `.validation/config.json`.

The current contract is authoritative. The loader does not infer missing
semantics, reinterpret fields from another version, or maintain parallel
configuration models.

`$schema` is an editor and tooling reference. `schemaVersion` is the runtime
contract discriminator; neither replaces the other.

## Extension Rules

Before adding a field or concept, answer these questions:

1. Which single layer owns the decision?
2. Can the behavior be expressed through an existing tool, check invocation,
   suite, or group?
3. Is the value declarative and inspectable without execution?
4. Does it preserve exact-token parameter expansion and deterministic ordering?
5. Can structural and semantic errors be rejected before process startup?
6. Does it avoid hidden inheritance, partial interpolation, and duplicated
   sources of truth?

A proposed field should be rejected or redesigned when its ownership is
ambiguous. In particular:

- project or package selectors belong to suite check parameters bound to
  placeholders in reusable check definitions;
- executable discovery belongs to tools, not suites;
- working directories belong to suites, not checks or groups;
- cross-context composition belongs to groups, not checks;
- suite-local execution dependencies belong to suite invocations, not global
  check definitions.

## Canonical Example

```json
{
  "$schema": "./config.schema.json",
  "schemaVersion": 6,
  "workspaceRoot": "..",
  "defaultGroup": "general",
  "outputLimitBytes": 1048576,
  "tools": [
    {
      "id": "cargo",
      "program": "cargo",
      "requiresTools": [],
      "versionArgs": ["--version"],
      "versionParser": "firstSemver"
    }
  ],
  "checks": [
    {
      "id": "rust.cargo.tests",
      "label": "Rust tests",
      "description": "Runs Rust tests selected by a suite.",
      "toolId": "cargo",
      "args": ["test", "{target}"],
      "requiresTools": [],
      "timeoutSeconds": 3600
    }
  ],
  "suites": [
    {
      "id": "rust.tests",
      "label": "Rust tests",
      "description": "Runs every Rust workspace test.",
      "checks": [
        {
          "checkId": "rust.cargo.tests",
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
      "id": "general",
      "label": "General validation",
      "description": "Runs the complete validation gate.",
      "members": [{ "kind": "suite", "id": "rust.tests" }]
    }
  ]
}
```

The example keeps the reusable operation and insertion position in the check.
The suite binds the workspace-specific target at that position, while direct
execution omits it and runs `cargo test`. The group composes the context without
learning how the command is executed.
