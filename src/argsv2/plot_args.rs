use clap::{Args, Subcommand, ValueEnum, ValueHint};
use derive_more::Display;
use std::path::PathBuf;
use thiserror::Error;

use crate::argsv2::analysis_args;
use crate::argsv2::extract_args::AnalysisProperty;

#[derive(Debug, Clone, Args)]
pub struct PlotArgs {
    /// Binary bundle file name or a directory containing r2ta_results.sqlite file
    #[clap(long, short = 'i', value_name = "FILENAME", value_hint = ValueHint::FilePath, default_value = analysis_args::filenames::BINARY_BUNDLE)]
    pub input: PathBuf,

    /// File to write the image to, if not present the data is written to stdout
    #[clap(long, short = 'o', value_name = "FILENAME", value_hint = ValueHint::FilePath)]
    pub output: Option<PathBuf>,

    /// Overwrite the output file if present
    #[clap(long, default_value = "false")]
    pub overwrite: bool,

    #[clap(flatten)]
    pub plot: PlotRequest,
}

#[derive(Debug, Display, Args, Clone)]
#[display("{plot} of {property}")]
pub struct PlotRequest {
    /// The property to plot
    pub property: PlottedValue,

    /// Identifies the element in the dependency graph for
    /// which to generate the plot
    pub element_id: i64,

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

    /// The type of plot to render the data as
    #[command(subcommand)]
    pub plot: PlotVariants,
}

#[derive(Debug, Display, ValueEnum, Clone, Copy, Default)]
pub enum PlotOutputFormat {
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

impl TryFrom<&str> for PlotOutputFormat {
    type Error = OutputFormatError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "svg" => Ok(Self::Svg),
            "png" => Ok(Self::Png),
            _ => Err(OutputFormatError::UnsupportedFormat(value.to_owned())),
        }
    }
}

pub type PlottedValue = AnalysisProperty;

#[derive(Debug, Display, Subcommand, Clone, Copy)]
pub enum PlotVariants {
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
