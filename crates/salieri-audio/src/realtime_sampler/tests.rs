use salieri_sampler::PreviewBuffer;

use crate::{
    fixtures::{assert_approx_eq, mono_sample},
    *,
};

#[test]
fn realtime_commands_are_plain_data_messages() {
    let command = RealtimeAudioCommand::TriggerSample {
        track_id: 1,
        sample_id: 1,
        frame: 128,
        gain: 0.5,
        pan: 0.0,
        pitch_ratio: 2.0,
    };

    assert_eq!(
        command,
        RealtimeAudioCommand::TriggerSample {
            track_id: 1,
            sample_id: 1,
            frame: 128,
            gain: 0.5,
            pan: 0.0,
            pitch_ratio: 2.0,
        }
    );
}

#[test]
fn prepares_realtime_samples_for_output_config() {
    let preview = PreviewBuffer {
        sample_rate: 2,
        channels: 1,
        frames: 2,
        data: vec![0.25, 0.75],
    };

    let prepared = prepare_realtime_sample(&preview, 4, 2);

    assert_eq!(prepared.sample_rate, 4);
    assert_eq!(prepared.channels, 2);
    assert_eq!(prepared.frames, 4);
    assert_eq!(prepared.data[0], 0.25);
    assert_eq!(prepared.data[1], 0.25);
    assert_approx_eq(prepared.data[2], 0.5);
    assert_approx_eq(prepared.data[3], 0.5);
}

#[test]
fn slices_preview_buffers_by_frame_window() {
    let preview = PreviewBuffer {
        sample_rate: 48_000,
        channels: 2,
        frames: 4,
        data: vec![0.0, 0.1, 1.0, 1.1, 2.0, 2.1, 3.0, 3.1],
    };

    let sliced = slice_preview_buffer(&preview, Some(1), Some(3));

    assert_eq!(sliced.sample_rate, 48_000);
    assert_eq!(sliced.channels, 2);
    assert_eq!(sliced.frames, 2);
    assert_eq!(sliced.data, vec![1.0, 1.1, 2.0, 2.1]);
}

#[test]
fn applies_preview_envelope_to_each_frame() {
    let preview = PreviewBuffer {
        sample_rate: 4,
        channels: 1,
        frames: 4,
        data: vec![1.0, 1.0, 1.0, 1.0],
    };

    let enveloped = apply_preview_envelope(&preview, 2, 0, 1.0, 2);

    assert_approx_eq(enveloped.data[0], 0.0);
    assert_approx_eq(enveloped.data[1], 0.5);
    assert_approx_eq(enveloped.data[2], 1.0);
    assert_approx_eq(enveloped.data[3], 0.5);
}

#[test]
fn realtime_sampler_renders_triggered_voices() {
    let mut sampler = RealtimeSampler::new(RealtimeSamplerConfig {
        sample_rate: 48_000,
        channels: 1,
        max_voices: 4,
    });
    sampler
        .register_sample(1, mono_sample(vec![0.25, 0.5, 0.75, 1.0]))
        .expect("register sample");

    sampler
        .handle_command(RealtimeAudioCommand::TriggerSample {
            track_id: 1,
            sample_id: 1,
            frame: 1,
            gain: 0.5,
            pan: 0.0,
            pitch_ratio: 2.0,
        })
        .expect("trigger");

    let rendered = sampler.render(4);

    assert_eq!(rendered.data[0], 0.0);
    assert_eq!(rendered.data[1], 0.125);
    assert_eq!(rendered.data[2], 0.375);
    assert_eq!(rendered.data[3], 0.0);
    assert_eq!(sampler.active_voice_count(), 0);
}

#[test]
fn realtime_sampler_applies_dsp_graph() {
    let mut sampler = RealtimeSampler::new(RealtimeSamplerConfig {
        sample_rate: 48_000,
        channels: 1,
        max_voices: 4,
    });
    sampler
        .register_sample(1, mono_sample(vec![1.0]))
        .expect("register sample");
    sampler.set_dsp_graph(DspGraphSpec {
        track_chains: vec![TrackDspChainSpec {
            track_id: 1,
            devices: vec![DspDeviceSpec {
                bypassed: false,
                kind: DspDeviceKind::Gain { gain: 0.5 },
            }],
        }],
        master: vec![DspDeviceSpec {
            bypassed: false,
            kind: DspDeviceKind::Gain { gain: 0.5 },
        }],
    });

    sampler
        .handle_command(RealtimeAudioCommand::TriggerSample {
            track_id: 1,
            sample_id: 1,
            frame: 0,
            gain: 1.0,
            pan: 0.0,
            pitch_ratio: 1.0,
        })
        .expect("trigger");

    let rendered = sampler.render(1);

    assert_approx_eq(rendered.data[0], 0.25);
}

#[test]
fn realtime_and_offline_match_for_native_utility_master_devices() {
    let preview = PreviewBuffer {
        sample_rate: 48_000,
        channels: 2,
        frames: 1,
        data: vec![1.0, 0.0],
    };
    let graph = DspGraphSpec {
        track_chains: Vec::new(),
        master: vec![
            DspDeviceSpec {
                bypassed: false,
                kind: DspDeviceKind::StereoWidth { width: 2.0 },
            },
            DspDeviceSpec {
                bypassed: false,
                kind: DspDeviceKind::PhaseInvert {
                    invert_left: true,
                    invert_right: false,
                },
            },
        ],
    };

    let offline = render_sampler_events_with_dsp(
        &[OfflineSamplerSample {
            sample_id: 1,
            buffer: preview.clone(),
        }],
        &[OfflineSamplerEvent {
            track_id: 1,
            sample_id: 1,
            frame: 0,
            gain: 1.0,
            pan: 0.0,
            pitch_ratio: 1.0,
            velocity: 127,
        }],
        OfflineRenderSpec {
            sample_rate: 48_000,
            channels: 2,
            frames: 1,
        },
        &graph,
    )
    .expect("offline render");

    let mut realtime = RealtimeSampler::new(RealtimeSamplerConfig {
        sample_rate: 48_000,
        channels: 2,
        max_voices: 4,
    });
    realtime
        .register_sample(1, preview)
        .expect("register sample");
    realtime.set_dsp_graph(graph);
    realtime
        .handle_command(RealtimeAudioCommand::TriggerSample {
            track_id: 1,
            sample_id: 1,
            frame: 0,
            gain: 1.0,
            pan: 0.0,
            pitch_ratio: 1.0,
        })
        .expect("trigger");
    let rendered = realtime.render(1);

    assert_eq!(rendered.data.len(), offline.data.len());
    for (actual, expected) in rendered.data.iter().zip(offline.data.iter()) {
        assert_approx_eq(*actual, *expected);
    }
}

#[test]
fn realtime_and_offline_match_for_native_utility_track_devices() {
    let preview = PreviewBuffer {
        sample_rate: 48_000,
        channels: 2,
        frames: 1,
        data: vec![1.0, 0.0],
    };
    let graph = DspGraphSpec {
        track_chains: vec![TrackDspChainSpec {
            track_id: 1,
            devices: vec![
                DspDeviceSpec {
                    bypassed: false,
                    kind: DspDeviceKind::StereoWidth { width: 2.0 },
                },
                DspDeviceSpec {
                    bypassed: false,
                    kind: DspDeviceKind::PhaseInvert {
                        invert_left: true,
                        invert_right: false,
                    },
                },
            ],
        }],
        master: Vec::new(),
    };

    let offline = render_sampler_events_with_dsp(
        &[OfflineSamplerSample {
            sample_id: 1,
            buffer: preview.clone(),
        }],
        &[OfflineSamplerEvent {
            track_id: 1,
            sample_id: 1,
            frame: 0,
            gain: 1.0,
            pan: 0.0,
            pitch_ratio: 1.0,
            velocity: 127,
        }],
        OfflineRenderSpec {
            sample_rate: 48_000,
            channels: 2,
            frames: 1,
        },
        &graph,
    )
    .expect("offline render");

    let mut realtime = RealtimeSampler::new(RealtimeSamplerConfig {
        sample_rate: 48_000,
        channels: 2,
        max_voices: 4,
    });
    realtime
        .register_sample(1, preview)
        .expect("register sample");
    realtime.set_dsp_graph(graph);
    realtime
        .handle_command(RealtimeAudioCommand::TriggerSample {
            track_id: 1,
            sample_id: 1,
            frame: 0,
            gain: 1.0,
            pan: 0.0,
            pitch_ratio: 1.0,
        })
        .expect("trigger");
    let rendered = realtime.render(1);

    assert_eq!(rendered.data.len(), offline.data.len());
    for (actual, expected) in rendered.data.iter().zip(offline.data.iter()) {
        assert_approx_eq(*actual, *expected);
    }
}

#[test]
fn realtime_sampler_can_trigger_at_current_callback_frame() {
    let mut sampler = RealtimeSampler::new(RealtimeSamplerConfig {
        sample_rate: 48_000,
        channels: 1,
        max_voices: 4,
    });
    sampler
        .register_sample(1, mono_sample(vec![0.25, 0.5]))
        .expect("register sample");
    let preroll = sampler.render(8);
    assert_eq!(preroll.data, vec![0.0; 8]);

    sampler
        .handle_command_now(RealtimeAudioCommand::TriggerSample {
            track_id: 1,
            sample_id: 1,
            frame: 0,
            gain: 1.0,
            pan: 0.0,
            pitch_ratio: 1.0,
        })
        .expect("trigger now");

    let rendered = sampler.render(2);
    assert_eq!(rendered.data, vec![0.25, 0.5]);
}

#[test]
fn realtime_sampler_bounds_and_clears_voices() {
    let mut sampler = RealtimeSampler::new(RealtimeSamplerConfig {
        sample_rate: 48_000,
        channels: 1,
        max_voices: 1,
    });
    sampler
        .register_sample(1, mono_sample(vec![1.0, 1.0]))
        .expect("register sample");
    let first_voice = sampler
        .handle_command(RealtimeAudioCommand::TriggerSample {
            track_id: 1,
            sample_id: 1,
            frame: 0,
            gain: 1.0,
            pan: 0.0,
            pitch_ratio: 1.0,
        })
        .expect("trigger first")
        .expect("first voice id");
    let second_voice = sampler
        .handle_command(RealtimeAudioCommand::TriggerSample {
            track_id: 1,
            sample_id: 1,
            frame: 0,
            gain: 1.0,
            pan: 0.0,
            pitch_ratio: 1.0,
        })
        .expect("trigger second")
        .expect("second voice id");

    assert_ne!(first_voice, second_voice);
    assert_eq!(sampler.active_voice_count(), 1);

    sampler
        .handle_command(RealtimeAudioCommand::StopVoice {
            voice_id: second_voice,
            frame: 0,
        })
        .expect("stop voice");
    assert_eq!(sampler.active_voice_count(), 0);

    sampler
        .handle_command(RealtimeAudioCommand::TriggerSample {
            track_id: 1,
            sample_id: 1,
            frame: 0,
            gain: 1.0,
            pan: 0.0,
            pitch_ratio: 1.0,
        })
        .expect("trigger third");
    sampler
        .handle_command(RealtimeAudioCommand::AllNotesOff { frame: 0 })
        .expect("all notes off");
    assert_eq!(sampler.active_voice_count(), 0);
}
