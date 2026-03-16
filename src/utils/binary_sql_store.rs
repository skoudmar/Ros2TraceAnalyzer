use crate::analyses::analysis::dependency_graph::{
    ActivationDelayExport, CallbackDurationExport, MessageLatencyExport, MessagesDelayExport,
    PublicationDelayExport,
};
use crate::extract::{RosChannelCompleteName, RosInterfaceCompleteName};

#[derive(thiserror::Error, std::fmt::Debug)]
pub enum BinarySQLStoreError {
    #[error("rusqlite error: {0}")]
    SQLiteError(#[from] rusqlite::Error),
    #[error("Binary bundle {0:?} does not exist")]
    NoStore(std::path::PathBuf),
    #[error("IO error: {0}")]
    IOError(#[from] std::io::Error),
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
                "Mismatched file version, expected: {}, got: {}",
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

        let sqlite_connection = rusqlite::Connection::open(sqlite_path)?;

        let mut store = BinarySqlStore {
            connection: sqlite_connection,
        };

        store.insert(&[Metadata {
            version: Self::VERSION,
        }])?;

        Ok(store)
    }

    pub fn insert<T: Entity>(&mut self, values: &[T]) -> Result<(), BinarySQLStoreError> {
        self.connection.execute(
            &format!(
                "CREATE TABLE IF NOT EXISTS {} ({})",
                T::table(),
                T::params()
                    .iter()
                    .map(|p| format!("{} {}", p.0, p.1))
                    .collect::<Vec<_>>()
                    .join(", "),
            ),
            (),
        )?;

        let tx = self.connection.transaction()?;

        {
            let mut query = tx.prepare_cached(&format!(
                "INSERT INTO {} ({}) VALUES ({})",
                T::table(),
                T::params()
                    .iter()
                    .map(|p| p.0)
                    .collect::<Vec<_>>()
                    .join(", "),
                (1..)
                    .take(T::params().len())
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
                T::params()
                    .iter()
                    .map(|p| p.0)
                    .collect::<Vec<_>>()
                    .join(", "),
                T::table()
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

pub trait Entity: Sized {
    /// Name of the table for this entity
    fn table<'a>() -> &'a str;

    /// Parse this entity from named row
    fn from_row(row: &rusqlite::Row) -> Result<Self, rusqlite::Error>;

    /// List of parameters this entity has with their type and constraints
    ///
    /// The format is this: [("<name>", "<type constraints>")]. An example would be
    /// [("id", "INT PRIMARY KEY")]
    fn params<'a>() -> &'a [(&'a str, &'a str)];

    /// Convert the entity to parameters for insertion
    ///
    /// the parameter order must be the same as given by the `params` method
    fn to_params(&self) -> impl rusqlite::Params;
}

impl Entity for MessageLatencyExport {
    fn table<'a>() -> &'a str {
        "message_latencies"
    }

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

    fn params<'a>() -> &'a [(&'a str, &'a str)] {
        &[
            ("id", "INT PRIMARY KEY"),
            ("source_node", "TEXT"),
            ("destination_node", "TEXT"),
            ("topic", "TEXT"),
            ("latencies", "BLOB"),
        ]
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
    fn table<'a>() -> &'a str {
        "message_delays"
    }

    fn from_row(row: &rusqlite::Row) -> Result<Self, rusqlite::Error> {
        Ok(MessagesDelayExport {
            id: row.get::<_, i64>("id")? as usize,
            name: RosInterfaceCompleteName {
                interface: row.get("interface")?,
                node: row.get("node")?,
            },
            messages_delays: postcard::from_bytes(&row.get::<_, Vec<_>>("latencies")?).unwrap(),
        })
    }

    fn params<'a>() -> &'a [(&'a str, &'a str)] {
        &[
            ("id", "INT PRIMARY KEY"),
            ("node", "TEXT"),
            ("interface", "TEXT"),
            ("delays", "BLOB"),
        ]
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
    fn table<'a>() -> &'a str {
        "callback_durations"
    }

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

    fn params<'a>() -> &'a [(&'a str, &'a str)] {
        &[
            ("id", "INT PRIMARY KEY"),
            ("node", "TEXT"),
            ("interface", "TEXT"),
            ("durations", "BLOB"),
        ]
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
    fn table<'a>() -> &'a str {
        "publication_delays"
    }

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

    fn params<'a>() -> &'a [(&'a str, &'a str)] {
        &[
            ("id", "INT PRIMARY KEY"),
            ("node", "TEXT"),
            ("interface", "TEXT"),
            ("delays", "BLOB"),
        ]
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
    fn table<'a>() -> &'a str {
        "activation_delays"
    }

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

    fn params<'a>() -> &'a [(&'a str, &'a str)] {
        &[
            ("id", "INT PRIMARY KEY"),
            ("node", "TEXT"),
            ("interface", "TEXT"),
            ("delays", "BLOB"),
        ]
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

struct Metadata {
    version: usize,
}

impl Entity for Metadata {
    fn table<'a>() -> &'a str {
        "metadata"
    }

    fn from_row(row: &rusqlite::Row) -> Result<Self, rusqlite::Error> {
        Ok(Metadata {
            version: row.get::<_, i64>("version")? as usize,
        })
    }

    fn params<'a>() -> &'a [(&'a str, &'a str)] {
        &[("id", "INT PRIMARY KEY"), ("version", "INT")]
    }

    fn to_params(&self) -> impl rusqlite::Params {
        (0, self.version as i64)
    }
}

pub struct DependencyGraph {
    pub graph: String,
}

impl Entity for DependencyGraph {
    fn table<'a>() -> &'a str {
        "graphs"
    }

    fn from_row(row: &rusqlite::Row) -> Result<Self, rusqlite::Error> {
        Ok(Self {
            graph: row.get("graph")?,
        })
    }

    fn params<'a>() -> &'a [(&'a str, &'a str)] {
        &[
            ("id", "INT PRIMARY KEY"),
            ("name", "TEXT"),
            ("graph", "TEXT"),
        ]
    }

    fn to_params(&self) -> impl rusqlite::Params {
        (0, "dependency_graph", &self.graph)
    }
}
