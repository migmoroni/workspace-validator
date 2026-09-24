---
name: workspace-validator
description: Safely run, diagnose, configure, and audit declarative validation workflows with the workspace-validator CLI. Use when an agent needs to execute a validation gate, interpret a JSON validation report, investigate FAIL, BLOCKED, or SKIPPED outcomes, modify .validation/config.json, or assess validation coverage and safety.
---

# Workspace Validator

- Tool: `workspace-validator`

Use this file as the only entry point. Load exactly one workflow reference at a
time unless the task genuinely crosses workflow boundaries.

## Check Compatibility

1. Read `manifest.json` from this skill directory.
2. Run `workspace-validator --version` once for the current task.
3. Compare the reported CLI version with `sourceCrateVersion` and
   `compatibleCli`.
4. Apply the following policy:
   - If the command is unavailable, report the missing prerequisite and stop.
     Do not install it without explicit user approval.
   - If the version equals `sourceCrateVersion`, continue.
   - If the CLI is newer but remains inside `compatibleCli`, warn that the
     copied skill may be stale, then continue using only documented compatible
     behavior.
   - If the CLI is older but remains inside `compatibleCli`, warn that the CLI
     predates the skill, then continue only with behavior covered by the
     declared compatibility range.
   - If it is outside `compatibleCli`, report the incompatibility and stop the
     validator-dependent workflow.

Never update, download, or replace this skill automatically. Local workspace
instructions take precedence over this bundled workflow.

## Route The Task

- Execute a group, suite, check, or final gate: read
  `references/run.md` only.
- Interpret an existing report or investigate a non-pass outcome: read
  `references/triage.md` only.
- Create or modify `.validation/config.json`: read
  `references/config.md` only.
- Evaluate validation coverage, cost, or safety without changing the profile:
  read `references/audit.md` only.

When execution produces a non-pass result, finish the run workflow before
loading the triage workflow. Do not preload every reference.

## Preserve Safety

- Treat validator configuration as trusted executable policy, not inert data.
- Do not execute a newly supplied or untrusted configuration without explicit
  user approval.
- Do not install dependencies, alter source, apply fixes, accept snapshots, or
  update generated output without explicit authorization.
- Do not duplicate a selected group by manually running its member checks.
- Do not retry an unchanged failure merely to seek a different result.
- Preserve the initial repository state and report validator-detected
  mutations.
- Prefer structured JSON reports for agent interpretation and expose only the
  diagnostic excerpts needed by the user.
- Keep project-specific commands and approval policy in the consumer
  workspace. This skill defines validator mechanics only.
