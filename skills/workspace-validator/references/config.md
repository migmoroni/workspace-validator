# Configure Validation

## Preserve The Ownership Model

Keep the configuration hierarchy explicit:

```text
tool -> check -> suite -> group
```

- A tool declares executable discovery, version parsing, and prerequisites.
- A check declares one reusable executable and argument template.
- A suite supplies parameters, dependencies, and its working directory.
- A group composes suites and other groups into an ordered DAG.

Do not place project targets or directories directly into a reusable check
when a suite can bind them through structured placeholders. Only suites own
`workingDirectory`. Use argument arrays and never encode shell pipelines,
redirection, substitutions, or command chaining.

## Make Intent Explicit

- Use stable, descriptive IDs, labels, and descriptions.
- Declare every prerequisite and meaningful version requirement.
- Set finite timeouts and a bounded output limit.
- Keep check dependencies inside suite invocations.
- Use groups for contextual composition, not for owning execution settings.
- Let shared suites execute once through graph deduplication.
- Keep a fast group representative and a general group complete when the
  workspace needs both.

Validation checks should be non-interactive and read-only by default. Reject
installers, watch modes, development servers, destructive commands, and
formatters that modify files unless the user explicitly defines a different
policy.

## Validate Before Execution

After an authorized configuration edit, run:

```sh
workspace-validator config validate
workspace-validator list --tree
```

Use `workspace-validator explain group`, `explain suite`, or `explain check`
for focused inspection. Do not execute the configured graph unless the user
also requested execution.

Do not change `schemaVersion` speculatively. Configuration and report schema
versions are exact contracts and evolve independently from the crate version.
Generate local schemas only when the consumer explicitly wants checked-in or
editor-facing copies.

## Respect Consumer Ownership

Derive project commands from current manifests and scripts. Do not invent
tools, dependencies, compatibility layers, or validation categories. Preserve
project-specific approval rules in local agent instructions rather than adding
them to this reusable skill.
