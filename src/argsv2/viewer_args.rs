use clap::{Args, ValueHint};
use std::path::PathBuf;

use crate::argsv2::analysis_args::filenames;

#[derive(Debug, Clone, Args)]
pub struct ViewerArgs {
    /// Binary bundle file name or a directory containing r2ta_results.sqlite file
    #[clap(value_name = "FILENAME", value_hint = ValueHint::FilePath, default_value = ".")]
    pub input: PathBuf,

    /// main.py file path or directory path containing the main.py file
    #[clap(long, value_name = "VIEWER", value_hint = ValueHint::FilePath,
        default_value_os_t = option_env!("R2TA_VIEWER")
                    .map(PathBuf::from)
                    .unwrap_or_else(|| {
                        PathBuf::from(".")
                            .join("py-src")
                            .join("xdotviewer")
                            .join("main.py")
                    })
    )]
    pub viewer: PathBuf,
}

impl ViewerArgs {
    pub fn input_path(&self) -> PathBuf {
        if self.input.is_dir() {
            self.input.join(filenames::BINARY_BUNDLE)
        } else {
            self.input.clone()
        }
    }
}
