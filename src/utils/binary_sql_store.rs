use std::fmt::Debug;
use std::path::Path;

#[derive(thiserror::Error, Debug)]
pub enum BinarySQLStoreError {
    #[error("An error occured in rusqlite {0}")]
    SQLiteError(rusqlite::Error),
    #[error("An error occured during parsing from SQL row {0}")]
    DecodeError(String),
    #[error("There is no such entry in the database")]
    NoSuchEntry,
}

pub struct BinarySQLStore {
    sqlite_connection: rusqlite::Connection,
}

impl BinarySQLStore {
    pub fn new(sqlite_file: &Path) -> Result<BinarySQLStore, BinarySQLStoreError> {
        let sqlite_connection =
            rusqlite::Connection::open(sqlite_file).map_err(BinarySQLStoreError::SQLiteError)?;

        Ok(BinarySQLStore { sqlite_connection })
    }

    pub fn define_table(
        &mut self,
        name: &str,
        attributes: &[&str],
    ) -> Result<(), BinarySQLStoreError> {
        self.sqlite_connection
            .execute(&format!("DROP TABLE IF EXISTS {};", name), ())
            .map_err(BinarySQLStoreError::SQLiteError)?;

        self.sqlite_connection
            .execute(
                &format!(
                    "CREATE TABLE {} (
                {}
            )",
                    name,
                    attributes.join(",\n")
                ),
                (),
            )
            .map_err(BinarySQLStoreError::SQLiteError)?;

        Ok(())
    }

    pub fn write_into<P: rusqlite::Params>(
        &mut self,
        table: &str,
        template: &str,
        data: impl Iterator<Item = P>,
        arg_count: usize,
    ) -> Result<(), BinarySQLStoreError> {
        let tx = self
            .sqlite_connection
            .transaction()
            .map_err(BinarySQLStoreError::SQLiteError)?;

        {
            let mut query = tx
                .prepare_cached(&format!(
                    "
                    INSERT INTO {} {} VALUES ({})
                ",
                    table,
                    template,
                    (1..)
                        .take(arg_count)
                        .map(|v| format!("?{v}"))
                        .collect::<Vec<_>>()
                        .join(",")
                ))
                .map_err(BinarySQLStoreError::SQLiteError)?;

            for entry in data {
                query
                    .execute(entry)
                    .map_err(BinarySQLStoreError::SQLiteError)?;
            }
        }

        tx.commit().map_err(BinarySQLStoreError::SQLiteError)?;
        Ok(())
    }

    pub fn read<T: FromRow>(
        &self,
        table: &str,
        template: &str,
        filter: &str,
        params: impl rusqlite::Params,
    ) -> Result<T, BinarySQLStoreError> {
        self.sqlite_connection
            .query_row(
                &format!("SELECT {} FROM {} WHERE {}", template, table, filter),
                params,
                |row| T::from_row(row),
            )
            .map_err(BinarySQLStoreError::SQLiteError)
    }
}

pub trait FromRow: Sized {
    fn from_row(row: &rusqlite::Row) -> Result<Self, rusqlite::Error>;
}

impl FromRow for String {
    fn from_row(row: &rusqlite::Row) -> Result<Self, rusqlite::Error> {
        row.get(0)
    }
}
