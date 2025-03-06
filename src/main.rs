#![forbid(unsafe_code, reason = "It shouldn't be needed")]

mod analysis;
mod args;
mod events_common;
mod model;
mod processed_events;
mod processor;
mod raw_events;
mod statistics;
mod utils;
mod visualization;

use std::ffi::{CStr, CString};
use std::io::{BufWriter, Write};

use analysis::{AnalysisOutputExt, EventAnalysis};
use args::{find_trace_paths, is_trace_path, AnalysisSubcommand, Args};
use bt2_sys::iterator::MessageIterator;
use bt2_sys::logging::LogLevel;
use bt2_sys::message::BtMessageType;
use clap::Parser;
use color_eyre::eyre::{bail, ensure, Context, Result};
use color_eyre::owo_colors::OwoColorize;

struct ProcessedEventsIter<'a> {
    iter: MessageIterator,
    on_unprocessed_event: fn(raw_events::FullEvent),
    analyses: Vec<&'a mut dyn analysis::EventAnalysis>,
    processor: processor::Processor,

    // Counters
    ros_processed_events: usize,
    ros_unsupported_events: usize,
    ros_processing_failures: usize,
    other_events: usize,
    other_messages: usize,
}

fn convert(level: clap_verbosity_flag::Level) -> LogLevel {
    match level {
        clap_verbosity_flag::Level::Error => LogLevel::Error,
        clap_verbosity_flag::Level::Warn => LogLevel::Warning,
        clap_verbosity_flag::Level::Info => LogLevel::Info,
        clap_verbosity_flag::Level::Debug => LogLevel::Debug,
        clap_verbosity_flag::Level::Trace => LogLevel::Trace,
    }
}

impl<'a> ProcessedEventsIter<'a> {
    fn new<L: clap_verbosity_flag::LogLevel>(
        trace_paths: &[&CStr],
        verbosity: &clap_verbosity_flag::Verbosity<L>,
    ) -> Self {
        let log_level = convert(
            verbosity
                .log_level()
                .unwrap_or(clap_verbosity_flag::Level::Error),
        );
        Self {
            iter: MessageIterator::new(trace_paths, log_level),
            on_unprocessed_event: |_event| {}, // Do nothing by default
            analyses: Vec::new(),
            processor: processor::Processor::new(),

            ros_processed_events: 0,
            ros_unsupported_events: 0,
            ros_processing_failures: 0,
            other_events: 0,
            other_messages: 0,
        }
    }

    fn add_analysis(&mut self, analysis: &'a mut dyn analysis::EventAnalysis) {
        analysis.initialize();
        self.analyses.push(analysis);
    }

    fn set_on_unprocessed_event(&mut self, on_unprocessed_event: fn(raw_events::FullEvent)) {
        self.on_unprocessed_event = on_unprocessed_event;
    }

    fn log_counters(&self) {
        log::info!(target: "trace_counters",
            "Ros events:\n\
            - processed: {}\n\
            - failed to process: {}\n\
            - unsupported: {}\n\
            Other events: {}\n\
            Other messages: {}",
            self.ros_processed_events,
            self.ros_processing_failures,
            self.ros_unsupported_events,
            self.other_events,
            self.other_messages
        );
    }

    fn print_counters(&self) {
        println!(
            "Ros events:\n\
            - processed: {}\n\
            - failed to process: {}\n\
            - unsupported: {}\n\
            Other events: {}\n\
            Other messages: {}",
            self.ros_processed_events,
            self.ros_processing_failures,
            self.ros_unsupported_events,
            self.other_events,
            self.other_messages
        );
    }
}

impl Iterator for ProcessedEventsIter<'_> {
    type Item = Result<processed_events::FullEvent>;

    fn next(&mut self) -> Option<Self::Item> {
        for message in self.iter.by_ref() {
            let event = match message.get_type() {
                BtMessageType::StreamBeginning
                | BtMessageType::StreamEnd
                | BtMessageType::PacketBeginning
                | BtMessageType::PacketEnd => {
                    // Silently skip these messages
                    continue;
                }
                BtMessageType::DiscardedEvents
                | BtMessageType::DiscardedPackets
                | BtMessageType::MessageIteratorInactivity => {
                    log::warn!(
                        "Skipping babeltrace2 message of type {:?}",
                        message.get_type()
                    );
                    self.other_messages += 1;
                    continue;
                }
                BtMessageType::Event => {
                    let event_msg = message.into_event_msg();
                    raw_events::get_full_event(&event_msg).ok_or(event_msg)
                }
            };

            let Ok(event) = event else {
                let event_msg = event.unwrap_err();
                let event = event_msg.get_event();
                log::debug!("Unsupported event: {event:?}");

                // Skip unsupported events
                self.other_events += 1;
                continue;
            };
            match self.processor.process_raw_event(event) {
                Ok(processor::MaybeProcessed::Processed(processed)) => {
                    self.ros_processed_events += 1;
                    for analysis in &mut self.analyses {
                        (*analysis).process_event(&processed);
                    }
                    return Some(Ok(processed));
                }
                Ok(processor::MaybeProcessed::Raw(raw)) => {
                    self.ros_unsupported_events += 1;
                    (self.on_unprocessed_event)(raw);
                    continue;
                }
                Err(err) => {
                    self.ros_processing_failures += 1;
                    return Some(Err(err));
                }
            }
        }

        for analysis in &mut self.analyses {
            analysis.finalize();
        }

        None
    }
}

fn print_headline(headline: &str) {
    println!("\n{:#^60}\n", headline.green());
}

fn run_analysis<'a, A: EventAnalysis>(
    args: &Args,
    analysis: &'a mut A,
    iter: &mut ProcessedEventsIter<'a>,
) -> Result<()> {
    iter.add_analysis(analysis);

    if args.should_print_unprocessed_events() {
        iter.set_on_unprocessed_event(|event| {
            println!("Unprocessed event: {event:?}");
        });
    }

    for event in iter.by_ref() {
        let event = event.wrap_err("Failed to process event")?;
        if args.should_print_events() {
            println!("{event}");
        }
    }

    iter.log_counters();

    Ok(())
}

fn prepare_trace_paths(args: &Args) -> Result<Vec<CString>> {
    let trace_paths: Vec<_> = if args.is_exact_path() {
        args.trace_paths_cstring()
            .into_iter()
            .filter_map(|path| {
                if is_trace_path(&path) {
                    Some(path)
                } else {
                    None
                }
            })
            .collect()
    } else {
        args.trace_paths()
            .iter()
            .map(AsRef::as_ref)
            .flat_map(find_trace_paths)
            .collect()
    };

    ensure!(
        !trace_paths.is_empty(),
        "No traces found in the provided paths."
    );

    println!("Found traces:");
    for path in &trace_paths {
        println!("  {}", path.to_string_lossy());
    }

    Ok(trace_paths)
}

fn run_analysis_based_on_args(args: &Args) -> Result<()> {
    let trace_paths = prepare_trace_paths(args)?;
    let trace_paths_cstr: Vec<_> = trace_paths.iter().map(CString::as_c_str).collect();
    let mut iter = ProcessedEventsIter::new(&trace_paths_cstr, &args.verbose);

    match &args.subcommand {
        args::AnalysisSubcommand::MessageLatency { common } => {
            let mut analysis = analysis::MessageLatency::new();
            run_analysis(args, &mut analysis, &mut iter)?;
            if let Some(json_path) = &common.json_dir_path {
                analysis.write_json_to_output_dir(json_path)?;
            } else {
                print_headline(" Message Latency Analysis ");
                analysis.print_stats();
            }
        }
        args::AnalysisSubcommand::Callback { common } => {
            let mut analysis = analysis::CallbackDuration::new();
            run_analysis(args, &mut analysis, &mut iter)?;
            if let Some(json_path) = &common.json_dir_path {
                analysis.write_json_to_output_dir(json_path)?;
            } else {
                print_headline(" Callback Analysis ");
                analysis.print_stats();
            }
        }
        args::AnalysisSubcommand::Utilization { quantile } => {
            let mut analysis = analysis::CallbackDuration::new();
            run_analysis(args, &mut analysis, &mut iter)?;
            print_headline(" Utilization Analysis ");
            let utilization = analysis::Utilization::new(&analysis);
            utilization.print_stats(*quantile);
        }
        args::AnalysisSubcommand::UtilizationReal => {
            let mut analysis = analysis::CallbackDuration::new();
            run_analysis(args, &mut analysis, &mut iter)?;
            print_headline(" Real Utilization Analysis ");
            let utilization = analysis::Utilization::new(&analysis);
            utilization.print_stats_real();
        }
        args::AnalysisSubcommand::DependencyGraph {
            dep_graph_args: dep_args,
            ..
        } => {
            let mut analysis = analysis::DependencyGraph::new();
            run_analysis(args, &mut analysis, &mut iter)?;
            let dot_output = analysis.display_as_dot(
                dep_args.color,
                dep_args.thickness,
                dep_args.min_multiplier,
            );
            let mut out_file_path = dep_args.output_path.clone();
            out_file_path.push("dependency_graph.dot");
            let mut out_file = std::fs::File::create(&out_file_path)
                .wrap_err_with(|| format!("Failed to create file: {:?}", &dep_args.output_path))?;
            out_file
                .write_fmt(format_args!("{dot_output}"))
                .wrap_err_with(|| {
                    format!("Failed to write to file: {:?}", &dep_args.output_path)
                })?;
        }
        &args::AnalysisSubcommand::All { .. } => {
            unreachable!("'All' subcommand should be handled separately.");
        }
    }

    Ok(())
}

fn run_all(args: &Args) -> Result<()> {
    let AnalysisSubcommand::All {
        common,
        utilization_quantile,
        dependency_graph: dependency_graph_arg,
    } = &args.subcommand
    else {
        bail!("Expected 'all' subcommand.");
    };
    let trace_paths = prepare_trace_paths(args)?;
    let trace_paths_cstr: Vec<_> = trace_paths.iter().map(CString::as_c_str).collect();
    let mut iter = ProcessedEventsIter::new(&trace_paths_cstr, &args.verbose);

    let mut message_latency_analysis = analysis::MessageLatency::new();
    let mut callback_analysis = analysis::CallbackDuration::new();
    let mut callback_dependency_analysis = analysis::CallbackDependency::new();
    let mut spin_to_callback_analysis = analysis::MessageTakeToCallbackLatency::new();
    let mut dependency_graph = analysis::DependencyGraph::new();
    let mut spin_duration_analysis = analysis::SpinDuration::new();

    iter.add_analysis(&mut message_latency_analysis);
    iter.add_analysis(&mut callback_analysis);
    iter.add_analysis(&mut callback_dependency_analysis);
    iter.add_analysis(&mut spin_to_callback_analysis);
    iter.add_analysis(&mut dependency_graph);
    iter.add_analysis(&mut spin_duration_analysis);

    if args.should_print_unprocessed_events() {
        iter.set_on_unprocessed_event(|event| {
            println!("Unprocessed event: {event:?}");
        });
    }

    for event in &mut iter {
        let event = event.wrap_err("Failed to process event")?;
        if args.should_print_events() {
            println!("{event}");
        }
    }

    print_headline(" Trace counters ");
    iter.print_counters();

    print_headline(" Objects ");
    iter.processor.print_objects();

    print_headline(" Message Latency Analysis ");
    if let Some(json_dir) = &common.json_dir_path {
        message_latency_analysis.write_json_to_output_dir(json_dir)?;
    } else {
        message_latency_analysis.print_stats();
    }

    print_headline(" Callback Analysis ");
    if let Some(json_dir) = &common.json_dir_path {
        callback_analysis.write_json_to_output_dir(json_dir)?;
    } else {
        callback_analysis.print_stats();
    }

    let graph = callback_dependency_analysis.get_graph().unwrap();
    let callback_dependency_path = dependency_graph_arg
        .output_path
        .join("callback_dependency.dot");
    let callback_dependency_file = std::fs::File::create(&callback_dependency_path)
        .wrap_err_with(|| format!("Failed to create file: {callback_dependency_path:?}"))?;
    let mut callback_dependency_file = BufWriter::new(callback_dependency_file);
    callback_dependency_file
        .write_fmt(format_args!("{}", graph.as_dot()))
        .wrap_err_with(|| format!("Failed to write to file: {callback_dependency_path:?}"))?;

    print_headline(" Publication in callback Analysis ");
    callback_dependency_analysis
        .get_publication_in_callback_analysis()
        .print_stats();

    print_headline(" Spin to Callback latency Analysis ");
    if let Some(json_dir) = &common.json_dir_path {
        println!("Writing output to {}", json_dir.display());
        spin_to_callback_analysis.write_json_to_output_dir(json_dir)?;
    } else {
        spin_to_callback_analysis.print_stats();
    }

    print_headline(" Utilization Analysis ");
    let utilization = analysis::Utilization::new(&callback_analysis);
    utilization.print_stats(*utilization_quantile);

    print_headline(" Real Utilization Analysis ");
    utilization.print_stats_real();

    print_headline(" Spin duration Analysis ");
    if let Some(json_dir) = &common.json_dir_path {
        println!("Writing output to {}", json_dir.display());
        spin_duration_analysis.write_json_to_output_dir(json_dir)?;
    } else {
        spin_duration_analysis.print_stats();
    }

    let out_dir = &dependency_graph_arg.output_path;

    let out_file_path = out_dir.join("dependency_graph.dot");
    let out_file = std::fs::File::create(&out_file_path)
        .wrap_err_with(|| format!("Failed to create file: {out_file_path:?}"))?;
    let mut out_file = BufWriter::new(out_file);
    let dot_output = dependency_graph.display_as_dot(
        dependency_graph_arg.color,
        dependency_graph_arg.thickness,
        dependency_graph_arg.min_multiplier,
    );
    out_file
        .write_fmt(format_args!("{dot_output}"))
        .wrap_err_with(|| format!("Failed to write to file: {out_file_path:?}"))?;

    Ok(())
}

fn main() -> color_eyre::eyre::Result<()> {
    color_eyre::install()?;
    let args = Args::parse();
    args.set_globals();
    env_logger::Builder::new()
        .filter_level(args.verbose.log_level_filter())
        .format_timestamp(None)
        .init();

    if matches!(args.subcommand, AnalysisSubcommand::All { .. }) {
        run_all(&args)?;
    } else {
        run_analysis_based_on_args(&args)?;
    }

    Ok(())
}
