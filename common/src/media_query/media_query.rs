use std::collections::HashSet;
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
use toml::Table;
use uuid::Uuid;
use crate::media_query::macros::DSLType;

// NOTE! make sure longer ops come first

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
        uuid(DSLUuid, Uuid) {
            Equal = "=",
            NotEqual = "!=",
            |x| {
                Ok(x.parse().map_err(|_| format!("invalid uuid format: {}", x))?)
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

impl DSLUuid {
    pub fn to_sql_string(&self) -> &'static str {
        match self {
            DSLUuid::Equal => "=",
            DSLUuid::NotEqual => "!=",
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
        order_by(string, OrderBy, []),
        asc(bool, Asc, []),
        limit(integer, Limit, []),
        page(integer, Page, []),
        path(string, Path, []),
        created_at(date, CreatedAt, []),
        is_screenshot(bool, IsScreenshot,[]),
        media_type(string, MediaType, []),
        has_gps(bool, HasGps, []),
        import_id(integer, ImportId, []),
        longitude(float, Longitude, []),
        latitude(float, Latitude, []),
        transcript(string, Transcript, [MediaExtra,]),
        vision_ocr(string, VisionOcr, [MediaExtra,]),
        full_search(string, FullSearch, [MediaExtra, CustomMetadata,]),
        album_uuid(uuid, AlbumUuid, [AlbumAll,]),
        tag(string, Tag, [MediaTag,]),
        has_thumbnail(bool, HasThumbnail, []),
    }
}

const FULL_SEARCH_QUERIES: [&'static str; 3] = ["media.name", "media_extra.whisper_transcript", "media_extra.vision_ocr_result"];

#[derive(PartialEq, Debug, Hash, Eq)]
pub enum JoinableTable {
    MediaExtra,
    AlbumAll,
    MediaTag,
    CustomMetadata,
}

impl JoinableTable {
    pub fn join_statement(&self) -> &'static str {
        match self {
            JoinableTable::MediaExtra => " LEFT JOIN media_extra ON media.id = media_extra.media_id ",
            JoinableTable::AlbumAll => " LEFT JOIN album_media ON media.id = album_media.media_id INNER JOIN album ON album_media.album_id = album.id ",
            JoinableTable::MediaTag => " LEFT JOIN media_tag ON media.id = media_tag.media_id ",
            JoinableTable::CustomMetadata => " LEFT JOIN custom_metadata ON media.id = custom_metadata.media_id ",
        }
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
            filters: self.filters.iter().filter(|f| {
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
                        }
                        MediaQueryType::Asc(op, _) => {
                            if op != &DSLBool::Equal {
                                return Err(MediaQueryError::InvalidOperator(filter.clone()));
                            }
                        }
                        MediaQueryType::Limit(op, _) | MediaQueryType::Page(op, _) => {
                            if op != &DSLInteger::Equal {
                                return Err(MediaQueryError::InvalidOperator(filter.clone()));
                            }
                        }
                        _ => unreachable!(),
                    }
                }
                _ => {
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

    // joins all necessary tables for this query
    pub fn add_tables(&self, query: &mut QueryBuilder<Sqlite>) {
        let mut tables = HashSet::new();
        for filter in &self.filters {
            tables.extend(filter.tables());
        }
        for table in tables {
            query.push(table.join_statement());
        }
    }

    pub fn sqlize(&self, query: &mut QueryBuilder<Sqlite>) -> Result<(), MediaQueryError> {
        self.add_tables(query);
        self.add_queries(query)?;
        Ok(())
    }


    pub fn add_queries(&self, query: &mut QueryBuilder<Sqlite>) -> Result<(), MediaQueryError> {
        self.validate()?;

        if !query.sql().contains("WHERE") {
            query.push(" WHERE 1=1 ");
        }

        let tags: Vec<(&DSLString, &String)> = self.filters
            .iter()
            .filter_map(|f| {
                if let MediaQueryType::Tag(op, tag) = f {
                    return Some((op, tag));
                }
                None
            }).collect();
        
        if !tags.is_empty() {
            query.push(" AND (1=2");
            for (op, search) in tags {
                query.push(" OR media_tag.tag ");
                query.push(" ");
                query.push(op.to_sql_string());
                query.push_bind(search.clone());
            }
            query.push(" )");
        }

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
                        .push(" AND media.import_id ")
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
                MediaQueryType::Transcript(op, search) => {
                    query.push(" AND media_extra.whisper_transcript ")
                        .push(op.to_sql_string())
                        .push_bind(search.clone());
                }
                MediaQueryType::VisionOcr(op, search) => {
                    query.push(" AND media_extra.vision_ocr_result ")
                        .push(op.to_sql_string())
                        .push_bind(search.clone());
                }
                MediaQueryType::AlbumUuid(op, album_uuid) => {
                    query.push(" AND album.uuid ")
                        .push(op.to_sql_string())
                        .push_bind(album_uuid.clone());
                }
                MediaQueryType::HasThumbnail(op, thumbnail) => {
                    query.push(" AND media.has_thumbnail ")
                    .push(op.to_sql_string())
                    .push_bind(thumbnail.clone());
                }
                MediaQueryType::FullSearch(op, search) => {
                    query.push(" AND (1=2");
                    for term in FULL_SEARCH_QUERIES {
                        query.push(" OR ");
                        query.push(term);
                        query.push(" ");
                        query.push(op.to_sql_string());
                        query.push_bind(search.clone());
                    }
                    query.push(" OR (custom_metadata.value ");
                    query.push(op.to_sql_string());
                    query.push_bind(search.clone());
                    query.push(" AND custom_metadata.include_search = TRUE) ");
                    query.push(" ) ");
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
                MediaQueryType::Tag(_, _) => {
                    // this is handled elsewhere
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