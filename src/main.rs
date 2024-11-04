#![forbid(unsafe_code, reason = "It shoudn't be needed")]

mod analysis;
mod args;
mod events_common;
mod model;
mod processed_events;
mod processor;
mod raw_events;
mod utils;

use std::ffi::CStr;

use args::{find_trace_paths, is_trace_path, Args};
use bt2_sys::iterator::MessageIterator;
use bt2_sys::message::BtMessageType;
use clap::Parser;
use color_eyre::eyre::{bail, ensure};
use color_eyre::owo_colors::OwoColorize;

struct ProcessedEventsIter<'a> {
    iter: MessageIterator,
    on_unprocessed_event: fn(raw_events::FullEvent),
    analyses: Vec<&'a mut dyn analysis::EventAnalysis>,
    processor: processor::Processor,
}

impl<'a> ProcessedEventsIter<'a> {
    fn new(trace_path: &CStr) -> Self {
        Self {
            iter: MessageIterator::new(trace_path),
            on_unprocessed_event: |_event| {}, // Do nothing by default
            analyses: Vec::new(),
            processor: processor::Processor::new(),
        }
    }

    fn add_analysis(&mut self, analysis: &'a mut dyn analysis::EventAnalysis) {
        analysis.initialize();
        self.analyses.push(analysis);
    }

    fn set_on_unprocessed_event(&mut self, on_unprocessed_event: fn(raw_events::FullEvent)) {
        self.on_unprocessed_event = on_unprocessed_event;
    }
}

impl<'a> Iterator for ProcessedEventsIter<'a> {
    type Item = processed_events::FullEvent;

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
                    continue;
                }
                BtMessageType::Event => raw_events::get_full_event(&message.into_event_msg()),
            };

            let Some(event) = event else {
                // Skip unsupported events
                continue;
            };
            match self.processor.process_raw_event(event) {
                processor::MaybeProccessed::Processed(proccessed) => {
                    for analysis in &mut self.analyses {
                        (*analysis).process_event(&proccessed);
                    }
                    return Some(proccessed);
                }
                processor::MaybeProccessed::Raw(raw) => {
                    (self.on_unprocessed_event)(raw);
                    continue;
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

    let trace_path = args.trace_path_cstring();

    let trace_paths = if args.is_exact_path() {
        [trace_path]
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
        find_trace_paths(args.trace_path())
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

    let mut iter = ProcessedEventsIter::new(&trace_paths[0]);

    iter.add_analysis(&mut message_latency_analysis);
    iter.add_analysis(&mut callback_duration_analysis);
    iter.add_analysis(&mut callback_dependency_analysis);

    if args.should_print_unprocessed_events() {
        iter.set_on_unprocessed_event(|event| {
            println!("Unprocessed event: {event:?}");
        });
    }

    for event in &mut iter {
        if args.should_print_events() {
            println!("{event}");
        }
    }

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

    Ok(())
}
