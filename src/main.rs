mod config;
mod assets;
mod db;
mod scanner;
mod api;
mod git_http;

use axum::{
    routing::{get, post},
    Router,
    Json,
    response::{IntoResponse, Response},
    http::{StatusCode, header, Uri},
};
use config::Config;
use assets::Assets;
use db::Database;
use scanner::RepoScanner;
use api::ApiState;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Serialize, Deserialize)]
struct ApiResponse {
    message: String,
    version: String,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "gitclub=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Config::load();
    config.ensure_dirs();
    tracing::info!("Configuration loaded");
    tracing::info!("Data path: {:?}", config.data_path);
    tracing::info!("Log path: {:?}", config.log_path);
    tracing::info!("Repos path: {:?}", config.repos_path);

    let db_path = config.data_path.join("gitclub.db");
    let db = Database::new(&db_path).await.expect("Failed to initialize database");
    tracing::info!("Database initialized at {:?}", db_path);

    let scanner = Arc::new(RepoScanner::new(config.repos_path.clone()));

    let api_state = ApiState {
        db: Arc::new(db),
        scanner: Arc::clone(&scanner),
    };

    let git_state = git_http::GitHttpState {
        scanner: Arc::clone(&scanner),
    };

    let app = Router::new()
        .route("/api/health", get(health_check))
        .route("/api/info", get(get_info))
        .route("/api/tree", get(api::get_tree))
        .route("/api/groups", post(api::create_group))
        .route("/api/repos", post(api::create_repo))
        .route("/api/repo/*path", get(api::get_repo_detail))
        .route("/api/repo-file/*path", get(api::get_file_content))
        .route("/api/group/*path", get(api::get_group_detail))
        .with_state(api_state)
        .route("/:group/:repo/info/refs", get(git_http::git_info_refs))
        .route("/:group/:repo/git-upload-pack", post(git_http::git_upload_pack))
        .route("/:group/:repo/git-receive-pack", post(git_http::git_receive_pack))
        .with_state(git_state)
        .fallback(static_handler)
        .layer(TraceLayer::new_for_http());

    let addr = SocketAddr::from((
        config.server_addr.parse::<std::net::IpAddr>().unwrap_or([127, 0, 0, 1].into()),
        config.server_port,
    ));

    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(listener) => {
            tracing::info!("GitClub server listening on {}", addr);
            tracing::info!("API available at http://{}/api", addr);
            tracing::info!("Web UI available at http://{}", addr);
            listener
        }
        Err(e) => {
            tracing::error!("Failed to bind to {}: {}", addr, e);
            tracing::error!("Port may already be in use. Please check the configuration.");
            std::process::exit(1);
        }
    };

    axum::serve(listener, app).await.unwrap();
}

async fn static_handler(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };

    match Assets::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            (
                [(header::CONTENT_TYPE, mime.as_ref())],
                content.data,
            ).into_response()
        }
        None => {
            // SPA fallback: 如果文件不存在，返回 index.html
            if let Some(index) = Assets::get("index.html") {
                (
                    [(header::CONTENT_TYPE, "text/html")],
                    index.data,
                ).into_response()
            } else {
                (StatusCode::NOT_FOUND, "Not Found").into_response()
            }
        }
    }
}

async fn health_check() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}

async fn get_info() -> impl IntoResponse {
    let response = ApiResponse {
        message: "GitClub - Git Hosting Server".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    };
    Json(response)
}
