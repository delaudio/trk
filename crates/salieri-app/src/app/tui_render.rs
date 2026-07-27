use super::*;
use ratatui::Frame;

impl App {
    pub(crate) fn render_interactions(&self, frame: &mut Frame<'_>) -> InteractionMap {
        let midi_ports = self.tui_midi_ports();
        let midi_settings = self.tui_midi_settings(&midi_ports);
        let notification = self.tui_notification();
        let sample_browser_entries = self.tui_sample_browser_entries();
        let sample_browser = self.tui_sample_browser_view(&sample_browser_entries);
        let project_browser_entries = self.tui_project_browser_entries();
        let project_browser = self.tui_project_browser_view(&project_browser_entries);
        let command_palette_entries = self.tui_command_palette_entries();
        let command_palette = self.tui_command_palette(&command_palette_entries);
        let midi_status = self.tui_midi_status();
        let ai_chat_messages = self.tui_ai_chat_messages();
        let ai_proposal_preview_lines = self.tui_ai_proposal_preview_lines();
        let ai_provider = format!("{} model={}", self.ai_config.provider, self.ai_config.model);
        let ai_provider_status =
            crate::task_integration::format_ai_provider_status(&self.ai_config);
        let ai_status = self.tui_ai_status(&ai_provider_status);
        let ai_context = format!(
            "Context: pattern {:02}, track {:02}, row {:02}",
            self.pattern_index + 1,
            self.cursor.track + 1,
            self.cursor.row
        );
        let ai_chat = self.tui_ai_chat_view(
            &ai_chat_messages,
            &ai_proposal_preview_lines,
            ai_provider.as_str(),
            ai_status.as_str(),
            ai_context.as_str(),
        );
        let active_view = self.tui_active_view();
        let mut tui_cursor = self.cursor;
        let tui_pattern_index = if active_view == TuiView::Clips {
            tui_cursor.track = self.clip_track_cursor;
            self.clip_scene_cursor
        } else {
            self.pattern_index
        };
        let tui_sequence_position = if active_view == TuiView::Clips {
            self.active_clip_scene
        } else {
            self.tui_sequence_position()
        };
        let tui_is_playing = if active_view == TuiView::Clips {
            self.queued_clip_scene.is_some() || self.active_clip_scene.is_some()
        } else {
            self.is_playing
        };

        render_with_interactions(
            frame,
            &self.song,
            TuiState {
                cursor: tui_cursor,
                row_offset: self.row_offset,
                track_offset: self.track_offset,
                pattern_index: tui_pattern_index,
                active_view,
                selection: self.selection_rect(),
                mode_label: self.mode.label(),
                octave: self.octave,
                edit_step: self.edit_step,
                dirty: self.dirty,
                show_line_numbers_hex: self.show_line_numbers_hex,
                row_number_offset: self.row_number_offset,
                pattern_divider_interval: self.pattern_divider_interval,
                pattern_highlight_interval: self.pattern_highlight_interval,
                show_pattern_top_info: self.show_pattern_top_info,
                command_line: self.command_line(),
                notification,
                show_help: self.mode == AppMode::Help,
                help_scroll: self.help_scroll,
                help_tab: self.help_tab,
                is_playing: tui_is_playing,
                loop_pattern: self.loop_pattern,
                playhead_row: self.playhead_row,
                midi_status: midi_status.as_str(),
                sequence_position: tui_sequence_position,
                quit_confirmation: self.quit_confirmation(),
                delete_confirmation: self.delete_confirmation_message(),
                midi_settings,
                command_palette,
                sampler_view: self.tui_sampler_view(),
                dsp_rack: Some(self.tui_dsp_rack_view()),
                sample_browser,
                project_browser,
                ai_chat,
                tracker_layout: self.tracker_layout,
            },
        )
    }
}
