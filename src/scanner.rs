use std::path::{Path, PathBuf};
use std::fs;

pub struct RepoScanner {
    repos_path: PathBuf,
}

impl RepoScanner {
    pub fn new(repos_path: PathBuf) -> Self {
        Self { repos_path }
    }

    pub fn repos_path(&self) -> &Path {
        &self.repos_path
    }

    pub fn scan_directory(&self) -> Vec<RepoEntry> {
        let mut entries = Vec::new();
        self.scan_recursive(&self.repos_path, "", &mut entries);
        entries
    }

    fn scan_recursive(&self, dir: &Path, relative_path: &str, entries: &mut Vec<RepoEntry>) {
        if let Ok(read_dir) = fs::read_dir(dir) {
            for entry in read_dir.flatten() {
                let path = entry.path();
                let name = entry.file_name().to_string_lossy().to_string();

                // 忽略 .meta 文件夹
                if name == ".meta" {
                    continue;
                }

                let current_path = if relative_path.is_empty() {
                    name.clone()
                } else {
                    format!("{}/{}", relative_path, name)
                };

                if path.is_dir() {
                    if name.ends_with(".git") {
                        entries.push(RepoEntry {
                            name,
                            path: current_path.clone(),
                            is_repo: true,
                        });
                    } else {
                        entries.push(RepoEntry {
                            name: name.clone(),
                            path: current_path.clone(),
                            is_repo: false,
                        });
                        self.scan_recursive(&path, &current_path, entries);
                    }
                }
            }
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct RepoEntry {
    pub name: String,
    pub path: String,
    pub is_repo: bool,
}
