use std::path::{Path, PathBuf};

use clap::{Args, Subcommand, ValueEnum, ValueHint};
use derive_more::Display;

use crate::argsv2::analysis_args::filenames;

#[derive(Debug, Clone, Args)]
pub struct ExtractArgs {
    /// Binary bundle file name or a directory containing r2ta_results.sqlite file
    #[clap(long, short = 'i', value_name = "FILENAME", value_hint = ValueHint::FilePath, default_value = super::analysis_args::filenames::BINARY_BUNDLE)]
    input: Option<PathBuf>,

    /// File to extract the data to, if not present the data is written to stdout
    #[clap(long, short = 'o', value_name = "FILENAME", value_hint = ValueHint::FilePath)]
    output: Option<PathBuf>,

    #[clap(subcommand)]
    extract_content: ExtractContentArgs,
}

impl ExtractArgs {
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

    pub fn output_path(&self) -> Option<&Path> {
        self.output.as_deref()
    }

    pub fn content(&self) -> &ExtractContentArgs {
        &self.extract_content
    }
}

#[derive(Debug, Clone, Subcommand)]
pub enum ExtractContentArgs {
    /// Extract dependency graph
    Graph,
    /// Extract property data for a node
    Property(ExtractPropertyArgs),
}

#[derive(Debug, Clone, Args)]
pub struct ExtractPropertyArgs {
    /// The property to extract from the node.
    property: AnalysisProperty,

    /// Identifies the element in the dependency graph for
    /// which to extract the data
    element_id: i64,
}

impl ExtractPropertyArgs {
    pub fn element_id(&self) -> i64 {
        self.element_id
    }

    pub fn property(&self) -> &AnalysisProperty {
        &self.property
    }
}

#[derive(
    Debug,
    Display,
    ValueEnum,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    serde::Serialize,
    serde::Deserialize,
)]
pub enum AnalysisProperty {
    /// Callback execution durations
    #[display("Callback execution time")]
    CallbackDuration,

    /// Delays between callback or timer activations
    #[display("Delays between activations")]
    ActivationDelay,

    /// Delays between publisher publications
    #[display("Delay between publication")]
    PublicationDelay,

    /// Delays between subscriber messages
    #[display("Delay between")]
    MessageDelay,

    /// Latency of a communication channel
    #[display("Message latency")]
    MessageLatency,
}
