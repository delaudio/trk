# Rename product and repository to trk

Owning ADR: `../../adr/0023-rename-salieri-tracker-to-trk.md`

GitHub issue: #309

## Scope

Perform a hard identity cutover across user-visible copy, Rust workspace
packages and imports, executable and build artifacts, persisted project and
schema identifiers, configuration and data paths, environment variables,
fixtures, documentation, automation, repository links, the GitHub repository,
and the local checkout. Do not retain compatibility aliases carrying the
legacy identity.

## Exit criteria

1. UI, help, documentation, examples, and generated reports identify the
   product and executable as `trk` (ADR AC1).
2. Cargo packages, crate directories, dependencies, imports, binary targets,
   and workspace metadata use only `trk` identities (ADR AC2).
3. Project extensions, configuration/data paths, environment variables,
   schemas, fixtures, and serialized identifiers use only `trk` identities
   (ADR AC3).
4. The GitHub repository, local remote, repository links, and local checkout
   directory are named `trk` (ADR AC4).
5. Case-insensitive tracked-content and tracked-path audits find no legacy
   identity outside the migration ADR; Git history remains unchanged
   (ADR AC5).
6. The complete repository gate, focused compatibility checks, and Lachesi
   review pass before PR integration (ADR AC6).

## Dependencies

- `../../adr/0001-record-architecture-decisions.md`
- The product owner has accepted the manual-migration hard cutover.
