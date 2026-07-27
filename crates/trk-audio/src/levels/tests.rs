use crate::{fixtures::assert_approx_eq, *};

#[test]
fn measures_rendered_audio_levels() {
    let audio = RenderedAudio {
        sample_rate: 48_000,
        channels: 2,
        frames: 2,
        data: vec![1.0, 0.5, -1.0, 0.0],
    };

    let levels = measure_levels(&audio);

    assert_eq!(levels.len(), 2);
    assert_approx_eq(levels[0].peak, 1.0);
    assert_approx_eq(levels[0].rms, 1.0);
    assert_approx_eq(levels[1].peak, 0.5);
    assert_approx_eq(levels[1].rms, (0.125_f32).sqrt());
}
