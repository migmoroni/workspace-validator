# Security Policy

## Supported Versions

Security fixes are provided for the latest `0.1.x` release. This policy is
updated when another release line becomes supported.

## Reporting A Vulnerability

Report vulnerabilities privately through
[GitHub Security Advisories](https://github.com/migmoroni/veterinary-clinic/security/advisories/new).
Do not open a public issue before coordinated disclosure.

Include, when available:

- the affected `workspace-validator` version and operating system;
- the configuration and command needed to reproduce the behavior;
- expected and observed results;
- security impact and prerequisites;
- logs or a minimal repository that contains no secrets.

An acknowledgement, impact assessment, and disclosure timeline are coordinated
through the private advisory. Public disclosure occurs after a fix or an
agreed mitigation is available.

## Trust Boundary

`.validation/config.json` is trusted executable input. It chooses programs and
arguments that run with the caller's operating-system permissions. The absence
of an implicit shell reduces command-interpretation risk but does not make an
untrusted configuration safe.

Repository mutation detection is an integrity signal for Git-visible files. It
is not a sandbox, permission boundary, malware scanner, or complete record of
filesystem and network side effects. Review third-party configuration before
execution and use operating-system isolation when the source is not trusted.
