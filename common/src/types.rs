use sqlx::{Pool, Sqlite, SqliteConnection, SqlitePool, Transaction};

pub type DbPool = Pool<Sqlite>;

pub trait SqliteAcquire<'a>: sqlx::Acquire<'a, Database = Sqlite> {}

// this implements the trait for:
//
// &sqlx::SqlitePool
// &mut sqlx::SqliteConnection
// &mut sqlx::Transaction<'_>
impl<'a, T> SqliteAcquire<'a> for T where T: sqlx::Acquire<'a, Database = Sqlite> {}


// there is surely a better way to do this but I have no idea
// we can use AcquireClone like this:
//
// pub async fn multiple_queries(conn: impl SqliteAcquire<'_>) -> Result<(), sqlx::Error> <-- this is like a model function that executes one or more sql queries
//
// pub async fn multiple(conn: &mut impl AcquireClone) -> Result<(), sqlx::Error> { <-- this is some normal function that will call a model function and accepts a generic db object
//     multiple_queries(conn.acquire_clone()).await?;
//     multiple_queries(conn.acquire_clone()).await?;
//     Ok(())
// }
// 
// 
// and call it like this:
//     multiple(&mut trans).await.unwrap(); <-- multiple is generic and can accept a connection, transaction or pool
//     multiple(&mut conn).await.unwrap();
//     multiple(&mut &pool).await.unwrap();
// or like this
//
//  let conn_acquired = conn.acquire_clone(); 
//     {
//         let mut acquired = conn_acquired.acquire().await?; <-- when we want to actually make a query, we do this
// 
//         let x_row = sqlx::query(
//             "SELECT 4",
//         ).fetch_one(&mut *acquired).await?;
//     }
//
// basically this means that that Model's should accept (db: impl SqliteAcquire<'_>) as a parameter
// then do this:
//
// let mut conn = db.acquire().await?;
// let x_row = sqlx::query("SELECT 4", ).fetch_one(&mut *conn).
// let y_row = sqlx::query("SELECT 5", ).fetch_one(&mut *conn).
//
//
// all other functions that either want to call Model function(s) should accept (db: &mut impl AcquireClone)
// they can call model functions like this: model_function(db.acquire_clone()).await?;
// or they can pass it to another function like this: multiple(db)
// note: if the model function needs to call another model function, it should accept (db: &mut impl AcquireClone)
pub trait AcquireClone {
    fn acquire_clone(&mut self) -> impl SqliteAcquire<'_> + Send;
}

impl AcquireClone for SqliteConnection {
    fn acquire_clone(&mut self) -> impl SqliteAcquire {
        self
    }
}

impl AcquireClone for Transaction<'_, Sqlite> {
    fn acquire_clone(&mut self) -> impl SqliteAcquire<'_> {
        self
    }
}

impl AcquireClone for &SqlitePool {
    fn acquire_clone(&mut self) -> impl SqliteAcquire<'_> {
        *self
    }
}

impl AcquireClone for &mut DbPool {
    fn acquire_clone(&mut self) -> impl SqliteAcquire<'_> {
        // bruh
        &**self
    }
}


impl AcquireClone for DbPool {
    fn acquire_clone(&mut self) -> impl SqliteAcquire<'_> {
        &*self
    }
}