use super::*;
use crate::app_effect::{AppEffect, PlaybackEffect};

impl App {
    pub(crate) fn open_project_browser_view(&mut self, start_dir: Option<PathBuf>) {
        let current_dir = start_dir
            .or_else(|| self.project_browser.start_dir.clone())
            .or_else(|| {
                self.project_path
                    .as_deref()
                    .and_then(Path::parent)
                    .map(Path::to_path_buf)
            })
            .or_else(|| std::env::current_dir().ok())
            .unwrap_or_else(|| PathBuf::from("."));
        let current_dir = if current_dir.is_file() {
            current_dir
                .parent()
                .map_or_else(|| PathBuf::from("."), Path::to_path_buf)
        } else {
            current_dir
        };

        self.project_browser_view = Some(AppProjectBrowserView {
            current_dir,
            entries: Vec::new(),
            cursor: 0,
            message: None,
        });
        self.refresh_project_browser_view();
        self.focus_panel(FocusPanel::ProjectBrowser);
    }

    pub(crate) fn refresh_project_browser_view(&mut self) {
        let Some(browser) = &self.project_browser_view else {
            return;
        };
        let current_dir = browser.current_dir.clone();
        let cursor = browser.cursor;

        match read_project_browser_entries(&current_dir, &self.recent_projects) {
            Ok(entries) => {
                let message = if entries.is_empty() {
                    Some("No .trk projects found".to_string())
                } else {
                    Some("Enter opens a project, Backspace goes to parent".to_string())
                };
                self.project_browser_view = Some(AppProjectBrowserView {
                    current_dir,
                    cursor: cursor.min(entries.len().saturating_sub(1)),
                    entries,
                    message,
                });
            }
            Err(error) => {
                self.project_browser_view = Some(AppProjectBrowserView {
                    current_dir,
                    entries: Vec::new(),
                    cursor: 0,
                    message: Some(format!("Failed to read projects: {error}")),
                });
            }
        }
    }

    pub(crate) fn move_project_browser_cursor(&mut self, delta: isize) {
        let Some(browser) = &mut self.project_browser_view else {
            return;
        };
        if browser.entries.is_empty() {
            return;
        }

        let max = browser.entries.len() - 1;
        browser.cursor = browser.cursor.saturating_add_signed(delta).min(max);
    }

    pub(crate) fn project_browser_parent(&mut self) {
        let Some(browser) = &mut self.project_browser_view else {
            return;
        };
        let Some(parent) = browser.current_dir.parent().map(Path::to_path_buf) else {
            return;
        };
        browser.current_dir = parent;
        browser.cursor = 0;
        self.refresh_project_browser_view();
    }

    pub(crate) fn select_project_browser_entry(&mut self) {
        let Some(browser) = &self.project_browser_view else {
            return;
        };
        let Some(entry) = browser.entries.get(browser.cursor).cloned() else {
            self.notify_warning("No project selected");
            return;
        };

        match entry.kind {
            ProjectBrowserEntryKind::Directory => {
                if let Some(browser) = &mut self.project_browser_view {
                    browser.current_dir = entry.path;
                    browser.cursor = 0;
                }
                self.refresh_project_browser_view();
            }
            ProjectBrowserEntryKind::Project | ProjectBrowserEntryKind::RecentProject => {
                self.request_open_project_file(entry.path);
            }
            ProjectBrowserEntryKind::MissingProject | ProjectBrowserEntryKind::InvalidProject => {
                if let Some(browser) = &mut self.project_browser_view {
                    browser.message = Some(entry.detail);
                }
                self.notify_warning("Project cannot be opened");
            }
        }
    }

    pub(crate) fn request_open_project_file(&mut self, path: PathBuf) {
        if self.dirty {
            let message = format!(
                "Open {} and discard unsaved changes?",
                path.file_name()
                    .map(|name| name.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.display().to_string())
            );
            self.dialog = Some(Dialog::OpenProjectDirty { path, message });
            self.capture_focus(FocusCapture::Dialog, AppMode::Dialog);
            self.notify_warning("Unsaved changes");
        } else {
            self.open_project_file(path);
        }
    }

    pub(crate) fn open_project_file(&mut self, path: PathBuf) {
        self.dispatch_intent(AppIntent::OpenProject(path));
    }

    pub(crate) fn apply_project_load(
        &mut self,
        request_id: RequestId,
        path: PathBuf,
        result: std::result::Result<crate::persistence::ProjectFile, String>,
    ) -> Vec<AppEffect> {
        if self.pending_project_load != Some(request_id) {
            tracing::debug!(
                request_id = request_id.get(),
                path = %path.display(),
                "ignored stale project load result"
            );
            return Vec::new();
        }
        self.pending_project_load = None;

        match result {
            Ok(project) => {
                self.clear_performance_state_for_project_change();
                self.song = project.song;
                self.variation_history = project.variation_history;
                self.clean_song = self.song.clone();
                self.clean_variation_history = self.variation_history.clone();
                self.project_path = Some(path.clone());
                self.refresh_project_watch(&path);
                self.pattern_index = 0;
                self.cursor = Cursor::new();
                self.row_offset = 0;
                self.selection = None;
                self.history.clear();
                self.variation_history_open = false;
                self.variation_history_cursor = 0;
                self.is_playing = false;
                self.playhead_row = None;
                self.sequence_position = None;
                self.sequence_cursor = 0;
                self.clip_scene_cursor = 0;
                self.clip_track_cursor = 0;
                self.active_clip_scene = None;
                self.queued_clip_scene = None;
                self.project_browser_view = None;
                self.clamp_cursor();
                self.clamp_sequence_cursor();
                self.clamp_clip_cursor();
                self.refresh_dirty();
                self.focus_panel(FocusPanel::Tracker);
                self.record_recent_project(path.clone());
                self.notify_success(format!("Project opened: {}", path.display()));
                vec![AppEffect::Playback(PlaybackEffect::Stop)]
            }
            Err(error) => {
                if let Some(browser) = &mut self.project_browser_view {
                    browser.message = Some(format!("Project open failed: {error}"));
                    self.focus_panel(FocusPanel::ProjectBrowser);
                } else {
                    self.focus_panel(FocusPanel::Tracker);
                }
                self.notify_error(format!("Project open failed: {error}"));
                Vec::new()
            }
        }
    }

    pub(crate) fn record_recent_project(&mut self, path: PathBuf) {
        let path = path.canonicalize().unwrap_or(path);
        let key = project_path_key(&path);
        self.recent_projects
            .retain(|candidate| project_path_key(candidate) != key);
        self.recent_projects.insert(0, path);
        self.recent_projects.truncate(self.recent_project_limit);
        if let Err(error) =
            save_recent_projects(self.recent_project_file.as_deref(), &self.recent_projects)
        {
            self.notify_warning(format!("Recent projects not saved: {error}"));
        }
    }
}
