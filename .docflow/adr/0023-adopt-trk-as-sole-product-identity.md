---
adr: 0023
title: Adopt trk as the sole product identity
status: Accepted
date: 2026-07-27
owner: default-agent
supersedes:
superseded-by:
depends-on: [0001]
tags: [identity, naming, repository, migration]
---

# ADR 0023 — Adopt trk as the sole product identity

## Context

The product identity is embedded across the repository name, local checkout,
Rust workspace and crate names, executable, public UI copy, command examples,
configuration and data paths, environment variables, file extensions, schema
identifiers, fixtures, documentation, and GitHub links. Renaming only the
visible application would leave conflicting identities in user workflows,
build artifacts, persisted formats, and contributor tooling.

The requested identity is the short name `trk`. This is an intentional hard
cutover: compatibility aliases carrying the legacy identity would violate the
single-name requirement and prolong ambiguity.

## Capability statement

The product, source tree, build artifacts, runtime identifiers, persisted
formats, documentation, local checkout, and GitHub repository use `trk` as
their single identity, with no runtime or source-level compatibility aliases
for the legacy name.

## User stories / scenarios

- As a user, I want one short product and command name, so that installation,
  configuration, project files, and terminal usage are consistent.
- As a contributor, I want crates, paths, schemas, fixtures, and documentation
  to share one identity, so that searches and build output do not expose a
  partial rename.
- As a maintainer, I want the GitHub repository and local checkout to match the
  product name, so that links, automation, and clone instructions stay
  unambiguous.

## Acceptance criteria

1. User-visible product copy, terminal help, documentation, examples, and
   generated reports identify the product and executable as `trk`.
2. Rust crate/package names, crate directories, dependency keys, imports,
   binary targets, workspace members, and build artifacts use `trk` or a
   `trk-*` name, and Cargo metadata resolves the renamed workspace.
3. Project files use `.trk`; configuration/data directories, environment
   variables, schema identifiers, fixtures, and serialized public identifiers
   use the `trk` identity.
4. The GitHub repository is named `delaudio/trk`, repository links and the
   local Git remote target it, and the local checkout directory is named
   `trk`.
5. A case-insensitive search of tracked working-tree content and tracked path
   names finds no legacy product identity outside this ADR; Git history is not
   rewritten.
6. The complete repository verification gate, focused CLI/persistence/import
   compatibility tests, and a Lachesi review pass before integration.

## Out of scope

- Backward-compatible command, extension, environment-variable, crate, schema,
  configuration-path, or repository-name aliases.
- Automatic migration of user files or configuration carrying the legacy
  identity.
- Rewriting Git commit history.
- Changing tracker behavior, project semantics, audio behavior, or UX beyond
  the identity cutover.

## Open questions

- None.

## References

- `0001-record-architecture-decisions.md`
- `../plan/todo/0021-rename-product-and-repository-to-trk.md`
- GitHub issue #309.

## Revision History

| Date | Revision | Author | Change |
|------|----------|--------|--------|
| 2026-07-27 | r1 | default-agent | Proposed the hard cutover to the `trk` identity. |
| 2026-07-27 | r2 | default-agent | Accepted the guided hard-cutover assessment and queued implementation. |

## Approvals

| Role | Name | Date | Signature |
|------|------|------|-----------|
| Maintainer | fdg | 2026-07-27 | Approved the guided hard cutover and instructed autonomous execution in chat. |
