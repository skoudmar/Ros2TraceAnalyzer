use std::path::PathBuf;
use clap::{Args, Subcommand, ValueEnum, ValueHint};
use derive_more::Display;

#[derive(Debug, Clone, Args)]
pub struct ChartArgs {
    /// Full name of the node to raw the chart for
    ///
    /// The name should include the namespace and node's name
    #[clap(long, short = 'n')]
    node: String,

    /// The input path, either a file of the data or a folder containing the default named file with the necessary data
    #[clap(long, short = 'i', value_name = "INPUT", value_hint = ValueHint::AnyPath)]
    input_path: Option<PathBuf>,

    /// The output path, either a folder to which the file will be generated or a file to write into
    #[clap(long, short = 'o', value_name = "OUTPUT", value_hint = ValueHint::AnyPath)]
    output_path: Option<PathBuf>,

    /// Whether the chart should be rerender from scratch
    ///
    /// if not set a preexisting chart will be used only if it matches all parameters
    #[clap(long, short = 'c', default_value = "false")]
    clean: bool,

    #[clap(flatten)]
    chart: ChartRequest,
}

impl ChartArgs {
    pub fn node(&self) -> &str {
        &self.node
    }

    pub fn input_path(&self) -> &Option<PathBuf> {
        &self.input_path
    }

    pub fn output_path(&self) -> &Option<PathBuf> {
        &self.output_path
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

    /// The size of the rendered image
    #[clap(long, default_value = "800")]
    pub size: u32,

    /// The filetype (output format) the rendered image should be in
    #[clap(long, default_value_t = ChartOutputFormat::default())]
    pub output_format: ChartOutputFormat
}

#[derive(Debug, Display, ValueEnum, Clone, Default)]
pub enum ChartOutputFormat {
    #[default]
    #[display("svg")]
    SVG,
    #[display("png")]
    PNG
}

#[derive(Debug, Display, ValueEnum, Clone)]
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

#[derive(Debug, Display, Subcommand, Clone)]
pub enum ChartVariants {
    #[display("Histogram")]
    Histogram(HistogramData),
    #[display("Scatter")]
    Scatter,
}

#[derive(Debug, Display, Args, Clone)]
#[display("Histogram data {{ bins: {bins:?} }}")]
pub struct HistogramData {
    /// Number of bins to split the data into
    #[arg(long, short = 'b', value_name = "BINS")]
    pub bins: Option<usize>
}