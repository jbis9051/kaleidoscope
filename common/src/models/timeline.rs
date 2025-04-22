use serde::Serialize;
use sqlx::Row;
use sqlx::sqlite::SqliteRow;
use crate::media_query::MediaQuery;
use crate::models::MediaError;
use crate::types::DbPool;

#[derive(Serialize, Debug)]
pub struct TimelineMonth {
    pub year: i32,
    pub month: i32,
    pub count: i32,
}

impl From<&SqliteRow> for TimelineMonth {
    fn from(row: &SqliteRow) -> Self {
        // "2024-01"
        let interval: String = row.get("interval");
        let interval: Vec<&str> = interval.split("-").collect();
        let year = interval[0].parse::<i32>().unwrap();
        let month = interval[1].parse::<i32>().unwrap();

        Self {
            year,
            month,
            count: row.get("count"),
        }
    }
}


#[derive(Serialize, Debug)]
pub struct TimelineDay {
    pub year: i32,
    pub month: i32,
    pub day: i32,
    pub count: i32,
}

impl From<&SqliteRow> for TimelineDay {
    fn from(row: &SqliteRow) -> Self {
        // "2024-01-01"
        let interval: String = row.get("interval");
        let interval: Vec<&str> = interval.split("-").collect();
        let year = interval[0].parse::<i32>().unwrap();
        let month = interval[1].parse::<i32>().unwrap();
        let day = interval[2].parse::<i32>().unwrap();

        Self {
            year,
            month,
            day,
            count: row.get("count"),
        }
    }
}

#[derive(Serialize, Debug)]
pub struct TimelineHour {
    pub year: i32,
    pub month: i32,
    pub day: i32,
    pub hour: i32,
    pub count: i32,
}

impl From<&SqliteRow> for TimelineHour {
    fn from(row: &SqliteRow) -> Self {
        // "2024-01-01 00"
        let interval: String = row.get("interval");
        let interval: Vec<&str> = interval.split(" ").collect();
        let date: Vec<&str> = interval[0].split("-").collect();
        let year = date[0].parse::<i32>().unwrap();
        let month = date[1].parse::<i32>().unwrap();
        let day = date[2].parse::<i32>().unwrap();
        let hour = interval[1].parse::<i32>().unwrap();

        Self {
            year,
            month,
            day,
            hour,
            count: row.get("count"),
        }
    }
}


pub struct Timeline;

impl Timeline {
    async fn timeline<T: for<'a> From<&'a SqliteRow>>(db: &DbPool, media_query: &MediaQuery, interval_query: &str) -> Result<Vec<T>, MediaError> {
        let mut query =  sqlx::QueryBuilder::new(format!(
            "SELECT
                    {} AS interval,
                    COUNT(*) AS count
                 FROM media ", interval_query));

        media_query.sqlize(&mut query).expect("bad query");
        
        query.push(" GROUP BY interval \
        ORDER BY interval ASC");

        let query = query.build();

        Ok(query
            .fetch_all(db)
            .await?
            .iter()
            .map(|row| row.into())
            .collect())
    }

    pub async fn timeline_months(db: &DbPool, media_query: &MediaQuery) -> Result<Vec<TimelineMonth>, MediaError> {
        Self::timeline::<TimelineMonth>(db, media_query,"STRFTIME('%Y-%m', media.created_at)")
            .await
    }

    pub async fn timeline_days(db: &DbPool, media_query: &MediaQuery) -> Result<Vec<TimelineDay>, MediaError> {
        Self::timeline::<TimelineDay>(db, media_query,"STRFTIME('%Y-%m-%d', media.created_at)")
            .await
    }

    pub async fn timeline_hours(db: &DbPool, media_query: &MediaQuery) -> Result<Vec<TimelineHour>, MediaError> {
        Self::timeline::<TimelineHour>(db, media_query,"STRFTIME('%Y-%m-%d %H', media.created_at)")
            .await
    }
}
