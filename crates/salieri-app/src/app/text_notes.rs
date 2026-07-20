use super::*;

impl App {
    pub(crate) fn handle_note_command(&mut self, arguments: &[&str]) {
        match arguments.split_first() {
            Some((head, rest)) => match *head {
                "add" | "new" | "set" => self.add_text_note(rest),
                "project" | "song" => {
                    self.add_scoped_text_note(TextAnnotationKind::Note, "project", rest)
                }
                "pattern" | "row" => {
                    self.add_scoped_text_note(TextAnnotationKind::Note, "pattern", rest)
                }
                "sequence" | "seq" => {
                    self.add_scoped_text_note(TextAnnotationKind::Cue, "sequence", rest)
                }
                "lyric" | "lyrics" => self.add_text_annotation_kind(
                    TextAnnotationKind::Lyric,
                    rest,
                    "Usage: :note lyric project|pattern|sequence [TARGET] TEXT",
                ),
                "cue" | "marker" => self.add_text_annotation_kind(
                    TextAnnotationKind::Cue,
                    rest,
                    "Usage: :note cue project|pattern|sequence [TARGET] TEXT",
                ),
                "list" | "show" | "view" => self.show_text_annotations(),
                "report" | "summary" => self.show_text_annotation_report(),
                "clear" | "delete" | "remove" => {
                    if let Some(id) = rest.first().and_then(|value| value.parse::<u32>().ok()) {
                        self.remove_text_annotation(id);
                    } else {
                        self.notify_warning("Usage: :note clear ID");
                    }
                }
                _ => self.notify_warning(
                    "Usage: :note project TEXT | pattern [ROW] TEXT | lyric pattern [ROW] TEXT | cue sequence POSITION TEXT | list | report | clear ID",
                ),
            },
            None => self.show_text_annotations(),
        }
    }

    fn add_text_note(&mut self, arguments: &[&str]) {
        match arguments.split_first() {
            Some((scope, rest))
                if matches!(
                    *scope,
                    "project" | "song" | "pattern" | "row" | "sequence" | "seq"
                ) =>
            {
                self.add_scoped_text_note(TextAnnotationKind::Note, scope, rest);
            }
            _ => self.notify_warning("Usage: :note add project|pattern|sequence [TARGET] TEXT"),
        }
    }

    fn add_text_annotation_kind(
        &mut self,
        kind: TextAnnotationKind,
        arguments: &[&str],
        usage: &'static str,
    ) {
        match arguments.split_first() {
            Some((scope, rest))
                if matches!(
                    *scope,
                    "project" | "song" | "pattern" | "row" | "sequence" | "seq"
                ) =>
            {
                self.add_scoped_text_note(kind, scope, rest);
            }
            _ => self.notify_warning(usage),
        }
    }

    fn add_scoped_text_note(
        &mut self,
        kind: TextAnnotationKind,
        scope_name: &str,
        arguments: &[&str],
    ) {
        let parsed = match parse_text_annotation_scope(self, scope_name, arguments) {
            Ok(parsed) => parsed,
            Err(message) => {
                self.notify_warning(message);
                return;
            }
        };
        let ParsedTextAnnotation { scope, text } = parsed;
        if text.trim().is_empty() {
            self.notify_warning("Annotation text cannot be empty");
            return;
        }
        let mut id = 0;
        self.mutate_song_with(TransactionSpec::new("Edit text annotations"), |song, _| {
            id = song.add_text_annotation(kind, scope, text);
        });
        self.notify_success(format!("Annotation #{id} added"));
    }

    fn remove_text_annotation(&mut self, id: u32) {
        let mut removed = false;
        self.mutate_song_with(TransactionSpec::new("Edit text annotations"), |song, _| {
            removed = song.remove_text_annotation(id);
        });
        if removed {
            self.notify_success(format!("Annotation #{id} removed"));
        } else {
            self.notify_warning(format!("Annotation #{id} not found"));
        }
    }

    fn show_text_annotations(&mut self) {
        if self.song.annotations.is_empty() {
            self.notify_info("No text annotations");
            return;
        }
        self.notify_info(format_text_annotations(&self.song.annotations));
    }

    fn show_text_annotation_report(&mut self) {
        self.notify_info(format_text_annotation_report(&self.song));
    }
}

struct ParsedTextAnnotation {
    scope: TextAnnotationScope,
    text: String,
}

fn parse_text_annotation_scope(
    app: &App,
    scope_name: &str,
    arguments: &[&str],
) -> Result<ParsedTextAnnotation, &'static str> {
    match scope_name {
        "project" | "song" => Ok(ParsedTextAnnotation {
            scope: TextAnnotationScope::Project,
            text: arguments.join(" "),
        }),
        "pattern" | "row" => {
            let pattern_id = app
                .song
                .patterns
                .get(app.pattern_index)
                .map(|pattern| pattern.id)
                .ok_or("No active pattern")?;
            let (row, text_start) = arguments
                .first()
                .and_then(|value| value.parse::<usize>().ok())
                .map_or((Some(app.cursor.row), 0), |row| (Some(row), 1));
            Ok(ParsedTextAnnotation {
                scope: TextAnnotationScope::Pattern {
                    pattern: pattern_id,
                    row,
                },
                text: arguments[text_start..].join(" "),
            })
        }
        "sequence" | "seq" => {
            let (position, text_start) = arguments
                .first()
                .and_then(|value| value.parse::<usize>().ok())
                .map_or((app.sequence_cursor, 0), |position| (position, 1));
            if position >= app.song.sequence.len() {
                return Err("Sequence position out of range");
            }
            Ok(ParsedTextAnnotation {
                scope: TextAnnotationScope::Sequence { position },
                text: arguments[text_start..].join(" "),
            })
        }
        _ => Err("Unknown annotation scope"),
    }
}

pub(crate) fn format_text_annotations(annotations: &[TextAnnotation]) -> String {
    annotations
        .iter()
        .map(format_text_annotation)
        .collect::<Vec<_>>()
        .join(" | ")
}

pub(crate) fn format_text_annotation_report(song: &Song) -> String {
    if song.annotations.is_empty() {
        return "Text annotations: none".to_string();
    }
    format!(
        "Text annotations report: {}",
        format_text_annotations(&song.annotations)
    )
}

fn format_text_annotation(annotation: &TextAnnotation) -> String {
    format!(
        "#{} {} {}: {}",
        annotation.id,
        format_annotation_kind(annotation.kind),
        format_annotation_scope(&annotation.scope),
        annotation.text
    )
}

fn format_annotation_kind(kind: TextAnnotationKind) -> &'static str {
    match kind {
        TextAnnotationKind::Note => "note",
        TextAnnotationKind::Lyric => "lyric",
        TextAnnotationKind::Cue => "cue",
    }
}

fn format_annotation_scope(scope: &TextAnnotationScope) -> String {
    match scope {
        TextAnnotationScope::Project => "project".to_string(),
        TextAnnotationScope::Pattern { pattern, row } => match row {
            Some(row) => format!("pattern {:?} row {row}", pattern),
            None => format!("pattern {:?}", pattern),
        },
        TextAnnotationScope::Sequence { position } => format!("sequence {position}"),
    }
}
