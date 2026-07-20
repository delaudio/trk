use super::*;

impl App {
    pub(crate) fn execute_command(&mut self) {
        let command = self.command_buffer.trim().to_string();
        self.command_buffer.clear();
        self.close_focus_capture();

        if let Err(error) = command::dispatch(self, &command) {
            self.notify_warning(error.to_string());
        }
    }

    pub(crate) fn execute_typed_command(&mut self, parsed: SalieriCommand) {
        match parsed {
            SalieriCommand::Help => self.open_help(),
            SalieriCommand::Config => self.notify_info(self.config_metadata.summary()),
            SalieriCommand::View(view) => match view {
                ViewCommand::Tracker => self.open_tracker_view(),
                ViewCommand::Patterns => self.open_patterns_view(),
                ViewCommand::Sequence => self.open_sequence_view(),
                ViewCommand::Clips => self.open_clip_launcher_view(),
                ViewCommand::Tracks => self.open_tracks_view(),
                ViewCommand::Sampler => self.open_sampler_view(),
            },
            SalieriCommand::Browse { browser, path } => match browser {
                BrowserCommand::Samples => self.open_sample_browser_view(path),
                BrowserCommand::Projects => self.open_project_browser_view(path),
            },
            SalieriCommand::Focus(target) => match target {
                FocusTarget::SampleBrowser => self.open_sample_browser_view(None),
                FocusTarget::ProjectBrowser => self.open_project_browser_view(None),
                target => self.focus_panel(FocusPanel::from_target(target)),
            },
            SalieriCommand::Layout(command) => self.handle_layout_command(command),
            SalieriCommand::Quit { force: false } => self.request_quit(false),
            SalieriCommand::Quit { force: true } => self.force_quit(),
            SalieriCommand::Write(path) => match path {
                Some(path) => self.save_as(path),
                None => self.save(),
            },
            SalieriCommand::SaveAs(path) => {
                self.save_as(path);
            }
            SalieriCommand::WriteQuit => {
                self.save_and_quit();
            }
            SalieriCommand::SetBpm(value) => {
                self.dispatch_intent(AppIntent::Parameter(ParameterIntent::SetBpm(value)));
                self.notify_success(format!("BPM set to {value}"));
            }
            SalieriCommand::SetLinesPerBeat(value) => {
                self.dispatch_intent(AppIntent::Parameter(ParameterIntent::SetLinesPerBeat(
                    value,
                )));
                self.notify_success(format!("LPB set to {value}"));
            }
            SalieriCommand::Domain {
                domain:
                    domain @ (CommandDomain::Fx
                    | CommandDomain::Fx2
                    | CommandDomain::Cell
                    | CommandDomain::Automation
                    | CommandDomain::ParameterLock
                    | CommandDomain::Mixer
                    | CommandDomain::Dsp
                    | CommandDomain::Ai
                    | CommandDomain::Report
                    | CommandDomain::Graph
                    | CommandDomain::Clip
                    | CommandDomain::Ableton
                    | CommandDomain::Preset
                    | CommandDomain::Workspace
                    | CommandDomain::MidiInput
                    | CommandDomain::Note),
                arguments,
            } => self.handle_typed_domain(domain, &arguments),
            SalieriCommand::Loop(command) => match command {
                LoopCommand::On => {
                    self.loop_pattern = true;
                    self.notify_info("Pattern loop ON");
                }
                LoopCommand::Off => {
                    self.loop_pattern = false;
                    self.notify_info("Pattern loop OFF");
                }
                LoopCommand::Toggle => self.toggle_loop(),
            },
            SalieriCommand::Domain {
                domain: CommandDomain::Midi,
                arguments,
            } => {
                let mut parts = arguments.iter().map(String::as_str);
                match parts.next() {
                    Some("outputs") | Some("settings") | Some("ports") => self.open_midi_settings(),
                    Some("connect") => {
                        if let Some(port_index) =
                            parts.next().and_then(|value| value.parse::<usize>().ok())
                        {
                            self.connect_midi(port_index);
                        } else {
                            self.notify_warning("Usage: :midi connect PORT_INDEX");
                        }
                    }
                    Some("disconnect") => self.disconnect_midi(),
                    Some("panic") => self.panic_midi(),
                    None | Some(_) => {
                        self.notify_warning("Usage: :midi outputs|connect|disconnect|panic")
                    }
                }
            }
            SalieriCommand::Play(command) => match command {
                PlayCommand::Sequence { position } => self.start_sequence_playback_at(position),
                PlayCommand::Pattern => self.start_playback(),
            },
            SalieriCommand::Stop => self.stop_playback(),
            SalieriCommand::Task(command) => self.handle_task_command(command),
            SalieriCommand::Domain {
                domain: CommandDomain::Track,
                arguments,
            } => {
                let mut parts = arguments.iter().map(String::as_str);
                match parts.next() {
                    Some("new") => self.create_track(),
                    Some("duplicate") | Some("dup") => {
                        let track_index = parts
                            .next()
                            .and_then(|value| value.parse::<usize>().ok())
                            .map_or(self.cursor.track, |value| value.saturating_sub(1));
                        self.duplicate_track(track_index);
                    }
                    Some("delete") | Some("del") => {
                        let track_index = parts
                            .next()
                            .and_then(|value| value.parse::<usize>().ok())
                            .map_or(self.cursor.track, |value| value.saturating_sub(1));
                        self.request_delete_track(track_index);
                    }
                    Some("move") | Some("mv") => {
                        let from = parts
                            .next()
                            .and_then(|value| value.parse::<usize>().ok())
                            .map_or(self.cursor.track, |value| value.saturating_sub(1));
                        let to = parts
                            .next()
                            .and_then(|value| value.parse::<usize>().ok())
                            .map(|value| value.saturating_sub(1));
                        if let Some(to) = to {
                            self.move_track(from, to);
                        } else {
                            self.notify_warning("Usage: :track move FROM TO");
                        }
                    }
                    Some("mute") => {
                        let track_index = parts
                            .next()
                            .and_then(|value| value.parse::<usize>().ok())
                            .map_or(self.cursor.track, |value| value.saturating_sub(1));
                        self.toggle_track_mute(track_index);
                    }
                    Some("solo") => {
                        let track_index = parts
                            .next()
                            .and_then(|value| value.parse::<usize>().ok())
                            .map_or(self.cursor.track, |value| value.saturating_sub(1));
                        self.toggle_track_solo(track_index);
                    }
                    Some("rename") => {
                        let values = parts.collect::<Vec<_>>();
                        if let Some((track_index, name)) =
                            parse_optional_numbered_name(&values, self.cursor.track)
                        {
                            self.rename_track(track_index, name);
                        }
                    }
                    Some("channel") | Some("ch") => {
                        let first = parts.next().and_then(|value| value.parse::<u8>().ok());
                        let second = parts.next().and_then(|value| value.parse::<u8>().ok());
                        match (first, second) {
                            (Some(channel), None) => {
                                self.set_track_midi_channel(self.cursor.track, channel);
                            }
                            (Some(track_number), Some(channel)) => {
                                self.set_track_midi_channel(
                                    usize::from(track_number.saturating_sub(1)),
                                    channel,
                                );
                            }
                            _ => {}
                        }
                    }
                    None | Some(_) => self.notify_warning(
                        "Usage: :track new|duplicate|delete|move|mute|solo|rename|channel",
                    ),
                }
            }
            SalieriCommand::Domain {
                domain: CommandDomain::Pattern,
                arguments,
            } => {
                let mut parts = arguments.iter().map(String::as_str);
                match parts.next() {
                    Some("new") => self.create_pattern(),
                    Some("duplicate") | Some("dup") => self.duplicate_current_pattern(),
                    Some("copy") => self.copy_pattern_operation(),
                    Some("paste") => self.paste_pattern_operation(),
                    Some("fill") => self.fill_pattern_operation(),
                    Some("invert") => self.invert_pattern_operation(),
                    Some("expand") => self.expand_pattern_operation(),
                    Some("shrink") => self.shrink_pattern_operation(),
                    Some("duplicate-selection" | "duplicate-region" | "dup-selection") => {
                        self.duplicate_pattern_region_operation();
                    }
                    Some("delete") | Some("del") => self.request_delete_current_pattern(),
                    Some("length") | Some("len") => {
                        if let Some(row_count) =
                            parts.next().and_then(|value| value.parse::<usize>().ok())
                        {
                            self.resize_current_pattern(row_count);
                        }
                    }
                    Some("rename") => {
                        let name = parts.collect::<Vec<_>>().join(" ");
                        self.rename_current_pattern(name);
                    }
                    Some("next") => self.select_pattern(self.pattern_index.saturating_add(1)),
                    Some("prev") => self.select_pattern(self.pattern_index.saturating_sub(1)),
                    Some(value) => {
                        if let Ok(pattern_number) = value.parse::<usize>() {
                            self.select_pattern(pattern_number.saturating_sub(1));
                        } else {
                            self.notify_warning(
                                "Usage: :pattern new|duplicate|copy|paste|fill|invert|expand|shrink|duplicate-selection|delete|length|rename|next|prev",
                            );
                        }
                    }
                    None => {}
                }
            }
            SalieriCommand::Domain {
                domain: CommandDomain::Sequence,
                arguments,
            } => {
                let mut parts = arguments.iter().map(String::as_str);
                match parts.next() {
                    Some("add") => {
                        let pattern_index = parts
                            .next()
                            .and_then(|value| value.parse::<usize>().ok())
                            .map_or(self.pattern_index, |value| value.saturating_sub(1));
                        self.add_sequence_pattern(pattern_index);
                    }
                    Some("remove") | Some("rm") => {
                        if let Some(position) =
                            parts.next().and_then(|value| value.parse::<usize>().ok())
                        {
                            self.remove_sequence_position(position);
                        }
                    }
                    Some("duplicate") | Some("dup") => {
                        if let Some(position) =
                            parts.next().and_then(|value| value.parse::<usize>().ok())
                        {
                            self.duplicate_sequence_position(position);
                        }
                    }
                    Some("set") => {
                        let position = parts.next().and_then(|value| value.parse::<usize>().ok());
                        let pattern_index = parts
                            .next()
                            .and_then(|value| value.parse::<usize>().ok())
                            .map(|value| value.saturating_sub(1));
                        if let (Some(position), Some(pattern_index)) = (position, pattern_index) {
                            self.set_sequence_pattern(position, pattern_index);
                        }
                    }
                    Some("move") | Some("mv") => {
                        let from = parts.next().and_then(|value| value.parse::<usize>().ok());
                        let to = parts.next().and_then(|value| value.parse::<usize>().ok());
                        if let (Some(from), Some(to)) = (from, to) {
                            self.move_sequence_position(from, to);
                        }
                    }
                    None | Some(_) => {
                        self.notify_warning("Usage: :sequence add|remove|duplicate|set|move")
                    }
                }
            }
            SalieriCommand::Domain {
                domain: CommandDomain::Sample,
                arguments,
            } => {
                let mut parts = arguments.iter().map(String::as_str);
                match parts.next() {
                Some("view") | Some("inspect") | Some("load") => {
                    let path = parts.collect::<Vec<_>>().join(" ");
                    if path.is_empty() {
                        self.open_sampler_view();
                    } else {
                        self.load_sampler_view(PathBuf::from(path));
                    }
                }
                Some("browse") | Some("browser") => {
                    let path = parts.collect::<Vec<_>>().join(" ");
                    self.open_sample_browser_view(if path.is_empty() {
                        None
                    } else {
                        Some(PathBuf::from(path))
                    });
                }
                Some("choose") | Some("external") => {
                    let path = parts.collect::<Vec<_>>().join(" ");
                    self.request_sample_browser(if path.is_empty() {
                        None
                    } else {
                        Some(PathBuf::from(path))
                    });
                }
                Some("render-selection") | Some("render-sel") | Some("bounce-selection") => {
                    let values = parts.collect::<Vec<_>>();
                    match parse_sample_render_selection_args(&values) {
                        Some((path, assign_track)) => {
                            self.render_selection_to_sample(path, assign_track);
                        }
                        None => self
                            .notify_warning("Usage: :sample render-selection PATH [--assign TRACK]"),
                    }
                }
                Some("assign") => {
                    let track_index = parts
                        .next()
                        .and_then(parse_track_number)
                        .unwrap_or(self.cursor.track);
                    self.assign_loaded_sample_to_track(track_index);
                }
                Some("replace") | Some("swap") => {
                    let track_index = parts
                        .next()
                        .and_then(parse_track_number)
                        .unwrap_or(self.cursor.track);
                    self.replace_track_sample_with_loaded_sample(track_index);
                }
                Some("unassign") | Some("clear") => {
                    let track_index = parts
                        .next()
                        .and_then(parse_track_number)
                        .unwrap_or(self.cursor.track);
                    self.unassign_sample_from_track(track_index);
                }
                Some("unload") => {
                    self.unload_current_sample();
                }
                Some("cleanup") | Some("prune") => {
                    self.cleanup_unused_sample_references();
                }
                Some("assignments") | Some("assigned") | Some("list") => {
                    self.show_sample_assignments();
                }
                Some("start") => {
                    match parts.next().map(parse_optional_frame_value) {
                        Some(Some(value)) => self.set_loaded_sample_frame_start(value),
                        Some(None) => self.notify_warning("Usage: :sample start FRAME|clear"),
                        None => self.show_loaded_sample_settings(),
                    }
                }
                Some("end") => {
                    match parts.next().map(parse_optional_frame_value) {
                        Some(Some(value)) => self.set_loaded_sample_frame_end(value),
                        Some(None) => self.notify_warning("Usage: :sample end FRAME|clear"),
                        None => self.show_loaded_sample_settings(),
                    }
                }
                Some("loop") => match parts.next() {
                    Some("off") | Some("none") | Some("clear") => {
                        self.set_loaded_sample_loop(SamplePlaybackMode::OneShot, None, None);
                    }
                    Some("on") => {
                        let start = parts.next().and_then(|value| value.parse::<usize>().ok());
                        let end = parts.next().and_then(|value| value.parse::<usize>().ok());
                        self.set_loaded_sample_loop(SamplePlaybackMode::Loop, start, end);
                    }
                    Some(start) => {
                        let start = start.parse::<usize>().ok();
                        let end = parts.next().and_then(|value| value.parse::<usize>().ok());
                        self.set_loaded_sample_loop(SamplePlaybackMode::Loop, start, end);
                    }
                    None => self.show_loaded_sample_settings(),
                },
                Some("envelope") | Some("env") => {
                    let envelope = (
                        parts.next().and_then(|value| value.parse::<f32>().ok()),
                        parts.next().and_then(|value| value.parse::<f32>().ok()),
                        parts.next().and_then(|value| value.parse::<f32>().ok()),
                        parts.next().and_then(|value| value.parse::<f32>().ok()),
                    );
                    if let (Some(attack), Some(decay), Some(sustain), Some(release)) = envelope {
                        self.set_loaded_sample_envelope(SampleEnvelope {
                            attack_seconds: attack,
                            decay_seconds: decay,
                            sustain,
                            release_seconds: release,
                        });
                    } else {
                        self.notify_warning("Usage: :sample envelope ATTACK DECAY SUSTAIN RELEASE");
                    }
                }
                Some("settings") | Some("info") => self.show_loaded_sample_settings(),
                None => self.open_sampler_view(),
                Some(_) => self.notify_warning(
                    "Usage: :sample view PATH | render-selection PATH [--assign TRACK] | assign [TRACK] | start FRAME|clear | end FRAME|clear | loop START END|off | envelope A D S R",
                ),
                }
            }
        }
    }

    pub(crate) fn handle_typed_domain(&mut self, domain: CommandDomain, arguments: &[String]) {
        let values = command_arguments(arguments);
        match domain {
            CommandDomain::Fx => self.handle_fx_command(&values),
            CommandDomain::Fx2 => self.handle_fx2_command(&values),
            CommandDomain::Cell => self.handle_cell_command(&values),
            CommandDomain::Automation => self.handle_automation_command(&values),
            CommandDomain::ParameterLock => self.handle_parameter_lock_command(&values),
            CommandDomain::Mixer => self.handle_mixer_command(&values),
            CommandDomain::Dsp => self.handle_dsp_command(&values),
            CommandDomain::Ai => self.handle_ai_command(&values),
            CommandDomain::Report => self.handle_report_command(&values),
            CommandDomain::Graph => self.handle_composition_graph_command(&values),
            CommandDomain::Clip => self.handle_clip_command(&values),
            CommandDomain::Ableton => self.handle_live_bridge_command(&values),
            CommandDomain::Preset => self.handle_preset_command(&values),
            CommandDomain::Workspace => self.handle_workspace_command(&values),
            CommandDomain::MidiInput => self.handle_midi_input_command(&values),
            CommandDomain::Note => self.handle_note_command(&values),
            _ => unreachable!("domain handled by dedicated executor"),
        }
    }

    fn handle_layout_command(&mut self, command: LayoutCommand) {
        match command {
            LayoutCommand::Select(preset) => {
                self.tracker_layout = TrackerLayoutState::from_preset(match preset {
                    LayoutPresetCommand::Compact => TrackerLayoutPreset::Compact,
                    LayoutPresetCommand::Balanced => TrackerLayoutPreset::Balanced,
                    LayoutPresetCommand::Studio => TrackerLayoutPreset::Studio,
                });
                self.open_tracker_view();
                self.notify_success(format!("Layout set to {:?}", self.tracker_layout.preset));
            }
            LayoutCommand::Fields(fields) => {
                self.tracker_layout.pattern_fields = fields;
                self.open_tracker_view();
                self.notify_success(format!("Pattern fields set to {}", fields.label()));
            }
            LayoutCommand::Toggle(panel) => {
                let panel = layout_panel_id(panel);
                self.tracker_layout.toggle_panel(panel);
                self.open_tracker_view();
                self.notify_info(format!(
                    "Layout panel {panel:?} {}",
                    if self.tracker_layout.panel_visible(panel) {
                        "shown"
                    } else {
                        "hidden"
                    }
                ));
            }
            LayoutCommand::Show(panel) => {
                let panel = layout_panel_id(panel);
                self.tracker_layout.set_panel_visible(panel, true);
                self.open_tracker_view();
                self.notify_info(format!("Layout panel {panel:?} shown"));
            }
            LayoutCommand::Hide(panel) => {
                let panel = layout_panel_id(panel);
                self.tracker_layout.set_panel_visible(panel, false);
                self.open_tracker_view();
                self.notify_info(format!("Layout panel {panel:?} hidden"));
            }
            LayoutCommand::Resize { panel, delta } => {
                let panel = layout_panel_id(panel);
                self.tracker_layout.resize_panel(panel, delta);
                self.open_tracker_view();
                self.notify_info(format!("Layout panel {panel:?} resized by {delta}"));
            }
        }
    }
}

fn layout_panel_id(panel: LayoutPanelCommand) -> ManagedPanelId {
    match panel {
        LayoutPanelCommand::Tracks => ManagedPanelId::Tracks,
        LayoutPanelCommand::Sequence => ManagedPanelId::Sequence,
        LayoutPanelCommand::Inspector => ManagedPanelId::Inspector,
        LayoutPanelCommand::TrackDesk => ManagedPanelId::TrackDesk,
    }
}

fn parse_sample_render_selection_args(values: &[&str]) -> Option<(PathBuf, Option<usize>)> {
    if values.is_empty() {
        return None;
    }
    let mut path_parts = Vec::new();
    let mut assign_track = None;
    let mut index = 0;
    while index < values.len() {
        match values[index] {
            "--assign" | "assign" => {
                index += 1;
                assign_track = values
                    .get(index)
                    .and_then(|value| parse_track_number(value));
            }
            value if value.starts_with("--assign=") => {
                assign_track = parse_track_number(value.trim_start_matches("--assign="));
            }
            value => path_parts.push(value),
        }
        index += 1;
    }
    (!path_parts.is_empty()).then(|| (PathBuf::from(path_parts.join(" ")), assign_track))
}
