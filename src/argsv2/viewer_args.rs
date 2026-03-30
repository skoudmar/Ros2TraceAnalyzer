use clap::{Args, ValueHint};
use std::path::PathBuf;

use crate::argsv2::analysis_args::filenames;

#[derive(Debug, Clone, Args)]
pub struct ViewerArgs {
    #[clap(long, value_name = "VIEWER", value_hint = ValueHint::FilePath)]
    /// The entry point to the python viewer (defaults to ./py-src/xdotviewer/main.py)
    pub viewer: Option<PathBuf>,

    /// The executable to run to invoke the Ros2TraceAnalyzer (defaults to ./target/release/Ros2TraceAnalyzer)
    #[clap(long, short = 't', value_name = "TRACE ANALYZER", value_hint = ValueHint::ExecutablePath)]
    pub tracer_exec: Option<String>,

    /// Binary bundle file name or a directory containing r2ta_results.sqlite file
    #[clap(long, short = 'd', value_name = "INPUT", value_hint = ValueHint::AnyPath)]
    pub input: Option<PathBuf>,
}

impl ViewerArgs {
    pub fn input_path(&self) -> PathBuf {
        match &self.input {
            Some(p) => {
                if p.is_dir() {
                    p.join(filenames::BINARY_BUNDLE)
                } else {
                    p.clone()
                }
            }
            None => std::env::current_dir()
                .unwrap()
                .join(filenames::BINARY_BUNDLE),
        }
    }
}
