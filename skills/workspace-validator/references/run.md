# Run Validations

## Select The Smallest Correct Scope

Use the target explicitly requested by the user. Use the configured default
group only for a general gate or when no narrower scope was requested.

```sh
workspace-validator validate --format=json
workspace-validator validate <group-or-suite> --format=json
workspace-validator check <check-id> --format=json
```

Configuration discovery already validates the complete document before
preflight. Do not run every suite or check manually before running a group.

## Execute Once

1. Record the repository status without cleaning or restoring it.
2. Confirm that the selected configuration is trusted.
3. Run one validator command for the selected scope.
4. Wait for that process to finish. Polling one running process is not a new
   validation; starting the command again is.
5. Parse the JSON report rather than inferring success from terminal text.

Do not start interactive commands, watch modes, development servers, or
packaging workflows as substitutes for declared checks.

## Interpret The Result

Declare success only when `summary.result` is `pass`. Present outcomes in this
order:

1. `fail`
2. `blocked`
3. `skipped`
4. `pass`

Include the selection, tool versions, totals, duration, relevant check IDs,
commands, exit codes, timeouts, truncation markers, and repository mutations.
Do not publish complete captured streams when a concise diagnostic is enough,
and redact credentials or sensitive values if a command emitted them.

## Prevent Validation Loops

- Do not rerun a successful validation for reassurance.
- Do not rerun an unchanged failure.
- When an environment prerequisite is restored, run only the scope that was
  blocked.
- After an authorized code correction, rerun only the affected check or suite.
- Run one final general gate only when the workspace policy or user requires
  it.

Load `triage.md` only after a report needs diagnosis.
