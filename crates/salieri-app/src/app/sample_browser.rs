use super::*;

impl App {
    pub(crate) fn load_sampler_view(&mut self, path: PathBuf) {
        match load_sample_view_data(path) {
            Ok(sample_view) => {
                let name = sample_view.sample.name.clone();
                self.sample_view = Some(sample_view);
                self.sample_waveform_zoom = 1;
                self.sample_waveform_offset = 0;
                self.focus_panel(FocusPanel::Sampler);
                self.notify_success(format!("Sample loaded: {name}"));
            }
            Err(error) => {
                self.focus_panel(FocusPanel::Sampler);
                self.notify_error(format!("Sample load failed: {error}"));
            }
        }
    }

    pub(crate) fn zoom_sample_waveform_in(&mut self) {
        self.set_sample_waveform_zoom(self.sample_waveform_zoom.saturating_mul(2));
    }

    pub(crate) fn zoom_sample_waveform_out(&mut self) {
        self.set_sample_waveform_zoom((self.sample_waveform_zoom / 2).max(1));
    }

    pub(crate) fn set_sample_waveform_zoom(&mut self, zoom: usize) {
        let Some(sample) = &self.sample_view else {
            self.notify_warning("Load a sample before zooming the waveform");
            return;
        };

        let bucket_count = sample.overview.buckets.len();
        if bucket_count == 0 {
            return;
        }

        self.sample_waveform_zoom = zoom.clamp(1, SAMPLE_WAVEFORM_MAX_ZOOM);
        let visible = sample_waveform_visible_buckets(bucket_count, self.sample_waveform_zoom);
        self.sample_waveform_offset = self
            .sample_waveform_offset
            .min(bucket_count.saturating_sub(visible));
        self.notify_info(self.sample_waveform_status());
    }

    pub(crate) fn pan_sample_waveform(&mut self, direction: isize) {
        let Some(sample) = &self.sample_view else {
            return;
        };
        let bucket_count = sample.overview.buckets.len();
        if bucket_count == 0 {
            return;
        }

        let visible = sample_waveform_visible_buckets(bucket_count, self.sample_waveform_zoom);
        let step = (visible / 4).max(1);
        self.sample_waveform_offset = self
            .sample_waveform_offset
            .saturating_add_signed(direction.saturating_mul(step as isize))
            .min(bucket_count.saturating_sub(visible));
        self.notify_info(self.sample_waveform_status());
    }

    pub(crate) fn jump_sample_waveform_start(&mut self) {
        self.sample_waveform_offset = 0;
        self.notify_info(self.sample_waveform_status());
    }

    pub(crate) fn jump_sample_waveform_end(&mut self) {
        let Some(sample) = &self.sample_view else {
            return;
        };
        let bucket_count = sample.overview.buckets.len();
        let visible = sample_waveform_visible_buckets(bucket_count, self.sample_waveform_zoom);
        self.sample_waveform_offset = bucket_count.saturating_sub(visible);
        self.notify_info(self.sample_waveform_status());
    }

    pub(crate) fn sample_waveform_window(&self) -> (usize, usize) {
        let Some(sample) = &self.sample_view else {
            return (0, 0);
        };
        let bucket_count = sample.overview.buckets.len();
        if bucket_count == 0 {
            return (0, 0);
        }
        let visible = sample_waveform_visible_buckets(bucket_count, self.sample_waveform_zoom);
        let start = self
            .sample_waveform_offset
            .min(bucket_count.saturating_sub(visible));
        (start, start.saturating_add(visible).min(bucket_count))
    }

    pub(crate) fn sample_waveform_status(&self) -> String {
        let (start, end) = self.sample_waveform_window();
        let Some(sample) = &self.sample_view else {
            return "Waveform".to_string();
        };
        let bucket_count = sample.overview.buckets.len().max(1);
        let duration = sample.overview.duration_seconds;
        let start_seconds = duration * (start as f32 / bucket_count as f32);
        let end_seconds = duration * (end as f32 / bucket_count as f32);
        format!(
            "Waveform {:.1}-{:.1} ms zoom {}x",
            start_seconds * 1000.0,
            end_seconds * 1000.0,
            self.sample_waveform_zoom
        )
    }

    pub(crate) fn request_sample_browser(&mut self, start_dir: Option<PathBuf>) {
        if self
            .sample_browser
            .chooser_command
            .as_deref()
            .is_none_or(str::is_empty)
        {
            self.focus_panel(FocusPanel::Sampler);
            self.notify_warning("Sample browser not configured");
            return;
        }

        self.pending_sample_browser = Some(SampleBrowserRequest { start_dir });
        self.focus_panel(FocusPanel::Sampler);
        self.notify_info("Opening sample browser");
    }

    pub(crate) fn open_sample_browser_view(&mut self, start_dir: Option<PathBuf>) {
        let current_dir = start_dir
            .or_else(|| self.sample_browser.start_dir.clone())
            .or_else(|| std::env::current_dir().ok())
            .unwrap_or_else(|| PathBuf::from("."));
        let current_dir = if current_dir.is_file() {
            current_dir
                .parent()
                .map_or_else(|| PathBuf::from("."), Path::to_path_buf)
        } else {
            current_dir
        };

        self.sample_browser_view = Some(AppSampleBrowserView {
            current_dir,
            entries: Vec::new(),
            cursor: 0,
            preview: None,
            message: None,
        });
        self.refresh_sample_browser_view();
        self.focus_panel(FocusPanel::SampleBrowser);
    }

    pub(crate) fn refresh_sample_browser_view(&mut self) {
        let Some(browser) = &mut self.sample_browser_view else {
            return;
        };

        match read_sample_browser_entries(&browser.current_dir) {
            Ok(entries) => {
                browser.entries = entries;
                browser.cursor = browser.cursor.min(browser.entries.len().saturating_sub(1));
                browser.message = if browser.entries.is_empty() {
                    Some("Directory is empty".to_string())
                } else {
                    None
                };
            }
            Err(error) => {
                browser.entries.clear();
                browser.cursor = 0;
                browser.preview = None;
                browser.message = Some(format!("Failed to read directory: {error}"));
                return;
            }
        }
        self.update_sample_browser_preview();
    }

    pub(crate) fn update_sample_browser_preview(&mut self) {
        let Some(browser) = &mut self.sample_browser_view else {
            return;
        };
        let selected = browser.entries.get(browser.cursor).cloned();

        match selected {
            Some(entry) if entry.kind == SampleBrowserEntryKind::SupportedSample => {
                match load_sample_view_data(entry.path) {
                    Ok(preview) => {
                        browser.preview = Some(preview);
                        browser.message = None;
                    }
                    Err(error) => {
                        browser.preview = None;
                        browser.message = Some(format!("Sample preview failed: {error}"));
                    }
                }
            }
            Some(entry) if entry.kind == SampleBrowserEntryKind::Directory => {
                browser.preview = None;
                browser.message = Some("Press Enter to open directory".to_string());
            }
            Some(_) => {
                browser.preview = None;
                browser.message = Some("Unsupported file type".to_string());
            }
            None => {
                browser.preview = None;
                browser.message = Some("No files".to_string());
            }
        }
    }

    pub(crate) fn move_sample_browser_cursor(&mut self, delta: isize) {
        let Some(browser) = &mut self.sample_browser_view else {
            return;
        };
        if browser.entries.is_empty() {
            return;
        }

        let max = browser.entries.len() - 1;
        browser.cursor = browser.cursor.saturating_add_signed(delta).min(max);
        self.update_sample_browser_preview();
    }

    pub(crate) fn sample_browser_parent(&mut self) {
        let Some(browser) = &mut self.sample_browser_view else {
            return;
        };
        let Some(parent) = browser.current_dir.parent().map(Path::to_path_buf) else {
            return;
        };
        browser.current_dir = parent;
        browser.cursor = 0;
        self.refresh_sample_browser_view();
    }

    pub(crate) fn select_sample_browser_entry(&mut self) {
        let Some(browser) = &self.sample_browser_view else {
            return;
        };
        let Some(entry) = browser.entries.get(browser.cursor).cloned() else {
            self.notify_warning("No sample selected");
            return;
        };

        match entry.kind {
            SampleBrowserEntryKind::Directory => {
                if let Some(browser) = &mut self.sample_browser_view {
                    browser.current_dir = entry.path;
                    browser.cursor = 0;
                }
                self.refresh_sample_browser_view();
            }
            SampleBrowserEntryKind::SupportedSample => self.load_sampler_view(entry.path),
            SampleBrowserEntryKind::UnsupportedFile => {
                self.notify_warning("Unsupported sample file");
                self.update_sample_browser_preview();
            }
        }
    }

    pub(crate) fn take_sample_browser_request(
        &mut self,
    ) -> Option<(SampleBrowserConfig, SampleBrowserRequest)> {
        self.pending_sample_browser
            .take()
            .map(|request| (self.sample_browser.clone(), request))
    }

    pub(crate) fn finish_sample_browser(&mut self, result: Result<Option<PathBuf>>) {
        self.dispatch_event(AppEvent::Runtime(RuntimeEvent::SampleBrowserFinished(
            result.map_err(|error| error.to_string()),
        )));
    }

    pub(crate) fn apply_sample_browser_result(
        &mut self,
        result: std::result::Result<Option<PathBuf>, String>,
    ) {
        match result {
            Ok(Some(path)) => self.load_sampler_view(path),
            Ok(None) => {
                self.focus_panel(FocusPanel::Sampler);
                self.notify_info("Sample browser closed");
            }
            Err(error) => {
                self.focus_panel(FocusPanel::Sampler);
                self.notify_error(format!("Sample browser failed: {error}"));
            }
        }
    }
}
