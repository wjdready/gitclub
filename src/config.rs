use ini::Ini;
use std::path::{Path, PathBuf};
use std::fs;

#[derive(Debug, Clone)]
pub struct Config {
    pub server_addr: String,
    pub server_port: u16,
    pub data_path: PathBuf,
    pub log_path: PathBuf,
    pub repos_path: PathBuf,
}

impl Config {
    pub fn load() -> Self {
        let exe_dir = get_exe_dir();
        let config_path = exe_dir.join("custom/conf/app.ini");

        if config_path.exists() {
            Self::from_file(&config_path, &exe_dir)
        } else {
            let config = Self::default_config(&exe_dir);
            config.save(&config_path);
            config
        }
    }

    fn from_file(path: &Path, exe_dir: &Path) -> Self {
        let ini_data = Ini::load_from_file(path).unwrap_or_else(|_| Ini::new());

        let server_addr = ini_data
            .get_from(Some("server"), "addr")
            .unwrap_or("127.0.0.1")
            .to_string();

        let server_port: u16 = ini_data
            .get_from(Some("server"), "port")
            .and_then(|s: &str| s.parse().ok())
            .unwrap_or(8080);

        let data_path = ini_data
            .get_from(Some("paths"), "data")
            .map(PathBuf::from)
            .unwrap_or_else(|| exe_dir.join("data"));

        let log_path = ini_data
            .get_from(Some("paths"), "log")
            .map(PathBuf::from)
            .unwrap_or_else(|| exe_dir.join("log"));

        let repos_path = ini_data
            .get_from(Some("paths"), "repos")
            .map(PathBuf::from)
            .unwrap_or_else(|| exe_dir.join("repos"));

        Self {
            server_addr,
            server_port,
            data_path,
            log_path,
            repos_path,
        }
    }

    fn default_config(exe_dir: &Path) -> Self {
        Self {
            server_addr: "127.0.0.1".to_string(),
            server_port: 8080,
            data_path: exe_dir.join("data"),
            log_path: exe_dir.join("log"),
            repos_path: exe_dir.join("repos"),
        }
    }

    fn save(&self, path: &Path) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).ok();
        }

        let mut ini_data = Ini::new();
        ini_data.with_section(Some("server"))
            .set("addr", &self.server_addr)
            .set("port", &self.server_port.to_string());

        ini_data.with_section(Some("paths"))
            .set("data", self.data_path.to_string_lossy().as_ref())
            .set("log", self.log_path.to_string_lossy().as_ref())
            .set("repos", self.repos_path.to_string_lossy().as_ref());

        ini_data.write_to_file(path).ok();
    }

    pub fn ensure_dirs(&self) {
        fs::create_dir_all(&self.data_path).ok();
        fs::create_dir_all(&self.log_path).ok();
        fs::create_dir_all(&self.repos_path).ok();
    }
}

fn get_exe_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
}
