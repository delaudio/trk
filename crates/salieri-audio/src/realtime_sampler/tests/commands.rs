use crate::*;

#[test]
fn realtime_commands_are_plain_data_messages() {
    let command = RealtimeAudioCommand::TriggerSample {
        track_id: 1,
        sample_id: 1,
        frame: 128,
        gain: 0.5,
        pan: 0.0,
        pitch_ratio: 2.0,
        playback: AudioSamplerPlaybackSettings::default(),
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
            playback: AudioSamplerPlaybackSettings::default(),
        }
    );
}
