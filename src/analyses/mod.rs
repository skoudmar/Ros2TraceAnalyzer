use std::ffi::CStr;
use std::io::Write;

use color_eyre::eyre::Context;

use crate::analyses::analysis::AnalysisOutputExt;
use crate::analyses::event_iterator::get_buf_writer_for_path;
use crate::argsv2::analysis_args::AnalysisArgs;

pub mod analysis;
pub mod event_iterator;

#[derive(Default)]
pub struct Analyses {
    message_latency_analysis: Option<analysis::MessageLatency>,
    callback_analysis: Option<analysis::CallbackDuration>,
    callback_dependency_analysis: Option<analysis::CallbackDependency>,
    message_take_to_callback_analysis: Option<analysis::MessageTakeToCallbackLatency>,
    dependency_graph: Option<analysis::DependencyGraph>,
    spin_duration_analysis: Option<analysis::SpinDuration>,
}

impl Analyses {
    pub fn all_as_mut(&mut self) -> impl Iterator<Item = &mut dyn analysis::EventAnalysis> {
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

    pub fn add_analyses_from_args(&mut self, args: &crate::argsv2::analysis_args::AnalysisArgs) {
        if args.message_latency_enabled() {
            self.message_latency_analysis = Some(analysis::MessageLatency::new());
        }

        if args.callback_duration_enabled()
            || args.utilization_enabled()
            || args.real_utilization_enabled()
        {
            self.callback_analysis = Some(analysis::CallbackDuration::new());
        }

        if args.callback_dependency_enabled() || args.callback_publications_enabled() {
            self.callback_dependency_analysis = Some(analysis::CallbackDependency::new());
        }

        if args.message_take_to_callback_latency_enabled() {
            self.message_take_to_callback_analysis =
                Some(analysis::MessageTakeToCallbackLatency::new());
        }

        if args.dependency_graph_enabled() {
            self.dependency_graph = Some(analysis::DependencyGraph::new());
        }

        if args.spin_duration_enabled() {
            self.spin_duration_analysis = Some(analysis::SpinDuration::new());
        }
    }

    pub fn analyze_trace<L: clap_verbosity_flag::LogLevel>(
        &mut self,
        trace_paths: Vec<&CStr>,
        verbose: &clap_verbosity_flag::Verbosity<L>,
    ) -> color_eyre::eyre::Result<()> {
        let mut iter = event_iterator::ProcessedEventsIter::new(&trace_paths, verbose);

        iter.add_add_analysis(self.all_as_mut());

        iter.set_on_unprocessed_event(|event| {
            log::debug!("Unprocessed event: {event:?}");
        });

        for event in &mut iter {
            let event = event.wrap_err("Failed to process event")?;
            log::trace!("{event}");
        }

        iter.log_counters();

        Ok(())
    }

    pub fn save_output(&self, args: &AnalysisArgs) -> color_eyre::eyre::Result<()> {
        if let Some(path) = args.message_latency_path() {
            let analysis = self.message_latency_analysis.as_ref().unwrap();
            analysis.write_json_to_output_dir(&path)?;
        }

        if let Some(path) = args.callback_duration_path() {
            let analysis = self.callback_analysis.as_ref().unwrap();
            analysis.write_json_to_output_dir(&path)?;
        }

        if let Some(path) = args.callback_publications_path() {
            let analysis = self.callback_dependency_analysis.as_ref().unwrap();
            let analysis = analysis.get_publication_in_callback_analysis();
            let mut writer = get_buf_writer_for_path(&path)?;
            analysis
                .write_stats(&mut writer)
                .wrap_err("Failed to write publication in callback stats")?;

            // TODO: Implement JSON output
            // analysis.write_json_to_output_dir(&path, None)?;
        }

        if let Some(path) = args.callback_dependency_path() {
            let analysis = self.callback_dependency_analysis.as_ref().unwrap();

            let graph = analysis.get_graph().unwrap();
            let mut writer = get_buf_writer_for_path(&path)?;
            writer
                .write_fmt(format_args!("{}", graph.as_dot()))
                .wrap_err("Failed to write dependency graph")?;
        }

        if let Some(path) = args.message_take_to_callback_latency_path() {
            let analysis = self.message_take_to_callback_analysis.as_ref().unwrap();
            analysis
                .write_json_to_output_dir(&path)
                .wrap_err("Failed to write message take to callback latency stats")?;
        }

        if let Some(path) = args.utilization_path() {
            let analysis = self.callback_analysis.as_ref().unwrap();
            let utilization = analysis::Utilization::new(analysis);

            let mut writer = get_buf_writer_for_path(&path)?;
            utilization
                .write_stats(&mut writer, args.utilization_quantile())
                .wrap_err("Failed to write utilization stats")?;

            // TODO: Implement JSON output
            // utilization.write_json_to_output_dir(&path, None)?;
        }

        if let Some(path) = args.real_utilization_path() {
            let analysis = self.callback_analysis.as_ref().unwrap();
            let utilization = analysis::Utilization::new(analysis);

            let mut writer = get_buf_writer_for_path(&path)?;
            utilization
                .write_stats_real(&mut writer)
                .wrap_err("Failed to write real utilization stats")?;

            // TODO: Implement JSON output
            // utilization.write_json_to_output_dir(&path)?;
        }

        if let Some(path) = args.spin_duration_path() {
            let analysis = self.spin_duration_analysis.as_ref().unwrap();
            analysis
                .write_json_to_output_dir(&path)
                .wrap_err("Failed to write spin duration stats")?;
        }

        if let Some(path) = args.dependency_graph_path() {
            let analysis = self.dependency_graph.as_ref().unwrap();
            let dot_output =
                analysis.display_as_dot(args.color(), args.thickness(), args.min_multiplier());
            let mut writer = get_buf_writer_for_path(&path)?;
            writer
                .write_fmt(format_args!("{dot_output}"))
                .wrap_err("Failed to write dependency graph")?;
        }

        Ok(())
    }
}
