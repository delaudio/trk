use salieri_core::{EffectDevice, InstrumentId, PatternCell, SampleEnvelope, SamplePlaybackMode};

#[derive(Debug, Clone, Default)]
pub(in crate::xrns) struct XrnsImportModel {
    pub(super) tracks: Vec<XrnsImportTrack>,
    pub(super) patterns: Vec<XrnsImportPattern>,
    pub(super) instruments: Vec<XrnsImportInstrument>,
    pub(super) sequence: Vec<usize>,
    pub(super) bpm: Option<u16>,
    pub(super) lines_per_beat: Option<u8>,
    pub(super) ticks_per_line: Option<u8>,
}

#[derive(Debug, Clone, Default)]
pub(super) struct XrnsImportTrack {
    pub(super) name: Option<String>,
    pub(super) gain: Option<f32>,
    pub(super) pan: Option<f32>,
    pub(super) effects: Vec<EffectDevice>,
}

#[derive(Debug, Clone, Default)]
pub(super) struct XrnsImportInstrument {
    pub(super) name: String,
    pub(super) samples: Vec<XrnsImportSampleMetadata>,
}

#[derive(Debug, Clone, Default)]
pub(super) struct XrnsImportSampleMetadata {
    pub(super) name: Option<String>,
    pub(super) root_pitch: Option<u8>,
    pub(super) transpose_semitones: Option<i8>,
    pub(super) fine_tune_cents: Option<i16>,
    pub(super) gain: Option<f32>,
    pub(super) pan: Option<f32>,
    pub(super) key_start: Option<u8>,
    pub(super) key_end: Option<u8>,
    pub(super) velocity_start: Option<u8>,
    pub(super) velocity_end: Option<u8>,
    pub(super) playback: XrnsImportSamplePlayback,
}

#[derive(Debug, Clone, Default)]
pub(super) struct XrnsImportSamplePlayback {
    pub(super) mode: Option<SamplePlaybackMode>,
    pub(super) start_frame: Option<usize>,
    pub(super) end_frame: Option<usize>,
    pub(super) loop_start_frame: Option<usize>,
    pub(super) loop_end_frame: Option<usize>,
    pub(super) envelope: Option<SampleEnvelope>,
}

#[derive(Debug, Clone, Default)]
pub(super) struct XrnsImportPattern {
    pub(super) rows: Option<usize>,
    pub(super) cells: Vec<XrnsImportCell>,
}

#[derive(Debug, Clone)]
pub(super) struct XrnsImportCell {
    pub(super) track: usize,
    pub(super) row: usize,
    pub(super) cell: PatternCell,
}

#[derive(Debug, Clone)]
pub(super) struct PendingXrnsLine {
    pub(super) track: usize,
    pub(super) row: Option<usize>,
    pub(super) cell: PatternCell,
    pub(super) effect_code: Option<String>,
    pub(super) effect_value: Option<u8>,
}

impl XrnsImportSampleMetadata {
    pub(super) fn envelope_mut(&mut self) -> &mut SampleEnvelope {
        self.playback
            .envelope
            .get_or_insert_with(SampleEnvelope::default)
    }

    pub(super) fn has_keyzone_mapping(&self) -> bool {
        self.key_start.is_some()
            || self.key_end.is_some()
            || self.velocity_start.is_some()
            || self.velocity_end.is_some()
    }
}

impl XrnsImportModel {
    pub(super) fn sample_metadata(
        &self,
        instrument: InstrumentId,
        sample_index: usize,
    ) -> Option<&XrnsImportSampleMetadata> {
        self.instruments
            .get(instrument.0 as usize)?
            .samples
            .get(sample_index)
            .or_else(|| self.instruments.get(instrument.0 as usize)?.samples.first())
    }
}
