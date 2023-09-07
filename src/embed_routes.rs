use axum::{
    body::{boxed, Full},
    http::Uri,
    response::{IntoResponse, Response},
    Router,
};
use reqwest::{header, StatusCode};
use rust_embed::RustEmbed;

pub fn routes() -> Router<crate::AppState> {
    Router::new().fallback(static_handler)
}

async fn static_handler(uri: Uri) -> impl IntoResponse {
    let uri = uri.path().trim_start_matches('/').to_string();
    let uri_html = uri.clone() + ".html";
    let uri_index = uri.clone() + "index.html";
    if Asset::get(uri.as_str()).is_some() {
        StaticFile(uri.to_string()).into_response()
    } else if Asset::get(uri_html.as_str()).is_some() {
        StaticFile(uri_html.to_string()).into_response()
    } else if Asset::get(uri_index.as_str()).is_some() {
        StaticFile(uri_index.to_string()).into_response()
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}

#[derive(RustEmbed)]
#[folder = "web/"]
struct Asset;

struct StaticFile<T>(pub T);

impl<T> IntoResponse for StaticFile<T>
where
    T: Into<String>,
{
    fn into_response(self) -> Response {
        let path = self.0.into();

        match Asset::get(path.as_str()) {
            Some(content) => {
                let body = boxed(Full::from(content.data));
                let mime = mime_guess::from_path(path).first_or_octet_stream();
                Response::builder()
                    .header(header::CONTENT_TYPE, mime.as_ref())
                    .body(body)
                    .unwrap()
            }
            None => Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(boxed(Full::from("404")))
                .unwrap(),
        }
    }
}
