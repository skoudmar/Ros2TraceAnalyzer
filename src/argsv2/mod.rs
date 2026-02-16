use std::sync::OnceLock;

use clap::{Parser, Subcommand};
use clap_verbosity_flag::{Verbosity, WarnLevel};

pub mod analysis_args;
pub mod chart_args;
pub mod viewer_args;
pub mod helpers;

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
            TracerCommand::Analyse(analysis_args) => &analysis_args,
            _ => {
                panic!("Tried to extract Analysis arguments subcommand while Charting subcommand has been provided")
            }
        }
    }

    pub fn as_analysis_args(self) -> analysis_args::AnalysisArgs {
        match self.command {
            TracerCommand::Analyse(analysis_args) => analysis_args,
            _ => {
                panic!("Tried to extract Analysis arguments subcommand while Charting subcommand has been provided")
            }
        }
    }
}

#[derive(Debug, Subcommand, Clone)]
pub enum TracerCommand {
    /// Analyse a Ros2 trace and generate graphs, json or bundle outputs
    Analyse(analysis_args::AnalysisArgs),

    /// Render a chart of a specific property of a RoS2 interface
    Chart(chart_args::ChartArgs),
    
    /// Start a .dot viewer capable of generating charts on demand
    Viewer(viewer_args::ViewerArgs),
}




#[cfg(test)]
mod test {
    use clap::{CommandFactory};

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
