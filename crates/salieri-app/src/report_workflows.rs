use super::*;

use crate::workflows::write_bytes_atomically;

pub(crate) fn run_report_project(args: &ReportArgs) -> Result<()> {
    run_report(args, "project report", format_project_report)
}

pub(crate) fn run_report_critique(args: &ReportArgs) -> Result<()> {
    run_report(args, "critique report", format_critique_report)
}

pub(crate) fn run_analyze(args: &AnalysisArgs) -> Result<()> {
    let input_path = args
        .input_path
        .as_deref()
        .context("missing analyze input path")?;
    let song = load_project(input_path)?;
    let analysis = analyze_style(&song);
    let output = format_analysis_output(&analysis, args.format)?;
    write_or_print_analysis(args.output_path.as_deref(), "analysis", &output)
}

pub(crate) fn run_compare(args: &CompareArgs) -> Result<()> {
    let left_path = args
        .left_path
        .as_deref()
        .context("missing compare left path")?;
    let right_path = args
        .right_path
        .as_deref()
        .context("missing compare right path")?;
    let left = load_project(left_path)?;
    let right = load_project(right_path)?;
    let comparison = compare_styles(&left, &right);
    let output = format_comparison_output(&comparison, args.format)?;
    write_or_print_analysis(args.output_path.as_deref(), "comparison", &output)
}

fn run_report(args: &ReportArgs, label: &str, formatter: fn(&Song) -> String) -> Result<()> {
    let input_path = args
        .input_path
        .as_deref()
        .with_context(|| format!("missing {label} input path"))?;
    let song = load_project(input_path)?;
    let report = formatter(&song);
    if let Some(output_path) = &args.output_path {
        write_bytes_atomically(output_path, report.as_bytes())
            .with_context(|| format!("failed to write {label} {}", output_path.display()))?;
        println!("Wrote {label} to {}", output_path.display());
    } else {
        print!("{report}");
    }
    Ok(())
}

fn write_or_print_analysis(output_path: Option<&Path>, label: &str, output: &str) -> Result<()> {
    if let Some(output_path) = output_path {
        write_bytes_atomically(output_path, output.as_bytes())
            .with_context(|| format!("failed to write {label} {}", output_path.display()))?;
        println!("Wrote {label} to {}", output_path.display());
    } else {
        print!("{output}");
    }
    Ok(())
}
