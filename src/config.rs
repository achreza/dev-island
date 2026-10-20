use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct QuickLink {
    pub title: String,
    pub target: String,
    pub is_url: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Snippet {
    pub title: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppConfig {
    pub focus_duration_mins: u32,
    pub task_note: String,
    pub git_repo_path: Option<String>,
    pub quick_links: Vec<QuickLink>,
    pub snippets: Vec<Snippet>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            focus_duration_mins: 25,
            task_note: "Phase 2 Dev Island".to_string(),
            git_repo_path: Some(".".to_string()),
            quick_links: vec![
                QuickLink {
                    title: "Terminal".to_string(),
                    target: "wt.exe".to_string(),
                    is_url: false,
                },
                QuickLink {
                    title: "Folder".to_string(),
                    target: ".".to_string(),
                    is_url: false,
                },
                QuickLink {
                    title: "Rust Docs".to_string(),
                    target: "https://doc.rust-lang.org/std/".to_string(),
                    is_url: true,
                },
            ],
            snippets: vec![
                Snippet {
                    title: "test".to_string(),
                    content: "cargo test".to_string(),
                },
                Snippet {
                    title: "status".to_string(),
                    content: "git status -s".to_string(),
                },
                Snippet {
                    title: "release".to_string(),
                    content: "cargo build --release".to_string(),
                },
            ],
        }
    }
}

impl AppConfig {
    pub fn config_path() -> PathBuf {
        if let Ok(app_data) = std::env::var("APPDATA") {
            let dir = Path::new(&app_data).join("DynamicIslandDev");
            let _ = fs::create_dir_all(&dir);
            dir.join("config.toml")
        } else {
            PathBuf::from("config.toml")
        }
    }

    pub fn load_or_default() -> Self {
        let path = Self::config_path();
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(cfg) = toml::from_str::<Self>(&content) {
                    return cfg;
                } else {
                    eprintln!("Warning: Malformed config.toml at {:?}. Using defaults.", path);
                }
            }
        }

        let default_cfg = Self::default();
        let _ = default_cfg.save();
        default_cfg
    }

    pub fn save(&self) -> Result<(), std::io::Error> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let serialized = toml::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        fs::write(path, serialized)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_serialization() {
        let cfg = AppConfig::default();
        let serialized = toml::to_string(&cfg).expect("Serialization failed");
        let deserialized: AppConfig = toml::from_str(&serialized).expect("Deserialization failed");
        assert_eq!(cfg, deserialized);
    }

    #[test]
    fn test_malformed_config_fallback() {
        let malformed = "invalid_toml = [";
        let res = toml::from_str::<AppConfig>(malformed);
        assert!(res.is_err());
    }
}
