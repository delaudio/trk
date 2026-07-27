use super::*;

pub(crate) fn run_external_sample_browser(
    config: &SampleBrowserConfig,
    request: &SampleBrowserRequest,
) -> Result<Option<PathBuf>> {
    let command_template = config
        .chooser_command
        .as_deref()
        .filter(|command| !command.trim().is_empty())
        .context("sample browser chooser_command is not configured")?;
    let chooser_file = temporary_chooser_file();
    let start_dir = request
        .start_dir
        .as_deref()
        .or(config.start_dir.as_deref())
        .unwrap_or_else(|| Path::new("."));
    let command = command_template
        .replace("{chooser_file}", &shell_quote(&chooser_file))
        .replace("{start_dir}", &shell_quote(start_dir));
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
    let status = ProcessCommand::new(shell)
        .arg("-lc")
        .arg(command)
        .env("TRK_CHOOSER_FILE", &chooser_file)
        .env("TRK_SAMPLE_START_DIR", start_dir)
        .status()
        .context("failed to launch sample browser")?;

    if !status.success() {
        let _ = std::fs::remove_file(&chooser_file);
        anyhow::bail!("sample browser exited with {status}");
    }

    let selected = std::fs::read_to_string(&chooser_file).unwrap_or_default();
    let _ = std::fs::remove_file(&chooser_file);
    let selected = selected.trim();
    if selected.is_empty() {
        Ok(None)
    } else {
        Ok(Some(PathBuf::from(selected)))
    }
}

pub(crate) fn temporary_chooser_file() -> PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    std::env::temp_dir().join(format!(
        "trk-sample-chooser-{}-{timestamp}.txt",
        std::process::id()
    ))
}

pub(crate) fn shell_quote(path: &Path) -> String {
    let value = path.to_string_lossy();
    format!("'{}'", value.replace('\'', "'\\''"))
}

pub(crate) fn read_sample_browser_entries(path: &Path) -> Result<Vec<AppSampleBrowserEntry>> {
    let mut entries = fs::read_dir(path)
        .with_context(|| format!("failed to read sample directory {}", path.display()))?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            let file_type = entry.file_type().ok()?;
            let name = entry.file_name().to_string_lossy().to_string();
            let kind = if file_type.is_dir() {
                SampleBrowserEntryKind::Directory
            } else if file_type.is_file() && is_supported_sample_path(&path) {
                SampleBrowserEntryKind::SupportedSample
            } else if file_type.is_file() {
                SampleBrowserEntryKind::UnsupportedFile
            } else {
                return None;
            };
            Some(AppSampleBrowserEntry { path, name, kind })
        })
        .collect::<Vec<_>>();

    entries.sort_by(|left, right| {
        sample_browser_kind_rank(left.kind)
            .cmp(&sample_browser_kind_rank(right.kind))
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
    });
    Ok(entries)
}

pub(crate) fn sample_browser_kind_rank(kind: SampleBrowserEntryKind) -> u8 {
    match kind {
        SampleBrowserEntryKind::Directory => 0,
        SampleBrowserEntryKind::SupportedSample => 1,
        SampleBrowserEntryKind::UnsupportedFile => 2,
    }
}

pub(crate) fn read_project_browser_entries(
    current_dir: &Path,
    recent_projects: &[PathBuf],
) -> Result<Vec<AppProjectBrowserEntry>> {
    let mut entries = Vec::new();
    let mut seen_projects = HashSet::new();

    for path in recent_projects {
        let key = project_path_key(path);
        if seen_projects.insert(key) {
            entries.push(project_browser_project_entry(
                path.clone(),
                ProjectBrowserEntryKind::RecentProject,
            ));
        }
    }

    let mut discovered = fs::read_dir(current_dir)
        .with_context(|| format!("failed to read project directory {}", current_dir.display()))?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            let file_type = entry.file_type().ok()?;
            let name = entry.file_name().to_string_lossy().to_string();
            if file_type.is_dir() {
                Some(AppProjectBrowserEntry {
                    path,
                    name,
                    kind: ProjectBrowserEntryKind::Directory,
                    detail: "Press Enter to open directory".to_string(),
                })
            } else if file_type.is_file() && is_trk_project_path(&path) {
                let key = project_path_key(&path);
                seen_projects
                    .insert(key)
                    .then(|| project_browser_project_entry(path, ProjectBrowserEntryKind::Project))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    discovered.sort_by(|left, right| {
        project_browser_kind_rank(left.kind)
            .cmp(&project_browser_kind_rank(right.kind))
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
    });
    entries.extend(discovered);
    Ok(entries)
}

pub(crate) fn project_browser_project_entry(
    path: PathBuf,
    preferred_kind: ProjectBrowserEntryKind,
) -> AppProjectBrowserEntry {
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string());

    if !path.exists() {
        return AppProjectBrowserEntry {
            path,
            name,
            kind: ProjectBrowserEntryKind::MissingProject,
            detail: "Missing: file is no longer at this path".to_string(),
        };
    }

    match load_project(&path) {
        Ok(song) => {
            let modified = project_modified_label(&path);
            AppProjectBrowserEntry {
                path,
                name,
                kind: preferred_kind,
                detail: format!(
                    "{} | {} tracks | {} patterns | {} sequence slots | {}",
                    song.metadata.title,
                    song.tracks.len(),
                    song.patterns.len(),
                    song.sequence.len(),
                    modified
                ),
            }
        }
        Err(error) => AppProjectBrowserEntry {
            path,
            name,
            kind: ProjectBrowserEntryKind::InvalidProject,
            detail: format!("Invalid project: {error}"),
        },
    }
}

pub(crate) fn project_browser_kind_rank(kind: ProjectBrowserEntryKind) -> u8 {
    match kind {
        ProjectBrowserEntryKind::RecentProject
        | ProjectBrowserEntryKind::MissingProject
        | ProjectBrowserEntryKind::InvalidProject => 0,
        ProjectBrowserEntryKind::Directory => 1,
        ProjectBrowserEntryKind::Project => 2,
    }
}

pub(crate) fn is_trk_project_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("trk"))
}

pub(crate) fn project_modified_label(path: &Path) -> String {
    fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .ok()
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
        .map_or_else(
            || "modified unknown".to_string(),
            |duration| format!("modified unix {}", duration.as_secs()),
        )
}

pub(crate) fn project_path_key(path: &Path) -> String {
    path.canonicalize()
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .to_lowercase()
}

pub(crate) fn load_recent_projects(path: Option<&Path>) -> Vec<PathBuf> {
    let Some(path) = path else {
        return Vec::new();
    };
    if !path.exists() {
        return Vec::new();
    }

    fs::read_to_string(path)
        .ok()
        .and_then(|contents| serde_json::from_str::<RecentProjectsFile>(&contents).ok())
        .map(|file| file.projects)
        .unwrap_or_default()
}

pub(crate) fn save_recent_projects(path: Option<&Path>, projects: &[PathBuf]) -> Result<()> {
    let Some(path) = path else {
        return Ok(());
    };
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!(
                "failed to create recent project directory {}",
                parent.display()
            )
        })?;
    }
    let contents = serde_json::to_string_pretty(&RecentProjectsFile {
        projects: projects.to_vec(),
    })?;
    fs::write(path, contents)
        .with_context(|| format!("failed to write recent projects {}", path.display()))
}

pub(crate) fn is_supported_sample_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("wav"))
}

pub(crate) fn load_sample_view_data(path: PathBuf) -> Result<AppSampleView> {
    let sample = Sample::load_wav(&path)
        .with_context(|| format!("failed to load sample {}", path.display()))?;
    let overview = sample.waveform_overview(SAMPLE_WAVEFORM_BUCKETS);
    Ok(AppSampleView {
        source_path: path,
        sample,
        overview,
    })
}
