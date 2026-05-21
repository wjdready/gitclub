mod config;
mod assets;
mod db;
mod scanner;
mod api;
mod git_http;
mod auth;

use axum::{
    routing::{get, post, delete},
    Router,
    Json,
    response::{IntoResponse, Response},
    http::{StatusCode, header, Uri, Request, Method},
    extract::State,
    body::Body,
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

    // 初始化管理员账户
    let admin_exists = db.get_user_by_username(&config.admin_username).await.ok().flatten();
    if admin_exists.is_none() {
        tracing::info!("Creating admin user: {}", config.admin_username);
        let password_hash = auth::hash_password(&config.admin_password)
            .expect("Failed to hash admin password");

        let admin_id = db.create_user(&config.admin_username, &password_hash, None, true)
            .await
            .expect("Failed to create admin user");

        // 创建管理员的 repos 文件夹
        let admin_folder = config.repos_path.join(&config.admin_username);
        std::fs::create_dir_all(&admin_folder).ok();

        tracing::info!("Admin user created with ID: {}", admin_id);
    } else {
        tracing::info!("Admin user already exists");
    }

    let scanner = Arc::new(RepoScanner::new(config.repos_path.clone()));

    let api_state = ApiState {
        db: Arc::new(db),
        scanner: Arc::clone(&scanner),
    };

    let git_state = git_http::GitHttpState {
        scanner: Arc::clone(&scanner),
        db: Arc::clone(&api_state.db),
    };

    let app = Router::new()
        .route("/api/health", get(health_check))
        .route("/api/info", get(get_info))
        .route("/api/auth/login", post(api::login))
        .route("/api/auth/register", post(api::register))
        .route("/api/auth/logout", post(api::logout))
        .route("/api/auth/me", get(api::get_current_user))
        .route("/api/user/profile", axum::routing::put(api::update_profile))
        .route("/api/tree", get(api::get_tree))
        .route("/api/groups", post(api::create_group))
        .route("/api/repos", post(api::create_repo))
        .route("/api/repo/*path", get(api::get_repo_detail))
        .route("/api/repo-file/*path", get(api::get_file_content))
        .route("/api/group/*path", get(api::get_group_detail))
        .route("/api/group-members", get(api::list_group_members_handler).post(api::add_group_member_handler))
        .route("/api/group-members/remove", delete(api::remove_group_member_handler))
        .route("/api/group-members/permissions", axum::routing::put(api::update_group_member_permissions_handler))
        .route("/api/repo-members", get(api::list_repo_members_handler).post(api::add_repo_member_handler))
        .route("/api/repo-members/remove", axum::routing::delete(api::remove_repo_member_handler))
        .with_state(api_state)
        .fallback(git_or_static_handler)
        .with_state(git_state)
        .layer(TraceLayer::new_for_http());

    let addr = SocketAddr::from((
        config.server_addr.parse::<std::net::IpAddr>().unwrap_or([127, 0, 0, 1].into()),
        config.server_port,
    ));

    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(listener) => {
            let display_addr = if addr.ip().is_unspecified() {
                format!("localhost:{}", addr.port())
            } else {
                addr.to_string()
            };
            tracing::info!("GitClub server listening on {}", display_addr);
            tracing::info!("API available at http://{}/api", display_addr);
            tracing::info!("Web UI available at http://{}", display_addr);
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

async fn git_or_static_handler(
    State(git_state): State<git_http::GitHttpState>,
    req: Request<Body>,
) -> Response {
    let uri = req.uri().clone();
    let path = uri.path();
    let method = req.method().clone();
    let headers = req.headers().clone();

    // Decode URL-encoded path to support non-ASCII characters (e.g., Chinese)
    let decoded_path = urlencoding::decode(path).unwrap_or_else(|_| path.into()).to_string();

    // Check if this is a git request
    if decoded_path.ends_with("/info/refs") {
        let query = uri.query().and_then(|q| {
            q.split('&')
                .find(|p| p.starts_with("service="))
                .and_then(|p| p.strip_prefix("service="))
        });

        if let Some(service) = query {
            let repo_path = decoded_path.strip_suffix("/info/refs").unwrap_or(&decoded_path);
            return git_http::handle_info_refs(repo_path, git_state, service, headers).await;
        }
    } else if decoded_path.ends_with("/git-upload-pack") && method == Method::POST {
        let repo_path = decoded_path.strip_suffix("/git-upload-pack").unwrap_or(&decoded_path);
        return git_http::handle_upload_pack(repo_path, git_state, headers, req.into_body()).await;
    } else if decoded_path.ends_with("/git-receive-pack") && method == Method::POST {
        let repo_path = decoded_path.strip_suffix("/git-receive-pack").unwrap_or(&decoded_path);
        return git_http::handle_receive_pack(repo_path, git_state, headers, req.into_body()).await;
    }

    // Otherwise serve static files
    static_handler(uri).await
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
