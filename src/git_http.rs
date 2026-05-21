use axum::{
    body::Body,
    response::{IntoResponse, Response},
    http::{StatusCode, header},
};
use http_body_util::BodyExt;
use std::process::Stdio;
use std::io::Read;
use tokio::process::Command;
use tokio_util::io::ReaderStream;
use flate2::read::GzDecoder;
use crate::scanner::RepoScanner;
use std::sync::Arc;

#[derive(Clone)]
pub struct GitHttpState {
    pub scanner: Arc<RepoScanner>,
}

pub async fn handle_info_refs(
    repo_path: &str,
    state: GitHttpState,
    service: &str,
) -> Response {
    let repo_path = repo_path.trim_start_matches('/');
    let full_path = state.scanner.repos_path().join(repo_path);

    if service != "git-upload-pack" && service != "git-receive-pack" {
        return (StatusCode::BAD_REQUEST, "Invalid service").into_response();
    }

    // 如果是 git-receive-pack (push) 且仓库不存在，则自动创建
    if !full_path.exists() {
        if service == "git-receive-pack" {
            tracing::info!("Creating repository on push (info/refs): {}", repo_path);

            // 创建父目录
            if let Some(parent) = full_path.parent() {
                if let Err(e) = std::fs::create_dir_all(parent) {
                    tracing::error!("Failed to create parent directories: {}", e);
                    return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to create repository directories").into_response();
                }
            }

            // 初始化裸仓库
            let _init_output = match Command::new("git")
                .arg("init")
                .arg("--bare")
                .arg(&full_path)
                .output()
                .await
            {
                Ok(output) if output.status.success() => output,
                Ok(output) => {
                    tracing::error!("Git init failed: {}", String::from_utf8_lossy(&output.stderr));
                    return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to initialize repository").into_response();
                }
                Err(e) => {
                    tracing::error!("Failed to run git init: {}", e);
                    return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to initialize repository").into_response();
                }
            };

            tracing::info!("Repository created successfully: {}", repo_path);
        } else {
            return (StatusCode::NOT_FOUND, "Repository not found").into_response();
        }
    }

    let git_command = service.strip_prefix("git-").unwrap_or(service);

    let output = match Command::new("git")
        .arg(git_command)
        .arg("--stateless-rpc")
        .arg("--advertise-refs")
        .arg(&full_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
    {
        Ok(output) if output.status.success() => output,
        _ => return (StatusCode::INTERNAL_SERVER_ERROR, "Git command failed").into_response(),
    };

    // Format response according to Git HTTP protocol
    let service_line = format!("# service={}\n", service);
    let mut response_body = Vec::new();

    // Packet line format: 4-byte hex length + content
    let pkt_line = format!("{:04x}{}", service_line.len() + 4, service_line);
    response_body.extend_from_slice(pkt_line.as_bytes());
    response_body.extend_from_slice(b"0000"); // flush packet
    response_body.extend_from_slice(&output.stdout);

    let content_type = format!("application/x-{}-advertisement", service);

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .header(header::CACHE_CONTROL, "no-cache")
        .body(Body::from(response_body))
        .unwrap()
}

pub async fn handle_upload_pack(
    repo_path: &str,
    state: GitHttpState,
    body: Body,
) -> Response {
    handle_git_rpc(repo_path, state, body, "upload-pack").await
}

pub async fn handle_receive_pack(
    repo_path: &str,
    state: GitHttpState,
    body: Body,
) -> Response {
    handle_git_rpc(repo_path, state, body, "receive-pack").await
}

async fn handle_git_rpc(
    repo_path: &str,
    state: GitHttpState,
    body: Body,
    service: &str,
) -> Response {
    let repo_path = repo_path.trim_start_matches('/');
    let full_path = state.scanner.repos_path().join(repo_path);

    // 如果是 receive-pack (push) 且仓库不存在，则自动创建
    if !full_path.exists() {
        if service == "receive-pack" {
            tracing::info!("Creating repository on push: {}", repo_path);

            // 创建父目录
            if let Some(parent) = full_path.parent() {
                if let Err(e) = std::fs::create_dir_all(parent) {
                    tracing::error!("Failed to create parent directories: {}", e);
                    return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to create repository directories").into_response();
                }
            }

            // 初始化裸仓库
            let _init_output = match Command::new("git")
                .arg("init")
                .arg("--bare")
                .arg(&full_path)
                .output()
                .await
            {
                Ok(output) if output.status.success() => output,
                Ok(output) => {
                    tracing::error!("Git init failed: {}", String::from_utf8_lossy(&output.stderr));
                    return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to initialize repository").into_response();
                }
                Err(e) => {
                    tracing::error!("Failed to run git init: {}", e);
                    return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to initialize repository").into_response();
                }
            };

            tracing::info!("Repository created successfully: {}", repo_path);
        } else {
            return (StatusCode::NOT_FOUND, "Repository not found").into_response();
        }
    }

    let body_bytes = match body.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(_) => return (StatusCode::BAD_REQUEST, "Failed to read request body").into_response(),
    };

    tracing::debug!("Git {} for repo: {}, body size: {} bytes", service, repo_path, body_bytes.len());

    // Check if body is gzip compressed (starts with 0x1f 0x8b)
    let decompressed_bytes = if body_bytes.len() >= 2 && body_bytes[0] == 0x1f && body_bytes[1] == 0x8b {
        tracing::debug!("Body is gzip compressed, decompressing...");
        let mut decoder = GzDecoder::new(&body_bytes[..]);
        let mut decompressed = Vec::new();
        match decoder.read_to_end(&mut decompressed) {
            Ok(size) => {
                tracing::debug!("Decompressed {} bytes to {} bytes", body_bytes.len(), size);
                decompressed
            }
            Err(e) => {
                tracing::error!("Failed to decompress gzip body: {}", e);
                return (StatusCode::BAD_REQUEST, "Failed to decompress request body").into_response();
            }
        }
    } else {
        body_bytes.to_vec()
    };

    let mut child = match Command::new("git")
        .arg(service)
        .arg("--stateless-rpc")
        .arg(&full_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to spawn git").into_response(),
    };

    // Write request body to stdin
    if let Some(mut stdin) = child.stdin.take() {
        use tokio::io::AsyncWriteExt;
        if let Err(_) = stdin.write_all(&decompressed_bytes).await {
            return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to write to git").into_response();
        }
        drop(stdin);
    }

    // Stream the response
    if let Some(stdout) = child.stdout.take() {
        let stream = ReaderStream::new(stdout);
        let body = Body::from_stream(stream);

        let content_type = format!("application/x-git-{}-result", service);

        Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, content_type)
            .header(header::CACHE_CONTROL, "no-cache")
            .body(body)
            .unwrap()
    } else {
        (StatusCode::INTERNAL_SERVER_ERROR, "Failed to get git output").into_response()
    }
}
