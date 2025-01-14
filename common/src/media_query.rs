use serde::Deserialize;
use sqlx::{QueryBuilder, Sqlite};
use sqlx::types::chrono::{DateTime, NaiveDateTime, Utc};
use crate::models::media::Media;
use chrono::serde::ts_milliseconds_option;

#[derive(Default, Debug, Deserialize, Clone)]
pub struct MediaQuery {
    pub order_by: Option<String>,
    pub asc: Option<bool>,
    pub limit: Option<i32>,
    pub page: Option<i32>,
    pub filter_path: Option<String>,
    pub filter_not_path: Option<String>,
    #[serde(default, with = "ts_milliseconds_option")]
    pub before: Option<DateTime<Utc>>,
    #[serde(default, with = "ts_milliseconds_option")]
    pub after: Option<DateTime<Utc>>
}

impl MediaQuery {
    pub fn new() -> Self {
        Self {
            order_by: None,
            asc: None,
            limit: None,
            page: None,
            filter_path: None,
            filter_not_path: None,
            before: None,
            after: None,
        }
    }

    pub fn to_count_query(&self) -> Self {
        Self {
            order_by: None,
            asc: None,
            limit: None,
            page: None,
            filter_path: self.filter_path.clone(),
            filter_not_path: self.filter_not_path.clone(),
            before: self.before.clone(),
            after: self.after.clone(),
        }
    }
    
    pub fn sqlize(&self, query: &mut QueryBuilder<Sqlite>) -> Result<(), sqlx::Error>{
        if let Some(filter_path) = &self.filter_path {
            query
                .push(" AND path LIKE ")
                .push_bind(filter_path.clone());
        }

        if let Some(filter_not_path) = &self.filter_not_path {
            query
                .push(" AND path NOT LIKE ")
                .push_bind(filter_not_path.clone());
        }
        
        if let Some(before) = &self.before {
            query
                .push(" AND created_at < ")
                .push_bind(*before);
        }
        
        if let Some(after) = &self.after {
            query
                .push(" AND created_at > ")
                .push_bind(*after);
        }
        
        if let Some(order_by) = &self.order_by {
            Media::safe_column(order_by)?;
            query
                .push(" ORDER BY ")
                .push(format!("media.{}", order_by));
        }
        
        if let Some(asc) = self.asc {
            query
                .push(if asc { " ASC" } else { " DESC" });
        }
        
        if let Some(limit) = self.limit {
            query
                .push(" LIMIT ")
                .push_bind(limit);

            if let Some(page) = self.page {
                query
                    .push(" OFFSET ")
                    .push_bind(page * self.limit.unwrap());
            }
        }
        
        Ok(())
    }
}