use trk_sampler::PreviewBuffer;

use crate::{fixtures::assert_approx_eq, prepare_realtime_sample};

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
