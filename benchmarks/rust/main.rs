use axum::{Router, routing::get, response::{IntoResponse, Response}, http::{StatusCode, header}};
use std::net::SocketAddr;

async fn text_handler() -> Response {
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/plain")],
        "Hello, World!\n",
    ).into_response()
}

async fn json_handler() -> Response {
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/json")],
        r#"{"message":"Hello, World!"}"#,
    ).into_response()
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/text", get(text_handler))
        .route("/json", get(json_handler));

    let addr = SocketAddr::from(([0, 0, 0, 0], 3002));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
