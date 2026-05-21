use axum::{
    body::Body,
    response::{IntoResponse, Response},
    http::{StatusCode, header, HeaderMap},
};
use http_body_util::BodyExt;
use std::process::Stdio;
use std::io::Read;
use tokio::process::Command;
use tokio_util::io::ReaderStream;
use flate2::read::GzDecoder;
use crate::scanner::RepoScanner;
use crate::db::Database;
use std::sync::Arc;

#[derive(Clone)]
pub struct GitHttpState {
    pub scanner: Arc<RepoScanner>,
    pub db: Arc<Database>,
}

fn extract_basic_auth(headers: &HeaderMap) -> Option<(String, String)> {
    let auth_header = headers.get(header::AUTHORIZATION)?;
    let auth_str = auth_header.to_str().ok()?;

    if !auth_str.starts_with("Basic ") {
        return None;
    }

    let encoded = auth_str.strip_prefix("Basic ")?;
    let decoded = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, encoded).ok()?;
    let decoded_str = String::from_utf8(decoded).ok()?;

    let mut parts = decoded_str.splitn(2, ':');
    let username = parts.next()?.to_string();
    let password = parts.next()?.to_string();

    Some((username, password))
}

async fn authenticate_user(db: &Database, username: &str, password: &str) -> Option<i64> {
    let user = db.get_user_by_username(username).await.ok()??;

    if crate::auth::verify_password(password, &user.password_hash).ok()? {
        Some(user.id)
    } else {
        None
    }
}

async fn check_repo_permission(db: &Database, user_id: i64, repo_path: &str, need_write: bool) -> bool {
    // 管理员有所有权限
    let user = match db.get_user_by_id(user_id).await {
        Ok(Some(user)) => {
            if user.is_admin {
                return true;
            }
            user
        }
        _ => return false,
    };

    // 检查是否是仓库所有者
    if let Ok(Some(repo)) = db.get_repository_by_path(repo_path).await {
        if repo.owner_id == Some(user_id) {
            return true;
        }

        // 检查是否是仓库成员
        if let Ok(members) = db.list_repository_members(repo.id).await {
            if let Some(member) = members.iter().find(|m| m.user_id == user_id) {
                // 读权限：任何成员都可以
                if !need_write {
                    return true;
                }
                // 写权限：需要 admin 或 member 角色（不包括 reader）
                return member.role == "admin" || member.role == "member";
            }
        }

        // 检查是否通过组成员关系有权限
        if let Ok(Some(group)) = db.get_group_by_id(repo.group_id).await {
            // 检查是否是组成员
            if let Ok(members) = db.list_group_members(group.id).await {
                if let Some(member) = members.iter().find(|m| m.user_id == user_id) {
                    // 读权限：任何组成员都可以访问组内的仓库
                    if !need_write {
                        return true;
                    }
                    // 写权限：需要 admin 或 member 角色
                    return member.role == "admin" || member.role == "member";
                }
            }
        }

        return false;
    }

    // 仓库不存在的情况（push-on-create）
    // 只有在需要写权限时才检查创建权限
    if !need_write {
        return false;
    }

    // 解析路径：username/... 或 username/group/.../repo
    let path_parts: Vec<&str> = repo_path.trim_start_matches('/').split('/').collect();
    if path_parts.is_empty() {
        return false;
    }

    let path_owner = path_parts[0];

    // 检查路径是否以当前用户名开头
    if path_owner == user.username {
        return true;
    }

    // 检查路径是否以用户所属的组开头
    // 路径格式可能是: username/groupname/repo 或 username/groupname/subgroup/repo
    if path_parts.len() >= 2 {
        let potential_group_path = format!("{}/{}", path_parts[0], path_parts[1]);

        // 尝试通过路径查找组
        if let Ok(Some(group)) = db.get_group_by_path(&potential_group_path).await {
            // 检查用户是否是该组的成员且有写权限
            if let Ok(members) = db.list_group_members(group.id).await {
                if let Some(member) = members.iter().find(|m| m.user_id == user_id) {
                    return member.role == "admin" || member.role == "member";
                }
            }
        }
    }

    false
}

fn unauthorized_response() -> Response {
    Response::builder()
        .status(StatusCode::UNAUTHORIZED)
        .header(header::WWW_AUTHENTICATE, "Basic realm=\"GitClub\"")
        .body(Body::from("Authentication required"))
        .unwrap()
}

pub async fn handle_info_refs(
    repo_path: &str,
    state: GitHttpState,
    service: &str,
    headers: HeaderMap,
) -> Response {
    let repo_path = repo_path.trim_start_matches('/');

    if service != "git-upload-pack" && service != "git-receive-pack" {
        return (StatusCode::BAD_REQUEST, "Invalid service").into_response();
    }

    // 身份验证
    let (username, password) = match extract_basic_auth(&headers) {
        Some(creds) => creds,
        None => return unauthorized_response(),
    };

    let user_id = match authenticate_user(&state.db, &username, &password).await {
        Some(id) => id,
        None => return unauthorized_response(),
    };

    // 权限检查
    let need_write = service == "git-receive-pack";
    if !check_repo_permission(&state.db, user_id, repo_path, need_write).await {
        return (StatusCode::FORBIDDEN, "Access denied").into_response();
    }

    let full_path = state.scanner.repos_path().join(repo_path);

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
    headers: HeaderMap,
    body: Body,
) -> Response {
    handle_git_rpc(repo_path, state, headers, body, "upload-pack").await
}

pub async fn handle_receive_pack(
    repo_path: &str,
    state: GitHttpState,
    headers: HeaderMap,
    body: Body,
) -> Response {
    handle_git_rpc(repo_path, state, headers, body, "receive-pack").await
}

async fn handle_git_rpc(
    repo_path: &str,
    state: GitHttpState,
    headers: HeaderMap,
    body: Body,
    service: &str,
) -> Response {
    let repo_path = repo_path.trim_start_matches('/');

    // 身份验证
    let (username, password) = match extract_basic_auth(&headers) {
        Some(creds) => creds,
        None => return unauthorized_response(),
    };

    let user_id = match authenticate_user(&state.db, &username, &password).await {
        Some(id) => id,
        None => return unauthorized_response(),
    };

    // 权限检查
    let need_write = service == "receive-pack";
    if !check_repo_permission(&state.db, user_id, repo_path, need_write).await {
        return (StatusCode::FORBIDDEN, "Access denied").into_response();
    }

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
