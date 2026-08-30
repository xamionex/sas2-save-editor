use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveEditorConfig {
    #[serde(default)]
    pub game_path: Option<PathBuf>,

    #[serde(default = "default_item_icon_size")]
    pub item_icon_size: f32,

    /// Font size of item/monster names in the item grids.
    #[serde(default = "default_grid_font_size")]
    pub grid_font_size: f32,

    /// Font size of the editor sidebars (item/bestiary detail panels).
    #[serde(default = "default_sidebar_font_size")]
    pub sidebar_font_size: f32,

    /// Font size of the top tab bar.
    #[serde(default = "default_tabs_font_size")]
    pub tabs_font_size: f32,

    /// Font size of the grid category headers (e.g. "Weapon - Greatsword").
    #[serde(default = "default_category_font_size")]
    pub category_font_size: f32,

    #[serde(default = "default_drag_sensitivity")]
    pub drag_value_sensitivity: f32,

    #[serde(default)]
    pub dummy_drag_value: f32,

    #[serde(default)]
    pub adjust_black_pearls_on_level_change: bool,

    #[serde(default)]
    pub sync_black_starstones: bool,

    #[serde(default)]
    pub add_gray_starstones: bool,

    #[serde(default)]
    pub remove_gray_starstones: bool,

    #[serde(default)]
    pub account_for_level: bool,

    #[serde(default)]
    pub account_for_starstones: bool,

    #[serde(default)]
    pub equipment_panel_width: f32,

    #[serde(default)]
    pub skilltree_panel_width: f32,

    #[serde(default)]
    pub bestiary_panel_width: f32,

    /// Remember and restore the window position on startup.
    #[serde(default = "default_true")]
    pub save_window_position: bool,

    /// Remember and restore the window state (maximized) on startup.
    #[serde(default = "default_true")]
    pub save_window_state: bool,

    /// Last window position (outer position of the root viewport).
    #[serde(default)]
    pub window_pos: Option<[f32; 2]>,

    /// Last window inner size.
    #[serde(default)]
    pub window_size: Option<[f32; 2]>,

    /// Last window maximized state.
    #[serde(default)]
    pub window_maximized: bool,
}

pub fn default_true() -> bool {
    true
}

pub fn default_item_icon_size() -> f32 {
    52.0
}
pub fn default_grid_font_size() -> f32 {
    12.0
}
pub fn default_sidebar_font_size() -> f32 {
    14.0
}
pub fn default_tabs_font_size() -> f32 {
    14.0
}
pub fn default_category_font_size() -> f32 {
    13.0
}
pub fn default_drag_sensitivity() -> f32 {
    0.025
}

impl Default for SaveEditorConfig {
    fn default() -> Self {
        Self {
            game_path: None,
            item_icon_size: default_item_icon_size(),
            grid_font_size: default_grid_font_size(),
            sidebar_font_size: default_sidebar_font_size(),
            tabs_font_size: default_tabs_font_size(),
            category_font_size: default_category_font_size(),
            drag_value_sensitivity: default_drag_sensitivity(),
            dummy_drag_value: 0.0,
            adjust_black_pearls_on_level_change: false,
            sync_black_starstones: false,
            add_gray_starstones: false,
            remove_gray_starstones: false,
            account_for_level: false,
            account_for_starstones: false,
            equipment_panel_width: 0.0,
            skilltree_panel_width: 0.0,
            bestiary_panel_width: 0.0,
            save_window_position: true,
            save_window_state: true,
            window_pos: None,
            window_size: None,
            window_maximized: false,
        }
    }
}

impl SaveEditorConfig {
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
