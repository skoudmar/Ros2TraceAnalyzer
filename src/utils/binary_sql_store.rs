use std::str::FromStr;

use crate::analyses::analysis::dependency_graph::{
    ActivationDelayExport, CallbackDurationExport, MessageLatencyExport, MessagesDelayExport,
    NodeOverviewExport, PublicationDelayExport,
};
use crate::extract::{RosChannelCompleteName, RosInterfaceCompleteName};

#[derive(thiserror::Error, std::fmt::Debug)]
pub enum BinarySQLStoreError {
    #[error("rusqlite error: {0}")]
    SQLiteError(#[source] rusqlite::Error),
    #[error("Binary bundle {0:?} does not exist")]
    NoStore(std::path::PathBuf),
    #[error("IO error: {0}")]
    IOError(#[from] std::io::Error),
    #[error("The query returned no results")]
    NoResults,
}

impl From<rusqlite::Error> for BinarySQLStoreError {
    fn from(value: rusqlite::Error) -> Self {
        match value {
            rusqlite::Error::QueryReturnedNoRows => BinarySQLStoreError::NoResults,
            e => BinarySQLStoreError::SQLiteError(e),
        }
    }
}

pub struct BinarySqlStore {
    connection: rusqlite::Connection,
}

impl BinarySqlStore {
    pub fn open(sqlite_path: &std::path::Path) -> Result<Self, BinarySQLStoreError> {
        if !sqlite_path.exists() {
            return Err(BinarySQLStoreError::NoStore(sqlite_path.to_path_buf()));
        }

        let sqlite_connection = rusqlite::Connection::open(sqlite_path)?;

        let store: Self = Self {
            connection: sqlite_connection,
        };

        let meta = store.get_by_id::<Metadata>(0)?;

        if meta.version != Self::VERSION {
            log::warn!(
                "Mismatched file version in {sqlite_path:?}, expected: {}, got: {}",
                Self::VERSION,
                meta.version
            );
        }

        Ok(store)
    }

    pub fn new(sqlite_path: &std::path::Path) -> Result<Self, BinarySQLStoreError> {
        if sqlite_path.exists() {
            std::fs::remove_file(sqlite_path)?;
        }

        let connection = rusqlite::Connection::open(sqlite_path)?;

        let mut store = BinarySqlStore { connection };

        store.insert(&[Metadata {
            version: Self::VERSION,
        }])?;

        Ok(store)
    }

    pub fn insert<T: Entity>(&mut self, values: &[T]) -> Result<(), BinarySQLStoreError> {
        self.connection.execute(
            &format!(
                "CREATE TABLE IF NOT EXISTS {} ({})",
                T::TABLE,
                T::PARAMS
                    .iter()
                    .map(|p| format!("{} {}", p.name, p.constraints))
                    .collect::<Vec<_>>()
                    .join(", "),
            ),
            (),
        )?;

        let tx = self.connection.transaction()?;

        {
            let mut query = tx.prepare_cached(&format!(
                "INSERT INTO {} ({}) VALUES ({})",
                T::TABLE,
                T::PARAMS
                    .iter()
                    .map(|p| p.name)
                    .collect::<Vec<_>>()
                    .join(", "),
                (1..)
                    .take(T::PARAMS.len())
                    .map(|v| format!("?{v}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ))?;

            for entry in values {
                query.execute(entry.to_params())?;
            }
        }

        tx.commit()?;

        Ok(())
    }

    pub fn get_by_id<T: Entity>(&self, id: usize) -> Result<T, BinarySQLStoreError> {
        Ok(self.connection.query_row(
            &format!(
                "SELECT {} FROM {} WHERE id = ?1",
                T::PARAMS
                    .iter()
                    .map(|p| p.name)
                    .collect::<Vec<_>>()
                    .join(", "),
                T::TABLE
            ),
            (id as i64,),
            |r| T::from_row(r),
        )?)
    }
}

impl BinarySqlStore {
    pub const VERSION: usize = 1;

    pub fn get_dependency_graph(&self) -> Result<DependencyGraph, BinarySQLStoreError> {
        // dependency graph has id 0 as defined in its Entity impl
        self.get_by_id(0)
    }
}

/// Definition of a table column for an `Entity`.
pub struct TableColumn {
    name: &'static str,
    constraints: &'static str, // example: INT PRIMARY KEY
}

impl TableColumn {
    const fn new(name: &'static str, constraints: &'static str) -> Self {
        Self { name, constraints }
    }
}

pub trait Entity: Sized {
    /// Name of the table for this entity
    const TABLE: &'static str;

    /// Parse this entity from named row
    fn from_row(row: &rusqlite::Row) -> Result<Self, rusqlite::Error>;

    /// List of parameters this entity has with their type and constraints
    const PARAMS: &'static [TableColumn];

    /// Convert the entity to parameters for insertion
    ///
    /// the parameter order must be the same as given by the `params` method
    fn to_params(&self) -> impl rusqlite::Params;
}

impl Entity for MessageLatencyExport {
    const PARAMS: &'static [TableColumn] = &[
        TableColumn::new("id", "INT PRIMARY KEY"),
        TableColumn::new("source_node", "TEXT"),
        TableColumn::new("destination_node", "TEXT"),
        TableColumn::new("topic", "TEXT"),
        TableColumn::new("latencies", "BLOB"),
    ];
    const TABLE: &'static str = "message_latency";

    fn from_row(row: &rusqlite::Row) -> Result<Self, rusqlite::Error> {
        Ok(MessageLatencyExport {
            id: row.get::<_, i64>("id")? as usize,
            name: RosChannelCompleteName {
                source_node: row.get("source_node")?,
                destination_node: row.get("destination_node")?,
                topic: row.get("topic")?,
            },
            messages_latencies: postcard::from_bytes(&row.get::<_, Vec<_>>("latencies")?).unwrap(),
        })
    }

    fn to_params(&self) -> impl rusqlite::Params {
        (
            self.id as i64,
            &self.name.source_node,
            &self.name.destination_node,
            &self.name.topic,
            postcard::to_allocvec(&self.messages_latencies).unwrap(),
        )
    }
}

impl Entity for MessagesDelayExport {
    const PARAMS: &'static [TableColumn] = &[
        TableColumn::new("id", "INT PRIMARY KEY"),
        TableColumn::new("node", "TEXT"),
        TableColumn::new("interface", "TEXT"),
        TableColumn::new("delays", "BLOB"),
    ];
    const TABLE: &'static str = "message_delay";

    fn from_row(row: &rusqlite::Row) -> Result<Self, rusqlite::Error> {
        Ok(MessagesDelayExport {
            id: row.get::<_, i64>("id")? as usize,
            name: RosInterfaceCompleteName {
                interface: row.get("interface")?,
                node: row.get("node")?,
            },
            messages_delays: postcard::from_bytes(&row.get::<_, Vec<_>>("delays")?).unwrap(),
        })
    }

    fn to_params(&self) -> impl rusqlite::Params {
        (
            self.id as i64,
            &self.name.node,
            &self.name.interface,
            postcard::to_allocvec(&self.messages_delays).unwrap(),
        )
    }
}

impl Entity for CallbackDurationExport {
    const PARAMS: &'static [TableColumn] = &[
        TableColumn::new("id", "INT PRIMARY KEY"),
        TableColumn::new("node", "TEXT"),
        TableColumn::new("interface", "TEXT"),
        TableColumn::new("durations", "BLOB"),
    ];
    const TABLE: &'static str = "callback_duration";

    fn from_row(row: &rusqlite::Row) -> Result<Self, rusqlite::Error> {
        Ok(CallbackDurationExport {
            id: row.get::<_, i64>("id")? as usize,
            name: RosInterfaceCompleteName {
                interface: row.get("interface")?,
                node: row.get("node")?,
            },
            callback_durations: postcard::from_bytes(&row.get::<_, Vec<_>>("durations")?).unwrap(),
        })
    }

    fn to_params(&self) -> impl rusqlite::Params {
        (
            self.id as i64,
            &self.name.node,
            &self.name.interface,
            postcard::to_allocvec(&self.callback_durations).unwrap(),
        )
    }
}

impl Entity for PublicationDelayExport {
    const PARAMS: &'static [TableColumn] = &[
        TableColumn::new("id", "INT PRIMARY KEY"),
        TableColumn::new("node", "TEXT"),
        TableColumn::new("interface", "TEXT"),
        TableColumn::new("delays", "BLOB"),
    ];
    const TABLE: &'static str = "publication_delay";

    fn from_row(row: &rusqlite::Row) -> Result<Self, rusqlite::Error> {
        Ok(PublicationDelayExport {
            id: row.get::<_, i64>("id")? as usize,
            name: RosInterfaceCompleteName {
                interface: row.get("interface")?,
                node: row.get("node")?,
            },
            publication_delays: postcard::from_bytes(&row.get::<_, Vec<_>>("delays")?).unwrap(),
        })
    }

    fn to_params(&self) -> impl rusqlite::Params {
        (
            self.id as i64,
            &self.name.node,
            &self.name.interface,
            postcard::to_allocvec(&self.publication_delays).unwrap(),
        )
    }
}

impl Entity for ActivationDelayExport {
    const PARAMS: &'static [TableColumn] = &[
        TableColumn::new("id", "INT PRIMARY KEY"),
        TableColumn::new("node", "TEXT"),
        TableColumn::new("interface", "TEXT"),
        TableColumn::new("delays", "BLOB"),
    ];
    const TABLE: &'static str = "activation_delay";

    fn from_row(row: &rusqlite::Row) -> Result<Self, rusqlite::Error> {
        Ok(ActivationDelayExport {
            id: row.get::<_, i64>("id")? as usize,
            name: RosInterfaceCompleteName {
                interface: row.get("interface")?,
                node: row.get("node")?,
            },
            activation_delays: postcard::from_bytes(&row.get::<_, Vec<_>>("delays")?).unwrap(),
        })
    }

    fn to_params(&self) -> impl rusqlite::Params {
        (
            self.id as i64,
            &self.name.node,
            &self.name.interface,
            postcard::to_allocvec(&self.activation_delays).unwrap(),
        )
    }
}

impl Entity for NodeOverviewExport {
    const PARAMS: &'static [TableColumn] = &[
        TableColumn::new("id", "INT PRIMARY KEY"),
        TableColumn::new("element_type", "TEXT"),
        TableColumn::new("analyses", "BLOB"),
    ];
    const TABLE: &'static str = "node_overview";

    fn from_row(row: &rusqlite::Row) -> Result<Self, rusqlite::Error> {
        Ok(NodeOverviewExport {
            id: row.get::<_, i64>("id")? as usize,
            element_type: crate::analyses::analysis::dependency_graph::ElementType::from_str(
                &row.get::<_, String>("element_type")?,
            )
            .unwrap(),
            analyses: postcard::from_bytes(&row.get::<_, Vec<_>>("analyses")?).unwrap(),
        })
    }

    fn to_params(&self) -> impl rusqlite::Params {
        (
            self.id as i64,
            self.element_type.to_string(),
            postcard::to_allocvec(&self.analyses).unwrap(),
        )
    }
}

struct Metadata {
    version: usize,
}

impl Entity for Metadata {
    const PARAMS: &'static [TableColumn] = &[
        TableColumn::new("id", "INT PRIMARY KEY"),
        TableColumn::new("version", "INT"),
    ];
    const TABLE: &'static str = "metadata";

    fn from_row(row: &rusqlite::Row) -> Result<Self, rusqlite::Error> {
        Ok(Metadata {
            version: row.get::<_, i64>("version")? as usize,
        })
    }

    fn to_params(&self) -> impl rusqlite::Params {
        (0, self.version as i64)
    }
}

pub struct DependencyGraph {
    pub graph: String,
}

impl Entity for DependencyGraph {
    const PARAMS: &'static [TableColumn] = &[
        TableColumn::new("id", "INT PRIMARY KEY"),
        TableColumn::new("name", "TEXT"),
        TableColumn::new("graph", "TEXT"),
    ];
    const TABLE: &'static str = "graphs";

    fn from_row(row: &rusqlite::Row) -> Result<Self, rusqlite::Error> {
        Ok(Self {
            graph: row.get("graph")?,
        })
    }

    fn to_params(&self) -> impl rusqlite::Params {
        (0, "dependency_graph", &self.graph)
    }
}
