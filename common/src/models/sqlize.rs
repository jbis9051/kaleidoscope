
#[macro_export]
macro_rules! question_marks {
    // Base case: when no arguments are provided, return an empty string
    () => {
        "".to_string()
    };
    // Recursive case: for each argument, add a "?" to the result
    ($head:expr) => {
        "?".to_string()
    };
    // Recursive case: for each argument, add a "?" to the result
    ($head:expr, $($tail:expr),*) => {
        format!("?,{}", question_marks!($($tail),*))
    };
}

#[macro_export]
macro_rules! update_set {
    ($head:ident) => {
        format!("{} = ?", stringify!($head))
    };
    ($head:ident, $($tail:ident),*) => {
        format!("{} = ?,{}", stringify!($head), update_set!($($tail),*))
    };
}

#[macro_export]
macro_rules! debug_sql {
     ($name: ty, $table: literal, $id: tt, [$($col: tt),*]) => {
        println!("{:?}", &format!("INSERT INTO {} ({}) VALUES ({})",
                            $table,
                            stringify!($($col),*),
                            question_marks!($($col),*)
                        ));

        println!("{:?}", &format!("UPDATE {} SET {} WHERE id = ?",
                            $table,
                            update_set!($($col),*)
                        ));
    }
}


// this should really be a derive macro but i'm lazy
#[macro_export]
macro_rules! sqlize {
    ($name: ty, $table: literal, $id: tt, [$($col: tt),*]) => {

        impl From<&SqliteRow> for $name {
            fn from(row: &SqliteRow) -> Self {
                Self {
                    $id: row.get(stringify!($id)),
                    $(
                        $col: row.get(stringify!($col)),
                    )*
                }
            }
        }

        impl $name {
            pub async fn create(&mut self, db: impl SqliteAcquire<'_>) -> Result<(), sqlx::Error> {
                let mut conn = db.acquire().await?;
                *self = sqlx::query(
                        &format!("INSERT INTO {} ({}) VALUES ({}) RETURNING *",
                            $table,
                            stringify!($($col),*),
                            question_marks!($($col),*)
                        )
                    )
                    $(
                        .bind(&self.$col)
                    )*
                    .fetch_one(&mut *conn)
                    .await?
                    .borrow()
                    .into();
                Ok(())
            }

            pub async fn update_by_id<'a, T: SqliteExecutor<'a>>(&self, db: T) -> Result<(), sqlx::Error> {
                sqlx::query(
                        &format!("UPDATE {} SET {} WHERE id = ?",
                            $table,
                            update_set!($($col),*)
                        )
                    )
                    $(
                        .bind(&self.$col)
                    )*
                    .bind(&self.$id)
                    .execute(db)
                    .await?;
                Ok(())
            }
        }
    };
}
