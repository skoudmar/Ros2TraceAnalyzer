#![forbid(unsafe_code, reason = "It shouldn't be needed")]

mod analysis;
mod args;
mod events_common;
mod graphviz_export;
mod model;
mod processed_events;
mod processor;
mod raw_events;
mod utils;

use std::ffi::CStr;
use std::io::Write;

use analysis::AnalysisOutputExt;
use args::{find_trace_paths, is_trace_path, Args, OutputFormat};
use bt2_sys::iterator::MessageIterator;
use bt2_sys::message::BtMessageType;
use clap::Parser;
use color_eyre::eyre::{bail, ensure, Context, OptionExt, Result};
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

impl<'a> ProcessedEventsIter<'a> {
    fn new(trace_path: &CStr) -> Self {
        Self {
            iter: MessageIterator::new(trace_path),
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

impl<'a> Iterator for ProcessedEventsIter<'a> {
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
                    println!("Skipping message of type {:?}", message.get_type());
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
                eprintln!("Unsupported event: {event:?}");

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

fn main() -> color_eyre::eyre::Result<()> {
    color_eyre::install()?;

    let args = Args::parse();

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
        "No traces found in the provided path."
    );

    println!("Found traces:");
    for path in &trace_paths {
        println!("  {}", path.to_string_lossy());
    }

    if trace_paths.len() > 1 {
        bail!("Processing multiple traces is not supported yet.");
    }

    let mut message_latency_analysis = analysis::MessageLatency::new();
    let mut callback_duration_analysis = analysis::CallbackDuration::new();
    let mut callback_dependency_analysis = analysis::CallbackDependency::new();
    let mut spin_to_callback_analysis = analysis::MessageTakeToCallbackLatency::new();
    let mut dependency_graph = analysis::DependencyGraph::new();

    let mut iter = ProcessedEventsIter::new(&trace_paths[0]);

    iter.add_analysis(&mut message_latency_analysis);
    iter.add_analysis(&mut callback_duration_analysis);
    iter.add_analysis(&mut callback_dependency_analysis);
    iter.add_analysis(&mut spin_to_callback_analysis);
    iter.add_analysis(&mut dependency_graph);

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

    if args.output_format() == OutputFormat::Text {
        print_headline(" Objects ");
        iter.processor.print_objects();

        print_headline(" Analysis ");
        message_latency_analysis.print_stats();

        print_headline(" Analysis ");
        callback_duration_analysis.print_stats();

        print_headline(" Analysis ");
        callback_dependency_analysis
            .get_graph()
            .unwrap()
            .print_graph();

        print_headline(" Analysis ");
        callback_dependency_analysis
            .get_publication_in_callback_analysis()
            .print_stats();

        print_headline(" Analysis ");
        spin_to_callback_analysis.print_stats();

        if let Some(out_dir) = args.output_dir() {
            let out_file_path = out_dir.join("dependency_graph.dot");
            let mut out_file = std::fs::File::create(&out_file_path)
                .wrap_err_with(|| format!("Failed to create file: {out_file_path:?}"))?;
            let dot_output = dependency_graph.display_as_dot();
            out_file
                .write_fmt(format_args!("{dot_output}"))
                .wrap_err_with(|| format!("Failed to write to file: {out_file_path:?}"))?;
        }
    } else if args.output_format() == OutputFormat::Csv {
        let output_dir = args
            .output_dir()
            .ok_or_eyre("Output directory not specified")?;

        callback_duration_analysis
            .write_csv_to_output_dir(output_dir)
            .wrap_err("Failed to write CSV")
            .wrap_err("Callback duration analysis serialization error")?;

        todo!("Not all analyses are serialized yet");
    }

    Ok(())
}
