use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;
use std::path::Path;

pub mod v1;

#[derive(thiserror::Error, Debug)]
pub enum BinarySQLStoreError {
    #[error("An error occured in rusqlite {0}")]
    SQLiteError(rusqlite::Error),
}

pub trait FromRow: Sized {
    fn from_row(row: &rusqlite::Row) -> Result<Self, rusqlite::Error>;
}

impl FromRow for String {
    fn from_row(row: &rusqlite::Row) -> Result<Self, rusqlite::Error> {
        row.get(0)
    }
}

impl FromRow for i64 {
    fn from_row(row: &rusqlite::Row) -> Result<Self, rusqlite::Error> {
        row.get(0)
    }
}

#[derive(Clone, Debug)]
pub struct SqlTable {
    pub name: &'static str,
    pub attributes: &'static [&'static str],
    pub query_attributes: &'static [&'static str],
    pub filter: &'static str,
}

pub trait BinarySqlStore<Table: Hash + Eq>: BinarySqlStoreBase<Table = Table> {
    fn from_file(sqlite_path: &Path, clear: bool) -> Result<Self, BinarySQLStoreError> {
        let reusing_file = sqlite_path.exists();
        let sqlite_connection =
            rusqlite::Connection::open(sqlite_path).map_err(BinarySQLStoreError::SQLiteError)?;

        let mut store: Self = Self::from_connection(sqlite_connection);

        if reusing_file {
            if clear {
                store.clear()?;
            } else {
                let version = store.get::<i64>(store.metadata_table(), ())?;

                if version != Self::VERSION {
                    log::warn!(
                        "Mismatched file version, expected: {}, got: {}",
                        Self::VERSION,
                        version
                    );
                }
            }
        } else {
            store.insert(store.metadata_table(), [(1,)].into_iter())?;
        }

        Ok(store)
    }

    fn insert(
        &mut self,
        into: Self::Table,
        values: impl Iterator<Item = impl rusqlite::Params>,
    ) -> Result<(), BinarySQLStoreError> {
        let table = self.tables().get(&into).unwrap().clone();

        self.connection()
            .execute(
                &format!(
                    "CREATE TABLE IF NOT EXISTS {} ({})",
                    table.name,
                    table.attributes.join(",\n")
                ),
                (),
            )
            .map_err(BinarySQLStoreError::SQLiteError)?;

        let tx = self
            .connection_mut()
            .transaction()
            .map_err(BinarySQLStoreError::SQLiteError)?;

        {
            let mut query = tx
                .prepare_cached(&format!(
                    "INSERT INTO {} ({}) VALUES ({})",
                    table.name,
                    table.query_attributes.join(", "),
                    (1..)
                        .take(table.query_attributes.len())
                        .map(|v| format!("?{v}"))
                        .collect::<Vec<_>>()
                        .join(", ")
                ))
                .map_err(BinarySQLStoreError::SQLiteError)?;

            for entry in values {
                query
                    .execute(entry)
                    .map_err(BinarySQLStoreError::SQLiteError)?;
            }
        }

        tx.commit().map_err(BinarySQLStoreError::SQLiteError)
    }

    fn get<T: FromRow>(
        &self,
        from: Self::Table,
        params: impl rusqlite::Params,
    ) -> Result<T, BinarySQLStoreError> {
        let table = self.tables().get(&from).unwrap();

        self.connection()
            .query_row(
                &format!(
                    "SELECT {} FROM {} WHERE {}",
                    table.query_attributes.join(", "),
                    table.name,
                    table.filter
                ),
                params,
                |row| T::from_row(row),
            )
            .map_err(BinarySQLStoreError::SQLiteError)
    }

    fn clear(&self) -> Result<(), BinarySQLStoreError> {
        let tables = self.tables();

        for v in tables.values() {
            self.connection()
                .execute(&format!("DROP TABLE IF EXISTS {}", v.name), ())
                .map_err(BinarySQLStoreError::SQLiteError)?;
        }

        Ok(())
    }
}

pub trait BinarySqlStoreBase: Sized {
    type Table: Hash + Eq;

    const VERSION: i64;

    fn from_connection(connection: rusqlite::Connection) -> Self;

    fn metadata_table(&self) -> Self::Table;

    fn tables(&self) -> &HashMap<Self::Table, SqlTable>;

    fn connection(&self) -> &rusqlite::Connection;

    fn connection_mut(&mut self) -> &mut rusqlite::Connection;
}

impl<B: BinarySqlStoreBase<Table = T>, T: Eq + Hash> BinarySqlStore<T> for B {}
