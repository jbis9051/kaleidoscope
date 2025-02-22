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
// pub async fn multiple(conn: &mut impl AcquireClone) -> Result<(), sqlx::Error> {
//     multiple_queries(conn.acquire_clone()).await?;
//     multiple_queries(conn.acquire_clone()).await?;
//     Ok(())
// }
// 
// 
// pub async fn multiple_queries(conn: impl SqliteAcquire<'_>) -> Result<(), sqlx::Error>
// 
// and call it like this
//     multiple(&mut trans).await.unwrap();
//     multiple(&mut conn).await.unwrap();
//     multiple(&mut &pool).await.unwrap();
// or like this
//
//  let conn_acquired = conn.acquire_clone();
//     {
//         let mut acquired = conn_acquired.acquire().await?;
// 
//         let x_row = sqlx::query(
//             "SELECT 4",
//         ).fetch_one(&mut *acquired).await?;
//     }

pub trait AcquireClone {
    fn acquire_clone(&mut self) -> impl SqliteAcquire<'_>;
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
