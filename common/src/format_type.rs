use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, Serialize, sqlx::Type, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
#[sqlx(type_name = "format_type", rename_all = "kebab-case")]
pub enum FormatType {
    Standard,
    Heif,
    Video,
    Raw,
    Unknown
}