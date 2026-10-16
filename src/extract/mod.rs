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
#[display("{node} {interface}")]
pub struct RosInterfaceCompleteName {
    pub interface: String,
    pub node: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Display, Debug)]
#[display("{topic} ({source_node} → {destination_node})")]
pub struct RosChannelCompleteName {
    pub source_node: String,
    pub destination_node: String,
    pub topic: String,
}

#[derive(Debug)]
pub struct PlottableData {
    pub title: String,
    pub data: Vec<i64>,
}

impl PlottableData {
    fn new(property: &AnalysisProperty, title: String, data: Vec<i64>) -> Self {
        PlottableData {
            title: format!("{property} of: {title}"),
            data,
        }
    }

    fn assert_valid(&self) -> Result<(), DataExtractionError> {
        if self.data.len() == 0 {
            Err(DataExtractionError::EmptyData)
        } else {
            Ok(())
        }
    }
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
    #[error("Element exists but has no data associated with it")]
    EmptyData,
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

    let plottable_data = match property {
        AnalysisProperty::CallbackDuration => {
            let d = store
                .get_by_id::<CallbackDurationExport>(element_id)
                .map_err(DataExtractionError::SourceDataParseError)?;
            PlottableData::new(property, d.name.to_string(), d.callback_durations)
        }
        AnalysisProperty::ActivationDelay => {
            let d = store
                .get_by_id::<ActivationDelayExport>(element_id)
                .map_err(DataExtractionError::SourceDataParseError)?;
            PlottableData::new(property, d.name.to_string(), d.activation_delays)
        }
        AnalysisProperty::PublicationDelay => {
            let d = store
                .get_by_id::<PublicationDelayExport>(element_id)
                .map_err(DataExtractionError::SourceDataParseError)?;
            PlottableData::new(property, d.name.to_string(), d.publication_delays)
        }
        AnalysisProperty::MessageDelay => {
            let d = store
                .get_by_id::<MessagesDelayExport>(element_id)
                .map_err(DataExtractionError::SourceDataParseError)?;

            PlottableData::new(property, d.name.to_string(), d.messages_delays)
        }
        AnalysisProperty::MessageLatency => {
            let d = store
                .get_by_id::<MessageLatencyExport>(element_id)
                .map_err(|e| match e {
                    BinarySQLStoreError::NoResults => {
                        DataExtractionError::NoSuchElement(element_id)
                    }
                    _ => e.into(),
                })?;
            PlottableData::new(property, d.name.to_string(), d.messages_latencies)
        }
    };

    let _ = plottable_data.assert_valid()?;

    Ok(plottable_data)
}

impl PlottableData {
    pub fn export_json(&self) -> color_eyre::eyre::Result<String> {
        let str = serde_json::to_string(&self.data)?;
        Ok(str)
    }
}
