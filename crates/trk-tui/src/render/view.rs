#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TuiView {
    Pattern,
    ParameterPage,
    PianoRoll { pitch: u8, rows: u8, ghosts: bool },
    Sequence,
    Clips,
    Tracks,
    Patterns,
    Sampler,
    DspRack,
    SampleBrowser,
    ProjectBrowser,
    AiChat,
}
