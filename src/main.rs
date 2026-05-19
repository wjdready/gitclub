use axum::{
    routing::get,
    Router,
    Json,
    response::IntoResponse,
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Serialize, Deserialize)]
struct ApiResponse {
    message: String,
    version: String,
}

#[tokio::main]
async fn main() {
    // 初始化日志
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "gitclub=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // 构建应用路由
    let app = Router::new()
        // API 路由
        .route("/api/health", get(health_check))
        .route("/api/info", get(get_info))
        // 静态文件服务（Vue 构建输出）
        .nest_service("/", ServeDir::new("target/dist"))
        .layer(TraceLayer::new_for_http());

    // 服务器地址
    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));

    // 启动服务器
    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(listener) => {
            tracing::info!("GitClub server listening on {}", addr);
            tracing::info!("API available at http://{}/api", addr);
            tracing::info!("Web UI available at http://{}", addr);
            listener
        }
        Err(e) => {
            tracing::error!("Failed to bind to {}: {}", addr, e);
            tracing::error!("Port 8080 may already be in use. Please stop the other service or change the port.");
            std::process::exit(1);
        }
    };

    axum::serve(listener, app).await.unwrap();
}

// 健康检查端点
async fn health_check() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}

// 获取服务信息
async fn get_info() -> impl IntoResponse {
    let response = ApiResponse {
        message: "GitClub - Git Hosting Server".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    };
    Json(response)
}
