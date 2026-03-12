use std::io::Write;
use std::path::Path;

use derive_more::Display;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::analyses::analysis::dependency_graph::{
    ActivationDelayExport, CallbackDurationExport, MessageLatencyExport, MessagesDelayExport,
    PublicationDelayExport,
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

pub enum ChartableData {
    I64(Vec<i64>),
}

#[derive(Error, Debug)]
pub enum DataExtractionError {
    #[error("An error occurred during data parsing\n{0}")]
    SourceDataParseError(BinarySQLStoreError),
}

pub fn extract_graph(input: &Path) -> color_eyre::eyre::Result<String> {
    let store = BinarySqlStore::open(input)?;

    Ok(store.get_dependency_graph()?.graph)
}

pub fn extract_property(
    input: &Path,
    element_id: i64,
    property: &AnalysisProperty,
) -> color_eyre::eyre::Result<ChartableData> {
    let store = BinarySqlStore::open(input)?;

    let element_id = element_id as usize;

    Ok(match property {
        AnalysisProperty::CallbackDurations => ChartableData::I64(
            store
                .get_by_id::<CallbackDurationExport>(element_id)
                .map_err(DataExtractionError::SourceDataParseError)?
                .callback_durations,
        ),
        AnalysisProperty::ActivationDelays => ChartableData::I64(
            store
                .get_by_id::<ActivationDelayExport>(element_id)
                .map_err(DataExtractionError::SourceDataParseError)?
                .activation_delays,
        ),
        AnalysisProperty::PublicationDelays => ChartableData::I64(
            store
                .get_by_id::<PublicationDelayExport>(element_id)
                .map_err(DataExtractionError::SourceDataParseError)?
                .publication_delays,
        ),
        AnalysisProperty::MessageDelays => ChartableData::I64(
            store
                .get_by_id::<MessagesDelayExport>(element_id)
                .map_err(DataExtractionError::SourceDataParseError)?
                .messages_delays,
        ),
        AnalysisProperty::MessageLatencies => ChartableData::I64(
            store
                .get_by_id::<MessageLatencyExport>(element_id)
                .map_err(DataExtractionError::SourceDataParseError)?
                .messages_latencies,
        ),
    })
}

impl ChartableData {
    pub fn export(&self, output: &mut impl Write) -> color_eyre::eyre::Result<()> {
        let data = match self {
            ChartableData::I64(items) => serde_json::to_string(&items)?,
        };

        writeln!(output, "{data}")?;

        Ok(())
    }
}
