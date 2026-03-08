use std::collections::HashMap;

use crate::argsv2::extract_args::AnalysisProperty;
use crate::utils::binary_sql_store::{BinarySqlStoreBase, FromRow, SqlTable};

pub struct BinarySqlStoreV1(
    rusqlite::Connection,
    HashMap<BinarySqlStoreV1Table, SqlTable>,
);

impl BinarySqlStoreBase for BinarySqlStoreV1 {
    type Table = BinarySqlStoreV1Table;

    const VERSION: i64 = 1;

    fn tables(&self) -> &HashMap<Self::Table, SqlTable> {
        &self.1
    }

    fn connection(&self) -> &rusqlite::Connection {
        &self.0
    }

    fn connection_mut(&mut self) -> &mut rusqlite::Connection {
        &mut self.0
    }

    fn from_connection(connection: rusqlite::Connection) -> Self {
        BinarySqlStoreV1(connection, Self::tables())
    }

    fn metadata_table(&self) -> Self::Table {
        BinarySqlStoreV1Table::Metadata
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum BinarySqlStoreV1Table {
    Metadata,
    Graphs,
    Property(AnalysisProperty),
}

impl BinarySqlStoreV1 {
    fn tables() -> HashMap<BinarySqlStoreV1Table, SqlTable> {
        HashMap::from([
            (
                BinarySqlStoreV1Table::Metadata,
                SqlTable {
                    name: "metadata",
                    attributes: &["version INT PRIMARY KEY"],
                    query_attributes: &["version"],
                    filter: "TRUE",
                },
            ),
            (
                BinarySqlStoreV1Table::Graphs,
                SqlTable {
                    name: "graphs",
                    attributes: &["name TEXT PRIMARY KEY", "graph TEXT"],
                    query_attributes: &["name", "graph"],
                    filter: "name LIKE ?1",
                },
            ),
            (
                BinarySqlStoreV1Table::Property(AnalysisProperty::MessageLatencies),
                SqlTable {
                    name: AnalysisProperty::MessageLatencies.table_name(),
                    attributes: &[
                        "id INT PRIMARY KEY",
                        "source_node TEXT",
                        "destination_node TEXT",
                        "topic TEXT",
                        "data BLOB",
                    ],
                    query_attributes: &["id", "source_node", "destination_node", "topic", "data"],
                    filter: "id = ?1",
                },
            ),
            (
                BinarySqlStoreV1Table::Property(AnalysisProperty::ActivationDelays),
                SqlTable {
                    name: AnalysisProperty::ActivationDelays.table_name(),
                    attributes: &[
                        "id INT PRIMARY KEY",
                        "node TEXT",
                        "interface TEXT",
                        "data BLOB",
                    ],
                    query_attributes: &["id", "node", "interface", "data"],
                    filter: "id = ?1",
                },
            ),
            (
                BinarySqlStoreV1Table::Property(AnalysisProperty::PublicationDelays),
                SqlTable {
                    name: AnalysisProperty::PublicationDelays.table_name(),
                    attributes: &[
                        "id INT PRIMARY KEY",
                        "node TEXT",
                        "interface TEXT",
                        "data BLOB",
                    ],
                    query_attributes: &["id", "node", "interface", "data"],
                    filter: "id = ?1",
                },
            ),
            (
                BinarySqlStoreV1Table::Property(AnalysisProperty::MessageDelays),
                SqlTable {
                    name: AnalysisProperty::MessageDelays.table_name(),
                    attributes: &[
                        "id INT PRIMARY KEY",
                        "node TEXT",
                        "interface TEXT",
                        "data BLOB",
                    ],
                    query_attributes: &["id", "node", "interface", "data"],
                    filter: "id = ?1",
                },
            ),
            (
                BinarySqlStoreV1Table::Property(AnalysisProperty::CallbackDurations),
                SqlTable {
                    name: AnalysisProperty::CallbackDurations.table_name(),
                    attributes: &[
                        "id INT PRIMARY KEY",
                        "node TEXT",
                        "interface TEXT",
                        "data BLOB",
                    ],
                    query_attributes: &["id", "node", "interface", "data"],
                    filter: "id = ?1",
                },
            ),
        ])
    }
}

pub struct GraphEntry {
    pub name: String,
    pub graph: String,
}

impl FromRow for GraphEntry {
    fn from_row(row: &rusqlite::Row) -> Result<Self, rusqlite::Error> {
        Ok(GraphEntry {
            name: row.get("name")?,
            graph: row.get("graph")?,
        })
    }
}
