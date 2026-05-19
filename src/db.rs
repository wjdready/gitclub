use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};
use std::path::Path;

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
        let result = sqlx::query(
            "INSERT INTO users (username, password_hash, email, is_admin) VALUES (?, ?, ?, ?)"
        )
        .bind(username)
        .bind(password_hash)
        .bind(email)
        .bind(is_admin)
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

    pub async fn create_group(&self, name: &str, path: &str, parent_id: Option<i64>, description: Option<&str>) -> Result<i64, sqlx::Error> {
        let result = sqlx::query(
            "INSERT INTO groups (name, path, parent_id, description) VALUES (?, ?, ?, ?)"
        )
        .bind(name)
        .bind(path)
        .bind(parent_id)
        .bind(description)
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

    pub async fn list_groups(&self, parent_id: Option<i64>) -> Result<Vec<Group>, sqlx::Error> {
        sqlx::query_as::<_, Group>("SELECT * FROM groups WHERE parent_id IS ? ORDER BY name")
            .bind(parent_id)
            .fetch_all(&self.pool)
            .await
    }

    pub async fn create_repository(&self, name: &str, path: &str, group_id: i64, description: Option<&str>) -> Result<i64, sqlx::Error> {
        let result = sqlx::query(
            "INSERT INTO repositories (name, path, group_id, description) VALUES (?, ?, ?, ?)"
        )
        .bind(name)
        .bind(path)
        .bind(group_id)
        .bind(description)
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
}
