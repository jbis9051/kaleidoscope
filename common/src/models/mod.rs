use sqlx::types::chrono::{DateTime, NaiveDateTime};
use std::time::{SystemTime, UNIX_EPOCH};

pub mod album;
pub mod kv;
pub mod media;
pub mod media_view;
pub mod sqlize;
pub mod timeline;
pub mod queue;

pub mod date {
    use serde::{self, Deserialize, Serializer};
    use sqlx::types::chrono::{DateTime, NaiveDateTime};

    // The signature of a serialize_with function must follow the pattern:
    //
    //    fn serialize<S>(&T, S) -> Result<S::Ok, S::Error>
    //    where
    //        S: Serializer
    //
    // although it may also be generic over the input types T.
    pub fn serialize<S>(date: &NaiveDateTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let unix_timestamp = date.and_utc().timestamp();
        serializer.serialize_i64(unix_timestamp)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<NaiveDateTime, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // Deserialize the option
        let milli_timestamp = i64::deserialize(deserializer)?;

        // Convert the timestamp in milliseconds to DateTime<Utc>
        Ok(DateTime::from_timestamp_millis(milli_timestamp)
            .ok_or_else(|| serde::de::Error::custom("Invalid timestamp"))?
            .naive_utc())
    }
}

pub fn system_time_to_naive_datetime(sys_time: SystemTime) -> NaiveDateTime {
    let duration_since_epoch = sys_time
        .duration_since(UNIX_EPOCH)
        .expect("SystemTime before UNIX EPOCH!");
    let secs = duration_since_epoch.as_secs();
    let nanos = duration_since_epoch.subsec_nanos();
    DateTime::from_timestamp(secs as i64, nanos)
        .unwrap()
        .naive_utc()
}

#[derive(thiserror::Error, Debug)]
pub enum MediaError {
    #[error("sqlx error: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("media_query error: {0}")]
    MediaQuery(#[from] crate::media_query::MediaQueryError),
}
