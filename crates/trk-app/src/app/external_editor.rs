use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Seek, Write},
    path::{Path, PathBuf},
    process::ExitStatus,
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use super::*;

#[path = "external_editor_process.rs"]
mod process;
pub(crate) use process::{run_external_editor, ExternalEditorRunError};
#[path = "external_editor_file.rs"]
mod project_file_io;
use project_file_io::open_regular_project;

const PROJECT_WATCH_INTERVAL: Duration = Duration::from_millis(250);
const PROJECT_CONTENT_VERIFY_INTERVAL: Duration = Duration::from_secs(5);
const FILE_OBSERVATION_ATTEMPTS: usize = 3;
const SCRATCH_CREATE_ATTEMPTS: usize = 128;
static SCRATCH_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone)]
pub(crate) struct ExternalEditorRequest {
    pub(crate) path: PathBuf,
    scratch: bool,
    scratch_directory: Option<PathBuf>,
    baseline_song: Song,
    baseline_variation_history: PatternVariationHistory,
}

impl App {
    pub(crate) fn take_external_editor_request(&mut self) -> Option<ExternalEditorRequest> {
        if !std::mem::take(&mut self.external_editor_requested) {
            return None;
        }
        match self.prepare_external_editor_request() {
            Ok(request) => Some(request),
            Err(error) => {
                self.notify_error(format!("External editor failed: {error}"));
                None
            }
        }
    }

    fn prepare_external_editor_request(&mut self) -> Result<ExternalEditorRequest, String> {
        let scratch = self.project_path.is_none() || self.dirty;
        let (path, scratch_directory) = if scratch {
            let project = crate::persistence::ProjectFile::with_history(
                self.song.clone(),
                self.variation_history.clone(),
            );
            let scratch = create_scratch_project(&project)?;
            (scratch.path, Some(scratch.directory))
        } else {
            let path = self
                .project_path
                .clone()
                .expect("named project required when scratch is false");
            if !is_regular_project_path(&path) {
                return Err(format!(
                    "active project path is missing, symlinked, or not a regular file: {}",
                    path.display()
                ));
            }
            (path, None)
        };
        Ok(ExternalEditorRequest {
            path,
            scratch,
            scratch_directory,
            baseline_song: self.song.clone(),
            baseline_variation_history: self.variation_history.clone(),
        })
    }

    pub(crate) fn finish_external_editor(
        &mut self,
        request: ExternalEditorRequest,
        result: Result<ExitStatus, ExternalEditorRunError>,
    ) {
        let status = match result {
            Ok(status) => status,
            Err(ExternalEditorRunError::Launch(error)) => {
                let cleanup = if request.scratch {
                    remove_scratch(&request.path, request.scratch_directory.as_deref())
                } else {
                    String::new()
                };
                self.notify_error(format!("External editor failed: {error}{cleanup}"));
                return;
            }
            Err(ExternalEditorRunError::Wait(error)) => {
                let recovery = if request.scratch {
                    format!("; scratch kept at {}", request.path.display())
                } else {
                    String::new()
                };
                self.notify_error(format!("External editor wait failed: {error}{recovery}"));
                return;
            }
        };
        if !status.success() {
            let recovery = if request.scratch {
                format!("; scratch kept at {}", request.path.display())
            } else {
                String::new()
            };
            self.notify_error(format!("External editor exited with {status}{recovery}"));
            return;
        }
        if self.song != request.baseline_song
            || self.variation_history != request.baseline_variation_history
        {
            let recovery = if request.scratch {
                format!("; scratch kept at {}", request.path.display())
            } else {
                String::new()
            };
            self.notify_warning(format!(
                "External edit conflicts with newer local changes and was not adopted{recovery}"
            ));
            return;
        }
        match load_stable_regular_project(&request.path) {
            Ok(project) => {
                if project.song == request.baseline_song
                    && project.variation_history == request.baseline_variation_history
                {
                    let cleanup = if request.scratch {
                        remove_scratch(&request.path, request.scratch_directory.as_deref())
                    } else {
                        self.refresh_project_watch(&request.path);
                        String::new()
                    };
                    if cleanup.is_empty() {
                        self.notify_info("External editor made no project changes");
                    } else {
                        self.notify_warning(format!(
                            "External editor made no project changes{cleanup}"
                        ));
                    }
                    return;
                }
                self.adopt_external_project(project, request.scratch);
                let cleanup = if request.scratch {
                    remove_scratch(&request.path, request.scratch_directory.as_deref())
                } else {
                    self.refresh_project_watch(&request.path);
                    String::new()
                };
                if cleanup.is_empty() {
                    self.notify_success("Project reloaded from external editor");
                } else {
                    self.notify_warning(format!("Project reloaded from external editor{cleanup}"));
                }
            }
            Err(error) => {
                if !request.scratch {
                    self.refresh_project_watch(&request.path);
                }
                let recovery = if request.scratch {
                    format!("; scratch kept at {}", request.path.display())
                } else {
                    String::new()
                };
                self.notify_error(format!("External edit is invalid: {error}{recovery}"));
            }
        }
    }

    pub(crate) fn finish_external_editor_terminal_failure(
        &mut self,
        request: &ExternalEditorRequest,
        error: &str,
    ) {
        let recovery = if request.scratch {
            format!("; scratch kept at {}", request.path.display())
        } else {
            String::new()
        };
        self.notify_error(format!(
            "External editor terminal handoff failed: {error}{recovery}"
        ));
    }

    pub(crate) fn poll_project_hot_reload(&mut self) {
        let Some(path) = self.project_path.clone() else {
            self.project_watch = None;
            return;
        };
        if self
            .project_watch
            .as_ref()
            .is_none_or(|watch| watch.path != path)
        {
            self.refresh_project_watch(&path);
            return;
        }
        let watch = self.project_watch.as_mut().expect("watch initialized");
        if watch.last_poll.elapsed() < PROJECT_WATCH_INTERVAL {
            return;
        }
        watch.last_poll = Instant::now();
        let metadata = observe_project_metadata(&path);
        let metadata_changed = !metadata_matches_observation(metadata, watch.observed);
        if !metadata_changed && watch.last_content_check.elapsed() < PROJECT_CONTENT_VERIFY_INTERVAL
        {
            return;
        }
        watch.last_content_check = Instant::now();
        let observation = observe_project_file(&path);
        if observation == watch.observed {
            watch.blocked_change = None;
            watch.last_reported_invalid = None;
            return;
        }
        if watch.blocked_change == Some(observation) {
            return;
        }
        if matches!(observation, ProjectFileObservation::Present(_)) && self.dirty {
            watch.blocked_change = Some(observation);
            self.notify_warning(
                "External project change blocked by local edits; rewrite it externally to reload",
            );
            return;
        }
        if watch.last_reported_invalid == Some(observation) {
            if matches!(observation, ProjectFileObservation::Present(_)) {
                self.reload_watched_project(path, false);
            }
            return;
        }
        watch.last_reported_invalid = Some(observation);
        match observation {
            ProjectFileObservation::Present(_) => self.reload_watched_project(path, true),
            ProjectFileObservation::Missing => {
                self.notify_warning("Active project file is missing");
            }
            ProjectFileObservation::Unreadable(kind) => {
                self.notify_error(format!("Active project metadata is unreadable: {kind:?}"));
            }
        }
    }

    fn reload_watched_project(&mut self, path: PathBuf, report_error: bool) {
        match load_stable_regular_project(&path) {
            Ok(project) => {
                self.adopt_external_project(project, false);
                self.refresh_project_watch(&path);
                self.notify_success("Hot-reloaded project changes from disk");
            }
            Err(error) if report_error => {
                self.notify_error(format!("External project change is invalid: {error}"));
            }
            Err(_) => {}
        }
    }

    fn adopt_external_project(&mut self, project: crate::persistence::ProjectFile, scratch: bool) {
        self.song = project.song;
        self.variation_history = project.variation_history;
        if !scratch {
            self.clean_song = self.song.clone();
            self.clean_variation_history = self.variation_history.clone();
        }
        self.history.clear();
        self.selection = None;
        self.dialog = None;
        self.variation_history_open = false;
        self.calibration_open = false;
        self.dsp_device_palette_open = false;
        self.pending_ai_proposal = None;
        self.pending_composition_graph = None;
        self.close_focus_capture();
        self.clamp_cursor();
        self.clamp_sequence_cursor();
        self.clamp_clip_cursor();
        self.refresh_dirty();
    }

    pub(crate) fn refresh_project_watch(&mut self, path: &Path) {
        self.project_watch = Some(ProjectWatchState {
            path: path.to_path_buf(),
            observed: observe_project_file(path),
            blocked_change: None,
            last_reported_invalid: None,
            last_poll: Instant::now(),
            last_content_check: Instant::now(),
        });
    }
}

fn observe_project_file(path: &Path) -> ProjectFileObservation {
    for _ in 0..FILE_OBSERVATION_ATTEMPTS {
        match fs::symlink_metadata(path) {
            Ok(metadata) if metadata.file_type().is_file() => {}
            Ok(_) => return ProjectFileObservation::Unreadable(std::io::ErrorKind::InvalidInput),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return ProjectFileObservation::Missing;
            }
            Err(error) => return ProjectFileObservation::Unreadable(error.kind()),
        }
        let mut file = match File::open(path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return ProjectFileObservation::Missing;
            }
            Err(error) => return ProjectFileObservation::Unreadable(error.kind()),
        };
        let before = match file.metadata() {
            Ok(metadata) => metadata,
            Err(error) => return ProjectFileObservation::Unreadable(error.kind()),
        };
        let (content_fingerprint, bytes_read) = match fingerprint_file(&mut file) {
            Ok(result) => result,
            Err(error) => return ProjectFileObservation::Unreadable(error.kind()),
        };
        let after = match file.metadata() {
            Ok(metadata) => metadata,
            Err(error) => return ProjectFileObservation::Unreadable(error.kind()),
        };
        if before.len() != after.len()
            || before.modified().ok() != after.modified().ok()
            || bytes_read != after.len()
        {
            continue;
        }
        return ProjectFileObservation::Present(ProjectFileSignature {
            modified: after.modified().ok(),
            length: after.len(),
            content_fingerprint,
        });
    }
    ProjectFileObservation::Unreadable(std::io::ErrorKind::WouldBlock)
}

fn fingerprint_file(file: &mut File) -> std::io::Result<(u64, u64)> {
    const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
    const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;
    let mut fingerprint = FNV_OFFSET_BASIS;
    let mut bytes_read = 0_u64;
    let mut buffer = [0_u8; 8 * 1024];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            return Ok((fingerprint, bytes_read));
        }
        bytes_read = bytes_read.saturating_add(count as u64);
        for byte in &buffer[..count] {
            fingerprint = (fingerprint ^ u64::from(*byte)).wrapping_mul(FNV_PRIME);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProjectFileMetadataObservation {
    Present {
        modified: Option<SystemTime>,
        length: u64,
    },
    Missing,
    Unreadable(std::io::ErrorKind),
}

fn observe_project_metadata(path: &Path) -> ProjectFileMetadataObservation {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_file() => ProjectFileMetadataObservation::Present {
            modified: metadata.modified().ok(),
            length: metadata.len(),
        },
        Ok(_) => ProjectFileMetadataObservation::Unreadable(std::io::ErrorKind::InvalidInput),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            ProjectFileMetadataObservation::Missing
        }
        Err(error) => ProjectFileMetadataObservation::Unreadable(error.kind()),
    }
}

fn is_regular_project_path(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|metadata| metadata.file_type().is_file())
}

fn load_stable_regular_project(path: &Path) -> Result<crate::persistence::ProjectFile, String> {
    if !is_regular_project_path(path) {
        return Err(format!(
            "project path is missing, symlinked, non-regular, or unreadable: {}",
            path.display()
        ));
    }
    let mut file = open_regular_project(path)
        .map_err(|error| format!("failed to open project {}: {error}", path.display()))?;
    let before = file
        .metadata()
        .map_err(|error| format!("failed to inspect project {}: {error}", path.display()))?;
    if !before.file_type().is_file() || !is_regular_project_path(path) {
        return Err(format!(
            "project path is missing, symlinked, non-regular, or unreadable: {}",
            path.display()
        ));
    }
    let (before_fingerprint, before_bytes) = fingerprint_file(&mut file)
        .map_err(|error| format!("failed to verify project {}: {error}", path.display()))?;
    if before_bytes != before.len() {
        return Err(format!(
            "project changed while it was being read: {}",
            path.display()
        ));
    }
    file.rewind()
        .map_err(|error| format!("failed to rewind project {}: {error}", path.display()))?;
    let project = crate::persistence::load_project_reader(&mut file, path)
        .map_err(|error| error.to_string())?;
    file.rewind()
        .map_err(|error| format!("failed to rewind project {}: {error}", path.display()))?;
    let (after_fingerprint, after_bytes) = fingerprint_file(&mut file)
        .map_err(|error| format!("failed to verify project {}: {error}", path.display()))?;
    let after = file
        .metadata()
        .map_err(|error| format!("failed to reinspect project {}: {error}", path.display()))?;
    if before.len() != after.len()
        || before.modified().ok() != after.modified().ok()
        || before_fingerprint != after_fingerprint
        || after_bytes != after.len()
        || !is_regular_project_path(path)
    {
        return Err(format!(
            "project changed while it was being read: {}",
            path.display()
        ));
    }
    Ok(project)
}

fn metadata_matches_observation(
    metadata: ProjectFileMetadataObservation,
    observation: ProjectFileObservation,
) -> bool {
    match (metadata, observation) {
        (
            ProjectFileMetadataObservation::Present { modified, length },
            ProjectFileObservation::Present(signature),
        ) => modified == signature.modified && length == signature.length,
        (ProjectFileMetadataObservation::Missing, ProjectFileObservation::Missing) => true,
        (
            ProjectFileMetadataObservation::Unreadable(left),
            ProjectFileObservation::Unreadable(right),
        ) => left == right,
        _ => false,
    }
}

struct ScratchProject {
    path: PathBuf,
    directory: PathBuf,
}

fn create_scratch_project(
    project: &crate::persistence::ProjectFile,
) -> Result<ScratchProject, String> {
    project
        .song
        .validate()
        .map_err(|error| format!("scratch project validation failed: {error}"))?;
    project
        .variation_history
        .validate_for_song(&project.song)
        .map_err(|error| format!("scratch variation history validation failed: {error}"))?;
    let mut contents = serde_json::to_vec_pretty(project)
        .map_err(|error| format!("scratch serialization failed: {error}"))?;
    contents.push(b'\n');

    for _ in 0..SCRATCH_CREATE_ATTEMPTS {
        let directory = temporary_scratch_directory();
        match create_private_scratch_directory(&directory) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(format!(
                    "failed to create private scratch directory {}: {error}",
                    directory.display()
                ));
            }
        }
        let path = directory.join("project.trk");
        match write_new_scratch(&path, &contents) {
            Ok(()) => return Ok(ScratchProject { path, directory }),
            Err(error) => {
                let _ = fs::remove_dir(&directory);
                return Err(format!(
                    "failed to create scratch project {}: {error}",
                    path.display()
                ));
            }
        }
    }
    Err("could not allocate a unique scratch project path".to_string())
}

fn create_private_scratch_directory(path: &Path) -> std::io::Result<()> {
    let mut builder = fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(path)
}

fn write_new_scratch(path: &Path, contents: &[u8]) -> std::io::Result<()> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    let result = file.write_all(contents).and_then(|()| file.sync_all());
    if result.is_err() {
        let _ = fs::remove_file(path);
    }
    result
}

fn temporary_scratch_directory() -> PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    std::env::temp_dir().join(format!(
        "trk-external-edit-{}-{timestamp}-{}",
        std::process::id(),
        SCRATCH_SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ))
}

#[cfg(test)]
fn temporary_project_path() -> PathBuf {
    temporary_scratch_directory().with_extension("trk")
}

fn remove_scratch(path: &Path, directory: Option<&Path>) -> String {
    if let Some(directory) = directory {
        if path.parent() != Some(directory) {
            return format!(
                "; scratch cleanup skipped for mismatched owner {}",
                directory.display()
            );
        }
    }
    if let Err(error) = fs::remove_file(path) {
        if error.kind() != std::io::ErrorKind::NotFound {
            return format!(
                "; scratch cleanup failed ({error}); file kept at {}",
                path.display()
            );
        }
    }
    let Some(directory) = directory else {
        return String::new();
    };
    match fs::remove_dir(directory) {
        Ok(()) => String::new(),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => format!(
            "; scratch directory cleanup failed ({error}); directory kept at {}",
            directory.display()
        ),
    }
}

#[cfg(test)]
#[path = "external_editor_tests.rs"]
mod tests;
