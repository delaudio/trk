use salieri_tui::TuiView;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandPaletteActionKind {
    Execute(&'static str),
    Prompt(&'static str),
    Internal(CommandPaletteInternalAction),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandPaletteInternalAction {
    ClearSelection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandPaletteAction {
    pub id: &'static str,
    pub title: &'static str,
    pub category: &'static str,
    pub kind: CommandPaletteActionKind,
    pub shortcut: Option<&'static str>,
    pub aliases: &'static [&'static str],
    pub keywords: &'static [&'static str],
    pub availability: ActionAvailability,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionAvailability {
    Always,
    WhenDirty,
    WhenPlaying,
    WhenSelection,
    WhenSampleLoaded,
    WhenSamplerViewSampleLoaded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandPaletteContext {
    pub active_view: TuiView,
    pub dirty: bool,
    pub is_playing: bool,
    pub has_selection: bool,
    pub has_loaded_sample: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandPaletteMatch {
    pub action: CommandPaletteAction,
    pub disabled_reason: Option<&'static str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RankedAction {
    action: CommandPaletteAction,
    disabled_reason: Option<&'static str>,
    score: i32,
    recent_rank: Option<usize>,
}

pub fn command_palette_results(
    query: &str,
    context: CommandPaletteContext,
    recent: &[String],
) -> Vec<CommandPaletteMatch> {
    let mut ranked = registered_actions()
        .into_iter()
        .filter_map(|action| {
            let disabled_reason = disabled_reason(action, context);
            let recent_rank = recent.iter().position(|id| id == action.id);
            let score = if query.trim().is_empty() {
                recent_rank.map_or(2_000, |rank| rank as i32)
            } else {
                fuzzy_score(query, action)?
                    + disabled_reason.map_or(0, |_| 500)
                    + recent_rank.map_or(100, |_| 0)
            };
            Some(RankedAction {
                action,
                disabled_reason,
                score,
                recent_rank,
            })
        })
        .collect::<Vec<_>>();

    ranked.sort_by(|left, right| {
        left.score
            .cmp(&right.score)
            .then_with(|| left.recent_rank.cmp(&right.recent_rank))
            .then_with(|| left.action.category.cmp(right.action.category))
            .then_with(|| left.action.title.cmp(right.action.title))
    });

    ranked
        .into_iter()
        .map(|ranked| CommandPaletteMatch {
            action: ranked.action,
            disabled_reason: ranked.disabled_reason,
        })
        .collect()
}

fn disabled_reason(
    action: CommandPaletteAction,
    context: CommandPaletteContext,
) -> Option<&'static str> {
    match action.availability {
        ActionAvailability::Always => None,
        ActionAvailability::WhenDirty if context.dirty => None,
        ActionAvailability::WhenDirty => Some("No unsaved changes"),
        ActionAvailability::WhenPlaying if context.is_playing => None,
        ActionAvailability::WhenPlaying => Some("Playback is stopped"),
        ActionAvailability::WhenSelection if context.has_selection => None,
        ActionAvailability::WhenSelection => Some("No active selection"),
        ActionAvailability::WhenSampleLoaded if context.has_loaded_sample => None,
        ActionAvailability::WhenSampleLoaded => Some("No sample loaded"),
        ActionAvailability::WhenSamplerViewSampleLoaded
            if context.has_loaded_sample && context.active_view == TuiView::Sampler =>
        {
            None
        }
        ActionAvailability::WhenSamplerViewSampleLoaded if context.has_loaded_sample => {
            Some("Open Sampler View first")
        }
        ActionAvailability::WhenSamplerViewSampleLoaded => Some("No sample loaded"),
    }
}

fn fuzzy_score(query: &str, action: CommandPaletteAction) -> Option<i32> {
    let query = normalize(query);
    if query.is_empty() {
        return Some(0);
    }
    let mut best = text_score(&query, action.title);
    best = best.min(text_score(&query, action.category) + 20);
    if let Some(shortcut) = action.shortcut {
        best = best.min(text_score(&query, shortcut) + 15);
    }
    let command = action.command_label();
    best = best.min(text_score(&query, command) + 10);
    for alias in action.aliases {
        best = best.min(text_score(&query, alias) + 5);
    }
    for keyword in action.keywords {
        best = best.min(text_score(&query, keyword) + 12);
    }
    (best < i32::MAX / 4).then_some(best)
}

fn text_score(query: &str, candidate: &str) -> i32 {
    let candidate = normalize(candidate);
    if candidate == query {
        return 0;
    }
    if candidate.starts_with(query) {
        return 2 + candidate.len() as i32 - query.len() as i32;
    }
    if candidate.contains(query) {
        return 20 + candidate.len() as i32 - query.len() as i32;
    }
    subsequence_score(query, &candidate).unwrap_or(i32::MAX / 2)
}

fn subsequence_score(query: &str, candidate: &str) -> Option<i32> {
    let mut score = 80;
    let mut last_index: Option<usize> = None;
    let mut search_from = 0;
    for needle in query.chars() {
        let haystack = &candidate[search_from..];
        let offset = haystack.find(needle)?;
        let index = search_from + offset;
        score += offset as i32;
        if let Some(last_index) = last_index {
            score += index.saturating_sub(last_index + 1) as i32;
        }
        last_index = Some(index);
        search_from = index + needle.len_utf8();
    }
    Some(score + candidate.len() as i32 - query.len() as i32)
}

fn normalize(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn registered_actions() -> Vec<CommandPaletteAction> {
    use ActionAvailability::{
        Always, WhenDirty, WhenPlaying, WhenSampleLoaded, WhenSamplerViewSampleLoaded,
        WhenSelection,
    };
    use CommandPaletteActionKind::{Execute, Internal, Prompt};
    use CommandPaletteInternalAction::ClearSelection;

    vec![
        action(
            "view.tracker",
            "Open Tracker",
            "View",
            Execute("tracker"),
            Some("Esc"),
            &["pattern"],
            &["layout", "normal"],
            Always,
        ),
        action(
            "view.sequence",
            "Open Sequence View",
            "View",
            Execute("sequence-view"),
            Some("F7"),
            &["sequence"],
            &["arrangement"],
            Always,
        ),
        action(
            "view.tracks",
            "Open Tracks View",
            "View",
            Execute("tracks"),
            Some("F9"),
            &["tracks"],
            &["mixer"],
            Always,
        ),
        action(
            "view.patterns",
            "Open Patterns View",
            "View",
            Execute("patterns"),
            Some("F10"),
            &["patterns"],
            &["manager"],
            Always,
        ),
        action(
            "view.sampler",
            "Open Sampler View",
            "View",
            Execute("sampler"),
            Some("Ctrl+J"),
            &["sample"],
            &["instrument"],
            Always,
        ),
        action(
            "view.sample-browser",
            "Open Sample Browser",
            "View",
            Execute("sample-browser"),
            None,
            &["browse samples"],
            &["wav", "library"],
            Always,
        ),
        action(
            "view.project-browser",
            "Open Project Browser",
            "View",
            Execute("project-browser"),
            None,
            &["open project"],
            &["recent"],
            Always,
        ),
        action(
            "help",
            "Open Help",
            "General",
            Execute("help"),
            Some("?"),
            &["h"],
            &["manual", "shortcuts"],
            Always,
        ),
        action(
            "config",
            "Show Config Summary",
            "General",
            Execute("config"),
            None,
            &["settings"],
            &["keymap"],
            Always,
        ),
        action(
            "save",
            "Save Project",
            "Project",
            Execute("write"),
            Some("Ctrl+S"),
            &["write"],
            &["persist"],
            Always,
        ),
        action(
            "save-as",
            "Save Project As...",
            "Project",
            Prompt("saveas "),
            Some("Ctrl+Shift+S"),
            &["write as"],
            &["path"],
            Always,
        ),
        action(
            "project.import-midi",
            "Import MIDI...",
            "Project",
            Prompt("midi import "),
            None,
            &["import smf"],
            &["mid", "standard midi file", "pattern"],
            Always,
        ),
        action(
            "save-quit",
            "Save and Quit",
            "Project",
            Execute("wq"),
            None,
            &["write quit"],
            &["exit"],
            Always,
        ),
        action(
            "quit",
            "Quit",
            "Project",
            Execute("quit"),
            Some("q"),
            &["close"],
            &["exit"],
            Always,
        ),
        action(
            "quit-force",
            "Force Quit",
            "Project",
            Execute("quit!"),
            None,
            &["discard quit"],
            &["exit"],
            WhenDirty,
        ),
        action(
            "play.pattern",
            "Play Pattern",
            "Transport",
            Execute("play"),
            Some("Space"),
            &["start playback"],
            &["transport"],
            Always,
        ),
        action(
            "play.sequence",
            "Play Sequence From Selection",
            "Transport",
            Execute("play sequence 0"),
            Some("Shift+Enter"),
            &["sequence play"],
            &["transport"],
            Always,
        ),
        action(
            "stop",
            "Stop Playback",
            "Transport",
            Execute("stop"),
            Some("F8"),
            &["pause"],
            &["transport"],
            WhenPlaying,
        ),
        action(
            "loop",
            "Toggle Pattern Loop",
            "Transport",
            Execute("loop"),
            Some("L"),
            &["loop playback"],
            &["transport"],
            Always,
        ),
        action(
            "midi.settings",
            "Open MIDI Settings",
            "MIDI",
            Execute("midi settings"),
            Some("F4"),
            &["ports"],
            &["connect"],
            Always,
        ),
        action(
            "midi.panic",
            "Send MIDI Panic",
            "MIDI",
            Execute("midi panic"),
            Some("Ctrl+Shift+P"),
            &["all notes off"],
            &["stuck notes"],
            Always,
        ),
        action(
            "track.new",
            "Create Track",
            "Tracks",
            Execute("track new"),
            Some("Ctrl+T"),
            &["new track"],
            &["insert"],
            Always,
        ),
        action(
            "track.duplicate",
            "Duplicate Current Track",
            "Tracks",
            Execute("track duplicate"),
            Some("D"),
            &["copy track"],
            &["clone"],
            Always,
        ),
        action(
            "track.rename",
            "Rename Current Track...",
            "Tracks",
            Prompt("track rename "),
            Some("r"),
            &["track name"],
            &["label"],
            Always,
        ),
        action(
            "track.delete",
            "Delete Current Track",
            "Tracks",
            Execute("track delete"),
            Some("Delete"),
            &["remove track"],
            &["destructive"],
            Always,
        ),
        action(
            "track.mute",
            "Toggle Current Track Mute",
            "Tracks",
            Execute("track mute"),
            Some("m"),
            &["mute"],
            &["mixer"],
            Always,
        ),
        action(
            "track.solo",
            "Toggle Current Track Solo",
            "Tracks",
            Execute("track solo"),
            Some("s"),
            &["solo"],
            &["mixer"],
            Always,
        ),
        action(
            "pattern.new",
            "Create Pattern",
            "Patterns",
            Execute("pattern new"),
            Some("N"),
            &["new pattern"],
            &["insert"],
            Always,
        ),
        action(
            "pattern.duplicate",
            "Duplicate Current Pattern",
            "Patterns",
            Execute("pattern duplicate"),
            Some("P"),
            &["copy pattern"],
            &["clone"],
            Always,
        ),
        action(
            "pattern.rename",
            "Rename Current Pattern...",
            "Patterns",
            Prompt("pattern rename "),
            Some("F3"),
            &["pattern name"],
            &["label"],
            Always,
        ),
        action(
            "pattern.length",
            "Set Pattern Length...",
            "Patterns",
            Prompt("pattern length "),
            Some("F6"),
            &["resize pattern"],
            &["rows"],
            Always,
        ),
        action(
            "pattern.delete",
            "Delete Current Pattern",
            "Patterns",
            Execute("pattern delete"),
            Some("X"),
            &["remove pattern"],
            &["destructive"],
            Always,
        ),
        action(
            "selection.clear",
            "Clear Selection",
            "Editing",
            Internal(ClearSelection),
            Some("Delete"),
            &["delete selection"],
            &["edit"],
            WhenSelection,
        ),
        action(
            "sampler.assign",
            "Assign Loaded Sample",
            "Sampler",
            Execute("sample assign"),
            None,
            &["sample to track"],
            &["instrument"],
            WhenSampleLoaded,
        ),
        action(
            "sampler.settings",
            "Show Loaded Sample Settings",
            "Sampler",
            Execute("sample settings"),
            None,
            &["sample info"],
            &["instrument"],
            WhenSamplerViewSampleLoaded,
        ),
        action(
            "sampler.envelope",
            "Edit Sample Envelope...",
            "Sampler",
            Prompt("sample envelope "),
            None,
            &["adsr"],
            &["attack", "decay", "sustain", "release"],
            WhenSamplerViewSampleLoaded,
        ),
    ]
}

#[allow(clippy::too_many_arguments)]
fn action(
    id: &'static str,
    title: &'static str,
    category: &'static str,
    kind: CommandPaletteActionKind,
    shortcut: Option<&'static str>,
    aliases: &'static [&'static str],
    keywords: &'static [&'static str],
    availability: ActionAvailability,
) -> CommandPaletteAction {
    CommandPaletteAction {
        id,
        title,
        category,
        kind,
        shortcut,
        aliases,
        keywords,
        availability,
    }
}

impl CommandPaletteAction {
    pub fn command_label(self) -> &'static str {
        match self.kind {
            CommandPaletteActionKind::Execute(command)
            | CommandPaletteActionKind::Prompt(command) => command,
            CommandPaletteActionKind::Internal(CommandPaletteInternalAction::ClearSelection) => {
                "selection clear"
            }
        }
    }
}

#[cfg(test)]
#[path = "command_palette_tests.rs"]
mod tests;
