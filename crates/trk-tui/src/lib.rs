pub mod color;
pub mod interaction;
pub mod layout;
pub mod render;
pub mod viewport;

pub use color::TerminalColorMode;
pub use interaction::{
    region as interaction_region, ConfirmationAction, DspRackChain, InteractionMap,
    InteractionPayload, InteractionRegion, InteractionRegionId, MidiSettingsAction, SamplerAction,
    SamplerEnvelopeField, ScrollTarget, TransportAction,
};
pub use layout::{
    resolve_managed_layout, resolve_tracker_layout, LayoutDiagnostic, ManagedLayoutDirection,
    ManagedLayoutNode, ManagedPanelId, ManagedSize, PatternFieldLayout, ResolvedManagedLayout,
    ResolvedPanel, ResolvedTrackerLayout, TrackerLayoutPreset, TrackerLayoutState,
};
pub use render::{
    render, render_calibration_overlay, render_waveform_overview,
    render_waveform_overview_with_glyphs, render_with_interactions, AiChatMessageRole,
    AiChatMessageView, AiChatProposalPreviewView, AiChatViewState, AiEngineEntryView,
    AiEngineSelectorViewState, CalibrationViewState, CommandPaletteEntryView,
    CommandPaletteViewState, DspDevicePaletteEntryView, DspDevicePaletteViewState,
    DspParameterLockStatusView, DspRackTargetView, DspRackViewState, HelpTab, MidiPortView,
    MidiSettingsState, NotificationKind, NotificationView, PatternVariationEntryView,
    PatternVariationHistoryViewState, ProjectBrowserEntryKind, ProjectBrowserEntryView,
    ProjectBrowserViewState, SampleBrowserEntryKind, SampleBrowserEntryView,
    SampleBrowserViewState, SamplerViewState, SelectionRect, TuiState, TuiView, WaveformGlyphs,
};
pub use viewport::{OverscrollPolicy, Viewport2D, ViewportAxis};
