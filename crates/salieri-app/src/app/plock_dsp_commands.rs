use super::domain_commands::{parameter_lock_edit, ParameterLockEdit};
use super::*;

impl App {
    pub(super) fn parse_dsp_parameter_lock_edit(
        &self,
        values: &[&str],
    ) -> Option<ParameterLockEdit> {
        match values {
            ["dsp", "track", "gain", action] => {
                let track = self.song.tracks.get(self.cursor.track)?;
                parameter_lock_edit(
                    ParameterLockTarget::TrackEffect {
                        track: track.id,
                        device: 1,
                    },
                    NATIVE_GAIN_PARAMETER_ID,
                    native_gain_descriptor(),
                    action,
                )
            }
            ["dsp", "track", "pan", action] => {
                let track = self.song.tracks.get(self.cursor.track)?;
                parameter_lock_edit(
                    ParameterLockTarget::TrackEffect {
                        track: track.id,
                        device: 2,
                    },
                    NATIVE_PAN_PARAMETER_ID,
                    native_pan_descriptor(),
                    action,
                )
            }
            ["dsp", "track", "balance", action] | ["dsp", "track", "bal", action] => {
                let track = self.song.tracks.get(self.cursor.track)?;
                parameter_lock_edit(
                    ParameterLockTarget::TrackEffect {
                        track: track.id,
                        device: 3,
                    },
                    NATIVE_BALANCE_PARAMETER_ID,
                    native_balance_descriptor(),
                    action,
                )
            }
            ["dsp", "track", "width", action] | ["dsp", "track", "stereo-width", action] => {
                let track = self.song.tracks.get(self.cursor.track)?;
                parameter_lock_edit(
                    ParameterLockTarget::TrackEffect {
                        track: track.id,
                        device: 4,
                    },
                    NATIVE_WIDTH_PARAMETER_ID,
                    native_width_descriptor(),
                    action,
                )
            }
            ["dsp", "track", "phase-left", action] => {
                let track = self.song.tracks.get(self.cursor.track)?;
                parameter_lock_edit(
                    ParameterLockTarget::TrackEffect {
                        track: track.id,
                        device: 5,
                    },
                    NATIVE_PHASE_INVERT_LEFT_PARAMETER_ID,
                    native_phase_invert_left_descriptor(),
                    action,
                )
            }
            ["dsp", "track", "phase-right", action] => {
                let track = self.song.tracks.get(self.cursor.track)?;
                parameter_lock_edit(
                    ParameterLockTarget::TrackEffect {
                        track: track.id,
                        device: 5,
                    },
                    NATIVE_PHASE_INVERT_RIGHT_PARAMETER_ID,
                    native_phase_invert_right_descriptor(),
                    action,
                )
            }
            ["dsp", "track", "filter-mode", action] => self.track_effect_lock(
                6,
                NATIVE_FILTER_MODE_PARAMETER_ID,
                native_filter_mode_descriptor(),
                action,
            ),
            ["dsp", "track", "filter-cutoff" | "filter-cutoff-hz", action] => self
                .track_effect_lock(
                    6,
                    NATIVE_FILTER_CUTOFF_PARAMETER_ID,
                    native_filter_cutoff_descriptor(),
                    action,
                ),
            ["dsp", "track", "filter-resonance" | "filter-res", action] => self.track_effect_lock(
                6,
                NATIVE_FILTER_RESONANCE_PARAMETER_ID,
                native_filter_resonance_descriptor(),
                action,
            ),
            ["dsp", "track", "filter-drive", action] => self.track_effect_lock(
                6,
                NATIVE_FILTER_DRIVE_PARAMETER_ID,
                native_filter_drive_descriptor(),
                action,
            ),
            ["dsp", "track", "filter-mix", action] => self.track_effect_lock(
                6,
                NATIVE_FILTER_MIX_PARAMETER_ID,
                native_filter_mix_descriptor(),
                action,
            ),
            ["dsp", "track", "delay-sync", action] => self.track_effect_lock(
                7,
                NATIVE_DELAY_SYNC_PARAMETER_ID,
                native_delay_sync_descriptor(),
                action,
            ),
            ["dsp", "track", "delay-left" | "delay-time-left", action] => self.track_effect_lock(
                7,
                NATIVE_DELAY_TIME_LEFT_PARAMETER_ID,
                native_delay_time_left_descriptor(),
                action,
            ),
            ["dsp", "track", "delay-right" | "delay-time-right", action] => self.track_effect_lock(
                7,
                NATIVE_DELAY_TIME_RIGHT_PARAMETER_ID,
                native_delay_time_right_descriptor(),
                action,
            ),
            ["dsp", "track", "delay-feedback" | "delay-fb", action] => self.track_effect_lock(
                7,
                NATIVE_DELAY_FEEDBACK_PARAMETER_ID,
                native_delay_feedback_descriptor(),
                action,
            ),
            ["dsp", "track", "delay-ping" | "delay-ping-pong", action] => self.track_effect_lock(
                7,
                NATIVE_DELAY_PING_PONG_PARAMETER_ID,
                native_delay_ping_pong_descriptor(),
                action,
            ),
            ["dsp", "track", "delay-low-cut", action] => self.track_effect_lock(
                7,
                NATIVE_DELAY_FILTER_LOW_CUT_PARAMETER_ID,
                native_delay_filter_low_cut_descriptor(),
                action,
            ),
            ["dsp", "track", "delay-high-cut", action] => self.track_effect_lock(
                7,
                NATIVE_DELAY_FILTER_HIGH_CUT_PARAMETER_ID,
                native_delay_filter_high_cut_descriptor(),
                action,
            ),
            ["dsp", "track", "delay-mix", action] => self.track_effect_lock(
                7,
                NATIVE_DELAY_MIX_PARAMETER_ID,
                native_delay_mix_descriptor(),
                action,
            ),
            ["dsp", "track", "delay-output" | "delay-out", action] => self.track_effect_lock(
                7,
                NATIVE_DELAY_OUTPUT_PARAMETER_ID,
                native_delay_output_descriptor(),
                action,
            ),
            ["dsp", "track", parameter, action] if reverb_lock_descriptor(parameter).is_some() => {
                let (parameter, descriptor) = reverb_lock_descriptor(parameter)?;
                self.track_effect_lock(8, parameter, descriptor, action)
            }
            ["dsp", "master", "gain", action] => self.master_effect_lock(
                1,
                NATIVE_GAIN_PARAMETER_ID,
                native_gain_descriptor(),
                action,
            ),
            ["dsp", "master", "pan", action] => {
                self.master_effect_lock(2, NATIVE_PAN_PARAMETER_ID, native_pan_descriptor(), action)
            }
            ["dsp", "master", "balance", action] | ["dsp", "master", "bal", action] => self
                .master_effect_lock(
                    3,
                    NATIVE_BALANCE_PARAMETER_ID,
                    native_balance_descriptor(),
                    action,
                ),
            ["dsp", "master", "width", action] | ["dsp", "master", "stereo-width", action] => self
                .master_effect_lock(
                    4,
                    NATIVE_WIDTH_PARAMETER_ID,
                    native_width_descriptor(),
                    action,
                ),
            ["dsp", "master", "phase-left", action] => self.master_effect_lock(
                5,
                NATIVE_PHASE_INVERT_LEFT_PARAMETER_ID,
                native_phase_invert_left_descriptor(),
                action,
            ),
            ["dsp", "master", "phase-right", action] => self.master_effect_lock(
                5,
                NATIVE_PHASE_INVERT_RIGHT_PARAMETER_ID,
                native_phase_invert_right_descriptor(),
                action,
            ),
            ["dsp", "master", "filter-mode", action] => self.master_effect_lock(
                6,
                NATIVE_FILTER_MODE_PARAMETER_ID,
                native_filter_mode_descriptor(),
                action,
            ),
            ["dsp", "master", "filter-cutoff" | "filter-cutoff-hz", action] => self
                .master_effect_lock(
                    6,
                    NATIVE_FILTER_CUTOFF_PARAMETER_ID,
                    native_filter_cutoff_descriptor(),
                    action,
                ),
            ["dsp", "master", "filter-resonance" | "filter-res", action] => self
                .master_effect_lock(
                    6,
                    NATIVE_FILTER_RESONANCE_PARAMETER_ID,
                    native_filter_resonance_descriptor(),
                    action,
                ),
            ["dsp", "master", "filter-drive", action] => self.master_effect_lock(
                6,
                NATIVE_FILTER_DRIVE_PARAMETER_ID,
                native_filter_drive_descriptor(),
                action,
            ),
            ["dsp", "master", "filter-mix", action] => self.master_effect_lock(
                6,
                NATIVE_FILTER_MIX_PARAMETER_ID,
                native_filter_mix_descriptor(),
                action,
            ),
            ["dsp", "master", "delay-sync", action] => self.master_effect_lock(
                7,
                NATIVE_DELAY_SYNC_PARAMETER_ID,
                native_delay_sync_descriptor(),
                action,
            ),
            ["dsp", "master", "delay-left" | "delay-time-left", action] => self.master_effect_lock(
                7,
                NATIVE_DELAY_TIME_LEFT_PARAMETER_ID,
                native_delay_time_left_descriptor(),
                action,
            ),
            ["dsp", "master", "delay-right" | "delay-time-right", action] => self
                .master_effect_lock(
                    7,
                    NATIVE_DELAY_TIME_RIGHT_PARAMETER_ID,
                    native_delay_time_right_descriptor(),
                    action,
                ),
            ["dsp", "master", "delay-feedback" | "delay-fb", action] => self.master_effect_lock(
                7,
                NATIVE_DELAY_FEEDBACK_PARAMETER_ID,
                native_delay_feedback_descriptor(),
                action,
            ),
            ["dsp", "master", "delay-ping" | "delay-ping-pong", action] => self.master_effect_lock(
                7,
                NATIVE_DELAY_PING_PONG_PARAMETER_ID,
                native_delay_ping_pong_descriptor(),
                action,
            ),
            ["dsp", "master", "delay-low-cut", action] => self.master_effect_lock(
                7,
                NATIVE_DELAY_FILTER_LOW_CUT_PARAMETER_ID,
                native_delay_filter_low_cut_descriptor(),
                action,
            ),
            ["dsp", "master", "delay-high-cut", action] => self.master_effect_lock(
                7,
                NATIVE_DELAY_FILTER_HIGH_CUT_PARAMETER_ID,
                native_delay_filter_high_cut_descriptor(),
                action,
            ),
            ["dsp", "master", "delay-mix", action] => self.master_effect_lock(
                7,
                NATIVE_DELAY_MIX_PARAMETER_ID,
                native_delay_mix_descriptor(),
                action,
            ),
            ["dsp", "master", "delay-output" | "delay-out", action] => self.master_effect_lock(
                7,
                NATIVE_DELAY_OUTPUT_PARAMETER_ID,
                native_delay_output_descriptor(),
                action,
            ),
            ["dsp", "master", parameter, action] if reverb_lock_descriptor(parameter).is_some() => {
                let (parameter, descriptor) = reverb_lock_descriptor(parameter)?;
                self.master_effect_lock(8, parameter, descriptor, action)
            }
            _ => None,
        }
    }

    fn track_effect_lock(
        &self,
        device: u32,
        parameter: &str,
        descriptor: ParameterDescriptor,
        action: &str,
    ) -> Option<ParameterLockEdit> {
        let track = self.song.tracks.get(self.cursor.track)?;
        parameter_lock_edit(
            ParameterLockTarget::TrackEffect {
                track: track.id,
                device,
            },
            parameter,
            descriptor,
            action,
        )
    }

    fn master_effect_lock(
        &self,
        device: u32,
        parameter: &str,
        descriptor: ParameterDescriptor,
        action: &str,
    ) -> Option<ParameterLockEdit> {
        parameter_lock_edit(
            ParameterLockTarget::MasterEffect { device },
            parameter,
            descriptor,
            action,
        )
    }
}

fn reverb_lock_descriptor(parameter: &str) -> Option<(&'static str, ParameterDescriptor)> {
    match parameter {
        "reverb-size" => Some((
            NATIVE_REVERB_SIZE_PARAMETER_ID,
            native_reverb_size_descriptor(),
        )),
        "reverb-predelay" | "reverb-pre" => Some((
            NATIVE_REVERB_PREDELAY_PARAMETER_ID,
            native_reverb_predelay_descriptor(),
        )),
        "reverb-decay" => Some((
            NATIVE_REVERB_DECAY_PARAMETER_ID,
            native_reverb_decay_descriptor(),
        )),
        "reverb-damping" | "reverb-damp" => Some((
            NATIVE_REVERB_DAMPING_PARAMETER_ID,
            native_reverb_damping_descriptor(),
        )),
        "reverb-low-cut" => Some((
            NATIVE_REVERB_LOW_CUT_PARAMETER_ID,
            native_reverb_low_cut_descriptor(),
        )),
        "reverb-high-cut" => Some((
            NATIVE_REVERB_HIGH_CUT_PARAMETER_ID,
            native_reverb_high_cut_descriptor(),
        )),
        "reverb-diffusion" | "reverb-diff" => Some((
            NATIVE_REVERB_DIFFUSION_PARAMETER_ID,
            native_reverb_diffusion_descriptor(),
        )),
        "reverb-width" => Some((
            NATIVE_REVERB_WIDTH_PARAMETER_ID,
            native_reverb_width_descriptor(),
        )),
        "reverb-early" | "reverb-early-reflections" => Some((
            NATIVE_REVERB_EARLY_REFLECTIONS_PARAMETER_ID,
            native_reverb_early_reflections_descriptor(),
        )),
        "reverb-mix" => Some((
            NATIVE_REVERB_MIX_PARAMETER_ID,
            native_reverb_mix_descriptor(),
        )),
        "reverb-output" | "reverb-out" => Some((
            NATIVE_REVERB_OUTPUT_PARAMETER_ID,
            native_reverb_output_descriptor(),
        )),
        _ => None,
    }
}
