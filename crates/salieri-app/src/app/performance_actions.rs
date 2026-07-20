use super::*;

impl App {
    pub(crate) fn handle_performance_command(&mut self, values: &[&str]) {
        match values {
            ["slot", slot, "track", track, effect, value] => {
                self.configure_performance_slot(slot, Some(track), effect, value)
            }
            ["slot", slot, effect, value] => {
                self.configure_performance_slot(slot, None, effect, value)
            }
            ["punch" | "trigger" | "on", slot] => self.punch_in_performance_slot(slot),
            ["release" | "off", slot] => self.release_performance_slot(slot),
            ["clear"] => {
                self.performance.active.clear();
                self.refresh_performance_playback();
                self.notify_success("Performance punch-ins released");
            }
            ["status"] | [] => self.show_performance_status(),
            _ => self.notify_warning(
                "Usage: :performance slot SLOT [track TRACK] gain|pan|sample-gain VALUE | punch SLOT | release SLOT | status",
            ),
        }
    }

    fn configure_performance_slot(
        &mut self,
        slot: &str,
        track: Option<&&str>,
        effect: &str,
        value: &str,
    ) {
        let Some(slot) = parse_one_based_index(slot) else {
            self.notify_warning("Performance slot must be 1-based");
            return;
        };
        let target_track = match track {
            Some(track) => match parse_track_number(track) {
                Some(track) => Some(track),
                None => {
                    self.notify_warning("Performance track must be 1-based");
                    return;
                }
            },
            None => None,
        };
        let Some(effect) = parse_performance_effect(effect, value) else {
            self.notify_warning(
                "Usage: :performance slot SLOT [track TRACK] gain|pan|sample-gain VALUE",
            );
            return;
        };

        if let Some(existing) = self
            .performance
            .slots
            .iter_mut()
            .find(|candidate| candidate.index == slot)
        {
            existing.target_track = target_track;
            existing.effect = effect;
        } else {
            self.performance.slots.push(PerformanceSlot {
                index: slot,
                target_track,
                effect,
            });
            self.performance.slots.sort_by_key(|slot| slot.index);
        }
        self.notify_success(format!(
            "Performance slot {:02} {}",
            slot + 1,
            format_performance_effect(effect)
        ));
    }

    fn punch_in_performance_slot(&mut self, slot: &str) {
        let Some(slot) = parse_one_based_index(slot) else {
            self.notify_warning("Performance slot must be 1-based");
            return;
        };
        if !self
            .performance
            .slots
            .iter()
            .any(|candidate| candidate.index == slot)
        {
            self.notify_warning(format!(
                "Performance slot {:02} is not configured",
                slot + 1
            ));
            return;
        }
        if !self
            .performance
            .active
            .iter()
            .any(|active| active.slot == slot)
        {
            self.performance
                .active
                .push(ActivePerformancePunchIn { slot });
        }
        self.refresh_performance_playback();
        self.notify_success(format!("Performance slot {:02} punched in", slot + 1));
    }

    fn release_performance_slot(&mut self, slot: &str) {
        let Some(slot) = parse_one_based_index(slot) else {
            self.notify_warning("Performance slot must be 1-based");
            return;
        };
        self.performance.active.retain(|active| active.slot != slot);
        self.refresh_performance_playback();
        self.notify_success(format!("Performance slot {:02} released", slot + 1));
    }

    fn show_performance_status(&mut self) {
        if self.performance.slots.is_empty() {
            self.notify_info("Performance: no slots configured");
            return;
        }
        let active = self
            .performance
            .active
            .iter()
            .map(|active| format!("{:02}", active.slot + 1))
            .collect::<Vec<_>>()
            .join(",");
        self.notify_info(format!(
            "Performance: {} slot(s), active={}",
            self.performance.slots.len(),
            if active.is_empty() { "none" } else { &active }
        ));
    }

    pub(crate) fn performance_playback_song(&self) -> Song {
        let mut song = self.song.clone();
        self.apply_performance_punch_ins(&mut song);
        song
    }

    fn apply_performance_punch_ins(&self, song: &mut Song) {
        for active in &self.performance.active {
            let Some(slot) = self
                .performance
                .slots
                .iter()
                .find(|slot| slot.index == active.slot)
            else {
                continue;
            };
            let track_index = slot.target_track.unwrap_or(self.cursor.track);
            match slot.effect {
                PerformanceEffect::TrackGain(gain) => {
                    let _ = song.set_track_mixer_gain(track_index, gain);
                }
                PerformanceEffect::TrackPan(pan) => {
                    let _ = song.set_track_mixer_pan(track_index, pan);
                }
                PerformanceEffect::SampleGain(gain) => {
                    set_track_primary_sample_gain(song, track_index, gain);
                }
            }
        }
    }

    fn refresh_performance_playback(&mut self) {
        if !self.is_playing {
            return;
        }
        let song = self.performance_playback_song();
        let sample_base_dir = self.sample_base_dir();
        if let Some(position) = self.sequence_position {
            self.playback
                .start_sequence(song, sample_base_dir, position);
        } else {
            self.playback.start_pattern_from(
                song,
                sample_base_dir,
                self.pattern_index,
                self.playhead_row.unwrap_or(self.cursor.row),
                self.loop_pattern,
            );
        }
    }
}

fn parse_performance_effect(effect: &str, value: &str) -> Option<PerformanceEffect> {
    let value = value.parse::<f32>().ok()?;
    match effect {
        "gain" | "volume" | "vol" if mixer_track_gain_descriptor().validate_f32(value) => {
            Some(PerformanceEffect::TrackGain(value))
        }
        "pan" if mixer_track_pan_descriptor().validate_f32(value) => {
            Some(PerformanceEffect::TrackPan(value))
        }
        "sample-gain" | "sample" if sample_gain_descriptor().validate_f32(value) => {
            Some(PerformanceEffect::SampleGain(value))
        }
        _ => None,
    }
}

fn parse_one_based_index(value: &str) -> Option<usize> {
    value.parse::<usize>().ok()?.checked_sub(1)
}

fn format_performance_effect(effect: PerformanceEffect) -> String {
    match effect {
        PerformanceEffect::TrackGain(gain) => format!("gain={gain:.3}"),
        PerformanceEffect::TrackPan(pan) => format!("pan={pan:.3}"),
        PerformanceEffect::SampleGain(gain) => format!("sample-gain={gain:.3}"),
    }
}

fn set_track_primary_sample_gain(song: &mut Song, track_index: usize, gain: f32) {
    let Some(track) = song.tracks.get(track_index) else {
        return;
    };
    let sample_id = song
        .instrument_for_track(track.id)
        .and_then(Instrument::primary_sample)
        .or_else(|| {
            song.sample_assignments
                .iter()
                .find(|assignment| assignment.track == track.id)
                .map(|assignment| assignment.sample)
        });
    let Some(sample_id) = sample_id else {
        return;
    };
    if let Some(sample) = song.sample_for_id_mut(sample_id) {
        sample.gain = gain;
    }
}
