use std::ffi::{CStr, CString};
use std::path::{Path, PathBuf};

use bt2_sys::graph::component::BtComponentType;
use bt2_sys::query::support_info;
use clap::builder::{PathBufValueParser, TypedValueParser};
use clap::{Parser, ValueEnum};
use clap_verbosity_flag::{Verbosity, WarnLevel};
use walkdir::WalkDir;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, ValueEnum)]
pub enum OutputFormat {
    #[default]
    Text,
    Csv,
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Path to a directory containing the trace to analyze
    ///
    /// Can be a super-directory of the trace directory.
    #[arg(value_parser = PathBufValueParser::new().try_map(to_directory_path_buf), num_args = 1.., required = true)]
    trace_paths: Vec<PathBuf>,

    /// If set to true, only the directory specified by `trace-path` is searched for traces, not its subdirectories.
    #[arg(long)]
    exact_trace_path: bool,

    /// Print processed events
    #[arg(long, short = 'p')]
    print_events: bool,

    /// Print unprocessed events
    #[arg(long, short = 'u')]
    print_unprocessed_events: bool,

    /// Path to a directory where the output files will be written
    #[arg(long, short = 'o', value_parser = PathBufValueParser::new().try_map(to_directory_path_buf))]
    output: Option<PathBuf>,

    /// Output format
    #[arg(long, short = 'f', value_enum, default_value_t = Default::default())]
    output_format: OutputFormat,

    #[command(flatten)]
    pub verbose: Verbosity<WarnLevel>,
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

    pub fn output_dir(&self) -> Option<&PathBuf> {
        self.output.as_ref()
    }

    pub const fn output_format(&self) -> OutputFormat {
        self.output_format
    }
}

fn to_directory_path_buf(path: PathBuf) -> Result<PathBuf, &'static str> {
    CString::new(path.to_str().ok_or("Path must be encoded as UTF-8")?)
        .map_err(|_| "Path must not contain null bytes")?;

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
