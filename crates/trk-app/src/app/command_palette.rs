use super::*;

impl App {
    pub(crate) fn open_command_palette(&mut self) {
        self.command_palette_query.clear();
        self.command_palette_selected = 0;
        self.capture_focus(FocusCapture::CommandPalette, AppMode::CommandPalette);
    }

    pub(crate) fn close_command_palette(&mut self) {
        self.command_palette_query.clear();
        self.command_palette_selected = 0;
        self.close_focus_capture();
    }

    pub(crate) fn handle_command_palette_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => self.close_command_palette(),
            KeyCode::Enter => self.execute_selected_command_palette_action(),
            KeyCode::Up => self.move_command_palette_selection(-1),
            KeyCode::Down => self.move_command_palette_selection(1),
            KeyCode::PageUp => self.move_command_palette_selection(-10),
            KeyCode::PageDown => self.move_command_palette_selection(10),
            KeyCode::Home => self.command_palette_selected = 0,
            KeyCode::End => {
                self.command_palette_selected =
                    self.command_palette_results().len().saturating_sub(1);
            }
            KeyCode::Backspace => {
                self.command_palette_query.pop();
                self.command_palette_selected = 0;
            }
            KeyCode::Char(value) => {
                self.command_palette_query.push(value);
                self.command_palette_selected = 0;
            }
            _ => {}
        }
    }

    pub(crate) fn move_command_palette_selection(&mut self, delta: isize) {
        let result_count = self.command_palette_results().len();
        if result_count == 0 {
            self.command_palette_selected = 0;
            return;
        }
        self.command_palette_selected = self
            .command_palette_selected
            .saturating_add_signed(delta)
            .min(result_count.saturating_sub(1));
    }

    pub(crate) fn handle_command_palette_mouse_wheel(
        &mut self,
        column: u16,
        row: u16,
        kind: MouseEventKind,
    ) {
        let over_results = self
            .interaction_map
            .hit_test(column, row)
            .is_some_and(|region| {
                matches!(
                    region.id,
                    interaction_region::COMMAND_PALETTE_RESULTS
                        | interaction_region::COMMAND_PALETTE_ENTRY
                )
            });
        if !over_results {
            return;
        }
        let delta = match kind {
            MouseEventKind::ScrollUp => -3,
            MouseEventKind::ScrollDown => 3,
            _ => return,
        };
        self.move_command_palette_selection(delta);
    }

    pub(crate) fn handle_command_palette_mouse_click(&mut self, column: u16, row: u16) {
        let target = self
            .interaction_map
            .hit_test(column, row)
            .filter(|region| region.id == interaction_region::COMMAND_PALETTE_ENTRY)
            .map(|region| region.payload);
        let Some(InteractionPayload::CommandPaletteEntry { index }) = target else {
            return;
        };
        let results = self.command_palette_results();
        let Some(result) = results.get(index).copied() else {
            return;
        };
        self.command_palette_selected = index;
        if let Some(reason) = result.disabled_reason {
            self.notify_warning(format!("{} unavailable: {reason}", result.action.title));
            return;
        }
        self.execute_selected_command_palette_action();
    }

    pub(crate) fn command_palette_results(&self) -> Vec<CommandPaletteMatch> {
        command_palette_results(
            &self.command_palette_query,
            self.command_palette_context(),
            &self.command_palette_recent,
        )
    }

    pub(crate) fn command_palette_context(&self) -> CommandPaletteContext {
        CommandPaletteContext {
            active_view: self.tui_active_view(),
            dirty: self.dirty,
            is_playing: self.is_playing,
            has_selection: self.selection.is_some(),
            has_loaded_sample: self.sample_view.is_some(),
        }
    }

    fn execute_selected_command_palette_action(&mut self) {
        let results = self.command_palette_results();
        let Some(result) = results.get(self.command_palette_selected).copied() else {
            self.notify_warning("No command palette action selected");
            return;
        };
        if let Some(reason) = result.disabled_reason {
            self.notify_warning(format!("{} unavailable: {reason}", result.action.title));
            return;
        }

        self.remember_command_palette_action(result.action.id);
        self.command_palette_query.clear();
        self.command_palette_selected = 0;
        self.close_focus_capture();

        match result.action.kind {
            CommandPaletteActionKind::Execute(command) => {
                if let Err(error) = command::dispatch(self, command) {
                    self.notify_warning(error.to_string());
                }
            }
            CommandPaletteActionKind::Prompt(prefix) => {
                self.command_buffer = prefix.to_string();
                self.capture_command_mode();
            }
            CommandPaletteActionKind::Internal(CommandPaletteInternalAction::ClearSelection) => {
                self.clear_selection_region();
            }
        }
    }

    fn remember_command_palette_action(&mut self, action_id: &str) {
        self.command_palette_recent
            .retain(|recent| recent != action_id);
        self.command_palette_recent.insert(0, action_id.to_string());
        self.command_palette_recent.truncate(8);
    }
}
