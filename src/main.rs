#![forbid(unsafe_code, reason = "It shouldn't be needed")]

mod analysis;
// mod args;
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
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use analysis::AnalysisOutputExt;
// use args::{find_trace_paths, is_trace_path, AnalysisSubcommand, Args};
use argsv2::{prepare_trace_paths, Args};
use color_eyre::eyre::{Context, Result};
use color_eyre::owo_colors::OwoColorize;

mod event_iterator {
    use std::ffi::CStr;

    use bt2_sys::message::BtMessageType;

    use color_eyre::eyre::Result;

    use bt2_sys::logging::LogLevel;

    use bt2_sys::iterator::MessageIterator;

    use crate::{analysis, processed_events, processor, raw_events};

    pub(crate) struct ProcessedEventsIter<'a> {
        pub(crate) iter: MessageIterator,
        pub(crate) on_unprocessed_event: fn(raw_events::FullEvent),
        pub(crate) analyses: Vec<&'a mut dyn analysis::EventAnalysis>,
        pub(crate) processor: processor::Processor,

        // Counters
        pub(crate) ros_processed_events: usize,
        pub(crate) ros_unsupported_events: usize,
        pub(crate) ros_processing_failures: usize,
        pub(crate) other_events: usize,
        pub(crate) other_messages: usize,
    }

    pub(crate) fn convert(level: clap_verbosity_flag::Level) -> LogLevel {
        match level {
            clap_verbosity_flag::Level::Error => LogLevel::Error,
            clap_verbosity_flag::Level::Warn => LogLevel::Warning,
            clap_verbosity_flag::Level::Info => LogLevel::Info,
            clap_verbosity_flag::Level::Debug => LogLevel::Debug,
            clap_verbosity_flag::Level::Trace => LogLevel::Trace,
        }
    }

    impl<'a> ProcessedEventsIter<'a> {
        pub(crate) fn new<L: clap_verbosity_flag::LogLevel>(
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

        pub(crate) fn add_analysis(&mut self, analysis: &'a mut dyn analysis::EventAnalysis) {
            analysis.initialize();
            self.analyses.push(analysis);
        }

        pub(crate) fn add_add_analysis(
            &mut self,
            analyses: impl IntoIterator<Item = &'a mut dyn analysis::EventAnalysis>,
        ) {
            self.analyses.extend(analyses.into_iter().map(|analysis| {
                analysis.initialize();
                analysis
            }));
        }

        pub(crate) fn set_on_unprocessed_event(
            &mut self,
            on_unprocessed_event: fn(raw_events::FullEvent),
        ) {
            self.on_unprocessed_event = on_unprocessed_event;
        }

        pub(crate) fn log_counters(&self) {
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

        pub(crate) fn print_counters(&self) {
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
}

// fn print_headline(headline: &str) {
//     println!("\n{:#^60}\n", headline.green());
// }

// fn run_analysis<'a, A: EventAnalysis>(
//     args: &Args,
//     analysis: &'a mut A,
//     iter: &mut event_iterator::ProcessedEventsIter<'a>,
// ) -> Result<()> {
//     iter.add_analysis(analysis);

//     if args.should_print_unprocessed_events() {
//         iter.set_on_unprocessed_event(|event| {
//             println!("Unprocessed event: {event:?}");
//         });
//     }

//     for event in iter.by_ref() {
//         let event = event.wrap_err("Failed to process event")?;
//         if args.should_print_events() {
//             println!("{event}");
//         }
//     }

//     iter.log_counters();

//     Ok(())
// }

// fn run_analysis_based_on_args(args: &Args) -> Result<()> {
//     let trace_paths = prepare_trace_paths(args)?;
//     let trace_paths_cstr: Vec<_> = trace_paths.iter().map(CString::as_c_str).collect();
//     let mut iter = event_iterator::ProcessedEventsIter::new(&trace_paths_cstr, &args.verbose);

//     match &args.subcommand {
//         args::AnalysisSubcommand::MessageLatency { common } => {
//             let mut analysis = analysis::MessageLatency::new();
//             run_analysis(args, &mut analysis, &mut iter)?;
//             if let Some(json_path) = &common.json_dir_path {
//                 analysis.write_json_to_output_dir(json_path, None)?;
//             } else {
//                 print_headline(" Message Latency Analysis ");
//                 analysis.print_stats();
//             }
//         }
//         args::AnalysisSubcommand::Callback { common } => {
//             let mut analysis = analysis::CallbackDuration::new();
//             run_analysis(args, &mut analysis, &mut iter)?;
//             if let Some(json_path) = &common.json_dir_path {
//                 analysis.write_json_to_output_dir(json_path, None)?;
//             } else {
//                 print_headline(" Callback Analysis ");
//                 analysis.print_stats();
//             }
//         }
//         args::AnalysisSubcommand::Utilization { quantile } => {
//             let mut analysis = analysis::CallbackDuration::new();
//             run_analysis(args, &mut analysis, &mut iter)?;
//             print_headline(" Utilization Analysis ");
//             let utilization = analysis::Utilization::new(&analysis);
//             utilization.print_stats(*quantile);
//         }
//         args::AnalysisSubcommand::UtilizationReal => {
//             let mut analysis = analysis::CallbackDuration::new();
//             run_analysis(args, &mut analysis, &mut iter)?;
//             print_headline(" Real Utilization Analysis ");
//             let utilization = analysis::Utilization::new(&analysis);
//             utilization.print_stats_real();
//         }
//         args::AnalysisSubcommand::DependencyGraph {
//             dep_graph_args: dep_args,
//             ..
//         } => {
//             let mut analysis = analysis::DependencyGraph::new();
//             run_analysis(args, &mut analysis, &mut iter)?;
//             let dot_output = analysis.display_as_dot(
//                 dep_args.color,
//                 dep_args.thickness,
//                 dep_args.min_multiplier,
//             );
//             let mut out_file_path = dep_args.output_path.clone();
//             out_file_path.push("dependency_graph.dot");
//             let mut out_file = std::fs::File::create(&out_file_path)
//                 .wrap_err_with(|| format!("Failed to create file: {:?}", &dep_args.output_path))?;
//             out_file
//                 .write_fmt(format_args!("{dot_output}"))
//                 .wrap_err_with(|| {
//                     format!("Failed to write to file: {:?}", &dep_args.output_path)
//                 })?;
//         }
//         &args::AnalysisSubcommand::All { .. } => {
//             unreachable!("'All' subcommand should be handled separately.");
//         }
//     }

//     Ok(())
// }

// fn run_all(args: &Args) -> Result<()> {
//     let AnalysisSubcommand::All {
//         common,
//         utilization_quantile,
//         dependency_graph: dependency_graph_arg,
//     } = &args.subcommand
//     else {
//         bail!("Expected 'all' subcommand.");
//     };
//     let trace_paths = prepare_trace_paths(args)?;
//     let trace_paths_cstr: Vec<_> = trace_paths.iter().map(CString::as_c_str).collect();
//     let mut iter = event_iterator::ProcessedEventsIter::new(&trace_paths_cstr, &args.verbose);

//     let mut message_latency_analysis = analysis::MessageLatency::new();
//     let mut callback_analysis = analysis::CallbackDuration::new();
//     let mut callback_dependency_analysis = analysis::CallbackDependency::new();
//     let mut spin_to_callback_analysis = analysis::MessageTakeToCallbackLatency::new();
//     let mut dependency_graph = analysis::DependencyGraph::new();
//     let mut spin_duration_analysis = analysis::SpinDuration::new();

//     iter.add_analysis(&mut message_latency_analysis);
//     iter.add_analysis(&mut callback_analysis);
//     iter.add_analysis(&mut callback_dependency_analysis);
//     iter.add_analysis(&mut spin_to_callback_analysis);
//     iter.add_analysis(&mut dependency_graph);
//     iter.add_analysis(&mut spin_duration_analysis);

//     if args.should_print_unprocessed_events() {
//         iter.set_on_unprocessed_event(|event| {
//             println!("Unprocessed event: {event:?}");
//         });
//     }

//     for event in &mut iter {
//         let event = event.wrap_err("Failed to process event")?;
//         if args.should_print_events() {
//             println!("{event}");
//         }
//     }

//     print_headline(" Trace counters ");
//     iter.print_counters();

//     print_headline(" Objects ");
//     iter.processor.print_objects();

//     print_headline(" Message Latency Analysis ");
//     if let Some(json_dir) = &common.json_dir_path {
//         println!("Writing output to {}", json_dir.display());
//         message_latency_analysis.write_json_to_output_dir(json_dir, None)?;
//     } else {
//         message_latency_analysis.print_stats();
//     }

//     print_headline(" Callback Analysis ");
//     if let Some(json_dir) = &common.json_dir_path {
//         println!("Writing output to {}", json_dir.display());
//         callback_analysis.write_json_to_output_dir(json_dir, None)?;
//     } else {
//         callback_analysis.print_stats();
//     }

//     let graph = callback_dependency_analysis.get_graph().unwrap();
//     let callback_dependency_path = dependency_graph_arg
//         .output_path
//         .join("callback_dependency.dot");
//     let callback_dependency_file = std::fs::File::create(&callback_dependency_path)
//         .wrap_err_with(|| format!("Failed to create file: {callback_dependency_path:?}"))?;
//     let mut callback_dependency_file = BufWriter::new(callback_dependency_file);
//     callback_dependency_file
//         .write_fmt(format_args!("{}", graph.as_dot()))
//         .wrap_err_with(|| format!("Failed to write to file: {callback_dependency_path:?}"))?;

//     print_headline(" Publication in callback Analysis ");
//     callback_dependency_analysis
//         .get_publication_in_callback_analysis()
//         .print_stats();

//     print_headline(" Spin to Callback latency Analysis ");
//     if let Some(json_dir) = &common.json_dir_path {
//         println!("Writing output to {}", json_dir.display());
//         spin_to_callback_analysis.write_json_to_output_dir(json_dir, None)?;
//     } else {
//         spin_to_callback_analysis.print_stats();
//     }

//     print_headline(" Utilization Analysis ");
//     let utilization = analysis::Utilization::new(&callback_analysis);
//     utilization.print_stats(*utilization_quantile);

//     print_headline(" Real Utilization Analysis ");
//     utilization.print_stats_real();

//     print_headline(" Spin duration Analysis ");
//     if let Some(json_dir) = &common.json_dir_path {
//         println!("Writing output to {}", json_dir.display());
//         spin_duration_analysis.write_json_to_output_dir(json_dir, None)?;
//     } else {
//         spin_duration_analysis.print_stats();
//     }

//     let out_dir = &dependency_graph_arg.output_path;

//     let out_file_path = out_dir.join("dependency_graph.dot");
//     let out_file = std::fs::File::create(&out_file_path)
//         .wrap_err_with(|| format!("Failed to create file: {out_file_path:?}"))?;
//     let mut out_file = BufWriter::new(out_file);
//     let dot_output = dependency_graph.display_as_dot(
//         dependency_graph_arg.color,
//         dependency_graph_arg.thickness,
//         dependency_graph_arg.min_multiplier,
//     );
//     out_file
//         .write_fmt(format_args!("{dot_output}"))
//         .wrap_err_with(|| format!("Failed to write to file: {out_file_path:?}"))?;

//     Ok(())
// }

pub(crate) fn get_buf_writer_for_path(path: &Path) -> Result<BufWriter<File>> {
    std::fs::create_dir_all(path.parent().unwrap())
        .wrap_err_with(|| format!("Failed to create directory: `{:?}`", path.parent()))?;
    let out_file =
        File::create(path).wrap_err_with(|| format!("Failed to create file: `{path:?}`"))?;
    Ok(BufWriter::new(out_file))
}

#[derive(Default)]
struct Analyses {
    message_latency_analysis: Option<analysis::MessageLatency>,
    callback_analysis: Option<analysis::CallbackDuration>,
    callback_dependency_analysis: Option<analysis::CallbackDependency>,
    message_take_to_callback_analysis: Option<analysis::MessageTakeToCallbackLatency>,
    dependency_graph: Option<analysis::DependencyGraph>,
    spin_duration_analysis: Option<analysis::SpinDuration>,
}

impl Analyses {
    fn all_as_mut(&mut self) -> impl Iterator<Item = &mut dyn analysis::EventAnalysis> {
        fn option_to_dyn_iter<T: analysis::EventAnalysis>(
            option: &mut Option<T>,
        ) -> impl Iterator<Item = &mut dyn analysis::EventAnalysis> {
            option
                .iter_mut()
                .map(|x| x as &mut dyn analysis::EventAnalysis)
        }

        option_to_dyn_iter(&mut self.message_latency_analysis)
            .chain(option_to_dyn_iter(&mut self.callback_analysis))
            .chain(option_to_dyn_iter(&mut self.callback_dependency_analysis))
            .chain(option_to_dyn_iter(
                &mut self.message_take_to_callback_analysis,
            ))
            .chain(option_to_dyn_iter(&mut self.dependency_graph))
            .chain(option_to_dyn_iter(&mut self.spin_duration_analysis))
    }
}

fn run() -> color_eyre::eyre::Result<()> {
    let args = Args::get();

    let trace_paths = prepare_trace_paths()?;
    let trace_paths_cstr: Vec<_> = trace_paths.iter().map(CString::as_c_str).collect();
    let mut iter = event_iterator::ProcessedEventsIter::new(&trace_paths_cstr, &args.verbose);

    let mut analysis = Analyses::default();

    if args.message_latency_enabled() {
        analysis.message_latency_analysis = Some(analysis::MessageLatency::new());
    }

    if args.callback_duration_enabled()
        || args.utilization_enabled()
        || args.real_utilization_enabled()
    {
        analysis.callback_analysis = Some(analysis::CallbackDuration::new());
    }

    if args.callback_dependency_enabled() || args.callback_publications_enabled() {
        analysis.callback_dependency_analysis = Some(analysis::CallbackDependency::new());
    }

    if args.message_take_to_callback_latency_enabled() {
        analysis.message_take_to_callback_analysis =
            Some(analysis::MessageTakeToCallbackLatency::new());
    }

    if args.dependency_graph_enabled() {
        analysis.dependency_graph = Some(analysis::DependencyGraph::new());
    }

    if args.spin_duration_enabled() {
        analysis.spin_duration_analysis = Some(analysis::SpinDuration::new());
    }

    iter.add_add_analysis(analysis.all_as_mut());

    iter.set_on_unprocessed_event(|event| {
        log::debug!("Unprocessed event: {event:?}");
    });

    for event in &mut iter {
        let event = event.wrap_err("Failed to process event")?;
        log::trace!("{event}");
    }

    iter.log_counters();

    if let Some(path) = args.message_latency_path() {
        let analysis = analysis.message_latency_analysis.as_ref().unwrap();
        analysis.write_json_to_output_dir(&path)?;
    }

    if let Some(path) = args.callback_duration_path() {
        let analysis = analysis.callback_analysis.as_ref().unwrap();
        analysis.write_json_to_output_dir(&path)?;
    }

    if let Some(path) = args.callback_publications_path() {
        let analysis = analysis.callback_dependency_analysis.as_ref().unwrap();
        let analysis = analysis.get_publication_in_callback_analysis();
        let mut writer = get_buf_writer_for_path(&path)?;
        analysis
            .write_stats(&mut writer)
            .wrap_err("Failed to write publication in callback stats")?;

        // TODO: Implement JSON output
        // analysis.write_json_to_output_dir(&path, None)?;
    }

    if let Some(path) = args.callback_dependency_path() {
        let analysis = analysis.callback_dependency_analysis.as_ref().unwrap();

        let graph = analysis.get_graph().unwrap();
        let mut writer = get_buf_writer_for_path(&path)?;
        writer
            .write_fmt(format_args!("{}", graph.as_dot()))
            .wrap_err("Failed to write dependency graph")?;
    }

    if let Some(path) = args.message_take_to_callback_latency_path() {
        let analysis = analysis.message_take_to_callback_analysis.as_ref().unwrap();
        analysis
            .write_json_to_output_dir(&path)
            .wrap_err("Failed to write message take to callback latency stats")?;
    }

    if let Some(path) = args.utilization_path() {
        let analysis = analysis.callback_analysis.as_ref().unwrap();
        let utilization = analysis::Utilization::new(analysis);

        let mut writer = get_buf_writer_for_path(&path)?;
        utilization
            .write_stats(&mut writer, args.utilization_quantile())
            .wrap_err("Failed to write utilization stats")?;

        // TODO: Implement JSON output
        // utilization.write_json_to_output_dir(&path, None)?;
    }

    if let Some(path) = args.real_utilization_path() {
        let analysis = analysis.callback_analysis.as_ref().unwrap();
        let utilization = analysis::Utilization::new(analysis);

        let mut writer = get_buf_writer_for_path(&path)?;
        utilization
            .write_stats_real(&mut writer)
            .wrap_err("Failed to write real utilization stats")?;

        // TODO: Implement JSON output
        // utilization.write_json_to_output_dir(&path)?;
    }

    if let Some(path) = args.spin_duration_path() {
        let analysis = analysis.spin_duration_analysis.as_ref().unwrap();
        analysis
            .write_json_to_output_dir(&path)
            .wrap_err("Failed to write spin duration stats")?;
    }

    if let Some(path) = args.dependency_graph_path() {
        let analysis = analysis.dependency_graph.as_ref().unwrap();
        let dot_output =
            analysis.display_as_dot(args.color(), args.thickness(), args.min_multiplier());
        let mut writer = get_buf_writer_for_path(&path)?;
        writer
            .write_fmt(format_args!("{dot_output}"))
            .wrap_err("Failed to write dependency graph")?;
    }

    Ok(())
}

fn main() -> color_eyre::eyre::Result<()> {
    color_eyre::install()?;

    env_logger::Builder::new()
        .filter_level(Args::get().verbose.log_level_filter())
        .format_timestamp(None)
        .init();

    run()
}
