use clap::{Args, Subcommand, ValueEnum, ValueHint};
use derive_more::Display;
use std::path::PathBuf;
use thiserror::Error;

use crate::argsv2::analysis_args;
use crate::argsv2::extract_args::AnalysisProperty;

#[derive(Debug, Clone, Args)]
pub struct ChartArgs {
    /// Identifies the element in the dependency graph for
    /// which to generate the chart
    #[clap(long, short = 'e')]
    pub element_id: i64,

    /// Binary bundle file name or a directory containing r2ta_results.sqlite file
    #[clap(long, short = 'i', value_name = "FILENAME", value_hint = ValueHint::FilePath, default_value = analysis_args::filenames::BINARY_BUNDLE)]
    pub input: PathBuf,

    /// Directory or filename where to store the chart
    ///
    /// For directories, the chart file name will be <ID>_<QUANTITY>.svg.
    ///
    /// For filenames, the output type is determined by its extension.
    /// Supported extensions are: SVG [default] and PNG.
    ///
    /// If not given, the current directory is used.
    #[clap(long, short = 'o', value_name = "FILENAME", value_hint = ValueHint::AnyPath)]
    pub output: Option<PathBuf>,

    /// Overwrite the output file if present
    #[clap(long, default_value = "false")]
    pub overwrite: bool,

    #[clap(flatten)]
    pub chart: ChartRequest,
}

#[derive(Debug, Display, Args, Clone)]
#[display("{plot} of {quantity}")]
pub struct ChartRequest {
    /// The value to plot into the chart
    #[clap(long)]
    pub quantity: ChartedValue,

    /// The size of the image in pixels
    ///
    /// - For PNG this directly translates to pixels
    ///
    /// - For SVG this is the size in pixels with scale 1.0
    #[clap(long, value_name = "WIDTHxHEIGHT", default_value = "800x600", value_parser = |s: &str| -> Result<(u32, u32), String> {
        s.split_once('x')
            .and_then(|(w, h)| Some((w.parse().ok()?, h.parse().ok()?)))
            .ok_or_else(|| "Format must be WIDTHxHEIGHT (e.g., 1024x768)".to_string())
    })]
    pub size: (u32, u32),

    /// The type of chart to render the data as
    #[command(subcommand)]
    pub plot: ChartVariants,
}

impl ChartRequest {
    pub(crate) fn name_descriptor(&self) -> String {
        let value = match self.quantity {
            ChartedValue::CallbackDuration => "execution_timing",
            ChartedValue::ActivationsDelay => "activations_delay",
            ChartedValue::PublicationsDelay => "publication_delay",
            ChartedValue::MessagesDelay => "message_delay",
            ChartedValue::MessagesLatency => "latency",
        };

        let plot = match &self.plot {
            ChartVariants::Histogram(hist_data) => {
                format!("histogram_{}", hist_data.bins.unwrap_or(0))
            }
            ChartVariants::Scatter => "scatter".to_owned(),
        };

        format!("{}_{}_{}x{}", value, plot, self.size.0, self.size.1)
    }
}

#[derive(Debug, Display, ValueEnum, Clone, Copy, Default)]
pub enum ChartOutputFormat {
    #[default]
    #[display("svg")]
    Svg,
    #[display("png")]
    Png,
}

#[derive(Debug, Error)]
pub enum OutputFormatError {
    #[error("The provided file format '{0}' is not supported")]
    UnsupportedFormat(String),
}

impl TryFrom<&str> for ChartOutputFormat {
    type Error = OutputFormatError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "svg" => Ok(Self::Svg),
            "png" => Ok(Self::Png),
            _ => Err(OutputFormatError::UnsupportedFormat(value.to_owned())),
        }
    }
}

#[derive(Debug, Display, ValueEnum, Clone, Copy)]
pub enum ChartedValue {
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

impl From<AnalysisProperty> for ChartedValue {
    fn from(value: AnalysisProperty) -> Self {
        match value {
            AnalysisProperty::CallbackDurations => ChartedValue::CallbackDuration,
            AnalysisProperty::ActivationDelays => ChartedValue::ActivationsDelay,
            AnalysisProperty::PublicationDelays => ChartedValue::PublicationsDelay,
            AnalysisProperty::MessageDelays => ChartedValue::MessagesDelay,
            AnalysisProperty::MessageLatencies => ChartedValue::MessagesLatency,
        }
    }
}

#[derive(Debug, Display, Subcommand, Clone, Copy)]
pub enum ChartVariants {
    #[display("Histogram")]
    Histogram(HistogramData),
    #[display("Scatter")]
    Scatter,
}

#[derive(Debug, Display, Args, Clone, Copy)]
#[display("Histogram data {{ bins: {bins:?} }}")]
pub struct HistogramData {
    /// Number of bins to split the data into
    #[arg(long, short = 'b', value_name = "BINS")]
    pub bins: Option<usize>,
}
