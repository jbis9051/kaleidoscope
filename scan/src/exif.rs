use exif::{Exif, In, Tag, Value};
use iso6709parse::ISO6709Coord;
use nom_exif::{EntryValue, LatLng, TrackInfo, TrackInfoTag};

#[derive(Debug)]
pub struct ExifMetadata {
    pub longitude: Option<f64>,
    pub latitude: Option<f64>,
    pub is_screenshot: bool,
}

pub fn extract_exif(exif: &Exif) -> Result<ExifMetadata, exif::Error> {
    let mut metadata = ExifMetadata {
        longitude: None,
        latitude: None,
        is_screenshot: false,
    };

    metadata.is_screenshot = exif.get_field(Tag::UserComment, In::PRIMARY).and_then(|field| parse_comment(&field.value)).map(|comment| comment.contains("Screenshot")).unwrap_or(false);

    if let (Some(direction), Some(values)) = (exif.get_field(Tag::GPSLatitudeRef, In::PRIMARY), exif.get_field(Tag::GPSLatitude, In::PRIMARY)) {
        metadata.latitude = parse_gps(&direction.value, &values.value);
    }

    if let (Some(direction), Some(values)) = (exif.get_field(Tag::GPSLongitudeRef, In::PRIMARY), exif.get_field(Tag::GPSLongitude, In::PRIMARY)) {
        metadata.longitude = parse_gps(&direction.value, &values.value);
    }

    Ok(metadata)
}

fn parse_comment(comment: &Value) -> Option<String> {
    if let Value::Undefined(ref bytes, _) = comment {
        // format is |<ASCII|UNICODE>|<NULL><NULL><NULL>|<data>|
        // first we need to find the first NULL byte
        let null = bytes.iter().position(|&b| b == 0).unwrap();
        let encoding = String::from_utf8_lossy(&bytes[0..null]);
        if encoding == "ASCII" || encoding == "UNICODE" {
            let data = &bytes[null + 3..];
            let comment = String::from_utf8_lossy(data);
            return Some(comment.to_string())
        }
    }
    None
}

fn parse_gps(direction: &Value, values: &Value) -> Option<f64> {
    match (direction, values) {
        (Value::Ascii(direction), Value::Rational(values)) => {
            if values.len() != 3 || direction.len() != 1 || direction[0].len() != 1 {
                return None;
            }

            let degrees = values[0].to_f64();
            let minutes = values[1].to_f64();
            let seconds = values[2].to_f64();
            let value = degrees + minutes / 60.0 + seconds / 3600.0;
            let direction = &direction[0][0];

            match direction {
                b'N' | b'E' => Some(value),
                b'S' | b'W' => Some(-value),
                _ => None,
            }

        }
        _ => None,
    }
}

pub fn extract_exif_nom(track_info: &TrackInfo) -> ExifMetadata {
    let mut metadata = ExifMetadata {
        longitude: None,
        latitude: None,
        is_screenshot: false,
    };

    let gps = track_info.get(TrackInfoTag::GpsIso6709);
    if let Some(gps) = gps {
        if let EntryValue::Text(gps) = gps {
            let iso: Result<ISO6709Coord,_> = iso6709parse::parse(gps);
            if let Ok(iso) = iso {
                metadata.latitude = Some(iso.lat);
                metadata.longitude = Some(iso.lon);
            }
        }
    }

    metadata
}
