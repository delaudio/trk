use trk_core::{parse_pitch_class, HarmonicScale, ScaleMode};

const MAX_NESTING: usize = 32;
const MAX_ARGUMENT: usize = 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceKind {
    Notes,
    Samples,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scale {
    pub root: u8,
    pub intervals: &'static [u8],
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    pub expression: Expr,
    pub source: SourceKind,
    pub scale: Option<Scale>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    Atom(String),
    Rest,
    Sequence(Vec<Self>),
    Layer(Vec<Self>),
    Alternation(Vec<Self>),
    Fast(Box<Self>, usize),
    Slow(Box<Self>, usize),
    Euclid {
        expression: Box<Self>,
        pulses: usize,
        steps: usize,
        rotation: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("mini-notation error at byte {position}: {message}")]
pub struct StrudelError {
    pub position: usize,
    pub message: String,
}

impl StrudelError {
    pub(crate) fn evaluation(message: impl Into<String>) -> Self {
        Self {
            position: 0,
            message: message.into(),
        }
    }
}

impl Program {
    pub fn parse(source: &str) -> Result<Self, StrudelError> {
        let source = source.trim();
        if source.is_empty() {
            return Err(error(0, "expression cannot be empty"));
        }

        let (notation, kind, suffix, notation_offset) = extract_call(source)?;
        let mut parser = Parser::new(notation, notation_offset);
        let mut expression = parser.parse_sequence(&[])?;
        parser.skip_space();
        if !parser.done() {
            return Err(parser.fail("unexpected token"));
        }
        let mut scale = None;
        let mut suffix = suffix.trim();
        while !suffix.is_empty() {
            if let Some(rest) = suffix.strip_prefix(".scale(") {
                if scale.is_some() {
                    return Err(error(
                        source.len() - suffix.len(),
                        "scale may only be specified once",
                    ));
                }
                let (value, remaining) = quoted_argument(rest, source.len() - suffix.len())?;
                scale = Some(parse_scale(&value, source.len() - suffix.len())?);
                suffix = remaining.trim();
            } else if let Some(rest) = suffix.strip_prefix(".euclid(") {
                let (values, remaining) = numeric_arguments(rest, source.len() - suffix.len())?;
                let (pulses, steps, rotation) = euclid_arguments(&values, 0)?;
                expression = Expr::Euclid {
                    expression: Box::new(expression),
                    pulses,
                    steps,
                    rotation,
                };
                suffix = remaining.trim();
            } else {
                return Err(error(
                    source.len() - suffix.len(),
                    "expected .scale(\"root:mode\") or .euclid(pulses,steps[,rotation])",
                ));
            }
        }

        Ok(Self {
            expression,
            source: kind,
            scale,
        })
    }
}

fn extract_call(source: &str) -> Result<(&str, SourceKind, &str, usize), StrudelError> {
    for (prefix, kind) in [
        ("note(\"", SourceKind::Notes),
        ("s(\"", SourceKind::Samples),
    ] {
        if let Some(rest) = source.strip_prefix(prefix) {
            let quote = closing_quote_paren(rest, prefix.len(), "pattern")?;
            let notation = &rest[..quote];
            if let Some(position) = notation.find('\\') {
                return Err(error(
                    prefix.len() + position,
                    "escape sequences are not supported in quoted patterns",
                ));
            }
            let suffix = &rest[quote + 2..];
            return Ok((notation, kind, suffix, prefix.len()));
        }
    }
    Ok((source, SourceKind::Notes, "", 0))
}

fn quoted_argument(source: &str, offset: usize) -> Result<(String, &str), StrudelError> {
    let rest = source
        .strip_prefix('"')
        .ok_or_else(|| error(offset, "expected quoted argument"))?;
    let end = closing_quote_paren(rest, offset + 1, "argument")?;
    Ok((rest[..end].to_string(), &rest[end + 2..]))
}

fn closing_quote_paren(
    source: &str,
    offset: usize,
    description: &str,
) -> Result<usize, StrudelError> {
    let mut escaped = false;
    for (index, character) in source.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        match character {
            '\\' => escaped = true,
            '"' if source[index + character.len_utf8()..].starts_with(')') => return Ok(index),
            '"' => {
                return Err(error(
                    offset + index + character.len_utf8(),
                    format!("expected ')' after quoted {description}"),
                ));
            }
            _ => {}
        }
    }
    Err(error(offset, format!("unterminated quoted {description}")))
}

fn numeric_arguments(source: &str, offset: usize) -> Result<(Vec<usize>, &str), StrudelError> {
    let end = source
        .find(')')
        .ok_or_else(|| error(offset, "unterminated numeric arguments"))?;
    let values = parse_numbers(&source[..end], offset)?;
    Ok((values, &source[end + 1..]))
}

fn parse_scale(value: &str, position: usize) -> Result<Scale, StrudelError> {
    let (root, mode) = value
        .split_once(':')
        .ok_or_else(|| error(position, "scale must be root:mode"))?;
    let root = parse_pitch_class(root).ok_or_else(|| error(position, "unknown scale root"))?;
    let mode = ScaleMode::parse(mode).ok_or_else(|| error(position, "unknown scale mode"))?;
    let intervals = HarmonicScale::new(root, mode)
        .expect("parsed pitch classes are bounded")
        .intervals();
    Ok(Scale {
        root,
        intervals,
        name: value.to_string(),
    })
}

pub(crate) fn pitch_class(value: &str) -> Option<u8> {
    parse_pitch_class(value)
}

struct Parser<'a> {
    source: &'a str,
    index: usize,
    offset: usize,
    depth: usize,
}

impl<'a> Parser<'a> {
    fn new(source: &'a str, offset: usize) -> Self {
        Self {
            source,
            index: 0,
            offset,
            depth: 0,
        }
    }

    fn parse_sequence(&mut self, stops: &[char]) -> Result<Expr, StrudelError> {
        let mut values = Vec::new();
        loop {
            self.skip_space();
            if self.done() || self.peek().is_some_and(|value| stops.contains(&value)) {
                break;
            }
            values.push(self.parse_term()?);
        }
        match values.len() {
            0 => Err(self.fail("expected pattern element")),
            1 => Ok(values.remove(0)),
            _ => Ok(Expr::Sequence(values)),
        }
    }

    fn parse_term(&mut self) -> Result<Expr, StrudelError> {
        let mut expression = match self.peek() {
            Some('[') => self.parse_bracket()?,
            Some('<') => self.parse_alternation()?,
            Some('~') => {
                self.bump();
                Expr::Rest
            }
            Some(_) => self.parse_atom()?,
            None => return Err(self.fail("expected pattern element")),
        };

        loop {
            self.skip_space();
            expression = match self.peek() {
                Some('*') => {
                    self.bump();
                    Expr::Fast(Box::new(expression), self.parse_number()?)
                }
                Some('/') => {
                    self.bump();
                    Expr::Slow(Box::new(expression), self.parse_number()?)
                }
                Some('(') => {
                    self.bump();
                    let start = self.index;
                    let end = self.source[start..]
                        .find(')')
                        .map(|value| start + value)
                        .ok_or_else(|| self.fail("unterminated Euclidean arguments"))?;
                    let values = parse_numbers(&self.source[start..end], self.offset + start)?;
                    self.index = end + 1;
                    let (pulses, steps, rotation) = euclid_arguments(&values, self.offset + start)?;
                    Expr::Euclid {
                        expression: Box::new(expression),
                        pulses,
                        steps,
                        rotation,
                    }
                }
                _ => break,
            };
        }
        Ok(expression)
    }

    fn parse_bracket(&mut self) -> Result<Expr, StrudelError> {
        self.enter('[')?;
        let mut layers = Vec::new();
        loop {
            layers.push(self.parse_sequence(&[',', ']'])?);
            self.skip_space();
            match self.peek() {
                Some(',') => {
                    self.bump();
                }
                Some(']') => {
                    self.bump();
                    self.depth -= 1;
                    break;
                }
                _ => return Err(self.fail("expected ',' or ']'")),
            }
        }
        if layers.len() == 1 {
            Ok(layers.remove(0))
        } else {
            Ok(Expr::Layer(layers))
        }
    }

    fn parse_alternation(&mut self) -> Result<Expr, StrudelError> {
        self.enter('<')?;
        let expression = self.parse_sequence(&['>'])?;
        if self.peek() != Some('>') {
            return Err(self.fail("expected '>'"));
        }
        self.bump();
        self.depth -= 1;
        let values = match expression {
            Expr::Sequence(values) => values,
            value => vec![value],
        };
        Ok(Expr::Alternation(values))
    }

    fn enter(&mut self, expected: char) -> Result<(), StrudelError> {
        debug_assert_eq!(self.peek(), Some(expected));
        if self.depth >= MAX_NESTING {
            return Err(self.fail("maximum nesting depth exceeded"));
        }
        self.depth += 1;
        self.bump();
        Ok(())
    }

    fn parse_atom(&mut self) -> Result<Expr, StrudelError> {
        let start = self.index;
        while self
            .peek()
            .is_some_and(|value| !value.is_whitespace() && !"[]<>,*/()".contains(value))
        {
            self.bump();
        }
        if self.index == start {
            return Err(self.fail("expected note, degree, sample, or rest"));
        }
        Ok(Expr::Atom(self.source[start..self.index].to_string()))
    }

    fn parse_number(&mut self) -> Result<usize, StrudelError> {
        self.skip_space();
        let start = self.index;
        while self.peek().is_some_and(|value| value.is_ascii_digit()) {
            self.bump();
        }
        if start == self.index {
            return Err(self.fail("expected positive integer"));
        }
        let value = self.source[start..self.index]
            .parse::<usize>()
            .map_err(|_| self.fail("invalid integer"))?;
        bounded_number(value, self.offset + start)
    }

    fn skip_space(&mut self) {
        while self.peek().is_some_and(char::is_whitespace) {
            self.bump();
        }
    }

    fn peek(&self) -> Option<char> {
        self.source[self.index..].chars().next()
    }

    fn bump(&mut self) {
        if let Some(value) = self.peek() {
            self.index += value.len_utf8();
        }
    }

    fn done(&self) -> bool {
        self.index == self.source.len()
    }

    fn fail(&self, message: impl Into<String>) -> StrudelError {
        error(self.offset + self.index, message)
    }
}

fn parse_numbers(source: &str, offset: usize) -> Result<Vec<usize>, StrudelError> {
    let mut values = Vec::new();
    let mut consumed = 0;
    for segment in source.split(',') {
        let segment_len = segment.len();
        let leading_space = segment_len - segment.trim_start().len();
        let position = offset + consumed + leading_space;
        let argument = segment.trim();
        if argument.is_empty() {
            return Err(error(position, "expected numeric argument"));
        }
        let parsed = argument
            .parse::<usize>()
            .map_err(|_| error(position, "expected numeric argument"))?;
        if parsed > MAX_ARGUMENT {
            return Err(error(
                position,
                format!("integer must be in 0..={MAX_ARGUMENT}"),
            ));
        }
        values.push(parsed);
        consumed += segment_len + 1;
    }
    Ok(values)
}

fn bounded_number(value: usize, position: usize) -> Result<usize, StrudelError> {
    if value == 0 || value > MAX_ARGUMENT {
        Err(error(
            position,
            format!("integer must be in 1..={MAX_ARGUMENT}"),
        ))
    } else {
        Ok(value)
    }
}

fn euclid_arguments(
    values: &[usize],
    position: usize,
) -> Result<(usize, usize, usize), StrudelError> {
    let (pulses, steps, rotation) = match values {
        [pulses, steps] => (*pulses, *steps, 0),
        [pulses, steps, rotation] => (*pulses, *steps, *rotation),
        _ => return Err(error(position, "Euclidean rhythm expects 2 or 3 arguments")),
    };
    if steps == 0 {
        return Err(error(position, "Euclidean steps must be positive"));
    }
    if pulses > steps {
        return Err(error(position, "Euclidean pulses cannot exceed steps"));
    }
    Ok((pulses, steps, rotation))
}

fn error(position: usize, message: impl Into<String>) -> StrudelError {
    StrudelError {
        position,
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_nested_operators_layers_and_methods() {
        let program =
            Program::parse("note(\"[c3 [eb3 g3], <0 2>*2] ~\").euclid(5,16,2).scale(\"d:minor\")")
                .expect("parse");
        assert_eq!(program.source, SourceKind::Notes);
        assert_eq!(program.scale.as_ref().map(|scale| scale.root), Some(2));
        assert!(matches!(
            program.expression,
            Expr::Euclid { rotation: 2, .. }
        ));
    }

    #[test]
    fn reports_source_positions_and_bounded_arguments() {
        let missing = Program::parse("[c4 d4").expect_err("missing bracket");
        assert!(missing.position > 0);
        assert!(missing.to_string().contains("expected ',' or ']'"));
        assert!(Program::parse("c4*0").is_err());
        assert!(Program::parse("c4(9,8)").is_err());
        assert!(Program::parse("c4(0,8,0)").is_ok());
        assert!(Program::parse("c4(1,0,0)").is_err());
        assert!(Program::parse("c4()").is_err());
        assert!(Program::parse("c4(3,)").is_err());
        assert!(Program::parse("note(\"0\").scale(\"c:major\").scale(\"d:minor\")").is_err());
        assert_eq!(Program::parse("c4(3, bad)").unwrap_err().position, 6);
        assert_eq!(Program::parse("c4(3 ,  bad)").unwrap_err().position, 8);
    }

    #[test]
    fn parses_sample_wrapper() {
        let program = Program::parse("s(\"bd [~ sd] hh*2\")").expect("parse samples");
        assert_eq!(program.source, SourceKind::Samples);
    }

    #[test]
    fn quoted_wrappers_do_not_end_at_escaped_quotes() {
        let boundary = r#"c4\") d4")"#;
        assert_eq!(
            closing_quote_paren(boundary, 0, "pattern").expect("closing quote"),
            boundary.rfind('"').expect("final quote")
        );

        let escaped = Program::parse(r#"note("c4\" d4")"#).expect_err("reject escapes");
        assert!(escaped.message.contains("escape sequences"));

        let error = Program::parse(r#"note("c4" d4")"#).expect_err("unescaped quote");
        assert!(error.message.contains("expected ')'"));
    }
}
