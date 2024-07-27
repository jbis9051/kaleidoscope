use serde::Deserialize;
use sqlx::{QueryBuilder, Sqlite};
use crate::models::media::Media;

#[derive(Default, Debug, Deserialize, Clone)]
pub struct MediaQuery {
    pub order_by: Option<String>,
    pub asc: Option<bool>,
    pub limit: Option<i32>,
    pub page: Option<i32>,
    pub filter_path: Option<String>
}

impl MediaQuery {
    pub fn new() -> Self {
        Self {
            order_by: None,
            asc: None,
            limit: None,
            page: None,
            filter_path: None
        }
    }
    
    pub fn sqlize(&self, query: &mut QueryBuilder<Sqlite>) -> Result<(), sqlx::Error>{
        if let Some(filter_path) = &self.filter_path {
            query
                .push(" AND path LIKE ")
                .push_bind(filter_path.clone());
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