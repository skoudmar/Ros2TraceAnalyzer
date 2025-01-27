use std::ffi::{CStr, CString};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use bt2_sys::graph::component::BtComponentType;
use bt2_sys::query::support_info;
use clap::builder::{PathBufValueParser, TypedValueParser};
use clap::{Parser, Subcommand, ValueEnum};
use clap_verbosity_flag::{Verbosity, WarnLevel};
use walkdir::WalkDir;

use crate::statistics::Quantile;

pub static ANALYSIS_CLI_ARGS: OnceLock<AnalysisArgsCommon> = OnceLock::new();

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, ValueEnum)]
pub enum OutputFormat {
    #[default]
    Text,
    Json,
}

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true, subcommand_precedence_over_arg = true)]
pub struct Args {
    /// Path to a directory containing the trace to analyze
    ///
    /// Can be a super-directory of the trace directory.
    #[arg(value_parser = PathBufValueParser::new().try_map(|p| to_directory_path_buf(p, false)), num_args = 1.., required = true)]
    trace_paths: Vec<PathBuf>,

    /// If set to true, only the directory specified by `TRACE_PATH` is searched for traces, not its subdirectories.
    #[arg(long)]
    exact_trace_path: bool,

    /// Print processed events
    #[arg(long, short = 'p')]
    print_events: bool,

    /// Print unprocessed events
    #[arg(long, short = 'u')]
    print_unprocessed_events: bool,

    #[command(flatten)]
    pub verbose: Verbosity<WarnLevel>,

    #[command(subcommand)]
    pub subcommand: AnalysisSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum AnalysisSubcommand {
    /// Analyze the latency of the messages.
    MessageLatency {
        #[command(flatten)]
        common: AnalysisArgsCommon,
    },

    /// Analyze the callback duration and inter-arrival time.
    Callback {
        #[command(flatten)]
        common: AnalysisArgsCommon,
    },

    /// Analyze the utilization of the system based on the quantile callback durations.
    Utilization {
        /// Callback duration quantile.
        #[arg(long, value_parser, default_value = "0.9")]
        quantile: Quantile,
    },
    /// Analyze the utilization of the system based on the real execution times.
    UtilizationReal,

    /// Construct a detailed dependency graph.
    DependencyGraph(DependencyGraphArgs),

    /// Run all analyses.
    All {
        #[command(flatten)]
        common: AnalysisArgsCommon,

        /// Callback duration quantile for utilization analysis.
        #[arg(long, value_parser, default_value = "0.9")]
        utilization_quantile: Quantile,

        #[command(flatten)]
        dependency_graph: DependencyGraphArgs,
    },
}

#[derive(Debug, clap::Args)]
pub struct DependencyGraphArgs {
    /// Color edges based on the latency measurements.
    #[arg(long)]
    pub color: bool,

    /// Set the edge thickness based on the latency measurements.
    #[arg(long)]
    pub thickness: bool,

    /// Minimum multiplier for edge coloring or thickness.
    ///
    /// Can be any positive number.
    ///
    /// The minimum multiplier is used to set the maximum value in gradients
    /// to be at least `min-multiplier` times the minimum value.
    ///
    /// The gradient range is exactly [minimum value, max(maximum value, minimum value * min_multiplier)].
    #[arg(long, default_value = "5.0")]
    pub min_multiplier: f64,

    /// Path where to write the graph in DOT format.
    #[arg(long, short = 'o', value_parser = PathBufValueParser::new().try_map(to_directory_path_buf))]
    pub output_path: PathBuf,
}

#[derive(Debug, Parser, Clone)]
pub struct AnalysisArgsCommon {
    /// Quantiles to compute for the latency and duration analysis.
    ///
    /// The quantiles must be in the range [0, 1].
    ///
    /// If not specified, the default quantiles are:
    /// 0 (minimum), 0.10, 0.5 (median), 0.90, 0.99, 1 (maximum)
    #[arg(long, value_parser, value_delimiter = ',', num_args = 1.., default_value = "0,0.10,0.5,0.90,0.99,1")]
    pub quantiles: Vec<Quantile>,

    /// Export the latency measurements to a JSON file.
    #[arg(long = "json-dir", value_parser)]
    pub json_dir_path: Option<PathBuf>,
}

impl Args {
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

    pub const fn should_print_events(&self) -> bool {
        self.print_events
    }

    pub const fn should_print_unprocessed_events(&self) -> bool {
        self.print_unprocessed_events
    }

    // pub fn output_dir(&self) -> Option<&PathBuf> {
    //     self.output.as_ref()
    // }

    // pub const fn output_format(&self) -> OutputFormat {
    //     self.output_format
    // }

    pub(crate) fn set_globals(&self) {
        use AnalysisSubcommand::{
            All, Callback, DependencyGraph, MessageLatency, Utilization, UtilizationReal,
        };
        match &self.subcommand {
            MessageLatency { common } | Callback { common } | All { common, .. } => {
                ANALYSIS_CLI_ARGS
                    .set(common.clone())
                    .expect("Failed to set global analysis CLI arguments");
            }
            Utilization { .. } | UtilizationReal | DependencyGraph { .. } => {
                ANALYSIS_CLI_ARGS
                    .set(AnalysisArgsCommon {
                        quantiles: vec![],
                        json_dir_path: None,
                    })
                    .expect("Failed to set global analysis CLI arguments");
            }
        }
    }
}

fn to_directory_path_buf(path: PathBuf, create: bool) -> Result<PathBuf, &'static str> {
    CString::new(path.to_str().ok_or("Path must be encoded as UTF-8")?)
        .map_err(|_| "Path must not contain null bytes")?;

    if !path.exists() {
        if create {
            std::fs::create_dir(&path).map_err(|_| "Failed to create directory")?;
            return Ok(path);
        }
        
        return Err("Path does not exist.");
    }

    if path.is_dir() {
        Ok(path)
    } else {
        Err("Path is not a directory.")
    }
}

const TRACE_PATH_LIKELIHOOD_THRESHOLD: f64 = 0.5;

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

#[cfg(test)]
mod test {
    use clap::CommandFactory;

    use super::*;

    #[test]
    fn verify_cli() {
        Args::command().debug_assert();
    }
}
