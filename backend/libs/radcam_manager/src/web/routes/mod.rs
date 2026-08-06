use axum::{
    Router,
    extract::Path,
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use include_dir::{Dir, include_dir};
use mime_guess::from_path;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::*;

pub mod v1;

/// Frontend staged by `build.rs`: compressible assets are stored gzipped under
/// `<name>.gz`, already-compressed ones (fonts, images) verbatim.
static HTML_DIST: Dir = include_dir!("$OUT_DIR/dist_gz"); // NOTE: frontend needs to be built first

#[instrument(level = "trace")]
pub fn router(default_api_version: u8) -> Router {
    let app = Router::new()
        .nest("/v1", v1::router())
        .route_service("/", get(root))
        .route_service("/{*path}", get(root))
        .fallback(handle_404())
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive());

    match default_api_version {
        1 => app.merge(v1::router()),
        _ => unimplemented!(),
    }
}

#[instrument(level = "trace", skip(headers))]
async fn root(filename: Option<Path<String>>, headers: HeaderMap) -> Response {
    let filename = filename
        .map(|Path(name)| {
            if name.is_empty() {
                "index.html".into()
            } else {
                name
            }
        })
        .unwrap_or_else(|| "index.html".into());

    // Determine the MIME type based on the file extension
    let mime_type = from_path(&filename).first_or_octet_stream();

    let Some(file) = HTML_DIST.get_file(format!("{filename}.gz")) else {
        return HTML_DIST.get_file(&filename).map_or_else(
            || handle_404().into_response(),
            |file| {
                (
                    [(header::CONTENT_TYPE, mime_type.as_ref())],
                    file.contents(),
                )
                    .into_response()
            },
        );
    };

    if accepts_gzip(&headers) {
        return (
            [
                (header::CONTENT_TYPE, mime_type.as_ref()),
                (header::CONTENT_ENCODING, "gzip"),
                (header::VARY, "accept-encoding"),
            ],
            file.contents(),
        )
            .into_response();
    }

    // Stored gzipped-only, so the rare client without gzip support is served an expansion.
    match inflate(file.contents()) {
        Ok(contents) => ([(header::CONTENT_TYPE, mime_type.as_ref())], contents).into_response(),
        Err(error) => {
            error!("Failed expanding embedded asset {filename:?}: {error}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "500 Internal Server Error",
            )
                .into_response()
        }
    }
}

/// Whether `Accept-Encoding` names gzip with a non-zero q-value.
fn accepts_gzip(headers: &HeaderMap) -> bool {
    let Some(value) = headers
        .get(header::ACCEPT_ENCODING)
        .and_then(|value| value.to_str().ok())
    else {
        return false;
    };

    value.split(',').any(|entry| {
        let mut parts = entry.split(';').map(str::trim);
        let coding = parts.next().unwrap_or_default();
        if !coding.eq_ignore_ascii_case("gzip") && coding != "*" {
            return false;
        }
        !parts.any(|parameter| {
            parameter
                .strip_prefix("q=")
                .and_then(|quality| quality.parse::<f32>().ok())
                .is_some_and(|quality| quality <= 0.0)
        })
    })
}

fn inflate(contents: &[u8]) -> std::io::Result<Vec<u8>> {
    use std::io::Read;

    let mut expanded = Vec::new();
    flate2::read::GzDecoder::new(contents).read_to_end(&mut expanded)?;

    Ok(expanded)
}

fn handle_404() -> (StatusCode, &'static str) {
    (StatusCode::NOT_FOUND, "404 Not Found")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accept_encoding_gzip_table() {
        let accepted = |value: &str| {
            let mut headers = HeaderMap::new();
            headers.insert(header::ACCEPT_ENCODING, value.parse().unwrap());
            accepts_gzip(&headers)
        };

        assert!(accepted("gzip"));
        assert!(accepted("gzip, deflate, br"));
        assert!(accepted("deflate, GZIP;q=1.0"));
        assert!(accepted("*"));
        assert!(!accepted("gzip;q=0"));
        assert!(!accepted("br, deflate"));
        assert!(!accepted(""));
        assert!(!accepts_gzip(&HeaderMap::new()));
    }
}
