use std::borrow::Cow;
use std::ffi::{CStr, CString};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use bt2_sys::graph::component::BtComponentType;
use bt2_sys::query::support_info;
use clap::builder::ArgPredicate;
use clap::Parser;
use clap_verbosity_flag::{Verbosity, WarnLevel};
use color_eyre::eyre::ensure;
use walkdir::WalkDir;

use crate::statistics::Quantile;

pub static CLI_ARGS: OnceLock<Args> = OnceLock::new();

mod filenames {
    pub const DEPENDENCY_GRAPH: &str = "dependency_graph.dot";
    pub const MESSAGE_LATENCY: &str = "message_latency.json";
    pub const CALLBACK_DURATION: &str = "callback_duration.json";
    pub const CALLBACK_PUBLICATIONS: &str = "callback_publications.txt";
    pub const CALLBACK_DEPENDENCY: &str = "callback_dependency.dot";
    pub const MESSAGE_TAKE_TO_CALLBACK_LATENCY: &str = "message_take_to_callback_latency.json";
    pub const UTILIZATION: &str = "utilization.json";
    pub const REAL_UTILIZATION: &str = "real_utilization.json";
    pub const SPIN_DURATION: &str = "spin_duration.json";
}

#[derive(Debug, Clone, Parser)]
#[allow(clippy::struct_excessive_bools)]
pub struct Args {
    #[command(flatten)]
    pub verbose: Verbosity<WarnLevel>,

    /// If set to true, only the directory specified by `TRACE_PATH` is searched for traces, not its subdirectories.
    #[arg(long)]
    exact_trace_path: bool,

    /// Directory to write output files
    #[arg(long, short = 'o')]
    out_dir: Option<PathBuf>,

    /// Run all analyses with their default output filenames
    ///
    /// The output `filename` can be changed by specific analysis option.
    ///
    /// This is enabled by default unless specific analysis option is provided.
    #[arg(long,
        default_value = "true",
        default_value_ifs([
            ("dependency_graph", ArgPredicate::IsPresent, "false"),
            ("message_latency", ArgPredicate::IsPresent, "false"),
            ("callback_duration", ArgPredicate::IsPresent, "false"),
            ("callback_publications", ArgPredicate::IsPresent, "false"),
            ("callback_dependency", ArgPredicate::IsPresent, "false"),
            ("message_take_to_callback_latency", ArgPredicate::IsPresent, "false"),
            ("utilization", ArgPredicate::IsPresent, "false"),
            ("real_utilization", ArgPredicate::IsPresent, "false"),
            ("spin_duration", ArgPredicate::IsPresent, "false"),
            ]))]
    all: bool,

    /// Construct a detailed dependency graph with timing statistics in DOT format.
    #[arg(long, value_name = "FILENAME", default_missing_value = filenames::DEPENDENCY_GRAPH, num_args = 0..=1, require_equals = true, default_value_if("all", "true", filenames::DEPENDENCY_GRAPH))]
    dependency_graph: Option<PathBuf>,

    /// Analyze the latency of messages
    #[arg(long, value_name = "FILENAME", default_missing_value = filenames::MESSAGE_LATENCY, num_args = 0..=1, require_equals = true, default_value_if("all", "true", filenames::MESSAGE_LATENCY))]
    message_latency: Option<PathBuf>,

    /// Analyze the callback duration and inter-arrival time.
    #[arg(long, value_name = "FILENAME", default_missing_value = filenames::CALLBACK_DURATION, num_args = 0..=1, require_equals = true, default_value_if("all", "true", filenames::CALLBACK_DURATION))]
    callback_duration: Option<PathBuf>,

    /// Analyze the publications made by callbacks
    #[arg(long, value_name = "FILENAME", default_missing_value = filenames::CALLBACK_PUBLICATIONS, num_args = 0..=1, require_equals = true, default_value_if("all", "true", filenames::CALLBACK_PUBLICATIONS))]
    callback_publications: Option<PathBuf>,

    /// Generate a callback dependency graph in DOT format
    #[arg(long, value_name = "FILENAME", default_missing_value = filenames::CALLBACK_DEPENDENCY, num_args = 0..=1, require_equals = true, default_value_if("all", "true", filenames::CALLBACK_DEPENDENCY))]
    callback_dependency: Option<PathBuf>,

    /// Analyze the latency between message take and callback execution
    #[arg(long, value_name = "FILENAME", default_missing_value = filenames::MESSAGE_TAKE_TO_CALLBACK_LATENCY, num_args = 0..=1, require_equals = true, default_value_if("all", "true", filenames::MESSAGE_TAKE_TO_CALLBACK_LATENCY))]
    message_take_to_callback_latency: Option<PathBuf>,

    /// Analyze system utilization based on quantile callback durations
    #[arg(long, value_name = "FILENAME", default_missing_value = filenames::UTILIZATION, num_args = 0..=1, require_equals = true, default_value_if("all", "true", filenames::UTILIZATION))]
    utilization: Option<PathBuf>,

    /// Analyze system utilization based on real execution times
    #[arg(long, value_name = "FILENAME", default_missing_value = filenames::REAL_UTILIZATION, num_args = 0..=1, require_equals = true, default_value_if("all", "true", filenames::REAL_UTILIZATION))]
    real_utilization: Option<PathBuf>,

    /// Analyze the duration of executor spins
    #[arg(long, value_name = "FILENAME", default_missing_value = filenames::SPIN_DURATION, num_args = 0..=1, require_equals = true, default_value_if("all", "true", filenames::SPIN_DURATION))]
    spin_duration: Option<PathBuf>,

    /// Quantiles to compute for the latency and duration analysis.
    ///
    /// The quantiles must be in the range [0, 1].
    ///
    /// If not specified, the default quantiles are:
    /// 0 (minimum), 0.10, 0.5 (median), 0.90, 0.99, 1 (maximum)
    #[arg(
        long,
        value_parser,
        value_delimiter = ',',
        default_value = "0,0.10,0.5,0.90,0.99,1"
    )]
    quantiles: Vec<Quantile>,

    /// Callback duration quantile to use for utilization analysis
    #[arg(long, value_parser, default_value = "0.9")]
    utilization_quantile: Quantile,

    /// Set the edge thickness in dependency graph based on its median latency.
    #[arg(long)]
    thickness: bool,

    /// Color edge in dependency graph based on its median latency.
    #[arg(long)]
    color: bool,

    /// Minimum multiplier for edge coloring or thickness.
    ///
    /// Can be any positive number.
    ///
    /// The minimum multiplier is used to set the maximum value in gradients
    /// to be at least `min-multiplier` times the minimum value.
    ///
    /// The gradient range is exactly [minimum value, max(maximum value, minimum value * `min_multiplier`)].
    #[arg(long, default_value = "5.0")]
    min_multiplier: f64,

    /// Paths to directories to search for the trace to analyze
    ///
    /// All subdirectories are automatically searched too.
    #[arg(value_parser/*  = PathBufValueParser::new().try_map(|p| to_directory_path_buf(p, false)) */, num_args = 1.., required = true)]
    trace_paths: Vec<PathBuf>,
}

impl Args {
    pub fn get() -> &'static Args {
        CLI_ARGS.get_or_init(|| Self::parse())
    }

    pub fn trace_paths(&self) -> &[PathBuf] {
        &self.trace_paths
    }

    pub fn trace_paths_cstring(&self) -> Vec<CString> {
        self.trace_paths
            .iter()
            .map(|p| CString::new(p.to_str().unwrap()).unwrap())
            .collect::<Vec<_>>()
    }

    pub const fn is_exact_path(&self) -> bool {
        self.exact_trace_path
    }

    fn concatenate_with_out_path<'a>(&'a self, path: &'a Path) -> Cow<'a, Path> {
        if path.is_absolute() {
            path.into()
        } else if let Some(out_dir) = &self.out_dir {
            out_dir.join(path).into()
        } else {
            path.into()
        }
    }

    pub fn dependency_graph_enabled(&self) -> bool {
        self.dependency_graph.is_some()
    }

    pub fn message_latency_enabled(&self) -> bool {
        self.message_latency.is_some()
    }

    pub fn callback_duration_enabled(&self) -> bool {
        self.callback_duration.is_some()
    }

    pub fn callback_publications_enabled(&self) -> bool {
        self.callback_publications.is_some()
    }

    pub fn callback_dependency_enabled(&self) -> bool {
        self.callback_dependency.is_some()
    }

    pub fn message_take_to_callback_latency_enabled(&self) -> bool {
        self.message_take_to_callback_latency.is_some()
    }

    pub fn utilization_enabled(&self) -> bool {
        self.utilization.is_some()
    }

    pub fn real_utilization_enabled(&self) -> bool {
        self.real_utilization.is_some()
    }

    pub fn spin_duration_enabled(&self) -> bool {
        self.spin_duration.is_some()
    }

    pub fn dependency_graph_path(&self) -> Option<Cow<Path>> {
        self.dependency_graph
            .as_ref()
            .map(|p| self.concatenate_with_out_path(p))
    }

    pub fn message_latency_path(&self) -> Option<Cow<Path>> {
        self.message_latency
            .as_ref()
            .map(|p| self.concatenate_with_out_path(p))
    }

    pub fn callback_duration_path(&self) -> Option<Cow<Path>> {
        self.callback_duration
            .as_ref()
            .map(|p| self.concatenate_with_out_path(p))
    }

    pub fn callback_publications_path(&self) -> Option<Cow<Path>> {
        self.callback_publications
            .as_ref()
            .map(|p| self.concatenate_with_out_path(p))
    }

    pub fn callback_dependency_path(&self) -> Option<Cow<Path>> {
        self.callback_dependency
            .as_ref()
            .map(|p| self.concatenate_with_out_path(p))
    }

    pub fn message_take_to_callback_latency_path(&self) -> Option<Cow<Path>> {
        self.message_take_to_callback_latency
            .as_ref()
            .map(|p| self.concatenate_with_out_path(p))
    }

    pub fn utilization_path(&self) -> Option<Cow<Path>> {
        self.utilization
            .as_ref()
            .map(|p| self.concatenate_with_out_path(p))
    }

    pub fn real_utilization_path(&self) -> Option<Cow<Path>> {
        self.real_utilization
            .as_ref()
            .map(|p| self.concatenate_with_out_path(p))
    }

    pub fn spin_duration_path(&self) -> Option<Cow<Path>> {
        self.spin_duration
            .as_ref()
            .map(|p| self.concatenate_with_out_path(p))
    }

    pub fn quantiles(&self) -> &[Quantile] {
        &self.quantiles
    }

    pub fn utilization_quantile(&self) -> Quantile {
        self.utilization_quantile
    }

    pub const fn thickness(&self) -> bool {
        self.thickness
    }

    pub const fn color(&self) -> bool {
        self.color
    }

    pub const fn min_multiplier(&self) -> f64 {
        self.min_multiplier
    }
}

// Valid trace path should have a weight set to 0.75 so we set the threshold slightly lower.
const TRACE_PATH_LIKELIHOOD_THRESHOLD: f64 = 0.74;

pub fn is_trace_path(path: &CStr) -> bool {
    let support_info_query =
        support_info::Query::new_prepared("ctf", "fs", BtComponentType::Source)
            .expect("Failed to prepare support info query");

    let path_cstr = CString::new(path.to_str().unwrap()).unwrap();

    let result = support_info_query
        .query(bt2_sys::query::SupportInfoParams::Directory(&path_cstr))
        .expect("Failed to query support info");

    result.weight() > TRACE_PATH_LIKELIHOOD_THRESHOLD
}

pub fn find_trace_paths(search_path: &Path) -> Vec<CString> {
    let support_info_query =
        support_info::Query::new_prepared("ctf", "fs", BtComponentType::Source)
            .expect("Failed to prepare support info query");

    let mut trace_paths = Vec::new();
    for dir in WalkDir::new(search_path)
        .into_iter()
        .filter_entry(|e| e.file_type().is_dir())
    {
        let dir = dir.expect("Failed to read directory");
        let path = dir.path();
        let path_cstr = CString::new(path.to_str().unwrap()).unwrap();

        let result = support_info_query
            .query(bt2_sys::query::SupportInfoParams::Directory(&path_cstr))
            .expect("Failed to query support info");

        if result.weight() > TRACE_PATH_LIKELIHOOD_THRESHOLD {
            trace_paths.push(path_cstr);
        }
    }

    trace_paths
}

pub fn prepare_trace_paths() -> color_eyre::Result<Vec<CString>> {
    let trace_paths: Vec<_> = if Args::get().is_exact_path() {
        Args::get()
            .trace_paths_cstring()
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
        Args::get()
            .trace_paths()
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

#[cfg(test)]
mod test {
    use clap::{CommandFactory, Parser};
    use std::path::Path;

    use super::*;

    #[test]
    #[ignore]
    fn print_help() {
        Args::command().print_help().unwrap();
    }

    #[test]
    fn verify_cli() {
        Args::command().debug_assert();
    }

    #[test]
    fn test_basic_args_parsing() {
        let args = Args::try_parse_from(["program", "/tmp/trace"])
            .unwrap_or_else(|e| panic!("Failed to parse arguments: {e}"));

        assert_eq!(args.trace_paths.len(), 1);
        assert_eq!(args.trace_paths[0], PathBuf::from("/tmp/trace"));
        assert!(!args.exact_trace_path);
        assert!(args.all);
        assert_eq!(args.out_dir, None);
    }

    #[test]
    fn test_multiple_trace_paths() {
        let args = Args::try_parse_from(["program", "/tmp/trace1", "/tmp/trace2"])
            .unwrap_or_else(|e| panic!("Failed to parse arguments: {e}"));

        assert_eq!(args.trace_paths.len(), 2);
        assert_eq!(args.trace_paths[0], PathBuf::from("/tmp/trace1"));
        assert_eq!(args.trace_paths[1], PathBuf::from("/tmp/trace2"));
    }

    #[test]
    fn test_exact_trace_path_flag() {
        let args = Args::try_parse_from(["program", "--exact-trace-path", "/tmp/trace"])
            .unwrap_or_else(|e| panic!("Failed to parse arguments: {e}"));

        assert!(args.exact_trace_path);
        assert!(args.is_exact_path());
    }

    #[test]
    fn test_output_directory() {
        // This test will be skipped if /tmp doesn't exist
        if !Path::new("/tmp").exists() {
            return;
        }

        let args = Args::try_parse_from(["program", "-o", "/tmp", "/tmp/trace"])
            .unwrap_or_else(|e| panic!("Failed to parse arguments: {e}"));

        assert_eq!(args.out_dir, Some(PathBuf::from("/tmp")));
    }

    #[test]
    fn test_quantiles_parsing() {
        let args =
            Args::try_parse_from(["program", "--quantiles", "0,0.25,0.5,0.75,1", "/tmp/trace"])
                .unwrap_or_else(|e| panic!("Failed to parse arguments: {e}"));

        assert_eq!(args.quantiles.len(), 5);
        assert_eq!(args.quantiles[0], 0.0.try_into().unwrap());
        assert_eq!(args.quantiles[1], 0.25.try_into().unwrap());
        assert_eq!(args.quantiles[2], 0.5.try_into().unwrap());
        assert_eq!(args.quantiles[3], 0.75.try_into().unwrap());
        assert_eq!(args.quantiles[4], 1.0.try_into().unwrap());
    }

    #[test]
    fn test_all_analysis_disabled() {
        let args = Args::try_parse_from(["program", "--all=false", "/tmp/trace"]);
        assert!(args.is_err(), "Disabling all analyses should be rejected");
    }

    #[test]
    fn test_specific_analysis_flags() {
        let args = Args::try_parse_from([
            "program",
            "--dependency-graph",
            "--message-latency=custom_latency.json",
            "/tmp/trace",
        ])
        .unwrap_or_else(|e| panic!("Failed to parse arguments: {e}"));

        assert!(!args.all); // Should be automatically set to false when any analysis flag is used
        assert_eq!(
            args.dependency_graph,
            Some(PathBuf::from(filenames::DEPENDENCY_GRAPH))
        );
        assert_eq!(
            args.message_latency,
            Some(PathBuf::from("custom_latency.json"))
        );
        assert_eq!(args.callback_duration, None);
    }

    #[test]
    fn test_empty_quantiles_rejected() {
        let result = Args::try_parse_from(["program", "--quantiles", "", "/tmp/trace"]);
        assert!(result.is_err(), "Empty quantiles list should be rejected");

        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("quantiles"),
            "Error message should mention quantiles: {err}"
        );
    }

    #[test]
    fn test_space_separated_quantiles_rejected() {
        // Space is a value terminator according to the quantiles arg definition
        let result = Args::try_parse_from(["program", "--quantiles", "0.1 0.5 0.9", "/tmp/trace"]);

        // This should not work due to terminator behavior
        assert!(
            result.is_err(),
            "Space-separated quantiles should be rejected"
        );

        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("quantiles"),
            "Error message should mention quantiles: {err}"
        );
    }

    #[test]
    fn test_all_and_specific_analysis_flags() {
        let args = Args::try_parse_from([
            "program",
            "--all",
            "--dependency-graph",
            "--message-latency=custom_latency.json",
            "/tmp/trace",
        ])
        .unwrap_or_else(|e| panic!("Failed to parse arguments: {e}"));

        assert!(args.all);
        assert_eq!(
            args.dependency_graph,
            Some(PathBuf::from(filenames::DEPENDENCY_GRAPH))
        );
        assert_eq!(
            args.message_latency,
            Some(PathBuf::from("custom_latency.json"))
        );
        assert_eq!(
            args.callback_duration,
            Some(PathBuf::from(filenames::CALLBACK_DURATION))
        );
    }

    #[test]
    fn test_implicit_all_flag() {
        let args = Args::try_parse_from(["program", "/tmp/trace"])
            .unwrap_or_else(|e| panic!("Failed to parse arguments: {e}"));

        assert!(args.all);
        assert_eq!(
            args.dependency_graph,
            Some(PathBuf::from(filenames::DEPENDENCY_GRAPH))
        );
        assert_eq!(
            args.message_latency,
            Some(PathBuf::from(filenames::MESSAGE_LATENCY))
        );
        assert_eq!(
            args.callback_duration,
            Some(PathBuf::from(filenames::CALLBACK_DURATION))
        );
    }
}
