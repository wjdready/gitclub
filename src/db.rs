use sqlx::{SqlitePool, sqlite::SqlitePoolOptions, Row};
use std::path::Path;

#[derive(Debug)]
pub struct Database {
    pool: SqlitePool,
}

impl Database {
    pub async fn new(db_path: &Path) -> Result<Self, sqlx::Error> {
        // 确保数据库文件的父目录存在
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }

        let db_url = format!("sqlite:{}?mode=rwc", db_path.display());

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(&db_url)
            .await?;

        let db = Self { pool };
        db.init().await?;
        Ok(db)
    }

    async fn init(&self) -> Result<(), sqlx::Error> {
        let schema = include_str!("../migrations/001_init.sql");
        sqlx::raw_sql(schema).execute(&self.pool).await?;

        // 运行第二个 migration（如果还没运行）
        // 检查列是否已存在
        let check = sqlx::query("PRAGMA table_info(users)")
            .fetch_all(&self.pool)
            .await?;

        let column_names: Vec<String> = check.iter()
            .filter_map(|row| row.try_get("name").ok())
            .collect();

        let columns_to_add = vec![
            "display_name",
            "avatar_url",
            "bio",
            "location",
            "website",
            "repos_folder",
        ];

        for column in columns_to_add {
            if !column_names.contains(&column.to_string()) {
                tracing::info!("Adding column {} to users table", column);
                // SQLite 不允许在 ALTER TABLE ADD COLUMN 时添加 UNIQUE 约束
                let sql = format!("ALTER TABLE users ADD COLUMN {} TEXT", column);
                sqlx::raw_sql(&sql).execute(&self.pool).await?;
            }
        }

        // 创建索引（repos_folder 使用唯一索引来保证唯一性）
        sqlx::raw_sql("CREATE INDEX IF NOT EXISTS idx_users_username ON users(username)")
            .execute(&self.pool).await?;
        sqlx::raw_sql("CREATE INDEX IF NOT EXISTS idx_users_email ON users(email)")
            .execute(&self.pool).await?;
        sqlx::raw_sql("CREATE UNIQUE INDEX IF NOT EXISTS idx_users_repos_folder ON users(repos_folder)")
            .execute(&self.pool).await?;

        // 运行第三个 migration：添加 owner_id 字段到 groups 和 repositories
        for table in &["groups", "repositories"] {
            let cols = sqlx::query(&format!("PRAGMA table_info({})", table))
                .fetch_all(&self.pool)
                .await?;
            let has_owner = cols.iter().any(|row| row.try_get::<String, _>("name").ok().as_deref() == Some("owner_id"));
            if !has_owner {
                tracing::info!("Adding owner_id column to {} table", table);
                let sql = format!("ALTER TABLE {} ADD COLUMN owner_id INTEGER", table);
                sqlx::raw_sql(&sql).execute(&self.pool).await?;
            }
        }
        sqlx::raw_sql("CREATE INDEX IF NOT EXISTS idx_groups_owner ON groups(owner_id)")
            .execute(&self.pool).await?;
        sqlx::raw_sql("CREATE INDEX IF NOT EXISTS idx_repositories_owner ON repositories(owner_id)")
            .execute(&self.pool).await?;

        Ok(())
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct User {
    pub id: i64,
    pub username: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub email: Option<String>,
    pub is_admin: bool,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub bio: Option<String>,
    pub location: Option<String>,
    pub website: Option<String>,
    pub repos_folder: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct Group {
    pub id: i64,
    pub name: String,
    pub path: String,
    pub parent_id: Option<i64>,
    pub description: Option<String>,
    pub owner_id: Option<i64>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct Repository {
    pub id: i64,
    pub name: String,
    pub path: String,
    pub group_id: i64,
    pub description: Option<String>,
    pub owner_id: Option<i64>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct GroupMember {
    pub id: i64,
    pub group_id: i64,
    pub user_id: i64,
    pub role: String,
    pub can_view_subgroups: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize, serde::Deserialize)]
pub struct RepositoryMember {
    pub id: i64,
    pub repository_id: i64,
    pub user_id: i64,
    pub role: String,
    pub created_at: String,
}

impl Database {
    pub async fn create_user(&self, username: &str, password_hash: &str, email: Option<&str>, is_admin: bool) -> Result<i64, sqlx::Error> {
        // 生成用户的 repos 文件夹路径
        let repos_folder = username.to_string();

        let result = sqlx::query(
            "INSERT INTO users (username, password_hash, email, is_admin, repos_folder) VALUES (?, ?, ?, ?, ?)"
        )
        .bind(username)
        .bind(password_hash)
        .bind(email)
        .bind(is_admin)
        .bind(&repos_folder)
        .execute(&self.pool)
        .await?;
        Ok(result.last_insert_rowid())
    }

    pub async fn get_user_by_username(&self, username: &str) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as::<_, User>("SELECT * FROM users WHERE username = ?")
            .bind(username)
            .fetch_optional(&self.pool)
            .await
    }

    pub async fn get_user_by_id(&self, user_id: i64) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = ?")
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await
    }

    pub async fn update_user_profile(
        &self,
        user_id: i64,
        display_name: Option<&str>,
        bio: Option<&str>,
        location: Option<&str>,
        website: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE users SET display_name = ?, bio = ?, location = ?, website = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?"
        )
        .bind(display_name)
        .bind(bio)
        .bind(location)
        .bind(website)
        .bind(user_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn update_user_avatar(&self, user_id: i64, avatar_url: &str) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE users SET avatar_url = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
            .bind(avatar_url)
            .bind(user_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn create_group(&self, name: &str, path: &str, parent_id: Option<i64>, description: Option<&str>, owner_id: i64) -> Result<i64, sqlx::Error> {
        let result = sqlx::query(
            "INSERT INTO groups (name, path, parent_id, description, owner_id) VALUES (?, ?, ?, ?, ?)"
        )
        .bind(name)
        .bind(path)
        .bind(parent_id)
        .bind(description)
        .bind(owner_id)
        .execute(&self.pool)
        .await?;
        Ok(result.last_insert_rowid())
    }

    pub async fn get_group_by_path(&self, path: &str) -> Result<Option<Group>, sqlx::Error> {
        sqlx::query_as::<_, Group>("SELECT * FROM groups WHERE path = ?")
            .bind(path)
            .fetch_optional(&self.pool)
            .await
    }

    pub async fn get_group_by_id(&self, group_id: i64) -> Result<Option<Group>, sqlx::Error> {
        sqlx::query_as::<_, Group>("SELECT * FROM groups WHERE id = ?")
            .bind(group_id)
            .fetch_optional(&self.pool)
            .await
    }

    /// 确保路径上的所有组都存在于数据库中，返回最深层组的 ID
    pub async fn ensure_group_path(&self, path: &str, owner_id: i64) -> Result<i64, sqlx::Error> {
        let parts: Vec<&str> = path.split('/').collect();
        let mut parent_id: Option<i64> = None;

        for (i, part) in parts.iter().enumerate() {
            let current_path = parts[..=i].join("/");

            let existing = self.get_group_by_path(&current_path).await?;
            if let Some(group) = existing {
                parent_id = Some(group.id);
            } else {
                let desc: Option<&str> = None;
                let result = sqlx::query(
                    "INSERT INTO groups (name, path, parent_id, description, owner_id) VALUES (?, ?, ?, ?, ?)"
                )
                .bind(part)
                .bind(&current_path)
                .bind(parent_id)
                .bind(desc)
                .bind(owner_id)
                .execute(&self.pool)
                .await?;
                parent_id = Some(result.last_insert_rowid());
            }
        }

        Ok(parent_id.unwrap_or(0))
    }

    pub async fn list_groups(&self, parent_id: Option<i64>) -> Result<Vec<Group>, sqlx::Error> {
        sqlx::query_as::<_, Group>("SELECT * FROM groups WHERE parent_id IS ? ORDER BY name")
            .bind(parent_id)
            .fetch_all(&self.pool)
            .await
    }

    pub async fn create_repository(&self, name: &str, path: &str, group_id: i64, description: Option<&str>, owner_id: i64) -> Result<i64, sqlx::Error> {
        let result = sqlx::query(
            "INSERT INTO repositories (name, path, group_id, description, owner_id) VALUES (?, ?, ?, ?, ?)"
        )
        .bind(name)
        .bind(path)
        .bind(group_id)
        .bind(description)
        .bind(owner_id)
        .execute(&self.pool)
        .await?;
        Ok(result.last_insert_rowid())
    }

    pub async fn get_repository_by_path(&self, path: &str) -> Result<Option<Repository>, sqlx::Error> {
        sqlx::query_as::<_, Repository>("SELECT * FROM repositories WHERE path = ?")
            .bind(path)
            .fetch_optional(&self.pool)
            .await
    }

    pub async fn list_repositories(&self, group_id: i64) -> Result<Vec<Repository>, sqlx::Error> {
        sqlx::query_as::<_, Repository>("SELECT * FROM repositories WHERE group_id = ? ORDER BY name")
            .bind(group_id)
            .fetch_all(&self.pool)
            .await
    }

    pub async fn add_group_member(&self, group_id: i64, user_id: i64, role: &str, can_view_subgroups: bool) -> Result<i64, sqlx::Error> {
        let result = sqlx::query(
            "INSERT INTO group_members (group_id, user_id, role, can_view_subgroups) VALUES (?, ?, ?, ?)"
        )
        .bind(group_id)
        .bind(user_id)
        .bind(role)
        .bind(can_view_subgroups)
        .execute(&self.pool)
        .await?;
        Ok(result.last_insert_rowid())
    }

    pub async fn add_repository_member(&self, repository_id: i64, user_id: i64, role: &str) -> Result<i64, sqlx::Error> {
        let result = sqlx::query(
            "INSERT INTO repository_members (repository_id, user_id, role) VALUES (?, ?, ?)"
        )
        .bind(repository_id)
        .bind(user_id)
        .bind(role)
        .execute(&self.pool)
        .await?;
        Ok(result.last_insert_rowid())
    }

    pub async fn list_group_members(&self, group_id: i64) -> Result<Vec<GroupMember>, sqlx::Error> {
        sqlx::query_as::<_, GroupMember>("SELECT * FROM group_members WHERE group_id = ? ORDER BY created_at")
            .bind(group_id)
            .fetch_all(&self.pool)
            .await
    }

    pub async fn list_repository_members(&self, repository_id: i64) -> Result<Vec<RepositoryMember>, sqlx::Error> {
        sqlx::query_as::<_, RepositoryMember>("SELECT * FROM repository_members WHERE repository_id = ? ORDER BY created_at")
            .bind(repository_id)
            .fetch_all(&self.pool)
            .await
    }

    pub async fn remove_group_member(&self, group_id: i64, user_id: i64) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM group_members WHERE group_id = ? AND user_id = ?")
            .bind(group_id)
            .bind(user_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn update_group_member_permissions(&self, group_id: i64, user_id: i64, can_view_subgroups: bool) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE group_members SET can_view_subgroups = ? WHERE group_id = ? AND user_id = ?")
            .bind(can_view_subgroups)
            .bind(group_id)
            .bind(user_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn remove_repository_member(&self, repository_id: i64, user_id: i64) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM repository_members WHERE repository_id = ? AND user_id = ?")
            .bind(repository_id)
            .bind(user_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// 获取用户有权限访问的所有组路径（作为所有者或成员）
    pub async fn get_accessible_group_paths(&self, user_id: i64) -> Result<Vec<String>, sqlx::Error> {
        // 作为所有者的组（所有者可以看到所有子组）
        let owned = sqlx::query_scalar::<_, String>(
            "SELECT path FROM groups WHERE owner_id = ?"
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        // 作为成员的组（通过 group_members 表）
        let member_groups = sqlx::query_as::<_, (String, bool)>(
            "SELECT g.path, gm.can_view_subgroups FROM groups g INNER JOIN group_members gm ON g.id = gm.group_id WHERE gm.user_id = ?"
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        let mut paths = owned.clone();

        // 对于所有者的组，添加所有子组
        for owner_path in &owned {
            let subgroups = sqlx::query_scalar::<_, String>(
                "SELECT path FROM groups WHERE path LIKE ?"
            )
            .bind(format!("{}/%", owner_path))
            .fetch_all(&self.pool)
            .await?;
            paths.extend(subgroups);
        }

        // 对于成员的组，根据 can_view_subgroups 决定是否包含子组
        for (group_path, can_view_subgroups) in member_groups {
            paths.push(group_path.clone());

            if can_view_subgroups {
                // 如果可以查看子组，添加所有子组
                let subgroups = sqlx::query_scalar::<_, String>(
                    "SELECT path FROM groups WHERE path LIKE ?"
                )
                .bind(format!("{}/%", group_path))
                .fetch_all(&self.pool)
                .await?;
                paths.extend(subgroups);
            }
        }

        paths.sort();
        paths.dedup();
        Ok(paths)
    }

    /// 获取用户有权限访问的所有仓库路径（作为所有者或成员）
    pub async fn get_accessible_repo_paths(&self, user_id: i64) -> Result<Vec<String>, sqlx::Error> {
        // 作为所有者的仓库
        let owned = sqlx::query_scalar::<_, String>(
            "SELECT path FROM repositories WHERE owner_id = ?"
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        // 作为成员的仓库（通过 repository_members 表）
        let member_of = sqlx::query_scalar::<_, String>(
            "SELECT r.path FROM repositories r INNER JOIN repository_members rm ON r.id = rm.repository_id WHERE rm.user_id = ?"
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        // 通过组成员关系可以访问的仓库
        // 1. 获取用户是成员的组
        let member_groups = sqlx::query_as::<_, (i64, String, bool)>(
            "SELECT g.id, g.path, gm.can_view_subgroups FROM groups g INNER JOIN group_members gm ON g.id = gm.group_id WHERE gm.user_id = ?"
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        let mut group_repos = Vec::new();
        for (group_id, group_path, can_view_subgroups) in member_groups {
            // 获取该组直接包含的仓库
            let direct_repos = sqlx::query_scalar::<_, String>(
                "SELECT path FROM repositories WHERE group_id = ?"
            )
            .bind(group_id)
            .fetch_all(&self.pool)
            .await?;
            group_repos.extend(direct_repos);

            // 如果可以查看子组，获取所有子组的仓库
            if can_view_subgroups {
                let subgroup_repos = sqlx::query_scalar::<_, String>(
                    "SELECT r.path FROM repositories r INNER JOIN groups g ON r.group_id = g.id WHERE g.path LIKE ?"
                )
                .bind(format!("{}/%", group_path))
                .fetch_all(&self.pool)
                .await?;
                group_repos.extend(subgroup_repos);
            }
        }

        let mut paths = owned;
        paths.extend(member_of);
        paths.extend(group_repos);
        paths.sort();
        paths.dedup();
        Ok(paths)
    }

    /// 检查用户是否有权限访问某条路径（通过组或仓库的成员关系）
    pub async fn user_can_access_path(&self, user_id: i64, path: &str) -> Result<bool, sqlx::Error> {
        // 检查是否是组的成员或所有者
        if let Some(group) = self.get_group_by_path(path).await? {
            if group.owner_id == Some(user_id) {
                return Ok(true);
            }
            if self.list_group_members(group.id).await?.iter().any(|m| m.user_id == user_id) {
                return Ok(true);
            }
        }

        // 检查是否是仓库的成员或所有者
        if let Some(repo) = self.get_repository_by_path(path).await? {
            if repo.owner_id == Some(user_id) {
                return Ok(true);
            }
            if self.list_repository_members(repo.id).await?.iter().any(|m| m.user_id == user_id) {
                return Ok(true);
            }
        }

        // 检查是否是某个祖先组的成员
        let parts: Vec<&str> = path.split('/').collect();
        for i in (1..parts.len()).rev() {
            let parent_path = parts[..i].join("/");
            if let Some(group) = self.get_group_by_path(&parent_path).await? {
                if self.list_group_members(group.id).await?.iter().any(|m| m.user_id == user_id) {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    /// 根据路径推断所有者用户 ID（路径的第一部分是用户名）
    pub async fn infer_owner_from_path(&self, path: &str) -> Option<i64> {
        let username = path.split('/').next()?;
        let user = self.get_user_by_username(username).await.ok()??;
        Some(user.id)
    }

    /// 确保组在数据库中存在，如果不存在则自动创建并设置 owner
    pub async fn ensure_group_exists(&self, path: &str) -> Result<Group, sqlx::Error> {
        // 先尝试获取
        if let Some(group) = self.get_group_by_path(path).await? {
            return Ok(group);
        }

        // 不存在，根据路径推断所有者并创建
        let owner_id = self.infer_owner_from_path(path).await.unwrap_or(1); // 默认使用 admin (id=1)

        let group_name = path.split('/').last().unwrap_or(path);
        let parent_path = if path.contains('/') {
            let parts: Vec<&str> = path.split('/').collect();
            Some(parts[..parts.len()-1].join("/"))
        } else {
            None
        };

        let parent_id = if let Some(ref parent) = parent_path {
            self.ensure_group_path(parent, owner_id).await.ok()
        } else {
            None
        };

        let group_id = self.create_group(group_name, path, parent_id, None, owner_id).await?;
        self.get_group_by_id(group_id).await?
            .ok_or_else(|| sqlx::Error::RowNotFound)
    }

    /// 确保仓库在数据库中存在，如果不存在则自动创建并设置 owner
    pub async fn ensure_repository_exists(&self, path: &str) -> Result<Repository, sqlx::Error> {
        // 先尝试获取
        if let Some(repo) = self.get_repository_by_path(path).await? {
            return Ok(repo);
        }

        // 不存在，根据路径推断所有者并创建
        let owner_id = self.infer_owner_from_path(path).await.unwrap_or(1); // 默认使用 admin (id=1)

        let repo_name = path.split('/').last().unwrap_or(path);
        let parent_path = if path.contains('/') {
            let parts: Vec<&str> = path.split('/').collect();
            parts[..parts.len()-1].join("/")
        } else {
            // 仓库在根目录，使用用户名作为父路径
            path.split('/').next().unwrap_or("admin").to_string()
        };

        let group_id = self.ensure_group_path(&parent_path, owner_id).await?;
        let repo_id = self.create_repository(repo_name, path, group_id, None, owner_id).await?;

        self.get_repository_by_path(path).await?
            .ok_or_else(|| sqlx::Error::RowNotFound)
    }
}
