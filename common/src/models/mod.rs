use std::time::{SystemTime, UNIX_EPOCH};
use sqlx::types::chrono::{DateTime, NaiveDateTime};

pub mod media;
pub mod album;
pub mod media_view;
pub mod kv;

pub mod date {
    use serde::{self, Deserialize, Serializer};
    use sqlx::types::chrono::{DateTime, NaiveDateTime, Utc};


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

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<DateTime<Utc>>, D::Error>
        where
            D: serde::Deserializer<'de>,
    {
        println!("Deserializing date");
        // Deserialize the option
        let opt = Option::<i64>::deserialize(deserializer);
        
        if opt.is_err() {
            println!("Error: {:?}", opt);
            return Err(serde::de::Error::custom("Invalid timestamp"));
        }
        let opt = opt.unwrap();

        // Convert the timestamp in milliseconds to DateTime<Utc>
        match opt {
            Some(milli_timestamp) => {
                DateTime::from_timestamp_millis(milli_timestamp)
                    .ok_or_else(|| serde::de::Error::custom("Invalid timestamp"))
                    .map(Some)
            }
            None => Ok(None),
        }
    }
}

pub fn system_time_to_naive_datetime(sys_time: SystemTime) -> NaiveDateTime {
    let duration_since_epoch = sys_time.duration_since(UNIX_EPOCH)
        .expect("SystemTime before UNIX EPOCH!");
    let secs = duration_since_epoch.as_secs();
    let nanos = duration_since_epoch.subsec_nanos();
    DateTime::from_timestamp(secs as i64, nanos).unwrap().naive_utc()
}