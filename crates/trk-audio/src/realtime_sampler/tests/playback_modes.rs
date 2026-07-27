use super::*;

#[test]
fn realtime_and_offline_match_for_sustained_pingpong_loop() {
    let playback = AudioSamplerPlaybackSettings {
        mode: AudioSamplerPlaybackMode::PingPongLoop,
        loop_start_frame: Some(1),
        loop_end_frame: Some(4),
    };
    let sample = mono_sample(vec![0.0, 1.0, 2.0, 3.0]);
    let mut realtime = RealtimeSampler::new(RealtimeSamplerConfig {
        sample_rate: 48_000,
        channels: 1,
        max_voices: 4,
    });
    realtime
        .register_sample(7, sample.clone())
        .expect("register");
    realtime
        .handle_command(RealtimeAudioCommand::TriggerSample {
            track_id: 1,
            sample_id: 7,
            frame: 0,
            gain: 1.0,
            pan: 0.0,
            pitch_ratio: 1.0,
            playback,
        })
        .expect("trigger");

    let realtime_rendered = realtime.render(8);
    let offline_rendered = render_sampler_events(
        &[OfflineSamplerSample {
            sample_id: 7,
            buffer: sample,
        }],
        &[OfflineSamplerEvent {
            track_id: 1,
            sample_id: 7,
            frame: 0,
            gain: 1.0,
            pan: 0.0,
            pitch_ratio: 1.0,
            velocity: 127,
            playback,
        }],
        OfflineRenderSpec {
            sample_rate: 48_000,
            channels: 1,
            frames: 8,
        },
    )
    .expect("offline");

    assert_eq!(realtime_rendered.data, offline_rendered.data);
    assert_eq!(realtime.active_voice_count(), 1);
}
