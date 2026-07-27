use super::*;

impl App {
    pub(crate) fn handle_clip_command(&mut self, values: &[&str]) {
        match values {
            [] | ["view"] | ["show"] => self.open_clip_launcher_view(),
            ["scene", "add"] | ["add"] => self.add_clip_scene_from_current_pattern(),
            ["set"] => self.set_selected_clip_to_current_pattern(),
            ["set", scene, track, pattern] => {
                if let (Ok(scene), Ok(track), Ok(pattern)) = (
                    scene.parse::<usize>(),
                    track.parse::<usize>(),
                    pattern.parse::<usize>(),
                ) {
                    self.clip_scene_cursor = scene;
                    self.clip_track_cursor = track.saturating_sub(1);
                    self.pattern_index = pattern.saturating_sub(1);
                    self.set_selected_clip_to_current_pattern();
                } else {
                    self.notify_warning("Usage: :clip set SCENE TRACK PATTERN");
                }
            }
            ["clear"] | ["rm"] => self.clear_selected_clip(),
            ["clear", scene, track] | ["rm", scene, track] => {
                if let (Ok(scene), Ok(track)) = (scene.parse::<usize>(), track.parse::<usize>()) {
                    self.clip_scene_cursor = scene;
                    self.clip_track_cursor = track.saturating_sub(1);
                    self.clear_selected_clip();
                } else {
                    self.notify_warning("Usage: :clip clear SCENE TRACK");
                }
            }
            ["launch"] | ["queue"] | ["launch", "scene"] => self.queue_selected_clip_scene(),
            ["launch", "scene", scene] | ["queue", scene] => {
                if let Ok(scene) = scene.parse::<usize>() {
                    self.clip_scene_cursor = scene;
                    self.queue_selected_clip_scene();
                } else {
                    self.notify_warning("Usage: :clip launch scene SCENE");
                }
            }
            ["commit"] | ["activate"] => self.launch_queued_clip_scene(),
            ["stop"] | ["stop", "all"] => self.stop_clip_launcher(),
            _ => self.notify_warning(
                "Usage: :clip view|add|set [SCENE TRACK PATTERN]|clear [SCENE TRACK]|launch [scene SCENE]|commit|stop",
            ),
        }
    }

    pub(crate) fn previous_clip_scene(&mut self) {
        self.clip_scene_cursor = self.clip_scene_cursor.saturating_sub(1);
        self.notify_info(format!("Clip scene {:02}", self.clip_scene_cursor));
    }

    pub(crate) fn next_clip_scene(&mut self) {
        if self.song.clip_scenes.is_empty() {
            self.notify_warning("No clip scenes");
            return;
        }
        self.clip_scene_cursor = self
            .clip_scene_cursor
            .saturating_add(1)
            .min(self.song.clip_scenes.len().saturating_sub(1));
        self.notify_info(format!("Clip scene {:02}", self.clip_scene_cursor));
    }

    pub(crate) fn previous_clip_track(&mut self) {
        self.clip_track_cursor = self.clip_track_cursor.saturating_sub(1);
        self.notify_info(format!("Clip track {:02}", self.clip_track_cursor + 1));
    }

    pub(crate) fn next_clip_track(&mut self) {
        if self.song.tracks.is_empty() {
            self.notify_warning("No tracks");
            return;
        }
        self.clip_track_cursor = self
            .clip_track_cursor
            .saturating_add(1)
            .min(self.song.tracks.len().saturating_sub(1));
        self.notify_info(format!("Clip track {:02}", self.clip_track_cursor + 1));
    }

    pub(crate) fn add_clip_scene_from_current_pattern(&mut self) {
        let pattern_index = self.pattern_index;
        let name = self
            .song
            .pattern(pattern_index)
            .map_or_else(|| "Scene".to_string(), |pattern| pattern.name.clone());
        let before = self.song.clip_scenes.len();
        self.mutate_song_with(TransactionSpec::new("Add clip scene"), |song, _| {
            let _ = song.create_clip_scene_from_pattern(name, pattern_index);
        });
        if self.song.clip_scenes.len() > before {
            self.clip_scene_cursor = self.song.clip_scenes.len().saturating_sub(1);
            self.notify_success(format!("Clip scene {:02} added", self.clip_scene_cursor));
        }
    }

    pub(crate) fn set_selected_clip_to_current_pattern(&mut self) {
        if self.song.clip_scenes.is_empty() {
            self.notify_warning("No clip scene selected");
            return;
        }
        let scene = self.clip_scene_cursor;
        let track = self.clip_track_cursor;
        let pattern = self.pattern_index;
        let mut next_song = self.song.clone();
        if let Err(error) = next_song.set_clip_slot(scene, track, pattern) {
            self.notify_warning(format!("Clip set failed: {error}"));
            return;
        }
        self.mutate_song_with(TransactionSpec::new("Set clip slot"), |song, _| {
            *song = next_song;
        });
        self.notify_success(format!(
            "Clip scene {scene:02} track {:02} set to pattern {:02}",
            track + 1,
            pattern + 1
        ));
    }

    pub(crate) fn clear_selected_clip(&mut self) {
        if self.song.clip_scenes.is_empty() {
            self.notify_warning("No clip scene selected");
            return;
        }
        let scene = self.clip_scene_cursor;
        let track = self.clip_track_cursor;
        let mut next_song = self.song.clone();
        if let Err(error) = next_song.clear_clip_slot(scene, track) {
            self.notify_warning(format!("Clip clear failed: {error}"));
            return;
        }
        self.mutate_song_with(TransactionSpec::new("Clear clip slot"), |song, _| {
            *song = next_song;
        });
        self.notify_success(format!(
            "Clip scene {scene:02} track {:02} cleared",
            track + 1
        ));
    }

    pub(crate) fn queue_selected_clip_scene(&mut self) {
        if self.song.clip_scenes.is_empty() {
            self.notify_warning("No clip scenes");
            return;
        }
        self.clamp_clip_cursor();
        self.queued_clip_scene = Some(self.clip_scene_cursor);
        self.notify_info(format!(
            "Clip scene {:02} queued for next boundary",
            self.clip_scene_cursor
        ));
    }

    pub(crate) fn launch_queued_clip_scene(&mut self) {
        let Some(scene) = self.queued_clip_scene else {
            self.notify_warning("No queued clip scene");
            return;
        };
        if scene >= self.song.clip_scenes.len() {
            self.queued_clip_scene = None;
            self.notify_warning("Queued clip scene no longer exists");
            return;
        }
        self.active_clip_scene = Some(scene);
        self.queued_clip_scene = None;
        self.is_playing = true;
        self.notify_success(format!("Clip scene {scene:02} active"));
    }

    pub(crate) fn stop_clip_launcher(&mut self) {
        self.active_clip_scene = None;
        self.queued_clip_scene = None;
        self.notify_info("Clip launcher stopped");
    }
}
