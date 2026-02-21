#![forbid(unsafe_code, reason = "It shouldn't be needed")]

mod analyses;
mod argsv2;
mod events_common;
mod model;
mod processed_events;
mod processor;
mod raw_events;
mod statistics;
mod utils;
mod visualization;

use std::ffi::CString;

use argsv2::helpers::prepare_trace_paths;
use argsv2::Args;

use crate::argsv2::analysis_args::AnalysisArgs;
use crate::argsv2::chart_args::ChartArgs;
use crate::argsv2::viewer_args::ViewerArgs;

use analyses::analysis;

fn run_analysis<L: clap_verbosity_flag::LogLevel>(
    args: &AnalysisArgs,
    verbose: &clap_verbosity_flag::Verbosity<L>,
) -> color_eyre::eyre::Result<()> {
    let trace_paths = prepare_trace_paths()?;
    let trace_paths_cstr: Vec<_> = trace_paths.iter().map(CString::as_c_str).collect();

    let mut analyses = analyses::Analyses::default();

    analyses.add_analyses_from_args(args);

    analyses.analyze_trace(trace_paths_cstr, verbose)?;

    analyses.save_output(args)?;

    Ok(())
}

fn run_charting(args: &ChartArgs) -> color_eyre::eyre::Result<()> {
    Ok(())
}

fn run_viewer(args: &ViewerArgs) -> color_eyre::eyre::Result<()> {
    Ok(())
}

fn main() -> color_eyre::eyre::Result<()> {
    color_eyre::install()?;

    env_logger::Builder::new()
        .filter_level(Args::get().verbose.log_level_filter())
        .format_timestamp(None)
        .init();

    let args = Args::get();
    match &args.command {
        argsv2::TracerCommand::Analyze(analysis_args) => {
            run_analysis(&analysis_args, &args.verbose)
        }
        argsv2::TracerCommand::Chart(chart_args) => run_charting(&chart_args),
        argsv2::TracerCommand::Viewer(viewer_args) => run_viewer(&viewer_args),
    }
}
