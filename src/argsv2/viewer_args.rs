use clap::{Args, ValueHint};
use std::path::PathBuf;

use crate::argsv2::analysis_args::filenames;

#[derive(Debug, Clone, Args)]
pub struct ViewerArgs {
    /// Binary bundle file name or a directory containing r2ta_results.sqlite file
    #[clap(value_name = "FILENAME", value_hint = ValueHint::FilePath)]
    pub input: Option<PathBuf>,

    /// main.py file path or directory path containing the main.py file
    ///
    /// Defaults to $PWD/py-src/xdotviewer/main.py
    #[clap(long, value_name = "VIEWER", value_hint = ValueHint::FilePath)]
    pub viewer: Option<PathBuf>,
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

    pub fn viewer_path(&self) -> PathBuf {
        match &self.viewer {
            Some(p) => {
                if p.is_dir() {
                    p.join("main.py")
                } else {
                    p.clone()
                }
            }
            None => std::env::current_dir()
                .unwrap()
                .join("py-src/xdotviewer/main.py"),
        }
    }
}
