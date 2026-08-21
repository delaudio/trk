use super::*;

#[test]
fn sampler_playback_frames_scale_from_source_to_render_rate() {
    let playback = AudioSamplerPlaybackSettings {
        start_frame: Some(10),
        end_frame: Some(20),
        loop_start_frame: Some(12),
        loop_end_frame: Some(18),
        ..AudioSamplerPlaybackSettings::default()
    };

    let scaled = scale_sampler_playback_frames(playback, 2.0);

    assert_eq!(scaled.start_frame, Some(20));
    assert_eq!(scaled.end_frame, Some(40));
    assert_eq!(scaled.loop_start_frame, Some(24));
    assert_eq!(scaled.loop_end_frame, Some(36));
    assert_eq!(scale_sampler_playback_frames(playback, f64::NAN), playback);
    assert_eq!(
        scale_sampler_playback_frames(
            AudioSamplerPlaybackSettings {
                start_frame: Some(usize::MAX),
                ..AudioSamplerPlaybackSettings::default()
            },
            f64::MAX,
        )
        .start_frame,
        Some(usize::MAX)
    );
}
