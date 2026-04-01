use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use directories::ProjectDirs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub game_path: Option<PathBuf>,
    pub item_icon_size: f32,
    pub item_font_size: f32,
    pub drag_value_sensitivity: f32,
    pub dummy_drag_value: f32,
}

pub fn default_item_icon_size() -> f32 { 52.0 }
pub fn default_item_font_size() -> f32 { 12.0 }
pub fn default_drag_sensitivity() -> f32 { 0.025 }

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            game_path: None,
            item_icon_size: default_item_icon_size(),
            item_font_size: default_item_font_size(),
            drag_value_sensitivity: default_drag_sensitivity(),
            dummy_drag_value: 0.0,
        }
    }
}

impl AppConfig {
    pub fn load() -> Self {
        if let Some(proj_dirs) = ProjectDirs::from("com", "amione", "SaS2SaveEditor") {
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
        if let Some(proj_dirs) = ProjectDirs::from("com", "amione", "SaS2SaveEditor") {
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