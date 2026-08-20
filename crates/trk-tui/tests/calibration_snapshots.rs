use trk_tui::{CalibrationViewState, TerminalColorMode};

#[allow(dead_code)]
mod support;
use support::{assert_snapshot, render_calibration_snapshot};

#[test]
fn snapshots_live_dsp_calibration_overlay() {
    assert_snapshot(
        "dsp-calibration-live",
        render_calibration_snapshot(CalibrationViewState {
            color_mode: TerminalColorMode::TrueColor,
            selected: 4,
            track_name: Some("Lead Synth"),
            master_gain: 1.2,
            track_gain: 0.8,
            low_gain: 1.1,
            mid_gain: 0.9,
            high_gain: 1.4,
            gate_threshold: 0.08,
            meter_decay: 0.35,
            auto_gain: true,
            meter_low: 0.25,
            meter_mid: 0.5,
            meter_high: 0.75,
            meter_rms: 0.42,
            meter_peak: 0.88,
        }),
    );
}
