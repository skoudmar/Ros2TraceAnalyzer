use std::ffi::CStr;
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

use bt2_sys::message::BtMessageType;

use color_eyre::eyre::{Context, Result};

use bt2_sys::logging::LogLevel;

use bt2_sys::iterator::MessageIterator;

use crate::analyses::analysis;
use crate::{processed_events, processor, raw_events};

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

pub(crate) fn convert(level: clap_verbosity_flag::log::Level) -> LogLevel {
    match level {
        clap_verbosity_flag::log::Level::Error => LogLevel::Error,
        clap_verbosity_flag::log::Level::Warn => LogLevel::Warning,
        clap_verbosity_flag::log::Level::Info => LogLevel::Info,
        clap_verbosity_flag::log::Level::Debug => LogLevel::Debug,
        clap_verbosity_flag::log::Level::Trace => LogLevel::Trace,
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
                .unwrap_or(clap_verbosity_flag::log::Level::Error),
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

pub(crate) fn get_buf_writer_for_path(path: &Path) -> Result<BufWriter<File>> {
    std::fs::create_dir_all(path.parent().unwrap())
        .wrap_err_with(|| format!("Failed to create directory: `{:?}`", path.parent()))?;
    let out_file =
        File::create(path).wrap_err_with(|| format!("Failed to create file: `{path:?}`"))?;
    Ok(BufWriter::new(out_file))
}
