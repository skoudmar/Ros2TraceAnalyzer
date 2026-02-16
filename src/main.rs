#![forbid(unsafe_code, reason = "It shouldn't be needed")]

mod analysis;
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
use argsv2::{helpers::prepare_trace_paths, Args};
use color_eyre::eyre::{Context, Result};

use crate::argsv2::analysis_args::AnalysisArgs;
use crate::argsv2::chart_args::ChartArgs;
use crate::argsv2::viewer_args::ViewerArgs;

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
            // False positive - fix breaks the code
            // https://github.com/rust-lang/rust-clippy/issues/13185
            #[allow(clippy::manual_inspect)]
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

fn run_analysis<L: clap_verbosity_flag::LogLevel>(
    args: &AnalysisArgs,
    verbose: &clap_verbosity_flag::Verbosity<L>,
) -> color_eyre::eyre::Result<()> {    let trace_paths = prepare_trace_paths()?;
    let trace_paths_cstr: Vec<_> = trace_paths.iter().map(CString::as_c_str).collect();
    let mut iter = event_iterator::ProcessedEventsIter::new(&trace_paths_cstr, verbose);

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

fn run_charting(
    args: &ChartArgs,
) -> color_eyre::eyre::Result<()> {
    Ok(())
}

fn run_viewer(
    args: &ViewerArgs
) -> color_eyre::eyre::Result<()> {
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
        argsv2::TracerCommand::Analyse(analysis_args) => run_analysis(&analysis_args, &args.verbose),
        argsv2::TracerCommand::Chart(chart_args) => run_charting(&chart_args),
        argsv2::TracerCommand::Viewer(viewer_args) => run_viewer(&viewer_args),
    }
}
