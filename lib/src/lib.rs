//! # Parvati SDK
//! Indeed, an ORM library, not a framework, written in Rust
//!
//! ## Features
//!
//! The main idea that I put into my ORM library is a minimum of stupid code and easy use of the library. I wanted users not to have to write long chains of function calls to construct a simple SQL query.
//!
//! - [x] SQLite support
//! - [x] MySQL support
//!
//! ## Usage
//!
//! Cargo.toml
//!
//! ```toml
//! [dependencies]
//! parvati = {version = "1.0.0", features = ["sqlite"]} # or "mysql"
//! parvati_derive = "1.0.0"
//! ```
//!
//! ```rust
//! #[tokio::test]
//! async fn test() -> Result<(), ORMError> {
//!
//!     let file = std::path::Path::new("file1.db");
//!     if file.exists() {
//!         std::fs::remove_file(file)?;
//!     }
//!
//!     let _ = env_logger::Builder::from_env(env_logger::Env::new().default_filter_or("debug")).try_init();
//!
//!     let conn = ORM::connect("file1.db".to_string())?;
//!     let init_script = "create_table_sqlite.sql";
//!     conn.init(init_script).await?;
//!
//!     #[derive(TableDeserialize, TableSerialize, Serialize, Deserialize, Debug, Clone)]
//!     #[table(name = "user")]
//!     pub struct User {
//!         pub id: i32,
//!         pub name: Option<String>,
//!         pub age: i32,
//!     }
//!
//!     let mut user = User {
//!         id: 0,
//!         name: Some("John".to_string()),
//!         age: 30,
//!     };
//!
//!     let mut user_from_db: User = conn.add(user.clone()).apply().await?;
//!
//!     user.name = Some("Mary".to_string());
//!     let  _: User = conn.add(user.clone()).apply().await?;
//!
//!     let user_opt: Option<User> = conn.find_one(user_from_db.id as u64).run().await?;
//!     log::debug!("User = {:?}", user_opt);
//!
//!     let user_all: Vec<User> = conn.find_all().run().await?;
//!     log::debug!("Users = {:?}", user_all);
//!
//!     user_from_db.name = Some("Mike".to_string());
//!     let _updated_rows: usize = conn.modify(user_from_db.clone()).run().await?;
//!
//!
//!     let user_many: Vec<User> = conn.find_many("id > 0").limit(2).run().await?;
//!     log::debug!("Users = {:?}", user_many);
//!
//!     let query = format!("select * from user where name like {}", conn.protect("M%"));
//!     let result_set: Vec<Row> = conn.query(query.as_str()).exec().await?;
//!     for row in result_set {
//!         let id: i32 = row.get(0).unwrap();
//!         let name: Option<String> = row.get(1);
//!         log::debug!("User = id: {}, name: {:?}", id, name);
//!     }
//!
//!     let updated_rows = conn.query_update("update user set age = 100").exec().await?;
//!     log::debug!("updated_rows: {}", updated_rows);
//!     let updated_rows: usize = conn.remove(user_from_db.clone()).run().await?;
//!     log::debug!("updated_rows: {}", updated_rows);
//!     conn.close().await?;
//!
//!     Ok(())
//! }
//! ```


#[cfg(any(feature = "sqlite", feature = "mysql"))]
mod serializer_error;
#[cfg(any(feature = "sqlite", feature = "mysql"))]
mod serializer_types;
#[cfg(any(feature = "sqlite", feature = "mysql"))]
mod serializer_values;
#[cfg(any(feature = "sqlite", feature = "mysql"))]
mod serializer_key_values;
#[cfg(any(feature = "sqlite", feature = "mysql"))]
mod deserializer_key_values;

// The following module is only compiled if the "sqlite" feature is enabled.
// This module contains the implementation details for SQLite database operations.
#[cfg(feature = "sqlite")]
pub mod sqlite;

// The following module is only compiled if the "mysql" feature is enabled.
// This module contains the implementation details for MySQL database operations.
#[cfg(feature = "mysql")]
pub mod mysql;

use std::collections::HashMap;
use anyhow::Result;

use std::fmt::Debug;
use std::str::FromStr;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use thiserror::Error;

/// `ORMError` is an enumeration of possible errors that can occur in the ORM library.
/// Each variant represents a different kind of error.
#[derive(Error, Debug)]
pub enum ORMError {
    /// This variant represents a standard I/O error.
    #[error("std::io::Error")]
    StdError(#[from] std::io::Error),

    /// This variant is only available if the "sqlite" feature is enabled.
    /// It represents an error from the `rusqlite` library.
    #[cfg(feature = "sqlite")]
    #[error("rusqlite::Error")]
    RusqliteError(#[from] rusqlite::Error),

    /// This variant is only available if the "mysql" feature is enabled.
    /// It represents an error from the `mysql_async` library.
    #[cfg(feature = "mysql")]
    #[error("mysql_async::Error")]
    MySQLError(#[from] mysql_async::Error),

    /// This variant represents an unknown error.
    #[error("unknown error")]
    Unknown,

    /// This variant represents an error that occurs during object insertion.
    #[error("Error in object insertion")]
    InsertError,

    /// This variant represents an error that occurs when there is no connection.
    #[error("No connection")]
    NoConnection,
}


/// `TableSerialize` is a trait that provides methods for serializing table data.
/// This trait is used to convert table data into a format that can be stored or transmitted.
pub trait TableSerialize {
    /// Returns the name of the table.
    fn name(&self) -> String{
        "Test".to_string()
    }

    /// Returns the ID of the table.
    fn get_id(&self) -> String {
        "0".to_string()
    }
}
/// `TableDeserialize` is a trait that provides methods for deserializing table data.
/// This trait is used to convert data from a stored or transmitted format into table data.
pub trait TableDeserialize {
    /// Returns the name of the table.
    fn same_name() -> String{
        "Test".to_string()
    }

    /// Returns the fields of the table as a vector of strings.
    fn fields() -> Vec<String>{
        Vec::new()
    }
}


/// `Row` is a struct that represents a row in a database table.
/// It contains a `HashMap` where the keys are column indices and the values are the column values.
#[derive(Debug, Clone)]
pub struct Row {
    pub columns: HashMap<i32,Option<String>>,
}

impl Row {
    /// Constructs a new `Row` with an empty `HashMap`.
    pub fn new() -> Self {
        let columns = HashMap::new();
        Row {
            columns
        }
    }

    /// Retrieves a value from the `Row` by its column index.
    /// The value is returned as an `Option` that contains the value if it exists and is of the correct type.
    /// If the value does not exist or is not of the correct type, `None` is returned.
    pub fn get<Z: FromStr>(&self, index: i32) -> Option<Z>
    {
        let value = self.columns.get(&index);
        match value {
            Some(v_opt) => {
                match v_opt {
                    None => {
                        None
                    }
                    Some(v) => {
                        let r = Z::from_str(v.as_str());
                        match r {
                            Ok(res) => {
                                Some(res)
                            }
                            Err(_) => {
                                None
                            }
                        }
                    }
                }

            }
            None => {
                None
            }
        }
    }

    /// Sets a value in the `Row` at the specified column index.
    /// The value is converted to a `String` before being stored.
    pub fn set<T: ToString>(&mut self, index: i32, value: Option<T>) {
        let value = match value {
            Some(v) => {
                Some(v.to_string())
            }
            None => {
                None
            }
        };
        self.columns.insert(index, value);
    }
}

/// `ORMTrait` is a trait that provides methods for interacting with a database.
/// This trait is used to perform operations such as adding data, finding data, modifying data, and removing data.
/// It also provides methods for executing arbitrary queries and escaping strings.
#[async_trait]
pub trait ORMTrait<O:ORMTrait<O>> {
    /// Adds a new record to the database.
    /// The data is serialized and inserted into the appropriate table.
    fn add<T>(&self, data: T) -> QueryBuilder<T, T, O>
        where T: for<'a> Deserialize<'a> + TableDeserialize + TableSerialize + Serialize + Debug + 'static;

    /// Returns the row ID of the last inserted record.
    async fn last_insert_rowid(&self)  -> Result<i64, ORMError>;

    /// Closes the database connection.
    async fn close(&self)  -> Result<(), ORMError>;

    /// Finds a record by its ID.
    /// Returns an `Option` that contains the record if it exists.
    fn find_one<T: TableDeserialize>(&self, id: u64) -> QueryBuilder<Option<T>, T, O>
    where T: TableDeserialize + TableSerialize + for<'a> Deserialize<'a> + 'static;

    /// Finds multiple records that match the provided WHERE clause.
    fn find_many<T>(&self, query_where: &str) -> QueryBuilder<Vec<T>, T, O>
        where T: for<'a> Deserialize<'a> + TableDeserialize + Debug + 'static;

    /// Finds all records in the table.
    fn find_all<T>(&self) -> QueryBuilder<Vec<T>, T, O>
        where T: for<'a> Deserialize<'a> + TableDeserialize + Debug + 'static;

    /// Modifies an existing record in the database.
    /// The data is serialized and updated in the appropriate table.
    fn modify<T>(&self, data: T) -> QueryBuilder<usize, (), O>
        where T: TableDeserialize + TableSerialize + Serialize + 'static;

    /// Removes a record from the database.
    fn remove<T>(&self, data: T) -> QueryBuilder<usize, (), O>
        where T: TableDeserialize + TableSerialize + Serialize + 'static;

    /// Executes an arbitrary query and returns the results.
    fn query<T>(&self, query: &str) -> QueryBuilder<Vec<T>, T, O>;

    /// Executes an arbitrary update query and returns the number of affected rows.
    fn query_update(&self, query: &str) -> QueryBuilder<usize, (), O>;

    /// Escapes a string to protect against SQL injection.
    fn protect(&self, value: &str) -> String;

    /// Escapes a string for use in a SQL query.
    fn escape(str: &str) -> String;

    /// Escapes a string for use in a JSON value.
    fn escape_json(input: &str) -> String;

    /// Initializes the database with a provided script.
    async fn init(&self, script: &str) -> Result<(), ORMError>;

    /// Executes an update query and returns a result.
    async fn change(&self, update_query: &str) -> Result<(), ORMError>;
}

/// `QueryBuilder` is a struct that represents a SQL query builder.
/// It is used to construct SQL queries in a safe and convenient manner.
/// The `QueryBuilder` struct is generic over the lifetime `'a`, the result type `R`, the entity type `E`, and the ORM type `O`.
/// The ORM type `O` must implement the `ORMTrait`.
#[allow(dead_code)]
pub struct QueryBuilder<'a, R, E, O: ORMTrait<O>> {
    /// `query` is a `String` that contains the SQL query.
    query: String,

    /// `entity` is a marker for the entity type `E`.
    /// It is used to ensure that the `QueryBuilder` is used correctly with respect to the entity type.
    entity:  std::marker::PhantomData<E>,

    /// `orm` is a reference to an ORM object that implements the `ORMTrait`.
    /// It is used to execute the SQL query.
    orm: &'a O,

    /// `result` is a marker for the result type `R`.
    /// It is used to ensure that the `QueryBuilder` is used correctly with respect to the result type.
    result: std::marker::PhantomData<std::marker::PhantomData<R>>,
}



#[cfg(test)]
mod tests {
    use crate::ORMError;

    #[tokio::test]
    async fn test() -> Result<(), ORMError> {
        Ok(())
    }
}