use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SalieriCommand {
    Help,
    View(ViewCommand),
    Browse {
        browser: BrowserCommand,
        path: Option<PathBuf>,
    },
    Focus(FocusTarget),
    Quit {
        force: bool,
    },
    Write(Option<PathBuf>),
    SaveAs(PathBuf),
    WriteQuit,
    SetBpm(u16),
    SetLinesPerBeat(u8),
    Loop(LoopCommand),
    Play(PlayCommand),
    Stop,
    Domain {
        domain: CommandDomain,
        arguments: Vec<String>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewCommand {
    Tracker,
    Patterns,
    Sequence,
    Tracks,
    Sampler,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrowserCommand {
    Samples,
    Projects,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusTarget {
    Tracker,
    Patterns,
    Sequence,
    Tracks,
    Sampler,
    SampleBrowser,
    ProjectBrowser,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopCommand {
    On,
    Off,
    Toggle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayCommand {
    Pattern,
    Sequence { position: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandDomain {
    Fx,
    Cell,
    Automation,
    Mixer,
    Dsp,
    Ai,
    Midi,
    MidiInput,
    Track,
    Pattern,
    Sequence,
    Sample,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CommandParseError {
    #[error("Unknown command: {0}")]
    UnknownCommand(String),
    #[error("{usage}")]
    InvalidArguments { usage: &'static str },
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CommandValidationError {
    #[error("BPM must be between 1 and 999")]
    BpmOutOfRange,
    #[error("LPB must be between 1 and 32")]
    LinesPerBeatOutOfRange,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CommandDispatchError<E> {
    #[error(transparent)]
    Parse(#[from] CommandParseError),
    #[error(transparent)]
    Validation(#[from] CommandValidationError),
    #[error("Command failed: {0}")]
    Execution(E),
}

pub trait CommandExecutor {
    type Error;

    fn execute(&mut self, command: SalieriCommand) -> Result<(), Self::Error>;
}

pub fn dispatch<C: CommandExecutor>(
    context: &mut C,
    input: &str,
) -> Result<bool, CommandDispatchError<C::Error>> {
    let Some(command) = SalieriCommand::parse(input)? else {
        return Ok(false);
    };
    command.validate()?;
    context
        .execute(command)
        .map_err(CommandDispatchError::Execution)?;
    Ok(true)
}

impl SalieriCommand {
    pub fn parse(input: &str) -> Result<Option<Self>, CommandParseError> {
        let mut parts = input.split_whitespace();
        let Some(name) = parts.next() else {
            return Ok(None);
        };
        let arguments = parts.map(str::to_string).collect::<Vec<_>>();

        let command = match name {
            "h" | "help" => Self::Help,
            "t" | "tracker" | "layout" | "normal" => Self::View(ViewCommand::Tracker),
            "p" | "patterns" => Self::View(ViewCommand::Patterns),
            "se" | "sequence-view" => Self::View(ViewCommand::Sequence),
            "tr" | "tracks" => Self::View(ViewCommand::Tracks),
            "sa" | "sam" | "samples" => Self::View(ViewCommand::Sampler),
            "sb" | "sample-browser" => Self::Browse {
                browser: BrowserCommand::Samples,
                path: joined_path(&arguments),
            },
            "o" | "open" | "projects" | "project-browser" => Self::Browse {
                browser: BrowserCommand::Projects,
                path: joined_path(&arguments),
            },
            "f" | "focus" => Self::Focus(parse_focus(&arguments)?),
            "q" | "quit" => Self::Quit { force: false },
            "q!" | "quit!" => Self::Quit { force: true },
            "w" | "write" | "save" => Self::Write(joined_path(&arguments)),
            "saveas" | "writeas" => Self::SaveAs(joined_path(&arguments).ok_or(
                CommandParseError::InvalidArguments {
                    usage: "Usage: :saveas PATH",
                },
            )?),
            "wq" => Self::WriteQuit,
            "bpm" => Self::SetBpm(parse_single(&arguments, "Usage: :bpm 140")?),
            "lpb" => Self::SetLinesPerBeat(parse_single(&arguments, "Usage: :lpb 4")?),
            "loop" => Self::Loop(parse_loop(&arguments)?),
            "play" => Self::Play(parse_play(&arguments)?),
            "stop" => Self::Stop,
            "fx" | "effect" => domain(CommandDomain::Fx, arguments),
            "cell" => domain(CommandDomain::Cell, arguments),
            "automation" | "auto" => domain(CommandDomain::Automation, arguments),
            "mixer" | "mix" => domain(CommandDomain::Mixer, arguments),
            "dsp" | "effect-chain" => domain(CommandDomain::Dsp, arguments),
            "ai" => domain(CommandDomain::Ai, arguments),
            "midi" => domain(CommandDomain::Midi, arguments),
            "midi-input" | "midi-in" => domain(CommandDomain::MidiInput, arguments),
            "track" => domain(CommandDomain::Track, arguments),
            "pattern" => domain(CommandDomain::Pattern, arguments),
            "sequence" | "seq" => domain(CommandDomain::Sequence, arguments),
            "sample" | "sampler" => domain(CommandDomain::Sample, arguments),
            _ => return Err(CommandParseError::UnknownCommand(name.to_string())),
        };
        Ok(Some(command))
    }

    pub fn validate(&self) -> Result<(), CommandValidationError> {
        match self {
            Self::SetBpm(value) if !(1..=999).contains(value) => {
                Err(CommandValidationError::BpmOutOfRange)
            }
            Self::SetLinesPerBeat(value) if !(1..=32).contains(value) => {
                Err(CommandValidationError::LinesPerBeatOutOfRange)
            }
            _ => Ok(()),
        }
    }
}

fn domain(domain: CommandDomain, arguments: Vec<String>) -> SalieriCommand {
    SalieriCommand::Domain { domain, arguments }
}

fn joined_path(arguments: &[String]) -> Option<PathBuf> {
    (!arguments.is_empty()).then(|| PathBuf::from(arguments.join(" ")))
}

fn parse_single<T: std::str::FromStr>(
    arguments: &[String],
    usage: &'static str,
) -> Result<T, CommandParseError> {
    match arguments.first() {
        Some(value) => value
            .parse()
            .map_err(|_| CommandParseError::InvalidArguments { usage }),
        None => Err(CommandParseError::InvalidArguments { usage }),
    }
}

fn parse_focus(arguments: &[String]) -> Result<FocusTarget, CommandParseError> {
    match arguments.first().map(String::as_str) {
        Some("t" | "tracker" | "layout" | "normal") | None => Ok(FocusTarget::Tracker),
        Some("p" | "patterns" | "pattern-manager") => Ok(FocusTarget::Patterns),
        Some("se" | "sequence" | "sequence-view") => Ok(FocusTarget::Sequence),
        Some("tr" | "tracks") => Ok(FocusTarget::Tracks),
        Some("sa" | "sampler" | "samples") => Ok(FocusTarget::Sampler),
        Some("sb" | "browser" | "sample-browser") => Ok(FocusTarget::SampleBrowser),
        Some("o" | "open" | "pr" | "projects" | "project-browser") => {
            Ok(FocusTarget::ProjectBrowser)
        }
        Some(_) => Err(CommandParseError::InvalidArguments {
            usage: "Usage: :focus [t|p|se|tr|sa|sb|pr]",
        }),
    }
}

fn parse_loop(arguments: &[String]) -> Result<LoopCommand, CommandParseError> {
    match arguments.first().map(String::as_str) {
        Some("on") => Ok(LoopCommand::On),
        Some("off") => Ok(LoopCommand::Off),
        Some("toggle") | None => Ok(LoopCommand::Toggle),
        Some(_) => Err(CommandParseError::InvalidArguments {
            usage: "Usage: :loop [on|off|toggle]",
        }),
    }
}

fn parse_play(arguments: &[String]) -> Result<PlayCommand, CommandParseError> {
    match arguments.first().map(String::as_str) {
        Some("sequence" | "seq") => {
            let position = arguments
                .get(1)
                .and_then(|value| value.parse().ok())
                .unwrap_or(0);
            Ok(PlayCommand::Sequence { position })
        }
        Some("pattern" | "pat") | None => Ok(PlayCommand::Pattern),
        Some(_) => Err(CommandParseError::InvalidArguments {
            usage: "Usage: :play [pattern|sequence [position]]",
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct RecordingExecutor {
        commands: Vec<SalieriCommand>,
    }

    impl CommandExecutor for RecordingExecutor {
        type Error = &'static str;

        fn execute(&mut self, command: SalieriCommand) -> Result<(), Self::Error> {
            self.commands.push(command);
            Ok(())
        }
    }

    #[test]
    fn parses_stable_view_and_browser_aliases() {
        for alias in ["t", "tracker", "layout", "normal"] {
            assert_eq!(
                SalieriCommand::parse(alias),
                Ok(Some(SalieriCommand::View(ViewCommand::Tracker)))
            );
        }
        assert_eq!(
            SalieriCommand::parse("sb fixtures/drum kits"),
            Ok(Some(SalieriCommand::Browse {
                browser: BrowserCommand::Samples,
                path: Some(PathBuf::from("fixtures/drum kits")),
            }))
        );
        assert_eq!(
            SalieriCommand::parse("focus pr"),
            Ok(Some(SalieriCommand::Focus(FocusTarget::ProjectBrowser)))
        );
    }

    #[test]
    fn parses_typed_transport_values() {
        assert_eq!(
            SalieriCommand::parse("bpm 140"),
            Ok(Some(SalieriCommand::SetBpm(140)))
        );
        assert_eq!(
            SalieriCommand::parse("play sequence 3"),
            Ok(Some(SalieriCommand::Play(PlayCommand::Sequence {
                position: 3
            })))
        );
        assert_eq!(
            SalieriCommand::parse("loop off"),
            Ok(Some(SalieriCommand::Loop(LoopCommand::Off)))
        );
    }

    #[test]
    fn preserves_owned_domain_arguments() {
        assert_eq!(
            SalieriCommand::parse("track rename 2 Lead Bass"),
            Ok(Some(SalieriCommand::Domain {
                domain: CommandDomain::Track,
                arguments: vec!["rename", "2", "Lead", "Bass"]
                    .into_iter()
                    .map(str::to_string)
                    .collect(),
            }))
        );
    }

    #[test]
    fn separates_unknown_commands_and_invalid_arguments() {
        assert_eq!(SalieriCommand::parse("  "), Ok(None));
        assert_eq!(
            SalieriCommand::parse("wat"),
            Err(CommandParseError::UnknownCommand("wat".to_string()))
        );
        assert!(matches!(
            SalieriCommand::parse("bpm fast"),
            Err(CommandParseError::InvalidArguments { .. })
        ));
    }

    #[test]
    fn separates_semantic_validation_from_syntax_errors() {
        let command = SalieriCommand::parse("bpm 0")
            .expect("syntax")
            .expect("command");
        assert_eq!(
            command.validate(),
            Err(CommandValidationError::BpmOutOfRange)
        );
        assert!(matches!(
            SalieriCommand::parse("bpm nope"),
            Err(CommandParseError::InvalidArguments { .. })
        ));

        let mut executor = RecordingExecutor::default();
        assert!(matches!(
            dispatch(&mut executor, "bpm 0"),
            Err(CommandDispatchError::Validation(
                CommandValidationError::BpmOutOfRange
            ))
        ));
        assert!(executor.commands.is_empty());
    }

    #[test]
    fn dispatches_typed_commands_through_narrow_executor_interface() {
        let mut executor = RecordingExecutor::default();
        assert_eq!(dispatch(&mut executor, "bpm 150"), Ok(true));
        assert_eq!(executor.commands, vec![SalieriCommand::SetBpm(150)]);
    }
}
