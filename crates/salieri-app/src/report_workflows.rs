use super::*;

use crate::workflows::write_bytes_atomically;

pub(crate) fn run_report_project(args: &ReportArgs) -> Result<()> {
    run_report(args, "project report", format_project_report)
}

pub(crate) fn run_report_critique(args: &ReportArgs) -> Result<()> {
    run_report(args, "critique report", format_critique_report)
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
