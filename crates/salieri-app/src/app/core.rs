use super::*;

impl Default for App {
    fn default() -> Self {
        Self::new(AppConfig::default())
    }
}

impl App {
    pub(crate) fn new(config: AppConfig) -> Self {
        let song = Song::empty();
        let history = UndoHistory::new(config.history.undo_limit);
        let keymap = Keymap::from_config(&config.keymap)
            .expect("application configuration keymap was validated");
        let default_midi_output = config.midi.default_output.trim().to_string();
        let default_midi_input = config.midi.default_input.trim().to_string();
        let mut sample_browser = config.sample_browser.clone();
        if let Some(sample_library) = &config.workspace.sample_library {
            sample_browser.start_dir = Some(sample_library.clone());
        }
        let mut project_browser = config.project_browser.clone();
        if let Some(project_library) = &config.workspace.project_library {
            project_browser.start_dir = Some(project_library.clone());
        }
        let project_library = config
            .workspace
            .project_library
            .clone()
            .or_else(|| config.project_browser.start_dir.clone());
        let recent_project_file = project_browser.recent_file();
        let recent_projects = load_recent_projects(recent_project_file.as_deref());
        let midi_status = if default_midi_output.is_empty() {
            "MIDI Disconnected".to_string()
        } else {
            format!("MIDI Disconnected ({default_midi_output})")
        };
        let mut app = Self {
            dispatcher: AppDispatcher::default(),
            next_request_id: 1,
            pending_project_load: None,
            task_runtime: TaskRuntime::default(),
            keymap,
            clean_song: song.clone(),
            song,
            project_path: None,
            focus: FocusManager::default(),
            pattern_index: 0,
            cursor: Cursor::new(),
            row_offset: 0,
            track_offset: 0,
            mode: AppMode::Normal,
            octave: config.keyboard.default_octave,
            edit_step: config.keyboard.edit_step,
            vim_navigation: config.keyboard.vim_navigation,
            pending_goto_start: false,
            follow_playhead: config.ui.follow_playhead,
            show_line_numbers_hex: config
                .ui
                .row_number_format
                .uses_hex(config.ui.show_line_numbers_hex),
            row_number_offset: config.ui.row_number_base.offset(),
            pattern_divider_interval: config.ui.pattern_divider_interval,
            pattern_highlight_interval: config.ui.pattern_highlight_interval,
            show_pattern_top_info: config.ui.show_pattern_top_info,
            tracker_layout: config.ui.layout.tracker_layout(),
            help_scroll: 0,
            help_tab: HelpTab::Basics,
            command_buffer: String::new(),
            command_palette_query: String::new(),
            command_palette_selected: 0,
            command_palette_recent: Vec::new(),
            clipboard: None,
            selection: None,
            history,
            playback: PlaybackRuntime::spawn(config.midi.log_file.clone()),
            is_playing: false,
            loop_pattern: true,
            playhead_row: None,
            sequence_position: None,
            performance: PerformanceState::default(),
            sequence_cursor: 0,
            clip_scene_cursor: 0,
            clip_track_cursor: 0,
            dsp_rack_target: DspRackTarget::Track,
            dsp_rack_cursor: 0,
            dsp_device_palette_open: false,
            dsp_device_palette_cursor: 0,
            active_clip_scene: None,
            queued_clip_scene: None,
            midi_status,
            midi_ports: Vec::new(),
            midi_port_cursor: 0,
            midi_input_status: "MIDI In Disconnected".to_string(),
            midi_input_ports: Vec::new(),
            midi_input: None,
            midi_record_armed: false,
            midi_clock_follow: false,
            midi_clock_ticks: 0,
            sample_view: None,
            sample_recorder: SampleRecorder::default(),
            sample_waveform_zoom: 1,
            sample_waveform_offset: 0,
            sampler_envelope_field: SamplerEnvelopeField::Attack,
            sample_browser,
            project_library,
            pending_sample_browser: None,
            sample_browser_view: None,
            project_browser,
            recent_project_file,
            recent_projects,
            recent_project_limit: config.workspace.recent_project_limit,
            config_metadata: config.metadata.clone(),
            ai_config: config.ai.clone(),
            ai_session_file: config.ai.session_file.clone(),
            ai_retention_messages: config.ai.retention_messages,
            ai_thread: AiThread::default(),
            ai_guidance: None,
            project_browser_view: None,
            pending_ai_proposal: None,
            pending_composition_graph: None,
            dirty: false,
            should_quit: false,
            dialog: None,
            notification: None,
            last_tick: Instant::now(),
        };
        app.connect_default_midi_output(&default_midi_output);
        app.connect_default_midi_input(&default_midi_input);
        app
    }

    pub(crate) fn from_file(path: &Path, config: AppConfig) -> Result<Self> {
        let song = load_project(path)?;
        let mut app = Self::new(config);
        app.clean_song = song.clone();
        app.midi_clock_follow = song.midi.clock_in || song.midi.transport_in;
        app.song = song;
        app.project_path = Some(path.to_path_buf());
        Ok(app)
    }

    pub(crate) fn sample_base_dir(&self) -> Option<PathBuf> {
        self.project_path
            .as_deref()
            .and_then(Path::parent)
            .map(Path::to_path_buf)
    }
}
