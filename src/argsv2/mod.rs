use std::sync::OnceLock;

use clap::{Parser, Subcommand};
use clap_verbosity_flag::{Verbosity, WarnLevel};

pub mod analysis_args;
pub mod chart_args;
pub mod extract_args;
pub mod helpers;
pub mod viewer_args;

pub static CLI_ARGS: OnceLock<Args> = OnceLock::new();

#[derive(Debug, Clone, Parser)]
pub struct Args {
    #[command(flatten)]
    pub verbose: Verbosity<WarnLevel>,

    #[command(subcommand)]
    pub command: TracerCommand,
}

impl Args {
    pub fn get() -> &'static Args {
        CLI_ARGS.get_or_init(Self::parse)
    }

    pub fn get_analyses_args() -> &'static analysis_args::AnalysisArgs {
        match &Self::get().command {
            TracerCommand::Analyze(analysis_args) => analysis_args,
            _ => {
                panic!(
                    "Tried to extract Analysis arguments subcommand but {} subcommand was used",
                    &Self::get().command
                )
            }
        }
    }

    pub fn into_analysis_args(self) -> analysis_args::AnalysisArgs {
        match self.command {
            TracerCommand::Analyze(analysis_args) => analysis_args,
            _ => {
                panic!(
                    "Tried to extract Analysis arguments subcommand but {} subcommand was used",
                    self.command
                )
            }
        }
    }
}

#[derive(Debug, Subcommand, Clone, derive_more::Display)]
pub enum TracerCommand {
    /// Analyze a ROS 2 trace and generate graphs, JSON or bundle outputs
    #[display("analyze")]
    Analyze(analysis_args::AnalysisArgs),

    /// Render a chart of a specific property of a ROS 2 interface
    #[display("chart")]
    Chart(chart_args::ChartArgs),

    /// Start a .dot viewer capable of generating charts on demand
    #[display("viewer")]
    Viewer(viewer_args::ViewerArgs),

    /// Retreive data from SQL binary bundle for the specified node into JSON
    #[display("extract")]
    Extract(extract_args::ExtractArgs),
}

#[cfg(test)]
mod test {
    use clap::CommandFactory;

    use super::*;

    #[test]
    #[ignore]
    fn print_help() {
        Args::command().print_help().unwrap();
    }

    #[test]
    #[ignore]
    fn print_long_help() {
        Args::command().print_long_help().unwrap();
    }

    #[test]
    fn verify_cli() {
        Args::command().debug_assert();
    }
}
