# Triage Validation Outcomes

## Start From Existing Evidence

Read the existing JSON report before executing anything else. Preserve the
original selection, tool results, checks, dependency states, repository
snapshot, and summary. Do not rerun validation solely to obtain diagnostics
that are already present.

## Find Primary Causes

Classify each non-pass outcome as one of:

- configuration: invalid contract, graph, reference, parameter, or directory;
- environment: missing executable, incompatible version, permission, or path;
- validation: a check started and returned a failing exit code;
- timeout or interruption;
- dependency propagation: a check was skipped because an upstream check did
  not pass;
- repository integrity: validation introduced, removed, or changed visible
  state;
- internal validator or reporting failure.

Identify primary causes before derivative `blocked` or `skipped` outcomes.
Group repeated diagnostics by cause rather than presenting the same failure for
every dependent check.

## Explain Before Acting

For each primary cause, report:

- status and check ID;
- concrete command and working directory;
- exit code, timeout, or blocking reason;
- the shortest useful stdout or stderr excerpt;
- affected files and lines when the diagnostic supplies them;
- whether the cause appears related to the current implementation.

Do not expose secrets from captured output. Do not treat pre-existing dirty
repository state as a validator mutation; use the report's `introduced`,
`removed`, and `changed` fields.

## Request Authorization

Ask the user in open text before installing a prerequisite or changing source,
configuration, snapshots, generated artifacts, or dependencies. Allow partial
authorization and user-performed actions. Continue independent analysis when
one branch remains blocked.

## Verify A Resolution

After an authorized change:

1. Run the smallest affected check or suite.
2. Compare the result with the original primary cause.
3. Stop and report if the same failure fingerprint repeats without a relevant
   change.
4. Run one final general gate only when required by local policy or the user.

Never turn repeated execution into a substitute for diagnosis.
