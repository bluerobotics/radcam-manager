use axum::{
    Router,
    extract::Path,
    http::{HeaderMap, HeaderValue, StatusCode, header},
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

    let entry_document = filename == "index.html";
    let mut response = embedded_asset_response(&filename, mime_type.as_ref(), &headers);

    if entry_document {
        response
            .headers_mut()
            .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"));
    }

    response
}

fn embedded_asset_response(filename: &str, mime_type: &str, headers: &HeaderMap) -> Response {
    let Some(file) = HTML_DIST.get_file(format!("{filename}.gz")) else {
        return HTML_DIST.get_file(filename).map_or_else(
            || handle_404().into_response(),
            |file| ([(header::CONTENT_TYPE, mime_type)], file.contents()).into_response(),
        );
    };

    if accepts_gzip(headers) {
        return (
            [
                (header::CONTENT_TYPE, mime_type),
                (header::CONTENT_ENCODING, "gzip"),
                (header::VARY, "accept-encoding"),
            ],
            file.contents(),
        )
            .into_response();
    }

    // Stored gzipped-only, so the rare client without gzip support is served an expansion.
    match inflate(file.contents()) {
        Ok(contents) => ([(header::CONTENT_TYPE, mime_type)], contents).into_response(),
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
    use axum::{
        body::Body,
        http::{Request, header},
    };
    use tower::ServiceExt;

    use super::*;

    fn index_html_embedded() -> bool {
        HTML_DIST.get_file("index.html").is_some() || HTML_DIST.get_file("index.html.gz").is_some()
    }

    fn hashed_embedded_asset_path() -> Option<String> {
        for file in HTML_DIST.files() {
            let path = file.path().to_string_lossy();
            if path == "index.html" {
                continue;
            }
            if path.ends_with(".js") || path.ends_with(".css") {
                return Some(path.into_owned());
            }
        }
        None
    }

    #[tokio::test]
    async fn entry_document_cache_control() {
        if !index_html_embedded() {
            return;
        }

        let entry = router(1)
            .oneshot(
                Request::builder()
                    .uri("/")
                    .header(header::ACCEPT_ENCODING, "gzip")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("serve index");
        assert_eq!(
            entry
                .headers()
                .get(header::CACHE_CONTROL)
                .map(|value| value.as_bytes()),
            Some("no-cache".as_bytes())
        );

        let Some(asset_path) = hashed_embedded_asset_path() else {
            return;
        };

        let asset = router(1)
            .oneshot(
                Request::builder()
                    .uri(format!("/{asset_path}"))
                    .header(header::ACCEPT_ENCODING, "gzip")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("serve asset");
        assert!(asset.headers().get(header::CACHE_CONTROL).is_none());
    }

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
