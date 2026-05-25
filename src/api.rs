use axum::{
    extract::State,
    response::IntoResponse,
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use crate::db::{self, Database};
use crate::scanner::RepoScanner;
use crate::auth::{AuthUser, OptionalAuthUser};
use std::sync::Arc;
use std::process::Command;
use std::path::Path;
use std::fs;
use sqlx;

fn generate_avatar_data_url(username: &str, user_id: i64) -> String {
    // 基于用户 ID 生成一个 5x5 的几何图案（类似 GitHub identicon）
    // 使用用户 ID 作为种子生成伪随机数
    let mut seed = user_id as u32;

    // 简单的伪随机数生成器
    let mut random = || {
        seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
        (seed / 65536) % 2
    };

    // 生成 5x5 网格，但只生成左半部分（3列），右半部分镜像
    let mut grid = [[false; 5]; 5];
    for row in 0..5 {
        for col in 0..3 {
            grid[row][col] = random() == 1;
            grid[row][4 - col] = grid[row][col]; // 镜像
        }
    }

    // 基于用户名生成颜色
    let hash = username.bytes().fold(0u32, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u32));
    let hue = (hash % 360) as f32;
    let saturation = 65.0;
    let lightness = 50.0;

    // 生成 SVG
    let cell_size = 20;
    let svg_size = cell_size * 5;
    let mut svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}">"#,
        svg_size, svg_size, svg_size, svg_size
    );

    // 背景
    svg.push_str(&format!(
        r#"<rect width="{}" height="{}" fill="hsl({}, {}%, {}%)"/>"#,
        svg_size, svg_size, hue, saturation, lightness + 35.0
    ));

    // 绘制网格
    for (row_idx, row) in grid.iter().enumerate() {
        for (col_idx, &cell) in row.iter().enumerate() {
            if cell {
                let x = col_idx * cell_size;
                let y = row_idx * cell_size;
                svg.push_str(&format!(
                    r#"<rect x="{}" y="{}" width="{}" height="{}" fill="hsl({}, {}%, {})"/>"#,
                    x, y, cell_size, cell_size, hue, saturation, lightness
                ));
            }
        }
    }

    svg.push_str("</svg>");

    // 转换为 data URL
    format!("data:image/svg+xml;base64,{}", base64::Engine::encode(&base64::engine::general_purpose::STANDARD, svg.as_bytes()))
}

fn generate_avatar_url(username: &str, user_id: i64) -> String {
    generate_avatar_data_url(username, user_id)
}

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

pub async fn get_tree(
    State(state): State<ApiState>,
    OptionalAuthUser(user): OptionalAuthUser,
) -> impl IntoResponse {
    let entries = state.scanner.scan_directory();

    // 根据用户权限过滤条目
    let filtered_entries = if let Some(user) = user {
        if user.is_admin {
            // 管理员可以看到所有内容
            entries
        } else {
            // 获取用户有权限访问的路径（作为所有者或成员）
            let accessible_groups = state.db.get_accessible_group_paths(user.user_id).await.unwrap_or_default();
            let accessible_repos = state.db.get_accessible_repo_paths(user.user_id).await.unwrap_or_default();

            // 获取用户是成员的组及其 can_view_subgroups 权限
            let member_groups = sqlx::query_as::<_, (String, bool)>(
                "SELECT g.path, gm.can_view_subgroups FROM groups g INNER JOIN group_members gm ON g.id = gm.group_id WHERE gm.user_id = ?"
            )
            .bind(user.user_id)
            .fetch_all(state.db.pool())
            .await
            .unwrap_or_default();

            // 普通用户可以看到自己的文件夹，以及被分享的组/仓库
            entries.into_iter()
                .filter(|entry| {
                    // 自己的文件夹
                    if entry.path.starts_with(&format!("{}/", user.username)) || entry.path == user.username {
                        return true;
                    }

                    // 精确匹配可见组
                    if accessible_groups.iter().any(|g| entry.path == *g) {
                        return true;
                    }

                    // 是某个可见组的父路径（需要显示父路径才能展开到授权的组）
                    if accessible_groups.iter().any(|g| g.starts_with(&format!("{}/", entry.path))) {
                        return true;
                    }

                    // 精确匹配被分享的仓库
                    if accessible_repos.iter().any(|r| entry.path == *r) {
                        return true;
                    }

                    // 是被分享仓库的父路径
                    if accessible_repos.iter().any(|r| r.starts_with(&format!("{}/", entry.path))) {
                        return true;
                    }

                    // 检查是否是某个有权限组的直接子项（仅当 can_view_subgroups = false 时）
                    // 或者是子孙项（当 can_view_subgroups = true 时）
                    for (group_path, can_view_subgroups) in &member_groups {
                        if entry.path.starts_with(&format!("{}/", group_path)) {
                            let relative = &entry.path[group_path.len() + 1..];

                            if *can_view_subgroups {
                                // 可以查看子组：显示所有子项
                                return true;
                            } else {
                                // 不能查看子组：只显示直接仓库（以 .git 结尾），不显示子组
                                if !relative.contains('/') && entry.path.ends_with(".git") {
                                    return true;
                                }
                            }
                        }
                    }

                    false
                })
                .collect()
        }
    } else {
        // 未登录用户看不到任何内容
        Vec::new()
    };

    let tree = build_tree(filtered_entries);
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
    user: AuthUser,
    Json(req): Json<CreateGroupRequest>,
) -> impl IntoResponse {
    // 检查路径中的任何部分是否以 .git 结尾
    for part in req.name.split('/') {
        if part.ends_with(".git") {
            return (StatusCode::BAD_REQUEST, "Group path cannot contain segments ending with .git. Use Repository type to create a git repository.").into_response();
        }
    }

    let path = if let Some(parent) = &req.parent_path {
        format!("{}/{}", parent, req.name)
    } else {
        req.name.clone()
    };

    // 权限检查：普通用户只能在自己的文件夹下创建
    if !user.is_admin {
        if !path.starts_with(&format!("{}/", user.username)) && path != user.username {
            return (StatusCode::FORBIDDEN, "You can only create groups in your own folder").into_response();
        }
    }

    let full_path = state.scanner.repos_path().join(&path);
    if let Err(e) = std::fs::create_dir_all(&full_path) {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to create directory: {}", e)).into_response();
    }

    // 创建 .meta/.meta 文件夹并保存组信息
    let meta_meta_path = full_path.join(".meta/.meta");
    if let Err(e) = std::fs::create_dir_all(&meta_meta_path) {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to create .meta/.meta directory: {}", e)).into_response();
    }

    // 保存组的元数据到 .meta/.meta/group.json
    let group_meta = serde_json::json!({
        "description": req.description,
        "created_at": chrono::Utc::now().to_rfc3339(),
    });

    let meta_file = meta_meta_path.join("group.json");
    if let Err(e) = std::fs::write(&meta_file, serde_json::to_string_pretty(&group_meta).unwrap()) {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to write metadata: {}", e)).into_response();
    }

    // 在数据库中记录组信息
    let parent_id = if let Some(ref parent) = req.parent_path {
        match state.db.ensure_group_path(parent, user.user_id).await {
            Ok(id) => Some(id),
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to record parent group: {}", e)).into_response(),
        }
    } else {
        None
    };

    if let Err(e) = state.db.create_group(&req.name, &path, parent_id, req.description.as_deref(), user.user_id).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to record group: {}", e)).into_response();
    }

    (StatusCode::CREATED, Json(serde_json::json!({ "path": path }))).into_response()
}

#[derive(Serialize, Deserialize)]
pub struct CreateRepoRequest {
    pub name: String,
    pub parent_path: Option<String>,
    pub description: Option<String>,
}

pub async fn create_repo(
    State(state): State<ApiState>,
    user: AuthUser,
    Json(req): Json<CreateRepoRequest>,
) -> impl IntoResponse {
    // 检查路径中间部分是否包含 .git
    for part in req.name.split('/') {
        if part.ends_with(".git") && part != req.name.split('/').last().unwrap() {
            return (StatusCode::BAD_REQUEST, "Repository path cannot contain .git in intermediate segments.").into_response();
        }
    }

    let repo_name = if req.name.ends_with(".git") {
        req.name.clone()
    } else {
        format!("{}.git", req.name)
    };

    let path = if let Some(parent) = &req.parent_path {
        format!("{}/{}", parent, repo_name)
    } else {
        repo_name.clone()
    };

    // 权限检查：普通用户只能在自己的文件夹下创建
    if !user.is_admin {
        if !path.starts_with(&format!("{}/", user.username)) && path != format!("{}/{}", user.username, repo_name) {
            return (StatusCode::FORBIDDEN, "You can only create repositories in your own folder").into_response();
        }
    }

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
            // 在父组的 .meta/<repo_name>/ 下保存仓库信息
            let parent_path = if let Some(parent) = &req.parent_path {
                state.scanner.repos_path().join(parent)
            } else {
                state.scanner.repos_path().to_path_buf()
            };

            let repo_meta_path = parent_path.join(".meta").join(&repo_name);
            if let Err(e) = std::fs::create_dir_all(&repo_meta_path) {
                return (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to create .meta/{} directory: {}", repo_name, e)).into_response();
            }

            // 保存仓库的元数据到父组的 .meta/<repo_name>/repo.json
            let repo_meta = serde_json::json!({
                "description": req.description,
                "created_at": chrono::Utc::now().to_rfc3339(),
            });

            let meta_file = repo_meta_path.join("repo.json");
            if let Err(e) = std::fs::write(&meta_file, serde_json::to_string_pretty(&repo_meta).unwrap()) {
                return (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to write metadata: {}", e)).into_response();
            }

            // 在数据库中记录仓库信息
            let group_id = if let Some(ref parent) = req.parent_path {
                match state.db.ensure_group_path(parent, user.user_id).await {
                    Ok(id) => id,
                    Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to record parent group: {}", e)).into_response(),
                }
            } else {
                // 没有父路径时，使用用户根组
                match state.db.ensure_group_path(&user.username, user.user_id).await {
                    Ok(id) => id,
                    Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to record root group: {}", e)).into_response(),
                }
            };

            if let Err(e) = state.db.create_repository(&repo_name, &path, group_id, req.description.as_deref(), user.user_id).await {
                return (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to record repository: {}", e)).into_response();
            }

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
    pub tags: Vec<TagInfo>,
    pub files: Vec<RepoFile>,
    pub readme_content: Option<String>,
    pub description: Option<String>,
    pub license: Option<String>,
    pub languages: Vec<LanguageInfo>,
    pub owner: Option<OwnerInfo>,
    pub members: Vec<MemberInfo>,
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
    OptionalAuthUser(user): OptionalAuthUser,
) -> impl IntoResponse {
    // 权限检查
    if let Some(user) = user {
        if !user.is_admin {
            let has_own_folder = repo_path.starts_with(&format!("{}/", user.username)) || repo_path.starts_with(&user.username);
            let has_db_access = state.db.user_can_access_path(user.user_id, &repo_path).await.unwrap_or(false);
            if !has_own_folder && !has_db_access {
                return (StatusCode::FORBIDDEN, "Access denied").into_response();
            }
        }
    } else {
        return (StatusCode::UNAUTHORIZED, "Authentication required").into_response();
    }

    let full_path = state.scanner.repos_path().join(&repo_path);

    if !full_path.exists() {
        return (StatusCode::NOT_FOUND, "Repository not found").into_response();
    }

    let default_branch = get_default_branch(&full_path);
    let branches = get_branches(&full_path);
    let tags = get_tags(&full_path);

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

    // 确保仓库在数据库中存在（如果不存在则自动创建并设置 owner）
    let _ = state.db.ensure_repository_exists(&repo_path).await;

    // 获取所有者和成员信息
    let (owner, members) = if let Ok(Some(repo)) = state.db.get_repository_by_path(&repo_path).await {
        let owner_info = if let Some(owner_id) = repo.owner_id {
            state.db.get_user_by_id(owner_id).await.ok().flatten().map(|u| OwnerInfo {
                user_id: u.id,
                username: u.username,
                display_name: u.display_name,
                avatar_url: u.avatar_url,
            })
        } else {
            None
        };

        let members_list = state.db.list_repository_members(repo.id).await.ok().unwrap_or_default();
        let mut member_infos = Vec::new();
        for m in members_list {
            if let Ok(Some(u)) = state.db.get_user_by_id(m.user_id).await {
                member_infos.push(MemberInfo {
                    user_id: u.id,
                    username: u.username,
                    role: m.role,
                    created_at: m.created_at,
                    can_view_subgroups: None,
                });
            }
        }
        (owner_info, member_infos)
    } else {
        (None, Vec::new())
    };

    let detail = RepoDetail {
        name: repo_path.split('/').last().unwrap_or(&repo_path).trim_end_matches(".git").to_string(),
        path: repo_path,
        default_branch: branch,
        branches,
        tags,
        files,
        readme_content,
        description: None,
        license: None,
        languages: Vec::new(),
        owner,
        members,
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

#[derive(Serialize)]
pub struct TagInfo {
    pub name: String,
    pub commit: String,
    pub message: Option<String>,
    pub tagger: Option<String>,
    pub date: Option<String>,
}

fn get_tags(repo_path: &std::path::Path) -> Vec<TagInfo> {
    let output = Command::new("git")
        .args(&["tag", "-l"])
        .current_dir(repo_path)
        .output();

    match output {
        Ok(output) if output.status.success() => {
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .filter(|line| !line.is_empty())
                .filter_map(|tag_name| {
                    // 获取 tag 的详细信息
                    let show_output = Command::new("git")
                        .args(&["show", "-s", "--format=%H|%an|%ar|%s", tag_name])
                        .current_dir(repo_path)
                        .output()
                        .ok()?;

                    if show_output.status.success() {
                        let info = String::from_utf8_lossy(&show_output.stdout);
                        let parts: Vec<&str> = info.trim().split('|').collect();
                        if parts.len() >= 4 {
                            Some(TagInfo {
                                name: tag_name.to_string(),
                                commit: parts[0].to_string(),
                                tagger: Some(parts[1].to_string()),
                                date: Some(parts[2].to_string()),
                                message: Some(parts[3].to_string()),
                            })
                        } else {
                            Some(TagInfo {
                                name: tag_name.to_string(),
                                commit: parts.get(0).unwrap_or(&"").to_string(),
                                tagger: None,
                                date: None,
                                message: None,
                            })
                        }
                    } else {
                        None
                    }
                })
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

fn get_repo_last_commit(repo_path: &std::path::Path, branch: &str) -> (Option<String>, Option<String>) {
    let output = Command::new("git")
        .args(&["log", "-1", "--format=%s|%ar", branch])
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
    OptionalAuthUser(user): OptionalAuthUser,
) -> impl IntoResponse {
    // 权限检查
    if let Some(user) = user {
        if !user.is_admin {
            let has_own_folder = repo_path.starts_with(&format!("{}/", user.username)) || repo_path.starts_with(&user.username);
            let has_db_access = state.db.user_can_access_path(user.user_id, &repo_path).await.unwrap_or(false);
            if !has_own_folder && !has_db_access {
                return (StatusCode::FORBIDDEN, "Access denied").into_response();
            }
        }
    } else {
        return (StatusCode::UNAUTHORIZED, "Authentication required").into_response();
    }

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

#[derive(Serialize)]
pub struct GroupDetail {
    pub name: String,
    pub path: String,
    pub total_size: u64,
    pub total_size_str: String,
    pub description: Option<String>,
    pub owner: Option<OwnerInfo>,
    pub members: Vec<MemberInfo>,
    pub repositories: Vec<GroupRepoInfo>,
    pub subgroups: Vec<GroupSubgroupInfo>,
}

#[derive(Serialize)]
pub struct OwnerInfo {
    pub user_id: i64,
    pub username: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
}

#[derive(Serialize)]
pub struct GroupRepoInfo {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub size_str: String,
    pub default_branch: Option<String>,
    pub last_commit_message: Option<String>,
    pub last_commit_date: Option<String>,
}

#[derive(Serialize)]
pub struct GroupSubgroupInfo {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub size_str: String,
}

fn dir_size(path: &Path) -> u64 {
    let mut total = 0u64;
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                total += dir_size(&p);
            } else if let Ok(m) = p.metadata() {
                total += m.len();
            }
        }
    }
    total
}

fn format_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KiB", "MiB", "GiB", "TiB"];
    if bytes == 0 {
        return "0 B".to_string();
    }
    let mut size = bytes as f64;
    let mut unit_idx = 0;
    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }
    if unit_idx == 0 {
        format!("{} {}", bytes, UNITS[unit_idx])
    } else {
        format!("{:.2} {}", size, UNITS[unit_idx])
    }
}

pub async fn get_group_detail(
    axum::extract::Path(group_path): axum::extract::Path<String>,
    State(state): State<ApiState>,
    OptionalAuthUser(user): OptionalAuthUser,
) -> impl IntoResponse {
    // 权限检查
    if let Some(ref user) = user {
        if !user.is_admin {
            let has_own_folder = group_path.starts_with(&format!("{}/", user.username)) || group_path == user.username;
            let has_db_access = state.db.user_can_access_path(user.user_id, &group_path).await.unwrap_or(false);
            if !has_own_folder && !has_db_access {
                return (StatusCode::FORBIDDEN, "Access denied").into_response();
            }
        }
    } else {
        return (StatusCode::UNAUTHORIZED, "Authentication required").into_response();
    }

    let full_path = state.scanner.repos_path().join(&group_path);

    if !full_path.exists() || !full_path.is_dir() {
        return (StatusCode::NOT_FOUND, "Group not found").into_response();
    }

    let mut repositories = Vec::new();
    let mut subgroups = Vec::new();

    // 检查当前用户对该组的权限
    let can_view_subgroups = if let Some(ref user) = user {
        if user.is_admin {
            true
        } else if let Ok(Some(group)) = state.db.get_group_by_path(&group_path).await {
            // 检查是否是所有者
            if group.owner_id == Some(user.user_id) {
                true
            } else {
                // 检查是否是成员，并获取 can_view_subgroups 权限
                let members = state.db.list_group_members(group.id).await.ok().unwrap_or_default();
                members.iter()
                    .find(|m| m.user_id == user.user_id)
                    .map(|m| m.can_view_subgroups)
                    .unwrap_or(false)
            }
        } else {
            false
        }
    } else {
        false
    };

    if let Ok(entries) = fs::read_dir(&full_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();

            // 忽略 .meta 文件夹
            if name == ".meta" {
                continue;
            }

            if path.is_dir() {
                if name.ends_with(".git") {
                    // 仓库：总是显示
                    let size = dir_size(&path);
                    let default_branch = get_default_branch(&path);
                    let (last_commit_message, last_commit_date) = if let Some(ref branch) = default_branch {
                        get_repo_last_commit(&path, branch)
                    } else {
                        (None, None)
                    };
                    repositories.push(GroupRepoInfo {
                        name: name.trim_end_matches(".git").to_string(),
                        path: format!("{}/{}", group_path, name),
                        size,
                        size_str: format_size(size),
                        default_branch,
                        last_commit_message,
                        last_commit_date,
                    });
                } else {
                    // 子组：只有在 can_view_subgroups = true 时才显示
                    if can_view_subgroups {
                        let size = dir_size(&path);
                        let sub_path = format!("{}/{}", group_path, name);
                        subgroups.push(GroupSubgroupInfo {
                            name,
                            path: sub_path,
                            size,
                            size_str: format_size(size),
                        });
                    }
                }
            }
        }
    }

    // total_size = 只统计直接仓库的大小，不包括子组
    let total_size: u64 = repositories.iter().map(|r| r.size).sum();

    // Sort: repositories by name, subgroups by name
    repositories.sort_by(|a, b| a.name.cmp(&b.name));
    subgroups.sort_by(|a, b| a.name.cmp(&b.name));

    let name = group_path.split('/').last().unwrap_or(&group_path).to_string();

    // 确保组在数据库中存在（如果不存在则自动创建并设置 owner）
    let _ = state.db.ensure_group_exists(&group_path).await;

    // 从 .meta/.meta/group.json 读取描述
    let description = {
        let meta_file = full_path.join(".meta/.meta/group.json");
        if meta_file.exists() {
            std::fs::read_to_string(&meta_file)
                .ok()
                .and_then(|content| serde_json::from_str::<serde_json::Value>(&content).ok())
                .and_then(|json| json.get("description").and_then(|v| v.as_str()).map(String::from))
        } else {
            None
        }
    };

    // 获取所有者和成员信息
    let (owner, members) = if let Ok(Some(group)) = state.db.get_group_by_path(&group_path).await {
        let owner_info = if let Some(owner_id) = group.owner_id {
            state.db.get_user_by_id(owner_id).await.ok().flatten().map(|u| OwnerInfo {
                user_id: u.id,
                username: u.username,
                display_name: u.display_name,
                avatar_url: u.avatar_url,
            })
        } else {
            None
        };

        let members_list = state.db.list_group_members(group.id).await.ok().unwrap_or_default();
        let mut member_infos = Vec::new();
        for m in members_list {
            if let Ok(Some(u)) = state.db.get_user_by_id(m.user_id).await {
                member_infos.push(MemberInfo {
                    user_id: u.id,
                    username: u.username,
                    role: m.role,
                    created_at: m.created_at,
                    can_view_subgroups: Some(m.can_view_subgroups),
                });
            }
        }
        (owner_info, member_infos)
    } else {
        (None, Vec::new())
    };

    let detail = GroupDetail {
        name,
        path: group_path,
        total_size,
        total_size_str: format_size(total_size),
        description,
        owner,
        members,
        repositories,
        subgroups,
    };

    Json(detail).into_response()
}

// ==================== 认证相关 API ====================

use crate::auth::{AuthResponse, LoginRequest, RegisterRequest, UserInfo, create_jwt, hash_password, verify_password};
use axum_extra::extract::cookie::{Cookie, CookieJar};

pub async fn login(
    State(state): State<ApiState>,
    jar: CookieJar,
    Json(req): Json<LoginRequest>,
) -> Result<(CookieJar, Json<AuthResponse>), (StatusCode, Json<AuthResponse>)> {
    // 查找用户
    let user = state.db.get_user_by_username(&req.username)
        .await
        .map_err(|_| (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(AuthResponse {
                success: false,
                message: "Database error".to_string(),
                user: None,
            })
        ))?;

    let user = user.ok_or_else(|| (
        StatusCode::UNAUTHORIZED,
        Json(AuthResponse {
            success: false,
            message: "Invalid username or password".to_string(),
            user: None,
        })
    ))?;

    // 验证密码
    let valid = verify_password(&req.password, &user.password_hash)
        .map_err(|_| (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(AuthResponse {
                success: false,
                message: "Password verification error".to_string(),
                user: None,
            })
        ))?;

    if !valid {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(AuthResponse {
                success: false,
                message: "Invalid username or password".to_string(),
                user: None,
            })
        ));
    }

    // 生成 JWT token
    let jwt_secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "change-this-secret-key-in-production".to_string());
    let token = create_jwt(&user.username, user.id, user.is_admin, &jwt_secret)
        .map_err(|_| (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(AuthResponse {
                success: false,
                message: "Failed to create token".to_string(),
                user: None,
            })
        ))?;

    // 设置 cookie
    let cookie = Cookie::build(("token", token))
        .path("/")
        .http_only(true)
        .max_age(time::Duration::days(7))
        .build();

    let jar = jar.add(cookie);

    Ok((jar, Json(AuthResponse {
        success: true,
        message: "Login successful".to_string(),
        user: Some(UserInfo {
            id: user.id,
            username: user.username,
            email: user.email,
            display_name: user.display_name,
            bio: user.bio,
            avatar_url: user.avatar_url,
            is_admin: user.is_admin,
        }),
    })))
}

pub async fn register(
    State(state): State<ApiState>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<AuthResponse>, (StatusCode, Json<AuthResponse>)> {
    // 验证用户名格式
    if req.username.is_empty() || req.username.len() > 50 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(AuthResponse {
                success: false,
                message: "Username must be between 1 and 50 characters".to_string(),
                user: None,
            })
        ));
    }

    // 检查用户名是否已存在
    let existing = state.db.get_user_by_username(&req.username)
        .await
        .map_err(|_| (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(AuthResponse {
                success: false,
                message: "Database error".to_string(),
                user: None,
            })
        ))?;

    if existing.is_some() {
        return Err((
            StatusCode::CONFLICT,
            Json(AuthResponse {
                success: false,
                message: "Username already exists".to_string(),
                user: None,
            })
        ));
    }

    // 哈希密码
    let password_hash = hash_password(&req.password)
        .map_err(|_| (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(AuthResponse {
                success: false,
                message: "Failed to hash password".to_string(),
                user: None,
            })
        ))?;

    // 创建用户
    let user_id = state.db.create_user(
        &req.username,
        &password_hash,
        Some(&req.email),
        false, // 普通用户，非管理员
    )
    .await
    .map_err(|_| (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(AuthResponse {
            success: false,
            message: "Failed to create user".to_string(),
            user: None,
        })
    ))?;

    // 创建用户的 repos 文件夹
    let repos_folder = state.scanner.repos_path().join(&req.username);
    if let Err(e) = std::fs::create_dir_all(&repos_folder) {
        tracing::error!("Failed to create user repos folder: {}", e);
    }

    // 生成默认头像（基于用户名和 ID）
    let avatar_url = generate_avatar_url(&req.username, user_id);

    // 更新用户信息（display_name 和 avatar_url）
    let _ = state.db.update_user_profile(
        user_id,
        req.display_name.as_deref(),
        None,
        None,
        Some(&avatar_url)
    ).await;

    Ok(Json(AuthResponse {
        success: true,
        message: "Registration successful".to_string(),
        user: Some(UserInfo {
            id: user_id,
            username: req.username,
            email: Some(req.email),
            display_name: req.display_name,
            bio: None,
            avatar_url: Some(avatar_url),
            is_admin: false,
        }),
    }))
}

pub async fn logout(jar: CookieJar) -> (CookieJar, Json<AuthResponse>) {
    let jar = jar.remove(Cookie::from("token"));
    (jar, Json(AuthResponse {
        success: true,
        message: "Logout successful".to_string(),
        user: None,
    }))
}

pub async fn get_current_user(
    State(state): State<ApiState>,
    jar: CookieJar,
) -> Result<Json<AuthResponse>, (StatusCode, Json<AuthResponse>)> {
    use crate::auth::Claims;
    use jsonwebtoken::{decode, DecodingKey, Validation};

    let token = jar
        .get("token")
        .map(|c| c.value())
        .ok_or_else(|| (
            StatusCode::UNAUTHORIZED,
            Json(AuthResponse {
                success: false,
                message: "Not authenticated".to_string(),
                user: None,
            })
        ))?;

    let jwt_secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "change-this-secret-key-in-production".to_string());

    let claims = decode::<Claims>(
        token,
        &DecodingKey::from_secret(jwt_secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|_| (
        StatusCode::UNAUTHORIZED,
        Json(AuthResponse {
            success: false,
            message: "Invalid token".to_string(),
            user: None,
        })
    ))?
    .claims;

    let user = state.db.get_user_by_id(claims.user_id)
        .await
        .map_err(|_| (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(AuthResponse {
                success: false,
                message: "Database error".to_string(),
                user: None,
            })
        ))?
        .ok_or_else(|| (
            StatusCode::UNAUTHORIZED,
            Json(AuthResponse {
                success: false,
                message: "User not found".to_string(),
                user: None,
            })
        ))?;

    Ok(Json(AuthResponse {
        success: true,
        message: "Authenticated".to_string(),
        user: Some(UserInfo {
            id: user.id,
            username: user.username,
            email: user.email,
            display_name: user.display_name,
            bio: user.bio,
            avatar_url: user.avatar_url,
            is_admin: user.is_admin,
        }),
    }))
}

#[derive(Serialize, Deserialize)]
pub struct UpdateProfileRequest {
    pub display_name: Option<String>,
    pub email: Option<String>,
    pub bio: Option<String>,
    pub avatar_url: Option<String>,
}

pub async fn update_profile(
    State(state): State<ApiState>,
    user: AuthUser,
    Json(req): Json<UpdateProfileRequest>,
) -> Result<Json<AuthResponse>, (StatusCode, Json<AuthResponse>)> {
    // 更新用户资料
    state.db.update_user_profile(
        user.user_id,
        req.display_name.as_deref(),
        req.email.as_deref(),
        req.bio.as_deref(),
        req.avatar_url.as_deref(),
    )
    .await
    .map_err(|_| (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(AuthResponse {
            success: false,
            message: "Failed to update profile".to_string(),
            user: None,
        })
    ))?;

    // 获取更新后的用户信息
    let updated_user = state.db.get_user_by_id(user.user_id)
        .await
        .map_err(|_| (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(AuthResponse {
                success: false,
                message: "Failed to fetch updated user".to_string(),
                user: None,
            })
        ))?
        .ok_or_else(|| (
            StatusCode::NOT_FOUND,
            Json(AuthResponse {
                success: false,
                message: "User not found".to_string(),
                user: None,
            })
        ))?;

    Ok(Json(AuthResponse {
        success: true,
        message: "Profile updated successfully".to_string(),
        user: Some(UserInfo {
            id: updated_user.id,
            username: updated_user.username,
            email: updated_user.email,
            display_name: updated_user.display_name,
            bio: updated_user.bio,
            avatar_url: updated_user.avatar_url,
            is_admin: updated_user.is_admin,
        }),
    }))
}

// ==================== 成员管理 API ====================

#[derive(Serialize, Deserialize)]
pub struct AddGroupMemberRequest {
    pub path: String,
    pub username: String,
    pub role: Option<String>,
    pub can_view_subgroups: Option<bool>,
}

#[derive(Serialize, Deserialize)]
pub struct AddRepoMemberRequest {
    pub path: String,
    pub username: String,
    pub role: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct RemoveGroupMemberRequest {
    pub path: String,
    pub username: String,
}

#[derive(Serialize, Deserialize)]
pub struct RemoveRepoMemberRequest {
    pub path: String,
    pub username: String,
}

#[derive(Serialize)]
pub struct MemberInfo {
    pub user_id: i64,
    pub username: String,
    pub role: String,
    pub created_at: String,
    pub can_view_subgroups: Option<bool>,
}

#[derive(Deserialize)]
pub struct PathQuery {
    pub path: String,
}

pub async fn list_group_members_handler(
    State(state): State<ApiState>,
    user: AuthUser,
    axum::extract::Query(query): axum::extract::Query<PathQuery>,
) -> Result<Json<Vec<MemberInfo>>, (StatusCode, Json<serde_json::Value>)> {
    let group = state.db.get_group_by_path(&query.path).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": format!("Database error: {}", e)}))))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "Group not found"}))))?;

    // 权限检查：只有成员、所有者或管理员可以查看
    let is_owner = group.owner_id == Some(user.user_id);
    let is_admin = user.is_admin;
    let is_member = state.db.list_group_members(group.id).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": format!("Database error: {}", e)}))))?
        .iter().any(|m| m.user_id == user.user_id);

    if !is_owner && !is_admin && !is_member {
        return Err((StatusCode::FORBIDDEN, Json(serde_json::json!({"error": "Access denied"}))));
    }

    let members = state.db.list_group_members(group.id).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": format!("Database error: {}", e)}))))?;

    let mut result = Vec::new();
    for m in &members {
        let user_info = state.db.get_user_by_id(m.user_id).await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": format!("Database error: {}", e)}))))?;
        if let Some(u) = user_info {
            result.push(MemberInfo {
                user_id: m.user_id,
                username: u.username,
                role: m.role.clone(),
                created_at: m.created_at.clone(),
                can_view_subgroups: Some(m.can_view_subgroups),
            });
        }
    }

    Ok(Json(result))
}

pub async fn add_group_member_handler(
    State(state): State<ApiState>,
    user: AuthUser,
    Json(req): Json<AddGroupMemberRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let group = state.db.get_group_by_path(&req.path).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": format!("Database error: {}", e)}))))?;

    let group = if let Some(g) = group {
        g
    } else {
        // 组在数据库中不存在，自动创建记录
        // 根据路径推断所有者：路径的第一部分是用户名
        let path_parts: Vec<&str> = req.path.split('/').collect();
        let owner_username = path_parts.first().copied().unwrap_or(&user.username);

        // 尝试查找对应的用户，如果不存在则使用当前用户
        let owner = state.db.get_user_by_username(owner_username).await
            .ok()
            .flatten()
            .unwrap_or_else(|| {
                // 如果路径第一部分不是有效用户，使用当前操作用户
                db::User {
                    id: user.user_id,
                    username: user.username.clone(),
                    password_hash: String::new(),
                    email: None,
                    is_admin: user.is_admin,
                    display_name: None,
                    avatar_url: None,
                    bio: None,
                    location: None,
                    website: None,
                    repos_folder: None,
                    created_at: String::new(),
                    updated_at: String::new(),
                }
            });

        let group_name = req.path.split('/').last().unwrap_or(&req.path);
        let parent_path = if req.path.contains('/') {
            let parts: Vec<&str> = req.path.split('/').collect();
            Some(parts[..parts.len()-1].join("/"))
        } else {
            None
        };

        let parent_id = if let Some(ref parent) = parent_path {
            state.db.ensure_group_path(parent, owner.id).await.ok()
        } else {
            None
        };

        let group_id = state.db.create_group(group_name, &req.path, parent_id, None, owner.id).await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": format!("Failed to create group record: {}", e)}))))?;

        state.db.get_group_by_id(group_id).await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": format!("Database error: {}", e)}))))?
            .ok_or_else(|| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "Failed to retrieve created group"}))))?
    };

    // 只有所有者或管理员可以添加成员
    if !user.is_admin && group.owner_id != Some(user.user_id) {
        return Err((StatusCode::FORBIDDEN, Json(serde_json::json!({"error": "Only the group owner or admin can manage members"}))));
    }

    let target_user = state.db.get_user_by_username(&req.username).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": format!("Database error: {}", e)}))))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "User not found"}))))?;

    let role = req.role.as_deref().unwrap_or("member");
    let can_view_subgroups = req.can_view_subgroups.unwrap_or(false);
    state.db.add_group_member(group.id, target_user.id, role, can_view_subgroups).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": format!("Failed to add member: {}", e)}))))?;

    Ok(Json(serde_json::json!({"success": true, "message": "Member added"})))
}

pub async fn remove_group_member_handler(
    State(state): State<ApiState>,
    user: AuthUser,
    Json(req): Json<RemoveGroupMemberRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let group = state.db.get_group_by_path(&req.path).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": format!("Database error: {}", e)}))))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "Group not found"}))))?;

    // 只有所有者或管理员可以移除成员
    if !user.is_admin && group.owner_id != Some(user.user_id) {
        return Err((StatusCode::FORBIDDEN, Json(serde_json::json!({"error": "Only the group owner or admin can manage members"}))));
    }

    let target_user = state.db.get_user_by_username(&req.username).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": format!("Database error: {}", e)}))))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "User not found"}))))?;

    state.db.remove_group_member(group.id, target_user.id).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": format!("Failed to remove member: {}", e)}))))?;

    Ok(Json(serde_json::json!({"success": true, "message": "Member removed"})))
}

pub async fn list_repo_members_handler(
    State(state): State<ApiState>,
    user: AuthUser,
    axum::extract::Query(query): axum::extract::Query<PathQuery>,
) -> Result<Json<Vec<MemberInfo>>, (StatusCode, Json<serde_json::Value>)> {
    let repo = state.db.get_repository_by_path(&query.path).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": format!("Database error: {}", e)}))))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "Repository not found"}))))?;

    // 权限检查
    let is_owner = repo.owner_id == Some(user.user_id);
    let is_admin = user.is_admin;
    let is_member = state.db.list_repository_members(repo.id).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": format!("Database error: {}", e)}))))?
        .iter().any(|m| m.user_id == user.user_id);

    if !is_owner && !is_admin && !is_member {
        return Err((StatusCode::FORBIDDEN, Json(serde_json::json!({"error": "Access denied"}))));
    }

    let members = state.db.list_repository_members(repo.id).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": format!("Database error: {}", e)}))))?;

    let mut result = Vec::new();
    for m in &members {
        let user_info = state.db.get_user_by_id(m.user_id).await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": format!("Database error: {}", e)}))))?;
        if let Some(u) = user_info {
            result.push(MemberInfo {
                user_id: m.user_id,
                username: u.username,
                role: m.role.clone(),
                created_at: m.created_at.clone(),
                can_view_subgroups: None,
            });
        }
    }

    Ok(Json(result))
}

pub async fn add_repo_member_handler(
    State(state): State<ApiState>,
    user: AuthUser,
    Json(req): Json<AddRepoMemberRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let repo = state.db.get_repository_by_path(&req.path).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": format!("Database error: {}", e)}))))?;

    let repo = if let Some(r) = repo {
        r
    } else {
        // 仓库在数据库中不存在，自动创建记录
        // 根据路径推断所有者：路径的第一部分是用户名
        let path_parts: Vec<&str> = req.path.split('/').collect();
        let owner_username = path_parts.first().copied().unwrap_or(&user.username);

        // 尝试查找对应的用户，如果不存在则使用当前用户
        let owner = state.db.get_user_by_username(owner_username).await
            .ok()
            .flatten()
            .unwrap_or_else(|| {
                // 如果路径第一部分不是有效用户，使用当前操作用户
                db::User {
                    id: user.user_id,
                    username: user.username.clone(),
                    password_hash: String::new(),
                    email: None,
                    is_admin: user.is_admin,
                    display_name: None,
                    avatar_url: None,
                    bio: None,
                    location: None,
                    website: None,
                    repos_folder: None,
                    created_at: String::new(),
                    updated_at: String::new(),
                }
            });

        let repo_name = req.path.split('/').last().unwrap_or(&req.path);
        let parent_path = if req.path.contains('/') {
            let parts: Vec<&str> = req.path.split('/').collect();
            parts[..parts.len()-1].join("/")
        } else {
            owner.username.clone()
        };

        let group_id = state.db.ensure_group_path(&parent_path, owner.id).await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": format!("Failed to ensure parent group: {}", e)}))))?;

        let _repo_id = state.db.create_repository(repo_name, &req.path, group_id, None, owner.id).await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": format!("Failed to create repository record: {}", e)}))))?;

        state.db.get_repository_by_path(&req.path).await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": format!("Database error: {}", e)}))))?
            .ok_or_else(|| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "Failed to retrieve created repository"}))))?
    };

    // 只有所有者或管理员可以添加成员
    if !user.is_admin && repo.owner_id != Some(user.user_id) {
        return Err((StatusCode::FORBIDDEN, Json(serde_json::json!({"error": "Only the repository owner or admin can manage members"}))));
    }

    let target_user = state.db.get_user_by_username(&req.username).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": format!("Database error: {}", e)}))))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "User not found"}))))?;

    let role = req.role.as_deref().unwrap_or("reader");
    state.db.add_repository_member(repo.id, target_user.id, role).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": format!("Failed to add member: {}", e)}))))?;

    Ok(Json(serde_json::json!({"success": true, "message": "Member added"})))
}

pub async fn remove_repo_member_handler(
    State(state): State<ApiState>,
    user: AuthUser,
    Json(req): Json<RemoveRepoMemberRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let repo = state.db.get_repository_by_path(&req.path).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": format!("Database error: {}", e)}))))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "Repository not found"}))))?;

    // 只有所有者或管理员可以移除成员
    if !user.is_admin && repo.owner_id != Some(user.user_id) {
        return Err((StatusCode::FORBIDDEN, Json(serde_json::json!({"error": "Only the repository owner or admin can manage members"}))));
    }

    let target_user = state.db.get_user_by_username(&req.username).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": format!("Database error: {}", e)}))))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "User not found"}))))?;

    state.db.remove_repository_member(repo.id, target_user.id).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": format!("Failed to remove member: {}", e)}))))?;

    Ok(Json(serde_json::json!({"success": true, "message": "Member removed"})))
}

#[derive(Serialize, Deserialize)]
pub struct UpdateGroupMemberPermissionsRequest {
    pub path: String,
    pub username: String,
    pub can_view_subgroups: bool,
}

pub async fn update_group_member_permissions_handler(
    State(state): State<ApiState>,
    user: AuthUser,
    Json(req): Json<UpdateGroupMemberPermissionsRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let group = state.db.get_group_by_path(&req.path).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": format!("Database error: {}", e)}))))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "Group not found"}))))?;

    // 只有所有者或管理员可以更新成员权限
    if !user.is_admin && group.owner_id != Some(user.user_id) {
        return Err((StatusCode::FORBIDDEN, Json(serde_json::json!({"error": "Only the group owner or admin can manage members"}))));
    }

    let target_user = state.db.get_user_by_username(&req.username).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": format!("Database error: {}", e)}))))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "User not found"}))))?;

    state.db.update_group_member_permissions(group.id, target_user.id, req.can_view_subgroups).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": format!("Failed to update permissions: {}", e)}))))?;

    Ok(Json(serde_json::json!({"success": true, "message": "Permissions updated"})))
}
