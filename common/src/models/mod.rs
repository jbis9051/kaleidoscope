use std::time::{SystemTime, UNIX_EPOCH};
use sqlx::types::chrono::{DateTime, NaiveDateTime};

pub mod media;
pub mod album;

pub mod date {
    use serde::{self, Serializer};
    use sqlx::types::chrono::{NaiveDateTime, };


    // The signature of a serialize_with function must follow the pattern:
    //
    //    fn serialize<S>(&T, S) -> Result<S::Ok, S::Error>
    //    where
    //        S: Serializer
    //
    // although it may also be generic over the input types T.
    pub fn serialize<S>(
        date: &NaiveDateTime,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
    {
        let unix_timestamp = date.and_utc().timestamp();
        serializer.serialize_i64(unix_timestamp)
    }
}

pub fn system_time_to_naive_datetime(sys_time: SystemTime) -> NaiveDateTime {
    let duration_since_epoch = sys_time.duration_since(UNIX_EPOCH)
        .expect("SystemTime before UNIX EPOCH!");
    let secs = duration_since_epoch.as_secs();
    let nanos = duration_since_epoch.subsec_nanos();
    DateTime::from_timestamp(secs as i64, nanos).unwrap().naive_utc()
}