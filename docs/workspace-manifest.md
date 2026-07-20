# Workspace Manifest

The workspace manifest is a portable JSON file named `.salieri-workspace.json`
at a user-chosen workspace root. It gives projects, samples, preset profiles,
reports, and guidance files one shared local model while keeping paths relative
to the workspace root.

Commands:

```text
:workspace init ROOT
:workspace status ROOT
:workspace index ROOT
:workspace trash ROOT PATH
:workspace restore ROOT PATH
```

`:workspace init ROOT` creates the root if needed, writes the manifest, and
creates default artifact directories:

- `projects`
- `samples`
- `presets`
- `reports`
- `guidance`
- `.salieri-trash`

`:workspace index ROOT` scans those directories and reports counts for project,
sample, preset profile, report, and guidance files. The index is local and
derived; the manifest remains the portable source of metadata.

`:workspace trash ROOT PATH` moves a file inside the workspace to
`.salieri-trash` and records the original and trashed relative paths in the
manifest. It does not delete the file. `:workspace restore ROOT PATH` restores a
record by original or trashed relative path and removes the trash record. Restore
fails if the target path already exists.

The manifest stores only relative paths for roots, favorites, recent files, and
trash records. Absolute paths are rejected so a workspace can be moved or shared
without rewriting metadata.
