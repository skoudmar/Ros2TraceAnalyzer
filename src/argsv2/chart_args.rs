use clap::{Args, Subcommand, ValueEnum, ValueHint};
use derive_more::Display;
use std::path::{Path, PathBuf};

use crate::argsv2::analysis_args;

#[derive(Debug, Clone, Args)]
pub struct ChartArgs {
    /// Identifies the element in the dependency graph for
    /// which to generate the chart
    #[clap(long, short = 'e')]
    element_id: i64,

    /// Path to the r2ta_results.sqlite file from which to retreive the data
    #[clap(long, short = 'i', value_name = "FILENAME", value_hint = ValueHint::FilePath, default_value = analysis_args::filenames::BINARY_BUNDLE)]
    input: Option<PathBuf>,

    /// Store the chart data to the given file
    #[clap(long, short = 'o', value_name = "FILENAME", value_hint = ValueHint::AnyPath)]
    output: Option<PathBuf>,

    /// Indicates whether the chart should be rendered from scratch.
    ///
    /// If not set, an existing chart will be reused only if it matches all specified parameters.
    #[clap(long, short = 'c', default_value = "false")]
    clean: bool,

    #[clap(flatten)]
    chart: ChartRequest,
}

impl ChartArgs {
    pub fn element_id(&self) -> i64 {
        self.element_id
    }

    pub fn input_path(&self) -> Option<&Path> {
        self.input.as_deref()
    }

    pub fn output_path(&self) -> Option<&Path> {
        self.output.as_deref()
    }

    pub fn clean(&self) -> bool {
        self.clean
    }

    pub fn chart(&self) -> &ChartRequest {
        &self.chart
    }
}

#[derive(Debug, Display, Args, Clone)]
#[display("ChartOf {{ value: {value}, {plot} }}")]
pub struct ChartRequest {
    /// The value to plot into the chart
    #[clap(long)]
    pub value: ChartedValue,

    /// The type of chart to render the data as
    #[command(subcommand)]
    pub plot: ChartVariants,

    /// The rectangular size of the rendered image in pixels
    ///
    /// - For PNG this directly translates to pixels
    /// - For SVG this is the size in pixels with scale 1.0
    #[clap(long, default_value = "800")]
    pub size: u32,

    /// The filetype (output format) the rendered image should be in
    #[clap(long, default_value_t = ChartOutputFormat::default())]
    pub output_format: ChartOutputFormat,
}

#[derive(Debug, Display, ValueEnum, Clone, Copy, Default)]
pub enum ChartOutputFormat {
    #[default]
    #[display("svg")]
    Svg,
    #[display("png")]
    Png,
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
