use std::io::Write;
use std::path::Path;

use derive_more::Display;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::analyses::analysis::dependency_graph::{
    ActivationDelayExport, CallbackDurationExport, MessageLatencyExport, MessagesDelayExport,
    NodeOverviewExport, PublicationDelayExport,
};
use crate::argsv2::extract_args::AnalysisProperty;
use crate::utils::binary_sql_store::{BinarySQLStoreError, BinarySqlStore};

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Display, Debug)]
#[display("{node}::{interface}")]
pub struct RosInterfaceCompleteName {
    pub interface: String,
    pub node: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Display, Debug)]
#[display("{source_node}-({topic})>{destination_node}")]
pub struct RosChannelCompleteName {
    pub source_node: String,
    pub destination_node: String,
    pub topic: String,
}

pub enum PlottableData {
    I64(Vec<i64>),
}

#[derive(Error, Debug)]
pub enum DataExtractionError {
    #[error("An error occurred during data parsing\n{0}")]
    SourceDataParseError(#[from] BinarySQLStoreError),
    #[error(
        "The requested property {property} is not available for element {element}. Available analyses are [{}]",
        properties.iter().map(|v| clap::ValueEnum::to_possible_value(v).unwrap().get_name().to_owned()).join(", ")
    )]
    IncompatibleElementAnalysis {
        property: AnalysisProperty,
        element: usize,
        properties: Vec<AnalysisProperty>,
    },
    #[error("There is no element with id {0}.")]
    NoSuchElement(usize),
}

pub fn extract_graph(input: &Path) -> color_eyre::eyre::Result<String> {
    let store = BinarySqlStore::open(input)?;

    Ok(store.get_dependency_graph()?.graph)
}

pub fn extract_property(
    input: &Path,
    element_id: i64,
    property: &AnalysisProperty,
) -> color_eyre::eyre::Result<PlottableData> {
    let store = BinarySqlStore::open(input)?;

    let element_id = element_id as usize;

    if *property != AnalysisProperty::MessageLatency {
        let id_node_meta =
            store
                .get_by_id::<NodeOverviewExport>(element_id)
                .map_err(|e| match e {
                    BinarySQLStoreError::NoResults => {
                        DataExtractionError::NoSuchElement(element_id)
                    }
                    _ => e.into(),
                })?;

        if !id_node_meta.analyses.contains(property) {
            return Err(DataExtractionError::IncompatibleElementAnalysis {
                property: *property,
                element: element_id,
                properties: id_node_meta.analyses,
            }
            .into());
        }
    }

    Ok(match property {
        AnalysisProperty::CallbackDuration => PlottableData::I64(
            store
                .get_by_id::<CallbackDurationExport>(element_id)
                .map_err(DataExtractionError::SourceDataParseError)?
                .callback_durations,
        ),
        AnalysisProperty::ActivationDelay => PlottableData::I64(
            store
                .get_by_id::<ActivationDelayExport>(element_id)
                .map_err(DataExtractionError::SourceDataParseError)?
                .activation_delays,
        ),
        AnalysisProperty::PublicationDelay => PlottableData::I64(
            store
                .get_by_id::<PublicationDelayExport>(element_id)
                .map_err(DataExtractionError::SourceDataParseError)?
                .publication_delays,
        ),
        AnalysisProperty::MessageDelay => PlottableData::I64(
            store
                .get_by_id::<MessagesDelayExport>(element_id)
                .map_err(DataExtractionError::SourceDataParseError)?
                .messages_delays,
        ),
        AnalysisProperty::MessageLatency => PlottableData::I64(
            store
                .get_by_id::<MessageLatencyExport>(element_id)
                .map_err(|e| match e {
                    BinarySQLStoreError::NoResults => {
                        DataExtractionError::NoSuchElement(element_id)
                    }
                    _ => e.into(),
                })?
                .messages_latencies,
        ),
    })
}

impl PlottableData {
    pub fn export(&self, output: &mut impl Write) -> color_eyre::eyre::Result<()> {
        let data = match self {
            PlottableData::I64(items) => serde_json::to_string(&items)?,
        };

        writeln!(output, "{data}")?;

        Ok(())
    }
}
