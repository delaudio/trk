use std::{
    fs, io,
    path::{Path, PathBuf},
};

use super::*;

const WORKSPACE_MANIFEST_SCHEMA: &str = "salieri.workspace.v1";
const WORKSPACE_MANIFEST_FILE: &str = ".salieri-workspace.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WorkspaceManifest {
    schema: String,
    name: String,
    roots: WorkspaceRoots,
    favorites: Vec<PathBuf>,
    recent_files: Vec<PathBuf>,
    naming: WorkspaceNaming,
    trash_records: Vec<WorkspaceTrashRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WorkspaceRoots {
    projects: PathBuf,
    samples: PathBuf,
    presets: PathBuf,
    reports: PathBuf,
    guidance: PathBuf,
    trash: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WorkspaceNaming {
    project_suffix: String,
    prefer_kebab_case: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WorkspaceTrashRecord {
    original: PathBuf,
    trashed: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct WorkspaceIndex {
    projects: Vec<PathBuf>,
    samples: Vec<PathBuf>,
    presets: Vec<PathBuf>,
    reports: Vec<PathBuf>,
    guidance: Vec<PathBuf>,
}

impl App {
    pub(crate) fn handle_workspace_command(&mut self, values: &[&str]) {
        match values {
            [] | ["status", ..] => self.workspace_status_command(values.get(1..).unwrap_or(&[])),
            ["init", path @ ..] => self.workspace_init_command(path),
            ["index", path @ ..] => self.workspace_index_command(path),
            ["trash", root, path @ ..] => self.workspace_trash_command(root, path),
            ["restore", root, path @ ..] => self.workspace_restore_command(root, path),
            _ => self.notify_warning(
                "Usage: :workspace init ROOT | status ROOT | index ROOT | trash ROOT PATH | restore ROOT PATH",
            ),
        }
    }

    fn workspace_init_command(&mut self, values: &[&str]) {
        let Ok(root) = command_path(values, "workspace root is required") else {
            self.notify_warning("Usage: :workspace init ROOT");
            return;
        };
        match init_workspace(&root) {
            Ok(manifest) => {
                self.notify_success(format!(
                    "Workspace ready: {} ({})",
                    manifest.name,
                    manifest_path(&root).display()
                ));
            }
            Err(error) => self.notify_warning(format!("Workspace init failed: {error}")),
        }
    }

    fn workspace_status_command(&mut self, values: &[&str]) {
        let Ok(root) = command_path(values, "workspace root is required") else {
            self.notify_warning("Usage: :workspace status ROOT");
            return;
        };
        match load_workspace_manifest(&root) {
            Ok(manifest) => self.notify_info(workspace_status_summary(&manifest)),
            Err(error) => self.notify_warning(format!("Workspace status failed: {error}")),
        }
    }

    fn workspace_index_command(&mut self, values: &[&str]) {
        let Ok(root) = command_path(values, "workspace root is required") else {
            self.notify_warning("Usage: :workspace index ROOT");
            return;
        };
        match load_workspace_manifest(&root).and_then(|manifest| index_workspace(&root, &manifest))
        {
            Ok(index) => {
                let summary = workspace_index_summary(&index);
                self.push_ai_message(AiMessageRole::Assistant, summary.clone());
                self.notify_info(summary);
            }
            Err(error) => self.notify_warning(format!("Workspace index failed: {error}")),
        }
    }

    fn workspace_trash_command(&mut self, root: &str, values: &[&str]) {
        let Ok(path) = command_path(values, "workspace file path is required") else {
            self.notify_warning("Usage: :workspace trash ROOT PATH");
            return;
        };
        match trash_workspace_file(Path::new(root), &path) {
            Ok(record) => self.notify_success(format!(
                "Moved to workspace trash: {} -> {}",
                record.original.display(),
                record.trashed.display()
            )),
            Err(error) => self.notify_warning(format!("Workspace trash failed: {error}")),
        }
    }

    fn workspace_restore_command(&mut self, root: &str, values: &[&str]) {
        let Ok(path) = command_path(values, "workspace restore path is required") else {
            self.notify_warning("Usage: :workspace restore ROOT PATH");
            return;
        };
        match restore_workspace_file(Path::new(root), &path) {
            Ok(record) => self.notify_success(format!(
                "Restored workspace file: {} -> {}",
                record.trashed.display(),
                record.original.display()
            )),
            Err(error) => self.notify_warning(format!("Workspace restore failed: {error}")),
        }
    }
}

fn command_path(values: &[&str], error: &str) -> Result<PathBuf, String> {
    let path = values.join(" ");
    let path = path.trim();
    if path.is_empty() {
        Err(error.to_string())
    } else {
        Ok(PathBuf::from(path))
    }
}

fn init_workspace(root: &Path) -> Result<WorkspaceManifest, String> {
    fs::create_dir_all(root)
        .map_err(|error| format!("cannot create {}: {error}", root.display()))?;
    let path = manifest_path(root);
    let manifest = if path.exists() {
        load_workspace_manifest(root)?
    } else {
        default_workspace_manifest(root)
    };
    for dir in workspace_dirs(&manifest) {
        fs::create_dir_all(root.join(dir))
            .map_err(|error| format!("cannot create workspace dir {}: {error}", dir.display()))?;
    }
    save_workspace_manifest(root, &manifest)?;
    Ok(manifest)
}

fn default_workspace_manifest(root: &Path) -> WorkspaceManifest {
    WorkspaceManifest {
        schema: WORKSPACE_MANIFEST_SCHEMA.to_string(),
        name: root
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("Salieri Workspace")
            .to_string(),
        roots: WorkspaceRoots {
            projects: PathBuf::from("projects"),
            samples: PathBuf::from("samples"),
            presets: PathBuf::from("presets"),
            reports: PathBuf::from("reports"),
            guidance: PathBuf::from("guidance"),
            trash: PathBuf::from(".salieri-trash"),
        },
        favorites: Vec::new(),
        recent_files: Vec::new(),
        naming: WorkspaceNaming {
            project_suffix: ".salieri".to_string(),
            prefer_kebab_case: true,
        },
        trash_records: Vec::new(),
    }
}

fn workspace_dirs(manifest: &WorkspaceManifest) -> Vec<&Path> {
    vec![
        manifest.roots.projects.as_path(),
        manifest.roots.samples.as_path(),
        manifest.roots.presets.as_path(),
        manifest.roots.reports.as_path(),
        manifest.roots.guidance.as_path(),
        manifest.roots.trash.as_path(),
    ]
}

fn manifest_path(root: &Path) -> PathBuf {
    root.join(WORKSPACE_MANIFEST_FILE)
}

fn save_workspace_manifest(root: &Path, manifest: &WorkspaceManifest) -> Result<(), String> {
    ensure_manifest_is_portable(manifest)?;
    let json = serde_json::to_string_pretty(manifest)
        .map_err(|error| format!("cannot encode workspace manifest: {error}"))?;
    fs::write(manifest_path(root), format!("{json}\n"))
        .map_err(|error| format!("cannot write manifest: {error}"))
}

pub(super) fn workspace_report_artifact_path(
    root: &Path,
    file_name: &str,
) -> Result<PathBuf, String> {
    let manifest = load_workspace_manifest(root)?;
    let reports_dir = root.join(&manifest.roots.reports);
    fs::create_dir_all(&reports_dir).map_err(|error| {
        format!(
            "cannot create workspace reports dir {}: {error}",
            reports_dir.display()
        )
    })?;
    Ok(reports_dir.join(file_name))
}

fn load_workspace_manifest(root: &Path) -> Result<WorkspaceManifest, String> {
    let path = manifest_path(root);
    let raw = fs::read_to_string(&path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let manifest = serde_json::from_str::<WorkspaceManifest>(&raw)
        .map_err(|error| format!("invalid workspace manifest JSON: {error}"))?;
    if manifest.schema != WORKSPACE_MANIFEST_SCHEMA {
        return Err(format!(
            "unsupported workspace manifest schema {:?}",
            manifest.schema
        ));
    }
    ensure_manifest_is_portable(&manifest)?;
    Ok(manifest)
}

fn ensure_manifest_is_portable(manifest: &WorkspaceManifest) -> Result<(), String> {
    for path in workspace_dirs(manifest)
        .into_iter()
        .chain(manifest.favorites.iter().map(PathBuf::as_path))
        .chain(manifest.recent_files.iter().map(PathBuf::as_path))
        .chain(
            manifest
                .trash_records
                .iter()
                .flat_map(|record| [record.original.as_path(), record.trashed.as_path()]),
        )
    {
        if path.is_absolute() {
            return Err(format!(
                "workspace manifest path must be relative: {}",
                path.display()
            ));
        }
    }
    Ok(())
}

fn index_workspace(root: &Path, manifest: &WorkspaceManifest) -> Result<WorkspaceIndex, String> {
    let mut index = WorkspaceIndex::default();
    collect_by_extension(
        &root.join(&manifest.roots.projects),
        &["salieri"],
        root,
        &mut index.projects,
    )?;
    collect_by_extension(
        &root.join(&manifest.roots.samples),
        &["wav"],
        root,
        &mut index.samples,
    )?;
    collect_by_extension(
        &root.join(&manifest.roots.presets),
        &["json"],
        root,
        &mut index.presets,
    )?;
    collect_by_extension(
        &root.join(&manifest.roots.reports),
        &["md", "txt", "json"],
        root,
        &mut index.reports,
    )?;
    collect_by_extension(
        &root.join(&manifest.roots.guidance),
        &["md", "txt", "json"],
        root,
        &mut index.guidance,
    )?;
    Ok(index)
}

fn collect_by_extension(
    dir: &Path,
    extensions: &[&str],
    root: &Path,
    output: &mut Vec<PathBuf>,
) -> Result<(), String> {
    match fs::read_dir(dir) {
        Ok(entries) => {
            for entry in entries {
                let path = entry
                    .map_err(|error| format!("cannot read {}: {error}", dir.display()))?
                    .path();
                if path.is_dir() {
                    collect_by_extension(&path, extensions, root, output)?;
                } else if path
                    .extension()
                    .and_then(|value| value.to_str())
                    .is_some_and(|ext| {
                        extensions
                            .iter()
                            .any(|expected| ext.eq_ignore_ascii_case(expected))
                    })
                {
                    output.push(relative_to_root(root, &path)?);
                }
            }
            output.sort();
            Ok(())
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("cannot read {}: {error}", dir.display())),
    }
}

fn trash_workspace_file(root: &Path, path: &Path) -> Result<WorkspaceTrashRecord, String> {
    let mut manifest = load_workspace_manifest(root)?;
    let original = relative_to_root(root, &root.join(path))?;
    let source = root.join(&original);
    if !source.is_file() {
        return Err(format!("file not found: {}", source.display()));
    }
    let trash_dir = root.join(&manifest.roots.trash);
    fs::create_dir_all(&trash_dir)
        .map_err(|error| format!("cannot create {}: {error}", trash_dir.display()))?;
    let file_name = source
        .file_name()
        .ok_or_else(|| format!("invalid file path: {}", source.display()))?;
    let target = unique_trash_path(&trash_dir, file_name);
    fs::rename(&source, &target).map_err(|error| {
        format!(
            "cannot move {} to {}: {error}",
            source.display(),
            target.display()
        )
    })?;
    let record = WorkspaceTrashRecord {
        original,
        trashed: relative_to_root(root, &target)?,
    };
    manifest.trash_records.push(record.clone());
    save_workspace_manifest(root, &manifest)?;
    Ok(record)
}

fn restore_workspace_file(root: &Path, selector: &Path) -> Result<WorkspaceTrashRecord, String> {
    let mut manifest = load_workspace_manifest(root)?;
    let selector = if selector.is_absolute() {
        relative_to_root(root, selector)?
    } else {
        selector.to_path_buf()
    };
    let index = manifest
        .trash_records
        .iter()
        .position(|record| record.original == selector || record.trashed == selector)
        .ok_or_else(|| format!("no trash record for {}", selector.display()))?;
    let record = manifest.trash_records.remove(index);
    let source = root.join(&record.trashed);
    let target = root.join(&record.original);
    if target.exists() {
        return Err(format!(
            "restore target already exists: {}",
            target.display()
        ));
    }
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
    }
    fs::rename(&source, &target).map_err(|error| {
        format!(
            "cannot restore {} to {}: {error}",
            source.display(),
            target.display()
        )
    })?;
    save_workspace_manifest(root, &manifest)?;
    Ok(record)
}

fn relative_to_root(root: &Path, path: &Path) -> Result<PathBuf, String> {
    let absolute_root = root
        .canonicalize()
        .map_err(|error| format!("cannot resolve {}: {error}", root.display()))?;
    let absolute_path = if path.exists() {
        path.canonicalize()
            .map_err(|error| format!("cannot resolve {}: {error}", path.display()))?
    } else {
        absolute_root.join(path)
    };
    absolute_path
        .strip_prefix(&absolute_root)
        .map(Path::to_path_buf)
        .map_err(|_| format!("path is outside workspace root: {}", path.display()))
}

fn unique_trash_path(dir: &Path, file_name: &std::ffi::OsStr) -> PathBuf {
    let first = dir.join(file_name);
    if !first.exists() {
        return first;
    }
    let name = file_name.to_string_lossy();
    for index in 1.. {
        let candidate = dir.join(format!("{index}-{name}"));
        if !candidate.exists() {
            return candidate;
        }
    }
    unreachable!("unbounded suffix search returns a candidate")
}

fn workspace_status_summary(manifest: &WorkspaceManifest) -> String {
    format!(
        "Workspace {}: projects={}, samples={}, presets={}, reports={}, guidance={}, trash records={}",
        manifest.name,
        manifest.roots.projects.display(),
        manifest.roots.samples.display(),
        manifest.roots.presets.display(),
        manifest.roots.reports.display(),
        manifest.roots.guidance.display(),
        manifest.trash_records.len()
    )
}

fn workspace_index_summary(index: &WorkspaceIndex) -> String {
    format!(
        "Workspace index: {} project(s), {} sample(s), {} preset profile(s), {} report(s), {} guidance file(s)",
        index.projects.len(),
        index.samples.len(),
        index.presets.len(),
        index.reports.len(),
        index.guidance.len()
    )
}
