use axum::body::Bytes;
use axum::extract::multipart::Field;
use axum::extract::{Multipart, Request};
use axum::response::{ErrorResponse, IntoResponse, Response};
use axum::{BoxError, RequestExt};
use futures::{Stream, TryStreamExt};
use reqwest::{Method, StatusCode};
use serde::de::DeserializeOwned;
use std::io;
use std::path::{Path, PathBuf};
use tokio::fs::File;
use tokio::io::BufWriter;
use tokio_util::io::StreamReader;
use uuid::Uuid;

pub struct MultipartHelper(Multipart);
impl MultipartHelper {
    pub async fn try_from_request(request: Request) -> Result<Self, ErrorResponse> {
        if request.method() != Method::POST {
            return Err((StatusCode::METHOD_NOT_ALLOWED, "only post is permitted").into());
        }

        let multipart: Multipart = request
            .extract()
            .await
            .map_err(|_| (StatusCode::BAD_REQUEST, "missing multipart"))?;
        Ok(Self(multipart))
    }

    pub async fn next_field(&mut self) -> Result<Option<Field>, ErrorResponse> {
        Ok(self
            .0
            .next_field()
            .await
            .map_err(|e| (StatusCode::BAD_REQUEST, format!("multipart error: {:?}", e)))?)
    }

    pub async fn field(&mut self, field_name: &str) -> Result<Field, ErrorResponse> {
        let field = self.next_field().await?.ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                format!("multipart missing field '{}'", field_name),
            )
        })?;

        if field.name() != Some(field_name) {
            return Err((
                StatusCode::BAD_REQUEST,
                format!(
                    "multipart expected field '{}' but found '{:?}'",
                    field_name,
                    field.name()
                ),
            )
                .into());
        }

        Ok(field)
    }

    pub async fn text(&mut self, field_name: &str) -> Result<String, ErrorResponse> {
        let field = self.field(field_name).await?;
        Ok(field
            .text()
            .await
            .map_err(|e| (StatusCode::BAD_REQUEST, format!("multipart error: {:?}", e)))?)
    }

    pub async fn json<T: DeserializeOwned>(
        &mut self,
        field_name: &str,
    ) -> Result<T, ErrorResponse> {
        let text = self.text(field_name).await?;
        serde_json::from_str(&text).map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                format!("json parse error for field '{}': {:?}", field_name, e),
            )
                .into()
        })
    }

    pub async fn file(
        &mut self,
        field_name: &str,
        file_extension: &str,
    ) -> Result<(PathBuf, String), ErrorResponse> {
        let field = self.field(field_name).await?;

        let file_name = field
            .file_name()
            .ok_or((
                StatusCode::BAD_REQUEST,
                format!("missing file name for field '{}'", field_name),
            ))?
            .to_string();
        let tmp_name = format!("{}{}", Uuid::new_v4(), file_extension);
        let to_path = std::env::temp_dir().join(tmp_name);

        stream_to_file(&to_path, field).await?;
        Ok((to_path, file_name))
    }
}

// https://github.com/tokio-rs/axum/blob/main/examples/stream-to-file/src/main.rs

async fn stream_to_file<P, S, E>(path: P, stream: S) -> Result<(), (StatusCode, String)>
where
    P: AsRef<Path>,
    S: Stream<Item = Result<Bytes, E>>,
    E: Into<BoxError>,
{
    async {
        // Convert the stream into an `AsyncRead`.
        let body_with_io_error = stream.map_err(io::Error::other);
        let body_reader = StreamReader::new(body_with_io_error);
        futures::pin_mut!(body_reader);

        // Create the file. `File` implements `AsyncWrite`.
        let mut file = BufWriter::new(File::create(path).await?);

        // Copy the body into the file.
        tokio::io::copy(&mut body_reader, &mut file).await?;

        Ok::<_, io::Error>(())
    }
    .await
    .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))
}


