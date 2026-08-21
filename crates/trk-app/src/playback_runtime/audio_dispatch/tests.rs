use super::*;
use trk_core::{
    EffectDevice, ParameterId, ParameterLock, ParameterValue, TrackSendLevel,
    NATIVE_GAIN_PARAMETER_ID,
};

#[test]
fn row_graph_resolution_uses_cached_graph_shape_and_preserves_song() {
    let mut song = Song::empty();
    let track = song.tracks[0].id;
    song.mixer.tracks[0]
        .effects
        .push(EffectDevice::gain(9, 1.0));
    song.mixer.tracks[0]
        .sends
        .push(TrackSendLevel { send: 4, gain: 0.1 });
    let pattern = song.current_pattern_mut().expect("pattern");
    for (target, parameter, value) in [
        (
            ParameterLockTarget::TrackEffect { track, device: 9 },
            NATIVE_GAIN_PARAMETER_ID,
            ParameterValue::Float(0.4),
        ),
        (
            ParameterLockTarget::TrackSend { track, send: 4 },
            MIXER_SEND_GAIN_PARAMETER_ID,
            ParameterValue::Float(0.8),
        ),
    ] {
        pattern
            .set_parameter_lock(
                3,
                0,
                ParameterLock {
                    target,
                    parameter: ParameterId::from(parameter),
                    action: ParameterLockAction::Set { value },
                },
            )
            .expect("row lock");
    }

    let mut graph = audio_dsp_graph(&song);
    apply_row_locks_to_dsp_graph(
        &mut graph,
        &song,
        song.current_pattern().expect("pattern"),
        3,
    );

    assert!(matches!(
        graph.track_chains[0].devices[0].kind,
        AudioDspDeviceKind::Gain { gain } if gain == 0.4
    ));
    assert_eq!(graph.track_sends[0].gain, 0.8);
    assert_eq!(song.mixer.tracks[0].sends[0].gain, 0.1);
}

#[test]
fn sample_frame_metadata_scales_with_the_prepared_buffer_rate() {
    let command = RealtimeAudioCommand::TriggerSample {
        track_id: 1,
        sample_id: 7,
        frame: 99,
        gain: 1.0,
        pan: 0.0,
        pitch_ratio: 1.0,
        playback: trk_audio::AudioSamplerPlaybackSettings {
            start_frame: Some(10),
            end_frame: Some(20),
            loop_start_frame: Some(12),
            loop_end_frame: Some(18),
            ..trk_audio::AudioSamplerPlaybackSettings::default()
        },
    };

    let scaled = scale_sample_command_frames(command, &HashMap::from([(7, 2.0)]));

    let RealtimeAudioCommand::TriggerSample {
        frame, playback, ..
    } = scaled
    else {
        panic!("expected trigger");
    };
    assert_eq!(frame, 99);
    assert_eq!(playback.start_frame, Some(20));
    assert_eq!(playback.end_frame, Some(40));
    assert_eq!(playback.loop_start_frame, Some(24));
    assert_eq!(playback.loop_end_frame, Some(36));
}
