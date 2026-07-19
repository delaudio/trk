use super::*;
use ratatui::{backend::TestBackend, Terminal};
use salieri_core::Song;

pub(super) fn test_waveform(buckets: Vec<salieri_sampler::WaveformBucket>) -> WaveformOverview {
    WaveformOverview {
        sample_rate: 44_100,
        channels: 1,
        frames: 44_100,
        duration_seconds: 1.0,
        buckets,
    }
}

pub(super) fn long_sequence_song(count: usize) -> Song {
    let mut song = Song::empty();
    song.sequence.clear();
    for index in 0..count {
        let pattern_id = if index == 0 {
            song.patterns[0].id
        } else {
            song.create_pattern(64)
        };
        song.rename_pattern(index, format!("Pattern {:02}", index + 1))
            .expect("rename pattern");
        song.sequence.push(pattern_id);
    }
    song
}

pub(super) fn long_track_song(count: usize) -> Song {
    let mut song = Song::empty();
    while song.tracks.len() < count {
        song.create_track();
    }
    for index in 0..song.tracks.len() {
        song.rename_track(index, format!("Track {:02}", index + 1))
            .expect("rename track");
    }
    song
}

pub(super) fn terminal_buffer_text(terminal: &Terminal<TestBackend>) -> String {
    terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>()
}

pub(super) fn line_text(line: &Line<'_>) -> String {
    line.spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect::<String>()
}
