use std::path::PathBuf;

use axum::{
    Router,
    extract::Path,
    http::{StatusCode, header},
    response::{Html, IntoResponse, Json},
    routing::get,
};
use serde_json::{json, Value};
use tower_http::cors::CorsLayer;

const INDEX_HTML: &str = include_str!("../static/index.html");

pub async fn serve(port: u16) {
    let app = Router::new()
        .route("/", get(index))
        .route("/api/health", get(health))
        .route("/api/files", get(list_files))
        .route("/api/analysis/{filename}", get(get_analysis))
        .layer(CorsLayer::permissive());

    let addr = format!("127.0.0.1:{}", port);
    println!("http://{}", addr);
    eprintln!("Sherlock web server listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Failed to bind port");

    axum::serve(listener, app)
        .await
        .expect("Server error");
}

async fn index() -> Html<&'static str> {
    Html(INDEX_HTML)
}

async fn health() -> Json<Value> {
    Json(json!({ "ok": true }))
}

async fn list_files() -> Json<Vec<String>> {
    let mut files = Vec::new();
    let out_dir = PathBuf::from("out");
    if let Ok(entries) = std::fs::read_dir(&out_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.ends_with(".json") {
                files.push(name);
            }
        }
    }
    files.sort();
    Json(files)
}

async fn get_analysis(Path(filename): Path<String>) -> impl IntoResponse {
    // Sanitize: only allow alphanumeric, dots, underscores, hyphens
    if !filename.chars().all(|c| c.is_alphanumeric() || c == '.' || c == '_' || c == '-') {
        return (StatusCode::BAD_REQUEST, [(header::CONTENT_TYPE, "application/json")], "{\"error\":\"invalid filename\"}".to_string());
    }

    let path = PathBuf::from("out").join(&filename);
    match std::fs::read_to_string(&path) {
        Ok(content) => (StatusCode::OK, [(header::CONTENT_TYPE, "application/json")], content),
        Err(_) => (StatusCode::NOT_FOUND, [(header::CONTENT_TYPE, "application/json")], "{\"error\":\"file not found\"}".to_string()),
    }
}
