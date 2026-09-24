# Changelog

All notable changes to `workspace-validator` are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and the project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Versioned agent skill bundle for safe validation execution, report triage,
  configuration, and coverage auditing with progressive disclosure.

## [0.1.0]

### Added

- Declarative version 6 configuration for tools, checks, suites, and groups.
- Shell-free command execution with structured parameters and suite working
  directories.
- Deterministic validation DAGs with timeout, cancellation, bounded output,
  dependency handling, and process-tree termination.
- Accessible human progress and completed reports with composable palettes and
  low-vision presentation.
- Version 4 JSON reports and checked-in JSON Schemas.
- Optional Git repository-integrity gate.
- Rust library API for configuration, planning, execution, progress events,
  reports, and themes.
- Supported execution on Linux, macOS, and Windows with portable process
  fixtures and descendant-process termination.
