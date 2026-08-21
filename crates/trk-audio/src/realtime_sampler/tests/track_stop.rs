use crate::{fixtures::mono_sample, *};

#[test]
fn realtime_sampler_stops_only_voices_on_muted_track() {
    let mut sampler = RealtimeSampler::new(RealtimeSamplerConfig {
        sample_rate: 48_000,
        channels: 1,
        max_voices: 4,
    });
    sampler
        .register_sample(1, mono_sample(vec![1.0, 1.0]))
        .expect("register sample");
    for track_id in [1, 2] {
        sampler
            .handle_command(RealtimeAudioCommand::TriggerSample {
                track_id,
                sample_id: 1,
                frame: 0,
                gain: 1.0,
                pan: 0.0,
                pitch_ratio: 1.0,
                playback: AudioSamplerPlaybackSettings::default(),
            })
            .expect("trigger voice");
    }

    sampler
        .handle_command(RealtimeAudioCommand::StopTrack { track_id: 1 })
        .expect("stop track");

    assert_eq!(sampler.active_voice_count(), 1);
}
