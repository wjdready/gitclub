use axum::{
    body::Body,
    extract::{Path, State, Query},
    response::{IntoResponse, Response},
    http::{StatusCode, header},
};
use http_body_util::BodyExt;
use std::process::Stdio;
use tokio::process::Command;
use crate::scanner::RepoScanner;
use std::sync::Arc;
use serde::Deserialize;

#[derive(Clone)]
pub struct GitHttpState {
    pub scanner: Arc<RepoScanner>,
}

#[derive(Deserialize)]
pub struct InfoRefsQuery {
    service: String,
}

pub async fn git_info_refs(
    Path((group, repo)): Path<(String, String)>,
    State(state): State<GitHttpState>,
    Query(query): Query<InfoRefsQuery>,
) -> Response {
    let service = &query.service;

    if !service.starts_with("git-") {
        return (StatusCode::BAD_REQUEST, "Invalid service").into_response();
    }

    let repo_path = format!("{}/{}", group, repo);
    let full_path = state.scanner.repos_path().join(&repo_path);
    if !full_path.exists() {
        return (StatusCode::NOT_FOUND, "Repository not found").into_response();
    }

    let output = Command::new("git")
        .arg(service)
        .arg("--stateless-rpc")
        .arg("--advertise-refs")
        .arg(&full_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await;

    match output {
        Ok(output) if output.status.success() => {
            let mut body = Vec::new();
            body.extend_from_slice(format!("# service={}\n", service).as_bytes());
            body.extend_from_slice(b"0000");
            body.extend_from_slice(&output.stdout);

            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, format!("application/x-{}-advertisement", service))
                .header(header::CACHE_CONTROL, "no-cache")
                .body(Body::from(body))
                .unwrap()
        }
        _ => (StatusCode::INTERNAL_SERVER_ERROR, "Git command failed").into_response(),
    }
}

pub async fn git_upload_pack(
    Path((group, repo)): Path<(String, String)>,
    State(state): State<GitHttpState>,
    body: Body,
) -> Response {
    let repo_path = format!("{}/{}", group, repo);
    git_rpc(repo_path, state, body, "git-upload-pack").await
}

pub async fn git_receive_pack(
    Path((group, repo)): Path<(String, String)>,
    State(state): State<GitHttpState>,
    body: Body,
) -> Response {
    let repo_path = format!("{}/{}", group, repo);
    git_rpc(repo_path, state, body, "git-receive-pack").await
}

async fn git_rpc(
    repo_path: String,
    state: GitHttpState,
    body: Body,
    service: &str,
) -> Response {
    let full_path = state.scanner.repos_path().join(&repo_path);
    if !full_path.exists() {
        return (StatusCode::NOT_FOUND, "Repository not found").into_response();
    }

    let body_bytes = match body.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(_) => return (StatusCode::BAD_REQUEST, "Failed to read request body").into_response(),
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
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to spawn git process").into_response(),
    };

    if let Some(mut stdin) = child.stdin.take() {
        use tokio::io::AsyncWriteExt;
        let _ = stdin.write_all(&body_bytes).await;
    }

    let output = match child.wait_with_output().await {
        Ok(output) => output,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Git command failed").into_response(),
    };

    if output.status.success() {
        Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, format!("application/x-{}-result", service))
            .header(header::CACHE_CONTROL, "no-cache")
            .body(Body::from(output.stdout))
            .unwrap()
    } else {
        (StatusCode::INTERNAL_SERVER_ERROR, "Git command failed").into_response()
    }
}
