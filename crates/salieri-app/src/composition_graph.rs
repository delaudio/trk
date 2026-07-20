use super::*;

use std::fmt::Write as _;

pub(crate) const COMPOSITION_GRAPH_SCHEMA: &str = "salieri.composition-graph.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CompositionGraph {
    pub(crate) schema: String,
    pub(crate) title: String,
    pub(crate) sections: Vec<CompositionGraphSection>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CompositionGraphSection {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) pattern: usize,
    pub(crate) repeats: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) motifs: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) evidence: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) transition: Option<String>,
}

pub(crate) fn load_composition_graph(path: &Path) -> Result<CompositionGraph> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read composition graph {}", path.display()))?;
    let graph = serde_json::from_str::<CompositionGraph>(&raw)
        .with_context(|| format!("invalid composition graph JSON {}", path.display()))?;
    validate_composition_graph(&graph)?;
    Ok(graph)
}

pub(crate) fn validate_composition_graph(graph: &CompositionGraph) -> Result<()> {
    if graph.schema != COMPOSITION_GRAPH_SCHEMA {
        anyhow::bail!("unsupported composition graph schema {:?}", graph.schema);
    }
    if graph.title.trim().is_empty() {
        anyhow::bail!("composition graph title cannot be empty");
    }
    if graph.sections.is_empty() {
        anyhow::bail!("composition graph must contain at least one section");
    }
    let mut ids = HashSet::new();
    for section in &graph.sections {
        if section.id.trim().is_empty() {
            anyhow::bail!("composition graph section id cannot be empty");
        }
        if !ids.insert(section.id.as_str()) {
            anyhow::bail!("duplicate composition graph section id {:?}", section.id);
        }
        if section.name.trim().is_empty() {
            anyhow::bail!(
                "composition graph section {:?} name cannot be empty",
                section.id
            );
        }
        if section.pattern == 0 {
            anyhow::bail!(
                "composition graph section {:?} pattern is 1-based",
                section.id
            );
        }
        if section.repeats == 0 {
            anyhow::bail!(
                "composition graph section {:?} repeats must be greater than zero",
                section.id
            );
        }
    }
    Ok(())
}

pub(crate) fn compile_composition_graph(song: &Song, graph: &CompositionGraph) -> Result<Song> {
    validate_composition_graph(graph)?;
    let mut compiled = song.clone();
    compiled.sequence.clear();
    for section in &graph.sections {
        let pattern = song.patterns.get(section.pattern - 1).with_context(|| {
            format!(
                "section {:?} references missing pattern {}",
                section.id, section.pattern
            )
        })?;
        for _ in 0..section.repeats {
            compiled.sequence.push(pattern.id);
        }
    }
    compiled
        .validate()
        .context("compiled composition graph produced invalid project")?;
    Ok(compiled)
}

pub(crate) fn draft_composition_graph(song: &Song, prompt: &str) -> CompositionGraph {
    let prompt = prompt.trim();
    let title = if prompt.is_empty() {
        format!("{} graph", song.metadata.title)
    } else {
        prompt.to_string()
    };
    let sections = song
        .patterns
        .iter()
        .enumerate()
        .map(|(index, pattern)| CompositionGraphSection {
            id: format!("section-{:02}", index + 1),
            name: pattern.name.clone(),
            pattern: index + 1,
            repeats: song
                .sequence
                .iter()
                .filter(|pattern_id| **pattern_id == pattern.id)
                .count()
                .max(1),
            motifs: Vec::new(),
            evidence: vec![format!("Source pattern {}", pattern.name)],
            transition: (index + 1 < song.patterns.len()).then(|| "next".to_string()),
        })
        .collect();
    CompositionGraph {
        schema: COMPOSITION_GRAPH_SCHEMA.to_string(),
        title,
        sections,
    }
}

pub(crate) fn format_composition_graph_preview(graph: &CompositionGraph) -> String {
    let mut output = String::new();
    writeln!(output, "# Composition Graph Preview").expect("write string");
    writeln!(output).expect("write string");
    writeln!(output, "- Title: {}", graph.title).expect("write string");
    writeln!(output, "- Sections: {}", graph.sections.len()).expect("write string");
    writeln!(output).expect("write string");
    for section in &graph.sections {
        writeln!(
            output,
            "- {}: pattern {}, repeat(s) {}{}",
            section.name,
            section.pattern,
            section.repeats,
            section
                .transition
                .as_ref()
                .map(|transition| format!(", transition {transition}"))
                .unwrap_or_default()
        )
        .expect("write string");
    }
    output
}
