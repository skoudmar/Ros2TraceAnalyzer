#![forbid(unsafe_code, reason = "It shouldn't be needed")]

mod analyses;
mod argsv2;
mod charting;
mod events_common;
mod extract;
mod model;
mod processed_events;
mod processor;
mod raw_events;
mod statistics;
mod utils;
mod visualization;

use core::panic;
use std::ffi::CString;
use std::io::Write;
use std::process::Stdio;

use argsv2::Args;
use argsv2::helpers::prepare_trace_paths;

use crate::argsv2::analysis_args::AnalysisArgs;
use crate::argsv2::chart_args::{ChartArgs, ChartOutputFormat};
use crate::argsv2::extract_args::ExtractArgs;
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
    let default_file_name = format!(
        "{}_{}.{}",
        args.element_id,
        args.chart.name_descriptor(),
        ChartOutputFormat::default()
    );

    let mut output_format = ChartOutputFormat::default();

    let output_file = match &args.output {
        Some(o) => {
            if o.is_dir() {
                o.join(&default_file_name)
            } else {
                match o.extension() {
                    Some(ext) => {
                        output_format = ChartOutputFormat::try_from(ext.to_str().unwrap())?;
                    }
                    _ => {
                        panic!("The selected file must have an extension")
                    }
                }
                o.to_path_buf()
            }
        }
        None => std::env::current_dir().unwrap().join(&default_file_name),
    };

    if args.overwrite || !output_file.exists() {
        let chart_data =
            extract::extract_property(&args.input, args.element_id, &args.chart.quantity)?;

        charting::render_chart(&output_file, chart_data, &args.chart, output_format)?;
    }

    println!("{}", output_file.to_string_lossy());

    Ok(())
}

fn run_viewer(args: &ViewerArgs) -> color_eyre::eyre::Result<()> {
    let viewer = args
        .viewer
        .clone()
        .unwrap_or(std::env::current_dir()?.join("./py-src/xdotviewer/main.py"));

    let tracer = args
        .tracer_exec
        .clone()
        .unwrap_or(String::from("Ros2TraceAnalyzer"));

    let input = args.input_path();

    let mut cmd = if args.viewer.is_some() {
        std::process::Command::new(viewer)
    } else {
        let mut cmd = std::process::Command::new("python");
        cmd.arg(viewer);
        cmd
    };
    cmd.arg(tracer).arg(&input);

    let mut viewer = cmd.stdin(Stdio::piped()).spawn()?;

    let mut stdin = viewer.stdin.take().unwrap();
    writeln!(stdin, "{}", extract::extract_graph(&input)?)?;
    stdin.flush()?;
    drop(stdin);

    viewer.wait()?;

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
        argsv2::TracerCommand::Chart(chart_args) => run_charting(chart_args),
        argsv2::TracerCommand::Viewer(viewer_args) => run_viewer(viewer_args),
        argsv2::TracerCommand::Extract(extract_args) => run_extract(extract_args),
    }
}
