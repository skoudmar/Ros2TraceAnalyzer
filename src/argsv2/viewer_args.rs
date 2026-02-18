use std::path::PathBuf;
use clap::{Args, ValueHint};

#[derive(Debug, Clone, Args)]
pub struct ViewerArgs {
    /// The dotfile to open
    pub dotfile: PathBuf,

    #[clap(long, value_name = "VIEWER", value_hint = ValueHint::FilePath)]
    /// The entry point to the python viewer (defaults to ./xdotviewer/main.py)
    pub viewer: Option<PathBuf>,

    /// The executable to run to invoke the Ros2TraceAnalyzer (defaults to ./target/release/Ros2TraceAnalyzer)
    #[clap(long, short = 't', value_name = "Ros2TraceAnalyzer", value_hint = ValueHint::ExecutablePath)]
    pub tracer_exec: Option<PathBuf>,

    /// The directory with the datafiles (defaults to CWD)
    #[clap(long, short = 'd', value_name = "DATA", value_hint = ValueHint::DirPath)]
    pub data: Option<PathBuf>
}