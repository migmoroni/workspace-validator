# Audit Validation Coverage

## Inspect Without Running Checks

Start with non-executing inspection commands:

```sh
workspace-validator config validate
workspace-validator list --tree
workspace-validator explain group <group-id>
workspace-validator explain suite <suite-id>
workspace-validator explain check <check-id>
```

Review the workspace manifests and existing scripts only far enough to compare
the declared profile with the technologies and commands the project actually
uses. Do not execute checks or edit configuration unless separately requested.

## Evaluate The Profile

Assess:

- coverage of formatting, static analysis, tests, builds, repository integrity,
  and domain-specific validation already supported by the workspace;
- separation between fast, general, and specialized groups;
- reusable checks versus suite-owned targets, parameters, and directories;
- duplicate or unnecessarily expensive execution paths;
- missing prerequisites or version requirements;
- unrealistic timeouts or output limits;
- dependency ordering and propagation behavior;
- commands that are interactive, destructive, mutating, or shell-dependent;
- untrusted configuration or parameters derived from untrusted input;
- whether failures, blocked prerequisites, skipped dependents, and mutations
  remain observable in reports.

Do not require every possible validation category. Judge coverage against the
workspace's current languages, artifacts, risk, and delivery process.

## Report Recommendations

Present findings by severity and distinguish:

- correctness or security defects;
- missing coverage with concrete risk;
- performance or maintenance improvements;
- intentional tradeoffs that require no change.

Reference the affected config entry and current command. Recommend the smallest
coherent change, but do not add tools, install dependencies, or modify the
profile without user approval.
