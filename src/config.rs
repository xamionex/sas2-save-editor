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

    /// How item upgrade levels are shown on grid icons: off, digits, or roman numerals.
    #[serde(default)]
    pub upgrade_style: crate::tabs::multisel::UpgradeStyle,

    /// How artifact seeds are shown on artifact icons: off or digits.
    #[serde(default)]
    pub artifact_seed_style: crate::tabs::multisel::UpgradeStyle,

    #[serde(default)]
    pub equipment_panel_width: f32,

    #[serde(default)]
    pub add_items_panel_width: f32,

    #[serde(default)]
    pub skilltree_panel_width: f32,

    #[serde(default)]
    pub bestiary_panel_width: f32,

    /// Last width of the artifacts tab's left artifact list panel.
    #[serde(default)]
    pub artifact_list_width: f32,

    /// Last width of the artifacts tab's right editor sidebar.
    #[serde(default)]
    pub artifact_sidebar_width: f32,

    /// Remember and restore the window position on startup.
    #[serde(default = "default_true")]
    pub save_window_position: bool,

    /// On Wayland, run through XWayland so the window position can be saved.
    /// Native Wayland does not let clients read their own position, this only matters on Wayland sessions and is off by default.
    #[serde(default)]
    pub force_x11_for_position: bool,

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

    /// Remember the artifact search settings (tier scope, sort key, sort direction) across restarts.
    #[serde(default)]
    pub remember_artifact_search: bool,

    /// Last artifact search tier scope.
    #[serde(default)]
    pub artifact_search_scope: crate::artifact::SearchTierScope,

    /// Last artifact result sort key.
    #[serde(default)]
    pub artifact_result_sort_key: crate::artifact::ResultSortKey,

    /// Last artifact result sort direction (true = descending).
    #[serde(default)]
    pub artifact_result_sort_desc: bool,

    /// Last artifact result secondary sort key.
    #[serde(default)]
    pub artifact_result_sub_sort_key: crate::artifact::ResultSortKey,

    /// Last artifact result secondary sort direction (true = descending).
    #[serde(default)]
    pub artifact_result_sub_sort_desc: bool,

    /// When true, the secondary artifact result sort is applied after the primary sort.
    #[serde(default)]
    pub artifact_use_sub_sort: bool,

    /// Last artifact result grouping (none, by tier, or by a field id).
    #[serde(default)]
    pub artifact_result_group_by: crate::artifact::ResultGroupBy,

    /// Always load all artifact search results, bypassing the load-more cap.
    #[serde(default)]
    pub always_load_all_results: bool,
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
            upgrade_style: crate::tabs::multisel::UpgradeStyle::Digits,
            artifact_seed_style: crate::tabs::multisel::UpgradeStyle::Digits,
            equipment_panel_width: 0.0,
            add_items_panel_width: 0.0,
            skilltree_panel_width: 0.0,
            bestiary_panel_width: 0.0,
            artifact_list_width: 0.0,
            artifact_sidebar_width: 0.0,
            save_window_position: true,
            force_x11_for_position: false,
            save_window_state: true,
            window_pos: None,
            window_size: None,
            window_maximized: false,
            remember_artifact_search: false,
            artifact_search_scope: crate::artifact::SearchTierScope::StaticTier,
            artifact_result_sort_key: crate::artifact::ResultSortKey::Closeness,
            artifact_result_sort_desc: false,
            artifact_result_sub_sort_key: crate::artifact::ResultSortKey::Closeness,
            artifact_result_sub_sort_desc: false,
            artifact_use_sub_sort: false,
            artifact_result_group_by: crate::artifact::ResultGroupBy::None,
            always_load_all_results: false,
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
