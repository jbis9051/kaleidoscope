use crate::media_query::macros::{format_value, parse_filter};
use sqlx::{QueryBuilder, Sqlite};
use sqlx::types::chrono::{DateTime, NaiveDateTime, Utc};
use crate::models::media::Media;
use chrono::serde::ts_milliseconds_option;
use crate::{dsl_types, query_dsl};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde::de::{self, Visitor};
use std::fmt;
use chrono::{NaiveDate, NaiveTime, TimeZone};
use crate::media_query::macros::DSLType;

dsl_types! {
        bool(DSLBool, bool) {
            Equal = "=",
            |x| {
                Ok(x.parse().map_err(|_| format!("invalid bool format: {}", x))?)
            }
        };
        integer(DSLInteger, i32) {
            GreaterEqual = ">=",
            LessEqual = "<=",
            Greater = ">",
            Less = "<",
            Equal = "=",
            |x| {
                Ok(x.parse().map_err(|_| format!("invalid number format: {}", x))?)
            }
        };
        float(DSLFloat, f64) {
            GreaterEqual = ">=",
            LessEqual = "<=",
            Greater = ">",
            Less = "<",
            Equal = "=",
            |x| {
                Ok(x.parse().map_err(|_| format!("invalid float format: {}", x))?)
            }
        };
        string(DSLString, String) {
            NotEqual = "!=",
            Equal = "=",
            Like = "%",
            NotLike = "!%",
            |x| {
                Ok(x.to_string())
            }
        };
        date(DSLDate, NaiveDate) {
            BeforeEqual = "<=",
            AfterEqual = ">=",
            Equal = "=",
            Before = "<",
            After = ">",
            |x| {
                // 2020-01-01
                Ok(NaiveDate::parse_from_str(x, "%Y-%m-%d").map_err(|_| "invalid date format".to_string())?)
            }
        };
}

impl DSLBool {
    pub fn to_sql_string(&self) -> &'static str {
        match self {
            DSLBool::Equal => "=",
        }
    }
}

impl DSLInteger {
    pub fn to_sql_string(&self) -> &'static str {
        match self {
            DSLInteger::GreaterEqual => ">=",
            DSLInteger::LessEqual => "<=",
            DSLInteger::Greater => ">",
            DSLInteger::Less => "<",
            DSLInteger::Equal => "=",
        }
    }
}

impl DSLFloat {
    pub fn to_sql_string(&self) -> &'static str {
        match self {
            Self::GreaterEqual => ">=",
            Self::LessEqual => "<=",
            Self::Greater => ">",
            Self::Less => "<",
            Self::Equal => "=",
        }
    }
}

impl DSLString {
    pub fn to_sql_string(&self) -> &'static str {
        match self {
            DSLString::NotEqual => "!=",
            DSLString::Equal => "=",
            DSLString::Like => "LIKE",
            DSLString::NotLike => "NOT LIKE",
        }
    }
}

impl DSLDate {
    pub fn to_sql_string(&self) -> &'static str {
        match self {
            DSLDate::Equal => "=",
            DSLDate::Before => "<",
            DSLDate::After => ">",
            DSLDate::BeforeEqual => "<=",
            DSLDate::AfterEqual => ">=",
        }
    }
}

query_dsl! {
    MediaQuery(MediaQueryType){
        order_by(string, OrderBy),
        asc(bool, Asc),
        limit(integer, Limit),
        page(integer, Page),
        path(string, Path),
        created_at(date, CreatedAt),
        is_screenshot(bool, IsScreenshot),
        media_type(string, MediaType),
        has_gps(bool, HasGps),
        import_id(integer, ImportId),
        longitude(float, Longitude),
        latitude(float, Latitude),
    }
}
impl MediaQuery {
    pub fn new() -> Self {
        Self {
            filters: vec![],
        }
    }

    pub fn to_count_query(&self) -> Self {
        Self {
            filters: self.filters.iter().filter(|f|{
                match f {
                    MediaQueryType::OrderBy(..) => false,
                    MediaQueryType::Asc(..) => false,
                    MediaQueryType::Limit(..) => false,
                    MediaQueryType::Page(..) => false,
                    _ => true,
                }
            }).cloned().collect(),
        }
    }

    pub fn validate(&self) -> Result<(), MediaQueryError> {
        let mut seen = [false; 4];
        let mut final_filter = None;

        for filter in self.filters.iter() {
            match filter {
                MediaQueryType::OrderBy(..) | MediaQueryType::Asc(..) | MediaQueryType::Limit(..) | MediaQueryType::Page(..) => {
                    if let None = final_filter {
                        final_filter = Some(filter);
                    }

                    let index = match filter {
                        MediaQueryType::OrderBy(..) => 0,
                        MediaQueryType::Asc(..) => 1,
                        MediaQueryType::Limit(..) => 2,
                        MediaQueryType::Page(..) => 3,
                        _ => unreachable!(),
                    };

                    if seen[index] { // duplicate OrderBy, Asc, Limit, Page
                        return Err(MediaQueryError::DuplicateFilter(filter.clone()));
                    }
                    seen[index] = true;

                    // ensure page is last
                    if seen[3] && index != 3 {
                        let page = self.filters.iter().find_map(|f| {
                            if let MediaQueryType::Page(_, _) = f {
                                Some(f)
                            } else {
                                None
                            }
                        }).unwrap();
                        return Err(MediaQueryError::InvalidFilterOrder(filter.clone(), page.clone()));
                    }

                    if let MediaQueryType::OrderBy(_, column) = filter { // OrderBy column checking
                        Media::safe_column(column).map_err(|e| MediaQueryError::UnknownColumn(column.to_string()))?;
                    }

                    match filter { // all filters must have the = operator
                        MediaQueryType::OrderBy(op, _) => {
                            if op != &DSLString::Equal {
                                return Err(MediaQueryError::InvalidOperator(filter.clone()));
                            }
                        },
                        MediaQueryType::Asc(op, _) => {
                            if op != &DSLBool::Equal {
                                return Err(MediaQueryError::InvalidOperator(filter.clone()));
                            }
                        },
                        MediaQueryType::Limit(op, _) | MediaQueryType::Page(op, _) => {
                            if op != &DSLInteger::Equal {
                                return Err(MediaQueryError::InvalidOperator(filter.clone()));
                            }
                        }
                        _ => unreachable!(),
                    }

                },
                _=> {
                    if let Some(final_filter) = final_filter {
                        return Err(MediaQueryError::InvalidFilterOrder(filter.clone(), final_filter.clone()));
                    }
                }
            }
        }

        if seen[3] && !seen[2] { // page without limit
            return Err(MediaQueryError::InvalidPage);
        }

        Ok(())

    }
    
    pub fn sqlize(&self, query: &mut QueryBuilder<Sqlite>) -> Result<(), MediaQueryError>{
        self.validate()?;

        for filter in &self.filters {
            match filter {
                MediaQueryType::Path(op, path) => {
                    query
                        .push(" AND media.path ")
                        .push(op.to_sql_string())
                        .push_bind(path.clone());
                }
                MediaQueryType::CreatedAt(op, date) => {
                    query
                        .push(" AND media.created_at ")
                        .push(op.to_sql_string())
                        .push_bind(date.clone());
                }
                MediaQueryType::IsScreenshot(op, screenshot) => {
                    query
                        .push(" AND media.is_screenshot ")
                        .push(op.to_sql_string())
                        .push_bind(screenshot.clone());
                }
                MediaQueryType::MediaType(op, media_type) => {
                    query
                        .push(" AND media.media_type ")
                        .push(op.to_sql_string())
                        .push_bind(media_type.clone());
                }
                MediaQueryType::HasGps(_, gps) => {
                    query
                        .push(" AND (media.latitude IS ")
                        .push(if *gps { "NOT " } else { "" })
                        .push("NULL AND media.longitude IS ")
                        .push(if *gps { "NOT " } else { "" })
                        .push("NULL)");
                }
                MediaQueryType::ImportId(op, import_id) => {
                    query
                        .push(" AND import_id ")
                        .push(op.to_sql_string())
                        .push_bind(import_id.clone());
                }
                MediaQueryType::Longitude(op, longitude) => {
                    query
                        .push(" AND media.longitude ")
                        .push(op.to_sql_string())
                        .push_bind(longitude.clone());
                }
                MediaQueryType::Latitude(op, latitude) => {
                    query
                        .push(" AND media.latitude ")
                        .push(op.to_sql_string())
                        .push_bind(latitude.clone());
                }

                MediaQueryType::OrderBy(_, col) => {
                    Media::safe_column(col).expect("unknown column for order by, this should have been caught in validation");
                    query
                        .push(" ORDER BY ")
                        .push(format!("media.{}", col));
                }
                MediaQueryType::Asc(_, asc) => {
                    query
                        .push(if *asc { " ASC" } else { " DESC" });
                }
                MediaQueryType::Limit(_, limit) => {
                    query
                        .push(" LIMIT ")
                        .push_bind(*limit);
                }
                MediaQueryType::Page(_, page) => {
                    let limit = self.filters.iter().find_map(|f| {
                        if let MediaQueryType::Limit(_, limit) = f {
                            Some(limit)
                        } else {
                            None
                        }
                    }).expect("cannot page without limit, this should have been caught in validation");

                    query
                        .push(" OFFSET ")
                        .push_bind(page * limit);
                }
            }
        }
        
        Ok(())
    }
}


#[derive(thiserror::Error, Debug)]
pub enum MediaQueryError {
    #[error("unknown column for order by: {0}")]
    UnknownColumn(String),
    #[error("duplicate filter for column: {0:?}")]
    DuplicateFilter(MediaQueryType),
    #[error("order of filters is invalid: {0:?} came after {1:?}")]
    InvalidFilterOrder(MediaQueryType, MediaQueryType),
    #[error("invalid operator {0:?}")]
    InvalidOperator(MediaQueryType),
    #[error("cannot page without limit")]
    InvalidPage,
}