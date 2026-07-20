use super::*;

impl App {
    pub(crate) fn expire_notification(&mut self) {
        if self
            .notification
            .as_ref()
            .is_some_and(|notification| Instant::now() >= notification.expires_at)
        {
            self.notification = None;
        }
    }

    pub(crate) fn tui_notification(&self) -> Option<NotificationView<'_>> {
        self.notification
            .as_ref()
            .map(|notification| NotificationView {
                kind: notification.kind,
                message: notification.message.as_str(),
            })
    }

    pub(crate) fn tui_midi_ports(&self) -> Vec<MidiPortView<'_>> {
        self.midi_ports
            .iter()
            .map(|port| MidiPortView {
                index: port.index,
                name: port.name.as_str(),
            })
            .collect()
    }

    pub(crate) fn tui_midi_status(&self) -> String {
        let mut input_status = self.midi_input_status.as_str();
        let input_active = self.midi_input.is_some()
            || self.midi_record_armed
            || self.midi_clock_follow
            || !matches!(input_status, "MIDI In Disconnected" | "MIDI In No Inputs");
        if self.midi_record_armed && self.midi_clock_follow {
            input_status = "MIDI In Rec+Clock";
        } else if self.midi_record_armed {
            input_status = "MIDI In Rec";
        } else if self.midi_clock_follow {
            input_status = "MIDI In Clock";
        }

        let midi_status = if input_active {
            format!("{} | {}", self.midi_status, input_status)
        } else {
            self.midi_status.clone()
        };
        match self.active_task_status() {
            Some(task) => format!("{midi_status} | Task {task}"),
            None => midi_status,
        }
    }

    pub(crate) fn tui_midi_settings<'a>(
        &'a self,
        ports: &'a [MidiPortView<'a>],
    ) -> Option<MidiSettingsState<'a>> {
        (self.mode == AppMode::MidiSettings).then_some(MidiSettingsState {
            ports,
            selected_port: self.midi_port_cursor.min(ports.len().saturating_sub(1)),
            status: self.midi_status.as_str(),
        })
    }

    pub(crate) fn tui_command_palette_entries(&self) -> Vec<CommandPaletteEntryView<'_>> {
        self.command_palette_results()
            .into_iter()
            .map(|result| CommandPaletteEntryView {
                title: result.action.title,
                category: result.action.category,
                command: result.action.command_label(),
                shortcut: result.action.shortcut,
                disabled_reason: result.disabled_reason,
                recent: self
                    .command_palette_recent
                    .iter()
                    .any(|recent| recent == result.action.id),
            })
            .collect()
    }

    pub(crate) fn tui_command_palette<'a>(
        &'a self,
        entries: &'a [CommandPaletteEntryView<'a>],
    ) -> Option<CommandPaletteViewState<'a>> {
        (self.mode == AppMode::CommandPalette).then_some(CommandPaletteViewState {
            query: self.command_palette_query.as_str(),
            entries,
            selected: self
                .command_palette_selected
                .min(entries.len().saturating_sub(1)),
        })
    }

    pub(crate) fn tui_sampler_view(&self) -> Option<SamplerViewState<'_>> {
        self.sample_view.as_ref().map(|sample| {
            let sample_path = sample.source_path.to_string_lossy();
            let sample_reference = self
                .song
                .samples
                .iter()
                .find(|reference| reference.path == sample_path.as_ref());
            let sample_id = sample_reference.map(|reference| reference.id);
            let playback = sample_reference
                .map(|reference| reference.playback)
                .unwrap_or_default();
            let assigned_tracks = sample_id.map_or_else(Vec::new, |sample_id| {
                self.song
                    .sample_assignments
                    .iter()
                    .filter(|assignment| assignment.sample == sample_id)
                    .filter_map(|assignment| {
                        self.song
                            .tracks
                            .iter()
                            .find(|track| track.id == assignment.track)
                    })
                    .collect::<Vec<_>>()
            });
            let instrument = sample_id.and_then(|sample_id| {
                self.song
                    .instruments
                    .iter()
                    .find(|instrument| instrument.references_sample(sample_id))
            });
            let (waveform_start_bucket, waveform_end_bucket) = self.sample_waveform_window();
            SamplerViewState {
                name: sample.sample.name.as_str(),
                source_path: sample.source_path.to_str().unwrap_or("<non-utf8 path>"),
                overview: &sample.overview,
                gain: sample_reference.map_or(1.0, |reference| reference.gain),
                waveform_start_bucket,
                waveform_end_bucket,
                waveform_zoom: self.sample_waveform_zoom,
                instrument: instrument.map(|instrument| instrument.name.as_str()),
                assigned_track: assigned_tracks.first().map(|track| track.name.as_str()),
                assigned_track_count: assigned_tracks.len(),
                playback_mode: match playback.mode {
                    SamplePlaybackMode::OneShot => "one-shot",
                    SamplePlaybackMode::Loop => "loop",
                },
                start_frame: playback.start_frame,
                end_frame: playback.end_frame,
                loop_start_frame: playback.loop_start_frame,
                loop_end_frame: playback.loop_end_frame,
                envelope: (
                    playback.envelope.attack_seconds,
                    playback.envelope.decay_seconds,
                    playback.envelope.sustain,
                    playback.envelope.release_seconds,
                ),
                selected_envelope: self.sampler_envelope_field,
            }
        })
    }

    pub(crate) fn tui_sample_browser_entries(&self) -> Vec<SampleBrowserEntryView<'_>> {
        self.sample_browser_view
            .as_ref()
            .map(|browser| {
                browser
                    .entries
                    .iter()
                    .map(|entry| SampleBrowserEntryView {
                        name: entry.name.as_str(),
                        kind: entry.kind,
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    pub(crate) fn tui_sample_browser_view<'a>(
        &'a self,
        entries: &'a [SampleBrowserEntryView<'a>],
    ) -> Option<SampleBrowserViewState<'a>> {
        self.sample_browser_view
            .as_ref()
            .map(|browser| SampleBrowserViewState {
                current_dir: browser
                    .current_dir
                    .to_str()
                    .unwrap_or("<non-utf8 directory>"),
                entries,
                selected: browser.cursor,
                preview: browser.preview.as_ref().map(|sample| SamplerViewState {
                    name: sample.sample.name.as_str(),
                    source_path: sample.source_path.to_str().unwrap_or("<non-utf8 path>"),
                    overview: &sample.overview,
                    gain: 1.0,
                    waveform_start_bucket: 0,
                    waveform_end_bucket: sample.overview.buckets.len(),
                    waveform_zoom: 1,
                    instrument: None,
                    assigned_track: None,
                    assigned_track_count: 0,
                    playback_mode: "one-shot",
                    start_frame: None,
                    end_frame: None,
                    loop_start_frame: None,
                    loop_end_frame: None,
                    envelope: (0.0, 0.0, 1.0, 0.0),
                    selected_envelope: SamplerEnvelopeField::Attack,
                }),
                message: browser.message.as_deref(),
            })
    }

    pub(crate) fn tui_project_browser_entries(&self) -> Vec<ProjectBrowserEntryView<'_>> {
        self.project_browser_view
            .as_ref()
            .map(|browser| {
                browser
                    .entries
                    .iter()
                    .map(|entry| ProjectBrowserEntryView {
                        name: entry.name.as_str(),
                        path: entry.path.to_str().unwrap_or("<non-utf8 path>"),
                        kind: entry.kind,
                        detail: entry.detail.as_str(),
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    pub(crate) fn tui_project_browser_view<'a>(
        &'a self,
        entries: &'a [ProjectBrowserEntryView<'a>],
    ) -> Option<ProjectBrowserViewState<'a>> {
        self.project_browser_view
            .as_ref()
            .map(|browser| ProjectBrowserViewState {
                current_dir: browser
                    .current_dir
                    .to_str()
                    .unwrap_or("<non-utf8 directory>"),
                entries,
                selected: browser.cursor,
                message: browser.message.as_deref(),
            })
    }

    pub(crate) fn tui_ai_chat_messages(&self) -> Vec<AiChatMessageView<'_>> {
        self.ai_thread
            .messages
            .iter()
            .map(|message| AiChatMessageView {
                role: match message.role {
                    AiMessageRole::System => AiChatMessageRole::System,
                    AiMessageRole::User => AiChatMessageRole::User,
                    AiMessageRole::Assistant => AiChatMessageRole::Assistant,
                    AiMessageRole::Error => AiChatMessageRole::Error,
                    AiMessageRole::Progress => AiChatMessageRole::Progress,
                },
                text: message.text.as_str(),
            })
            .collect()
    }

    pub(crate) fn tui_ai_status(&self, provider_status: &str) -> String {
        match self.active_task_status() {
            Some(task) => format!("{provider_status} | Task {task}"),
            None => provider_status.to_string(),
        }
    }

    pub(crate) fn tui_ai_chat_view<'a>(
        &'a self,
        messages: &'a [AiChatMessageView<'a>],
        provider: &'a str,
        status: &'a str,
        context: &'a str,
    ) -> Option<AiChatViewState<'a>> {
        (self.mode == AppMode::Ai).then_some(AiChatViewState {
            provider,
            status,
            composer: self.ai_thread.composer.as_str(),
            messages,
            selected_context: context,
        })
    }
}
