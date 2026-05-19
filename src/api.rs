use axum::{
    extract::State,
    response::IntoResponse,
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use crate::db::Database;
use crate::scanner::RepoScanner;
use std::sync::Arc;
use std::process::Command;

#[derive(Clone)]
pub struct ApiState {
    pub db: Arc<Database>,
    pub scanner: Arc<RepoScanner>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct TreeNode {
    pub name: String,
    pub path: String,
    pub is_repo: bool,
    pub children: Vec<TreeNode>,
}

pub async fn get_tree(State(state): State<ApiState>) -> impl IntoResponse {
    let entries = state.scanner.scan_directory();
    let tree = build_tree(entries);
    Json(tree)
}

fn build_tree(entries: Vec<crate::scanner::RepoEntry>) -> Vec<TreeNode> {
    let mut root_nodes: Vec<TreeNode> = Vec::new();
    let mut path_map: std::collections::HashMap<String, Vec<TreeNode>> = std::collections::HashMap::new();

    for entry in entries {
        let node = TreeNode {
            name: entry.name.clone(),
            path: entry.path.clone(),
            is_repo: entry.is_repo,
            children: Vec::new(),
        };

        let parts: Vec<&str> = entry.path.split('/').collect();
        if parts.len() == 1 {
            root_nodes.push(node);
        } else {
            let parent_path = parts[..parts.len() - 1].join("/");
            path_map.entry(parent_path).or_insert_with(Vec::new).push(node);
        }
    }

    attach_children(&mut root_nodes, &path_map);
    root_nodes
}

fn attach_children(nodes: &mut Vec<TreeNode>, path_map: &std::collections::HashMap<String, Vec<TreeNode>>) {
    for node in nodes.iter_mut() {
        if let Some(children) = path_map.get(&node.path) {
            node.children = children.clone();
            attach_children(&mut node.children, path_map);
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct CreateGroupRequest {
    pub name: String,
    pub parent_path: Option<String>,
    pub description: Option<String>,
}

pub async fn create_group(
    State(state): State<ApiState>,
    Json(req): Json<CreateGroupRequest>,
) -> impl IntoResponse {
    let path = if let Some(parent) = &req.parent_path {
        format!("{}/{}", parent, req.name)
    } else {
        req.name.clone()
    };

    let full_path = state.scanner.repos_path().join(&path);
    if let Err(e) = std::fs::create_dir_all(&full_path) {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to create directory: {}", e)).into_response();
    }

    (StatusCode::CREATED, Json(serde_json::json!({ "path": path }))).into_response()
}

#[derive(Serialize, Deserialize)]
pub struct CreateRepoRequest {
    pub name: String,
    pub group_path: String,
    pub description: Option<String>,
}

pub async fn create_repo(
    State(state): State<ApiState>,
    Json(req): Json<CreateRepoRequest>,
) -> impl IntoResponse {
    let repo_name = if req.name.ends_with(".git") {
        req.name.clone()
    } else {
        format!("{}.git", req.name)
    };

    let path = format!("{}/{}", req.group_path, repo_name);
    let full_path = state.scanner.repos_path().join(&path);

    if let Err(e) = std::fs::create_dir_all(&full_path) {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to create directory: {}", e)).into_response();
    }

    // 初始化 Git 裸仓库
    let output = std::process::Command::new("git")
        .args(&["init", "--bare"])
        .current_dir(&full_path)
        .output();

    match output {
        Ok(output) if output.status.success() => {
            (StatusCode::CREATED, Json(serde_json::json!({ "path": path }))).into_response()
        }
        _ => {
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to initialize git repository").into_response()
        }
    }
}

#[derive(Serialize)]
pub struct RepoFile {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
    pub commit_message: Option<String>,
    pub commit_date: Option<String>,
}

#[derive(Serialize)]
pub struct RepoDetail {
    pub name: String,
    pub path: String,
    pub default_branch: Option<String>,
    pub branches: Vec<String>,
    pub files: Vec<RepoFile>,
    pub readme_content: Option<String>,
    pub description: Option<String>,
    pub license: Option<String>,
    pub languages: Vec<LanguageInfo>,
}

#[derive(Serialize)]
pub struct LanguageInfo {
    pub name: String,
    pub percent: f32,
    pub color: String,
}

#[derive(Deserialize)]
pub struct RepoDetailQuery {
    pub path: Option<String>,
    pub branch: Option<String>,
}

pub async fn get_repo_detail(
    axum::extract::Path(repo_path): axum::extract::Path<String>,
    axum::extract::Query(query): axum::extract::Query<RepoDetailQuery>,
    State(state): State<ApiState>,
) -> impl IntoResponse {
    let full_path = state.scanner.repos_path().join(&repo_path);

    if !full_path.exists() {
        return (StatusCode::NOT_FOUND, "Repository not found").into_response();
    }

    let default_branch = get_default_branch(&full_path);
    let branches = get_branches(&full_path);

    let branch = query.branch.or(default_branch.clone());
    let file_path = query.path.as_deref().unwrap_or("");

    let files = if let Some(ref b) = branch {
        get_files(&full_path, b, file_path)
    } else {
        Vec::new()
    };

    let readme_content = if file_path.is_empty() {
        if let Some(ref b) = branch {
            get_readme_content(&full_path, b)
        } else {
            None
        }
    } else {
        None
    };

    let detail = RepoDetail {
        name: repo_path.split('/').last().unwrap_or(&repo_path).trim_end_matches(".git").to_string(),
        path: repo_path,
        default_branch: branch,
        branches,
        files,
        readme_content,
        description: None,
        license: None,
        languages: Vec::new(),
    };

    Json(detail).into_response()
}

fn get_default_branch(repo_path: &std::path::Path) -> Option<String> {
    // 尝试获取 HEAD 指向的分支
    let output = Command::new("git")
        .args(&["symbolic-ref", "HEAD"])
        .current_dir(repo_path)
        .output()
        .ok()?;

    if output.status.success() {
        let branch = String::from_utf8_lossy(&output.stdout);
        if let Some(branch) = branch.trim().strip_prefix("refs/heads/") {
            return Some(branch.to_string());
        }
    }

    // 如果 symbolic-ref 失败，尝试获取第一个分支
    let output = Command::new("git")
        .args(&["branch"])
        .current_dir(repo_path)
        .output()
        .ok()?;

    if output.status.success() {
        let branches = String::from_utf8_lossy(&output.stdout);
        let first_branch = branches
            .lines()
            .next()?
            .trim()
            .trim_start_matches("* ");
        return Some(first_branch.to_string());
    }

    None
}

fn get_branches(repo_path: &std::path::Path) -> Vec<String> {
    let output = Command::new("git")
        .args(&["branch", "-a"])
        .current_dir(repo_path)
        .output();

    match output {
        Ok(output) if output.status.success() => {
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .map(|line| line.trim().trim_start_matches("* ").to_string())
                .collect()
        }
        _ => Vec::new()
    }
}

fn get_files(repo_path: &std::path::Path, branch: &str, file_path: &str) -> Vec<RepoFile> {
    let tree_ref = if file_path.is_empty() {
        branch.to_string()
    } else {
        format!("{}:{}", branch, file_path)
    };

    let output = Command::new("git")
        .args(&["ls-tree", "-l", &tree_ref])
        .current_dir(repo_path)
        .output();

    match output {
        Ok(output) if output.status.success() => {
            let mut files: Vec<RepoFile> = String::from_utf8_lossy(&output.stdout)
                .lines()
                .filter_map(|line| {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 4 {
                        let is_dir = parts[1] == "tree";
                        let size = if is_dir { 0 } else { parts[3].parse().unwrap_or(0) };
                        let name = parts.get(4..).map(|p| p.join(" ")).unwrap_or_default();

                        let full_path = if file_path.is_empty() {
                            name.clone()
                        } else {
                            format!("{}/{}", file_path, name)
                        };

                        // 获取该文件的最后提交信息
                        let (commit_message, commit_date) = get_file_last_commit(repo_path, branch, &full_path);

                        Some(RepoFile {
                            name: name.clone(),
                            path: full_path,
                            is_dir,
                            size,
                            commit_message,
                            commit_date,
                        })
                    } else {
                        None
                    }
                })
                .collect();

            // 排序：文件夹在前，文件在后，同类型按名字排序
            files.sort_by(|a, b| {
                match (a.is_dir, b.is_dir) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => a.name.cmp(&b.name),
                }
            });

            files
        }
        _ => Vec::new()
    }
}

fn get_file_last_commit(repo_path: &std::path::Path, branch: &str, file_path: &str) -> (Option<String>, Option<String>) {
    let output = Command::new("git")
        .args(&["log", "-1", "--format=%s|%ar", branch, "--", file_path])
        .current_dir(repo_path)
        .output();

    match output {
        Ok(output) if output.status.success() => {
            let result = String::from_utf8_lossy(&output.stdout);
            let parts: Vec<&str> = result.trim().split('|').collect();
            if parts.len() == 2 {
                (Some(parts[0].to_string()), Some(parts[1].to_string()))
            } else {
                (None, None)
            }
        }
        _ => (None, None)
    }
}

fn get_readme_content(repo_path: &std::path::Path, branch: &str) -> Option<String> {
    for readme_name in &["README.md", "README", "readme.md", "Readme.md"] {
        let output = Command::new("git")
            .args(&["show", &format!("{}:{}", branch, readme_name)])
            .current_dir(repo_path)
            .output();

        if let Ok(output) = output {
            if output.status.success() {
                return Some(String::from_utf8_lossy(&output.stdout).to_string());
            }
        }
    }
    None
}

#[derive(Serialize)]
pub struct FileContent {
    pub content: String,
    pub is_binary: bool,
    pub size: u64,
}

#[derive(Deserialize)]
pub struct FileContentQuery {
    pub file: String,
    pub branch: Option<String>,
}

pub async fn get_file_content(
    axum::extract::Path(repo_path): axum::extract::Path<String>,
    axum::extract::Query(query): axum::extract::Query<FileContentQuery>,
    State(state): State<ApiState>,
) -> impl IntoResponse {
    let full_repo_path = state.scanner.repos_path().join(&repo_path);

    if !full_repo_path.exists() {
        return (StatusCode::NOT_FOUND, "Repository not found").into_response();
    }

    let default_branch = get_default_branch(&full_repo_path);
    let branch = query.branch.or(default_branch);

    if branch.is_none() {
        return (StatusCode::BAD_REQUEST, "No branch specified and no default branch found").into_response();
    }

    let branch = branch.unwrap();
    let output = Command::new("git")
        .args(&["show", &format!("{}:{}", branch, query.file)])
        .current_dir(&full_repo_path)
        .output();

    match output {
        Ok(output) if output.status.success() => {
            let content = output.stdout;
            let is_binary = content.iter().take(8000).any(|&b| b == 0);

            if is_binary {
                Json(FileContent {
                    content: String::from("[Binary file]"),
                    is_binary: true,
                    size: content.len() as u64,
                }).into_response()
            } else {
                Json(FileContent {
                    content: String::from_utf8_lossy(&content).to_string(),
                    is_binary: false,
                    size: content.len() as u64,
                }).into_response()
            }
        }
        _ => (StatusCode::NOT_FOUND, "File not found").into_response()
    }
}
