use std::path::Path;
use serde::{Deserialize, Serialize};
use crate::match_format;
use crate::media_processors::format::{Format, MediaMetadata};

#[derive(Debug, Copy, Clone, Serialize, sqlx::Type, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
#[sqlx(type_name = "format_type", rename_all = "kebab-case")]
pub enum FormatType {
    Standard,
    Heif,
    Video,
    Raw,
    NonThumb,
    Unknown
}
impl FormatType {
    pub const fn all() -> &'static [FormatType] {
        &[
            FormatType::Standard,
            FormatType::Heif,
            FormatType::Video,
            FormatType::Raw
        ]
    }

    pub const fn thumbnailable() -> &'static [FormatType] {
        &[
            FormatType::Standard,
            FormatType::Heif,
            FormatType::Video,
            FormatType::Raw, 
        ]
    }
}

const fn thumbnailable_check() -> bool {
    let thumbnailable = FormatType::thumbnailable();
    thumbnailabele_rc(thumbnailable)
}

const fn thumbnailabele_rc(thumbs: &[FormatType]) -> bool { // replace this once const fn array iteration is stable
    match thumbs {
        [] => true,
        [one, rest @ ..] => {
            if !match_format!(thumbnailable  one, |ActualFormat| { true }, { false }) {
                return false;
            }
            thumbnailabele_rc(rest)
        }
    }
}

const THUMBNAIL_CHECK: () = assert!(thumbnailable_check(), "the thumbnail compiler check failed, FormatType::thumbnailable is not consistent with 'match_format!'");
