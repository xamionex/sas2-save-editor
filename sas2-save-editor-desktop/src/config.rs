use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use directories::ProjectDirs;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    pub game_path: Option<PathBuf>,
}

impl AppConfig {
    pub fn load() -> Self {
        if let Some(proj_dirs) = ProjectDirs::from("com", "yourdomain", "SaS2SaveEditor") {
            let config_file = proj_dirs.config_dir().join("config.json");
            if let Ok(data) = fs::read_to_string(&config_file) {
                if let Ok(config) = serde_json::from_str(&data) {
                    return config;
                }
            }
        }
        Self::default()
    }

    pub fn save(&self) {
        if let Some(proj_dirs) = ProjectDirs::from("com", "yourdomain", "SaS2SaveEditor") {
            let config_dir = proj_dirs.config_dir();
            if let Err(e) = fs::create_dir_all(config_dir) {
                eprintln!("Failed to create config directory: {}", e);
                return;
            }
            let config_file = config_dir.join("config.json");
            if let Ok(data) = serde_json::to_string_pretty(self) {
                let _ = fs::write(config_file, data);
            }
        }
    }
}