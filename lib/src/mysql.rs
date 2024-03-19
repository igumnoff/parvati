//! `mysql` is a module that contains the `ORM` struct that represents an Object-Relational Mapping (ORM) for a MySQL database.

use std::fmt::Debug;
use std::sync::Arc;
use async_trait::async_trait;
use futures::lock::Mutex;
use mysql_async::Conn;
use mysql_async::prelude::*;

use serde::{Deserialize, Serialize};
use crate::{deserializer_key_values, ORMError, ORMTrait, QueryBuilder, Row, serializer_error, serializer_key_values, serializer_types, serializer_values, TableDeserialize, TableSerialize};

/// `ORM` is a struct that represents an Object-Relational Mapping (ORM) for a MySQL database.
/// It contains a `Mutex` that guards an `Option` wrapping a `Conn` object from the `mysql_async` crate.
/// The `Conn` object represents a connection to the MySQL database.
#[derive(Debug)]
pub struct ORM {
    conn: Mutex<Option<Conn>>,
}

impl ORM {
    /// `connect` is an asynchronous function that establishes a connection to a MySQL database.
    /// It takes a `String` parameter `url` which is the URL of the MySQL database.
    /// It returns a `Result` that contains an `Arc<ORM>` if the connection is successful.
    /// The `Arc<ORM>` is a thread-safe reference-counted pointer to the `ORM` object.
    /// If the connection is not successful, the `Result` contains an `ORMError`.
    pub async fn connect(url: String) -> Result<Arc<ORM>, ORMError>
        where Arc<ORM>: Send + Sync + 'static
    {
        let pool = mysql_async::Pool::new(url.as_str());
        let conn = pool.get_conn().await?;
        Ok(Arc::new(ORM {
            conn: Mutex::new(Some(conn)),
        }))
    }
}
/// This is the implementation of the `ORMTrait` for the `ORM` struct.
/// The `ORMTrait` provides a set of methods for interacting with a database.
/// These methods include adding data, finding data, modifying data, and removing data.
/// It also provides methods for executing arbitrary queries and escaping strings.
/// The `ORM` struct represents an Object-Relational Mapping (ORM) for a MySQL database.
#[async_trait]
impl ORMTrait<ORM> for ORM {
    /// `add` is a method that constructs a SQL insert query for a given data object.
    /// It takes a generic parameter `T` that represents the data object.
    /// The data object must implement the `Deserialize`, `TableDeserialize`, `TableSerialize`, `Serialize`, and `Debug` traits.
    /// The `Deserialize` trait is used to deserialize the data object from a serialized format.
    /// The `TableDeserialize` and `TableSerialize` traits are used to convert the data object to and from a table format.
    /// The `Serialize` trait is used to serialize the data object into a serialized format.
    /// The `Debug` trait is used to format the data object for output during debugging.
    /// The method returns a `QueryBuilder` object that represents the SQL insert query.
    /// The `QueryBuilder` object is generic over the lifetime `'a`, the result type `R`, the entity type `E`, and the ORM type `O`.
    /// The ORM type `O` must implement the `ORMTrait`.
    fn add<T>(&self, data: T) -> QueryBuilder<T, T, ORM>
        where T: for<'a> Deserialize<'a> + TableDeserialize + TableSerialize + Serialize + Debug + 'static
    {
        let table_name = data.name();
        let types = serializer_types::to_string(&data).unwrap();
        let values = serializer_values::to_string(&data).unwrap();
        let query: String = format!("insert into {table_name} {types} values {values}");
        let qb = QueryBuilder::<T,T, ORM> {
            query: query,
            entity: Default::default(),
            orm: self,
            result: std::marker::PhantomData,
        };
        qb
    }
    /// `last_insert_rowid` is an asynchronous method that retrieves the row ID of the last inserted record.
    /// It returns a `Result` that contains the row ID as an `i64` if the operation is successful.
    /// If the operation is not successful, the `Result` contains an `ORMError`.
    /// Currently, this method is hardcoded to always return `0` as the row ID.
    /// It first locks the `conn` field of the `ORM` struct, which is a `Mutex` guarding an `Option` wrapping a `Conn` object.
    /// If the `conn` field is `None`, it returns an `ORMError::NoConnection`.
    /// Otherwise, it returns `Ok(0)`.
    async fn last_insert_rowid(&self)  -> Result<i64, ORMError>{
        let conn = self.conn.lock().await;
        if conn.is_none() {
            return Err(ORMError::NoConnection);
        }
        Ok(0)
    }
    /// `close` is an asynchronous method that closes the database connection.
    /// It first locks the `conn` field of the `ORM` struct, which is a `Mutex` guarding an `Option` wrapping a `Conn` object.
    /// If the `conn` field is `None`, it returns an `ORMError::NoConnection`.
    /// Otherwise, it attempts to disconnect the `Conn` object.
    /// If the disconnection is successful, it returns `Ok(())`.
    /// If the disconnection is not successful, it returns an `ORMError::MySQLError` containing the error from the `mysql_async` library.
    async fn close(&self)  -> Result<(), ORMError>{
        let mut conn_lock = self.conn.lock().await;
        if conn_lock.is_none() {
            return Err(ORMError::NoConnection);
        }
        let conn = conn_lock.take();
        let r = conn.unwrap().disconnect().await;
        match r {
            Ok(_) => {
                Ok(())
            }
            Err(e) => {
                Err(ORMError::MySQLError(e))
            }
        }
    }
    /// `find_one` is a method that constructs a SQL select query to find a record by its ID.
    /// It takes a generic parameter `T` that represents the data object and an `id` of type `u64`.
    /// The data object must implement the `Deserialize`, `TableDeserialize`, `TableSerialize` traits and have a static lifetime.
    /// The `Deserialize` trait is used to deserialize the data object from a serialized format.
    /// The `TableDeserialize` and `TableSerialize` traits are used to convert the data object to and from a table format.
    /// The method returns a `QueryBuilder` object that represents the SQL select query.
    /// The `QueryBuilder` object is generic over the lifetime `'a`, the result type `R`, the entity type `E`, and the ORM type `O`.
    /// The ORM type `O` must implement the `ORMTrait`.
    fn find_one<T: TableDeserialize>(&self, id: u64) -> QueryBuilder<Option<T>, T, ORM>
        where T: TableDeserialize + TableSerialize + for<'a> Deserialize<'a> + 'static
    {
        let table_name = T::same_name();

        let query: String = format!("select * from {table_name} where id = {id}");

        let qb = QueryBuilder::<Option<T>, T, ORM> {
            query,
            entity: std::marker::PhantomData,
            orm: self,
            result: std::marker::PhantomData,
        };
        qb
    }
    /// `find_many` is a method that constructs a SQL select query to find multiple records that match the provided WHERE clause.
    /// It takes a generic parameter `T` that represents the data object and a `query_where` of type `&str` which is the WHERE clause of the SQL query.
    /// The data object must implement the `Deserialize`, `TableDeserialize` traits and have a static lifetime.
    /// The `Deserialize` trait is used to deserialize the data object from a serialized format.
    /// The `TableDeserialize` trait is used to convert the data object from a table format.
    /// The method returns a `QueryBuilder` object that represents the SQL select query.
    /// The `QueryBuilder` object is generic over the lifetime `'a`, the result type `R`, the entity type `E`, and the ORM type `O`.
    /// The ORM type `O` must implement the `ORMTrait`.
    fn find_many<T>(&self, query_where: &str) -> QueryBuilder<Vec<T>, T, ORM>
        where T: for<'a> Deserialize<'a> + TableDeserialize + Debug + 'static

    {

        let table_name = T::same_name();

        let query: String = format!("select * from {table_name} where {query_where}");

        let qb = QueryBuilder::<Vec<T>, T, ORM> {
            query,
            entity: std::marker::PhantomData,
            orm: self,
            result: std::marker::PhantomData,
        };
        qb
    }

    /// `find_all` is a method that constructs a SQL select query to find all records in a table.
    /// It takes a generic parameter `T` that represents the data object.
    /// The data object must implement the `Deserialize`, `TableDeserialize` traits and have a static lifetime.
    /// The `Deserialize` trait is used to deserialize the data object from a serialized format.
    /// The `TableDeserialize` trait is used to convert the data object from a table format.
    /// The method returns a `QueryBuilder` object that represents the SQL select query.
    /// The `QueryBuilder` object is generic over the lifetime `'a`, the result type `R`, the entity type `E`, and the ORM type `O`.
    /// The ORM type `O` must implement the `ORMTrait`.
    fn find_all<T>(&self) -> QueryBuilder<Vec<T>, T, ORM>
        where T: for<'a> Deserialize<'a> + TableDeserialize + Debug + 'static {
        let table_name = T::same_name();

        let query: String = format!("select * from {table_name}");

        let qb = QueryBuilder::<Vec<T>, T, ORM> {
            query,
            entity: std::marker::PhantomData,
            orm: self,
            result: std::marker::PhantomData,
        };
        qb
    }
    /// `modify` is a method that constructs a SQL update query for a given data object.
    /// It takes a generic parameter `T` that represents the data object.
    /// The data object must implement the `TableDeserialize`, `TableSerialize`, `Serialize` traits and have a static lifetime.
    /// The `TableDeserialize` and `TableSerialize` traits are used to convert the data object to and from a table format.
    /// The `Serialize` trait is used to serialize the data object into a serialized format.
    /// The method returns a `QueryBuilder` object that represents the SQL update query.
    /// The `QueryBuilder` object is generic over the lifetime `'a`, the result type `R`, the entity type `E`, and the ORM type `O`.
    /// The ORM type `O` must implement the `ORMTrait`.

    fn modify<T>(&self, data: T) -> QueryBuilder<usize, (), ORM>
        where T: TableDeserialize + TableSerialize + Serialize + 'static
    {
        let table_name = data.name();
        let key_value_str = serializer_key_values::to_string(&data).unwrap();
        // remove first and last char
        let key_value = &key_value_str[1..key_value_str.len()-1];
        let id = data.get_id();
        let query: String = format!("update {table_name} set {key_value} where id = {id}");
        let qb = QueryBuilder::<usize, (), ORM> {
            query,
            entity: std::marker::PhantomData,
            orm: self,
            result: std::marker::PhantomData,
        };
        qb
    }
    /// `remove` is a method that constructs a SQL delete query for a given data object.
    /// It takes a generic parameter `T` that represents the data object.
    /// The data object must implement the `TableDeserialize`, `TableSerialize`, `Serialize` traits and have a static lifetime.
    /// The `TableDeserialize` and `TableSerialize` traits are used to convert the data object to and from a table format.
    /// The `Serialize` trait is used to serialize the data object into a serialized format.
    /// The method returns a `QueryBuilder` object that represents the SQL delete query.
    /// The `QueryBuilder` object is generic over the lifetime `'a`, the result type `R`, the entity type `E`, and the ORM type `O`.
    /// The ORM type `O` must implement the `ORMTrait`.
    fn remove<T>(&self, data: T) -> QueryBuilder<usize, (), ORM>
        where T: TableDeserialize + TableSerialize + Serialize + 'static
    {
        let table_name = data.name();
        let id = data.get_id();
        let query: String = format!("delete from {table_name} where id = {id}");
        let qb = QueryBuilder::<usize, (), ORM> {
            query,
            entity: std::marker::PhantomData,
            orm: self,
            result: std::marker::PhantomData,
        };
        qb
    }
    /// `query` is a method that constructs a `QueryBuilder` for a given SQL query.
    /// It takes a `query` of type `&str` which is the SQL query.
    /// The method returns a `QueryBuilder` object that represents the SQL query.
    /// The `QueryBuilder` object is generic over the lifetime `'a`, the result type `R`, the entity type `E`, and the ORM type `O`.
    /// The ORM type `O` must implement the `ORMTrait`.
    fn query<T>(&self, query: &str) -> QueryBuilder<Vec<T>, T, ORM> {
        let qb = QueryBuilder::<Vec<T>, T, ORM> {
            query: query.to_string(),
            entity: std::marker::PhantomData,
            orm: self,
            result: std::marker::PhantomData,
        };
        qb
    }
    /// `query_update` is a method that constructs a `QueryBuilder` for a given SQL update query.
    /// It takes a `query` of type `&str` which is the SQL update query.
    /// The method returns a `QueryBuilder` object that represents the SQL update query.
    /// The `QueryBuilder` object is generic over the lifetime `'a`, the result type `R`, the entity type `E`, and the ORM type `O`.
    /// The ORM type `O` must implement the `ORMTrait`.
    fn query_update(&self, query: &str) -> QueryBuilder<usize, (), ORM> {
        let qb = QueryBuilder::<usize, (), ORM> {
            query: query.to_string(),
            entity: std::marker::PhantomData,
            orm: self,
            result: std::marker::PhantomData,
        };
        qb
    }

    fn protect(&self, value: &str) -> String {
        let protected: String = format!("\"{}\"", ORM::escape(value));
        protected

    }
    fn escape(str: &str) -> String {
        let mut escaped = String::new();

        for c in str.chars() {
            match c {
                // '\'' => escaped.push_str("\\'"),
                '"' => escaped.push_str("\"\""),
                // '\\' => escaped.push_str("\\\\"),
                // '\n' => escaped.push_str("\\n"),
                // '\r' => escaped.push_str("\\r"),
                // '\t' => escaped.push_str("\\t"),
                // '\x08' => escaped.push_str("\\b"),
                // '\x0C' => escaped.push_str("\\f"),
                _ => escaped.push(c),
            }
        }

        escaped
    }

    fn escape_json(input: &str) -> String {
        let input = input.to_string();
        let mut escaped = input.clone();
        escaped = escaped.replace("\\", "\\\\");
        escaped = escaped.replace("\"", "\\\"");
        // escaped = escaped.replace("\\\"\\\\\"", "\\\"\\\"");

        // for c in input.chars() {
        //     match c {
        //         '"' => escaped.push_str("\\\""),
        //         // '\\' => escaped.push_str("\\\\"),
        //         // '\n' => escaped.push_str("\\n"),
        //         // '\r' => escaped.push_str("\\r"),
        //         // '\t' => escaped.push_str("\\t"),
        //         // '\x08' => escaped.push_str("\\b"),
        //         // '\x0C' => escaped.push_str("\\f"),
        //         _ => escaped.push(c),
        //     }
        // }
        escaped
    }

    /// `init` is an asynchronous method that initializes the database with a provided script.
    /// It takes a `script` of type `&str` which is the path to the script file.
    /// The script file should contain SQL queries that initialize the database.
    /// The method reads the script file and executes the SQL queries in the script.
    /// It returns a `Result` that contains `()` if the operation is successful.
    /// If the operation is not successful, the `Result` contains an `ORMError`.
    async fn init(&self, script: &str) -> Result<(), ORMError>  {
        let query = std::fs::read_to_string(script)?;
        let _updated_rows: usize = self.query_update(query.as_str()).exec().await?;

        Ok(())
    }

    async fn change(&self, _update_query: &str) -> anyhow::Result<(), ORMError> {
        todo!()
    }
}

/// Implementation of the `QueryBuilder` struct for the `ORM` struct.
/// The `QueryBuilder` struct is used to construct SQL queries in a safe and convenient manner.
impl<T> QueryBuilder<'_, usize, T, ORM>{

    /// `exec` is an asynchronous method that executes the SQL query represented by the `QueryBuilder` object.
    /// It first locks the `conn` field of the `ORM` struct, which is a `Mutex` guarding an `Option` wrapping a `Conn` object.
    /// If the `conn` field is `None`, it returns an `ORMError::NoConnection`.
    /// Otherwise, it executes the SQL query and returns a `Result` that contains the number of affected rows as an `usize`.
    /// If the execution of the SQL query is not successful, the `Result` contains an `ORMError`.
    pub async fn exec(&self) -> Result<usize, ORMError> {
        log::debug!("{:?}", self.query);
        let mut conn = self.orm.conn.lock().await;
        if conn.is_none() {
            return Err(ORMError::NoConnection);
        }
        let conn = conn.as_mut().unwrap();
        let r = conn.query_iter(self.query.as_str()).await.map(|result| {
            result.affected_rows()
        })?;
        Ok(r as usize)
    }
}
/// Implementation of the `QueryBuilder` struct for the `ORM` struct.
/// The `QueryBuilder` struct is used to construct SQL queries in a safe and convenient manner.
impl<T> QueryBuilder<'_, T,T, ORM>{

    /// `apply` is an asynchronous method that executes the SQL insert query represented by the `QueryBuilder` object and returns the inserted record.
    /// It first locks the `conn` field of the `ORM` struct, which is a `Mutex` guarding an `Option` wrapping a `Conn` object.
    /// If the `conn` field is `None`, it returns an `ORMError::NoConnection`.
    /// Otherwise, it executes the SQL insert query and retrieves the row ID of the last inserted record.
    /// If the row ID is `None`, it returns an `ORMError::InsertError`.
    /// Otherwise, it constructs a SQL select query to find the inserted record by its row ID and executes the select query.
    /// If the select query does not return any records, it returns an `ORMError::InsertError`.
    /// Otherwise, it returns a `Result` that contains the inserted record as `T`.
    /// If the execution of the SQL select query is not successful, the `Result` contains an `ORMError`.
    pub async fn apply(&self) -> Result<T, ORMError>
        where T: for<'a> Deserialize<'a> + TableDeserialize + TableSerialize + Debug + 'static
    {
        log::debug!("{:?}", self.query);
        let r = {
            let mut conn = self.orm.conn.lock().await;
            if conn.is_none() {
                return Err(ORMError::NoConnection);
            }
            let conn = conn.as_mut().unwrap();
            let r = conn.query_iter(self.query.as_str()).await.map(|result| {
                result.last_insert_id()
            })?;
            if r.is_none() {
                return Err(ORMError::InsertError);
            }
            r.unwrap()

        };
        let rows: Vec<T> = self.orm.find_many(format!("id = {}", r).as_str()).run().await?;
        if rows.len() == 0 {
            return Err(ORMError::InsertError);
        }
        let t_opt = rows.into_iter().next();
        match t_opt {
            Some(t) => Ok(t),
            None => Err(ORMError::InsertError),
        }

    }
}
/// Implementation of the `QueryBuilder` struct for the `ORM` struct.
/// The `QueryBuilder` struct is used to construct SQL queries in a safe and convenient manner.
impl<T> QueryBuilder<'_, usize,T, ORM> {

    /// `run` is an asynchronous method that executes the SQL query represented by the `QueryBuilder` object.
    /// It first locks the `conn` field of the `ORM` struct, which is a `Mutex` guarding an `Option` wrapping a `Conn` object.
    /// If the `conn` field is `None`, it returns an `ORMError::NoConnection`.
    /// Otherwise, it executes the SQL query and returns a `Result` that contains the number of affected rows as an `usize`.
    /// If the execution of the SQL query is not successful, the `Result` contains an `ORMError`.
    pub async fn run(&self) -> Result<usize, ORMError> {
        log::debug!("{:?}", self.query);
        let mut conn = self.orm.conn.lock().await;
        if conn.is_none() {
            return Err(ORMError::NoConnection);
        }
        let conn = conn.as_mut().unwrap();
        let r = conn.query_iter(self.query.as_str()).await?;
        Ok(r.affected_rows() as usize)
    }
}
/// Implementation of the `QueryBuilder` struct for the `ORM` struct.
/// The `QueryBuilder` struct is used to construct SQL queries in a safe and convenient manner.
impl<T> QueryBuilder<'_, Option<T>,T, ORM>
    where T: for<'a> Deserialize<'a> + TableDeserialize + Debug + 'static
{
    /// `run` is an asynchronous method that executes the SQL select query represented by the `QueryBuilder` object and returns the selected record.
    /// It first executes the SQL select query and retrieves the rows that match the query.
    /// If no rows match the query, it returns `Ok(None)`.
    /// Otherwise, it constructs a JSON string that represents the selected record.
    /// The JSON string is constructed by iterating over the rows and columns and formatting them as key-value pairs.
    /// The keys are the column names and the values are the column values.
    /// The column values are escaped using the `ORM::escape_json` method to ensure they are valid JSON strings.
    /// If a column value is `None`, it is represented as `"null"` in the JSON string.
    /// The JSON string is then deserialized into the data object `T` using the `deserializer_key_values::from_str` function.
    /// If the deserialization is successful, it returns `Ok(Some(T))`.
    /// If the deserialization is not successful, it returns an `ORMError::Unknown`.
    pub async fn run(&self) -> Result<Option<T>, ORMError> {

        let rows  = self.orm.query(self.query.clone().as_str()).exec().await?;
        let columns: Vec<String> =T::fields();
        if rows.len() == 0 {
            return Ok(None);
        } else {
            let mut column_str: Vec<String> = Vec::new();
            for row in rows {
                let mut i = 0;
                for column in columns.iter() {
                    let value_opt:Option<String> = row.get(i);
                    let value = match value_opt {
                        Some(v) => {
                            format!("\"{}\"", ORM::escape_json(v.as_str()))
                        }
                        None => {
                            "null".to_string()
                        }
                    };
                    column_str.push(format!("\"{}\":{}", column, value));
                    i = i + 1;
                }
            }
            let user_str = format!("{{{}}}", column_str.join(","));
            // log::debug!("zzz{}", user_str);
            let user: T = deserializer_key_values::from_str(&user_str).unwrap();
            Ok(Some(user))

        }

    }
}

/// Implementation of the `QueryBuilder` struct for the `ORM` struct.
/// The `QueryBuilder` struct is used to construct SQL queries in a safe and convenient manner.
impl<R> QueryBuilder<'_, Vec<Row>,R, ORM> {

    /// `exec` is an asynchronous method that executes the SQL query represented by the `QueryBuilder` object.
    /// It first locks the `conn` field of the `ORM` struct, which is a `Mutex` guarding an `Option` wrapping a `Conn` object.
    /// If the `conn` field is `None`, it returns an `ORMError::NoConnection`.
    /// Otherwise, it executes the SQL query and retrieves the rows that match the query.
    /// It then iterates over the rows and columns to construct a `Row` object for each row.
    /// The `Row` object contains a `HashMap` where the keys are column indices and the values are the column values.
    /// The column values are retrieved from the row using the `get` method.
    /// If a column value is `None`, it breaks the loop and moves on to the next row.
    /// Otherwise, it sets the column value in the `Row` object using the `set` method.
    /// The `Row` object is then pushed to the `result` vector.
    /// After all rows have been processed, it returns a `Result` that contains the `result` vector.
    /// If the execution of the SQL query is not successful, the `Result` contains an `ORMError`.
    pub async fn exec(&self) -> Result<Vec<Row>, ORMError>
    {
        log::debug!("{:?}", self.query);
        let mut conn = self.orm.conn.lock().await;
        if conn.is_none() {
            return Err(ORMError::NoConnection);
        }
        let conn = conn.as_mut().unwrap();
        let stmt_result = conn.query_iter( self.query.as_str()).await;
         if stmt_result.is_err() {
            let e = stmt_result.err().unwrap();
            log::error!("{:?}", e);
            return Err(ORMError::MySQLError(e));
        }
        let mut stmt = stmt_result.unwrap();
        let columns =stmt.columns();
        let columns = columns.unwrap();
        let columns_type: Vec<bool> = columns.iter().map(|column| {
            column.column_type().is_numeric_type()
        }).collect();
        let mut result: Vec<Row> = Vec::new();
        // println!("{:?}", columns_type);
        stmt.for_each(|row| {
            let mut i = 0;
            let mut r: Row = Row::new();
            loop {
                if i > columns_type.len() - 1 {
                    break;
                }
                if columns_type[i] {
                    let res: Option<i32>= row.get(i);
                    if res.is_none() {
                        break;
                    }
                    r.set(i.try_into().unwrap(), res);
                } else {
                    let res: Option<String>= row.get(i);
                    if res.is_none() {
                        break;
                    }
                    r.set(i.try_into().unwrap(), res);
                }
                i = i + 1;
            }
            result.push(r);
        }).await?;

        // log::debug!("{:?}", result);

        Ok(result)
    }


}

/// Implementation of the `QueryBuilder` struct for the `ORM` struct.
/// The `QueryBuilder` struct is used to construct SQL queries in a safe and convenient manner.
impl<T> QueryBuilder<'_, Vec<T>,T, ORM> {

    /// `run` is an asynchronous method that executes the SQL select query represented by the `QueryBuilder` object and returns the selected records.
    /// It first executes the SQL select query and retrieves the rows that match the query.
    /// It then iterates over the rows and columns to construct a JSON string for each row.
    /// The JSON string is constructed by formatting the column names and values as key-value pairs.
    /// The column values are escaped using the `ORM::escape_json` method to ensure they are valid JSON strings.
    /// If a column value is `None`, it is represented as `"null"` in the JSON string.
    /// The JSON string is then deserialized into the data object `T` using the `deserializer_key_values::from_str` function.
    /// If the deserialization is successful, the data object is pushed to the `result` vector.
    /// After all rows have been processed, it returns a `Result` that contains the `result` vector.
    /// If the deserialization is not successful, it returns an `ORMError::Unknown`.
    /// If the execution of the SQL select query is not successful, the `Result` contains an `ORMError`.
    pub async fn run(&self) -> Result<Vec<T>, ORMError>
        where T: for<'a> Deserialize<'a> + TableDeserialize + Debug + 'static
    {

        let mut result: Vec<T> = Vec::new();
        let rows  = self.orm.query(self.query.clone().as_str()).exec().await?;
        let columns: Vec<String> =T::fields();
        for row in rows {
            let mut column_str: Vec<String> = Vec::new();
            let mut i = 0;
            // println!("{:?}", row);
            for column in columns.iter() {
                let value_opt:Option<String> = row.get(i);
                let value = match value_opt {
                    Some(v) => {
                        format!("\"{}\"", ORM::escape_json(v.as_str()))
                    }
                    None => {
                        "null".to_string()
                    }
                };
                column_str.push(format!("\"{}\":{}", column, value));
                i = i + 1;
            }
            let user_str = format!("{{{}}}", column_str.join(","));
            // log::info!("{}", user_str);
            let user_result: std::result::Result<T, serializer_error::Error> = deserializer_key_values::from_str(&user_str);
            match user_result {
                Ok(user) => {
                    result.push(user);
                }
                Err(e) => {
                    log::error!("{:?}", e);
                    log::error!("{}", user_str);
                    return Err(ORMError::Unknown);
                }
            }

        }

        Ok(result)
    }
    /// `limit` is a method that modifies the SQL query represented by the `QueryBuilder` object to limit the number of records returned.
    /// It takes a parameter `limit` of type `i32` which is the maximum number of records to return.
    /// The method constructs a new SQL query by appending "limit {limit}" to the existing SQL query, where {limit} is the `limit` parameter.
    /// It then returns a new `QueryBuilder` object that represents the modified SQL query.
    /// The `QueryBuilder` object is generic over the lifetime `'a`, the result type `R`, the entity type `E`, and the ORM type `O`.
    /// The ORM type `O` must implement the `ORMTrait`.
    pub fn limit(&self, limit: i32) -> QueryBuilder<Vec<T>, T, ORM> {

        let qb =  QueryBuilder::<Vec<T>,T, ORM> {
            query: format!("{} limit {}", self.query, limit),
            entity: std::marker::PhantomData,
            orm: self.orm,
            result: std::marker::PhantomData,
        };
        qb
    }
}

