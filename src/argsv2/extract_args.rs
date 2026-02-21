use std::path::PathBuf;

use clap::{Args, ValueEnum, ValueHint};
use derive_more::Display;

const DEFAULT_BUNDLE_NAME: &'static str = "binary_bundle.sqlite";
const DEFAULT_OUTPUT_NAME: &'static str = "extract_data.json";

#[derive(Debug, Clone, Args)]
pub struct ExtractArgs {
    /// Identifier of the element for which to extract the data
    ///
    /// - For nodes (graphviz nodes) the namespace (ROS node), type (ROS interface) and parameters (ROS topic) need to be specified  
    /// - For edges (graphviz edges) name (type + topic) of the source and target node should be provided
    ///
    /// The expected format is URL encoded map
    element_id: String,

    /// The property to extract from the node
    property: AnalysisProperty,

    /// The input path, either a file of the data or a folder containing the default named file with the necessary data
    #[clap(long, short = 'i', value_name = "INPUT", value_hint = ValueHint::AnyPath)]
    input_path: Option<PathBuf>,

    /// The output path, either a folder to which the file will be generated or a file to write into
    #[clap(long, short = 'o', value_name = "OUTPUT", value_hint = ValueHint::AnyPath)]
    output_path: Option<PathBuf>,
}

impl ExtractArgs {
    pub fn element_id(&self) -> &str {
        &self.element_id
    }

    pub fn property(&self) -> &AnalysisProperty {
        &self.property
    }

    pub fn input_path(&self) -> PathBuf {
        match &self.input_path {
            Some(p) => {
                if p.is_dir() {
                    p.join(DEFAULT_BUNDLE_NAME)
                } else {
                    p.clone()
                }
            }
            None => std::env::current_dir().unwrap().join(DEFAULT_BUNDLE_NAME),
        }
    }

    pub fn output_path(&self) -> PathBuf {
        match &self.output_path {
            Some(p) => {
                if p.is_dir() {
                    p.join(DEFAULT_OUTPUT_NAME)
                } else {
                    p.clone()
                }
            }
            None => std::env::current_dir().unwrap().join(DEFAULT_OUTPUT_NAME),
        }
    }
}

#[derive(Debug, Display, ValueEnum, Clone)]
pub enum AnalysisProperty {
    /// Callback execution durations
    #[display("Callback execution time")]
    CallbackDuration,
    /// Delays between callback or timer activations
    #[display("Delays between activations")]
    ActivationsDelay,

    /// Delays between publisher publications
    #[display("Delay between publication")]
    PublicationsDelay,

    /// Delays between subscriber messages
    #[display("Delay between")]
    MessagesDelay,

    /// Latency of a communication channel
    #[display("Latency")]
    MessagesLatency,
}
