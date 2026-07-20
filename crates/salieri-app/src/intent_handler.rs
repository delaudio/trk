use crate::{
    app_effect::{AppEffect, PlaybackEffect},
    app_event::{
        AiIntent, AppEvent, AppIntent, NavigationIntent, ParameterIntent, RequestId, TrackerIntent,
        TransportIntent,
    },
    App,
};

impl App {
    pub(crate) fn dispatch_intent(&mut self, intent: AppIntent) {
        self.dispatch_event(AppEvent::Intent(intent));
    }

    pub(crate) fn apply_intent(&mut self, intent: AppIntent) -> Vec<AppEffect> {
        match intent {
            AppIntent::KeyInput(key) => self.handle_key_action(key),
            AppIntent::Command(command) => self.execute_typed_command(command),
            AppIntent::Tracker(intent) => self.apply_tracker_intent(intent),
            AppIntent::Navigation(intent) => self.apply_navigation_intent(intent),
            AppIntent::Transport(intent) => return self.apply_transport_intent(intent),
            AppIntent::Parameter(intent) => self.apply_parameter_intent(intent),
            AppIntent::Ai(AiIntent::Propose(prompt)) => {
                return self
                    .prepare_ai_proposal_effect(prompt)
                    .into_iter()
                    .collect();
            }
            AppIntent::OpenProject(path) => {
                let request_id = self.allocate_request_id();
                self.pending_project_load = Some(request_id);
                return vec![AppEffect::LoadProject { request_id, path }];
            }
            AppIntent::SaveProject { path, quit_after } => {
                let path = match self.resolve_project_save_path(path) {
                    Ok(path) => path,
                    Err(error) => {
                        self.notify_error(format!("Save failed: {error}"));
                        return Vec::new();
                    }
                };
                return vec![AppEffect::SaveProject {
                    path,
                    song: self.song.clone(),
                    quit_after,
                }];
            }
        }
        Vec::new()
    }

    fn apply_tracker_intent(&mut self, intent: TrackerIntent) {
        match intent {
            TrackerIntent::InsertNote(pitch) => self.insert_note(pitch),
            TrackerIntent::InsertNoteEvent(note) => self.insert_note_event(note),
            TrackerIntent::EnterHexDigit(digit) => self.enter_cell_hex_digit(digit),
            TrackerIntent::ClearCell => self.clear_current_cell(),
        }
    }

    fn apply_navigation_intent(&mut self, intent: NavigationIntent) {
        match intent {
            NavigationIntent::MoveCursor(direction) => self.move_cursor(direction),
            NavigationIntent::PageUp => self.page_cursor_up(),
            NavigationIntent::PageDown => self.page_cursor_down(),
            NavigationIntent::NextTrack => self.next_track(),
            NavigationIntent::PreviousTrack => self.previous_track(),
        }
    }

    fn apply_parameter_intent(&mut self, intent: ParameterIntent) {
        match intent {
            ParameterIntent::SetBpm(value) => self.set_bpm(value),
            ParameterIntent::AdjustBpm(delta) => self.adjust_bpm(delta),
            ParameterIntent::SetLinesPerBeat(value) => self.set_lpb(value),
            ParameterIntent::AdjustLinesPerBeat(delta) => self.adjust_lpb(delta),
        }
    }

    fn apply_transport_intent(&mut self, intent: TransportIntent) -> Vec<AppEffect> {
        match intent {
            TransportIntent::TogglePlayback => {
                if self.is_playing {
                    self.apply_transport_intent(TransportIntent::Stop)
                } else {
                    self.apply_transport_intent(TransportIntent::StartPattern)
                }
            }
            TransportIntent::StartPattern => self.start_pattern_effect(0),
            TransportIntent::StartPatternFromCursor => self.start_pattern_effect(self.cursor.row),
            TransportIntent::StartSequence { position } => self.start_sequence_effect(position),
            TransportIntent::StartSelectedSequence => self
                .selected_sequence_position()
                .map_or_else(Vec::new, |position| self.start_sequence_effect(position)),
            TransportIntent::Stop => {
                self.is_playing = false;
                self.playhead_row = None;
                self.sequence_position = None;
                self.notify_info("Playback stopped");
                vec![AppEffect::Playback(PlaybackEffect::Stop)]
            }
            TransportIntent::ToggleLoop => {
                self.loop_pattern = !self.loop_pattern;
                let state = if self.loop_pattern { "ON" } else { "OFF" };
                self.notify_info(format!("Pattern loop {state}"));
                Vec::new()
            }
            TransportIntent::ConnectMidi { port_index } => {
                self.midi_status = format!("MIDI Connecting {port_index}");
                self.notify_info(format!("Connecting MIDI output {port_index}"));
                vec![AppEffect::Playback(PlaybackEffect::ConnectMidi(port_index))]
            }
            TransportIntent::DisconnectMidi => {
                self.notify_info("Disconnecting MIDI output");
                vec![AppEffect::Playback(PlaybackEffect::DisconnectMidi)]
            }
            TransportIntent::PanicMidi => {
                self.is_playing = false;
                self.playhead_row = None;
                self.sequence_position = None;
                self.notify_warning("MIDI panic sent");
                vec![AppEffect::Playback(PlaybackEffect::PanicMidi)]
            }
        }
    }

    fn start_pattern_effect(&mut self, row: usize) -> Vec<AppEffect> {
        if self.song.pattern(self.pattern_index).is_none() {
            self.notify_warning("No pattern to play");
            return Vec::new();
        }

        self.is_playing = true;
        self.playhead_row = Some(row);
        self.sequence_position = None;
        if row == 0 {
            self.notify_info("Playing pattern from start");
        } else {
            self.notify_info(format!("Playing pattern from row {row:02}"));
        }
        vec![AppEffect::Playback(PlaybackEffect::StartPattern {
            song: self.performance_playback_song(),
            sample_base_dir: self.sample_base_dir(),
            pattern_index: self.pattern_index,
            row,
            loop_pattern: self.loop_pattern,
        })]
    }

    fn start_sequence_effect(&mut self, position: usize) -> Vec<AppEffect> {
        if self.song.sequence.is_empty() {
            self.notify_warning("Sequence is empty");
            return Vec::new();
        }
        if position >= self.song.sequence.len() {
            self.notify_warning("Sequence position out of range");
            return Vec::new();
        }

        if let Some(first_pattern_id) = self.song.sequence.get(position) {
            if let Some(pattern_index) = self
                .song
                .patterns
                .iter()
                .position(|pattern| pattern.id == *first_pattern_id)
            {
                self.pattern_index = pattern_index;
            }
        }
        self.is_playing = true;
        self.playhead_row = Some(0);
        self.sequence_position = Some(position);
        self.notify_info(format!("Playing sequence from {position}"));
        vec![AppEffect::Playback(PlaybackEffect::StartSequence {
            song: self.performance_playback_song(),
            sample_base_dir: self.sample_base_dir(),
            position,
        })]
    }

    fn allocate_request_id(&mut self) -> RequestId {
        let request_id = RequestId::new(self.next_request_id);
        self.next_request_id = self.next_request_id.wrapping_add(1).max(1);
        request_id
    }
}
