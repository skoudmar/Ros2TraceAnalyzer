#![forbid(unsafe_code, reason = "It shouldn't be needed")]

mod analyses;
mod argsv2;
mod events_common;
mod extract;
mod model;
mod plotting;
mod processed_events;
mod processor;
mod raw_events;
mod statistics;
mod utils;
mod visualization;

use std::ffi::CString;
use std::io::Write;

use argsv2::Args;
use argsv2::helpers::prepare_trace_paths;

use crate::argsv2::analysis_args::AnalysisArgs;
use crate::argsv2::extract_args::ExtractArgs;
use crate::argsv2::plot_args::{PlotArgs, PlotOutputFormat};
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

fn run_plotting(args: &PlotArgs) -> color_eyre::eyre::Result<()> {
    let mut output = match &args.output {
        Some(o) => {
            if args.overwrite {
                Box::new(std::fs::File::create(o)?)
            } else {
                Box::new(std::fs::File::create_new(o)?)
            }
        }
        None => Box::new(std::io::stdout()) as Box<dyn Write>,
    };

    let output_format = args
        .output
        .clone()
        .map(|p| match p.extension() {
            Some(ext) => PlotOutputFormat::try_from(ext.to_str().unwrap()).unwrap_or_default(),
            None => PlotOutputFormat::default(),
        })
        .unwrap_or_default();

    if args.overwrite || !args.output.clone().map(|p| p.exists()).unwrap_or(false) {
        let plot_data =
            extract::extract_property(&args.input, args.element_id, &args.plot.quantity)?;

        plotting::render_plot(&mut output, plot_data, &args.plot, output_format)?;
    }

    Ok(())
}

fn run_viewer(_args: &ViewerArgs) -> color_eyre::eyre::Result<()> {
    Ok(())
}

fn run_extract(args: &ExtractArgs) -> color_eyre::eyre::Result<()> {
    let source_file = args.input_path();
    let mut output: Box<dyn Write> = args
        .output_path()
        .map(|p| std::fs::File::create(p).map(|f| Box::new(f) as Box<dyn Write>))
        .unwrap_or_else(|| Ok(Box::new(std::io::stdout()) as Box<dyn Write>))?;

    match args.content() {
        argsv2::extract_args::ExtractContentArgs::Graph => {
            let graph = extract::extract_graph(&source_file)?;

            writeln!(output, "{graph}")?;
        }
        argsv2::extract_args::ExtractContentArgs::Property(args) => {
            let data = extract::extract_property(&source_file, args.element_id(), args.property())?;

            data.export(&mut output)?;
        }
    }

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
        argsv2::TracerCommand::Analyze(analysis_args) => run_analysis(analysis_args, &args.verbose),
        argsv2::TracerCommand::Plot(plot_args) => run_plotting(plot_args),
        argsv2::TracerCommand::Viewer(viewer_args) => run_viewer(viewer_args),
        argsv2::TracerCommand::Extract(extract_args) => run_extract(extract_args),
    }
}
