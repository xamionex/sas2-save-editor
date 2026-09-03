use crate::atlas::{ItemAtlas, MonsterTextureCache};
use crate::catalog::{
    load_loot_catalog, load_monster_catalog, load_skilltree_catalog, load_skilltree_texture,
};
use crate::config::{
    SaveEditorConfig, default_category_font_size, default_drag_sensitivity, default_grid_font_size,
    default_item_icon_size, default_sidebar_font_size, default_tabs_font_size,
};
use crate::export::{
    ExportState, XnbNode, build_xnb_tree, show_export_picker, show_export_progress,
    start_export_job,
};
use crate::tabs::{EquipmentSubTab, Tab};
use eframe::{Frame, egui};
use egui::{Rect, TextureHandle, Ui};
use rfd::FileDialog;
use sas2_parser::SaveData;
use sas2_parser::loot_catalog::LootCatalog;
use sas2_parser::monster_catalog::MonsterCatalog;
use sas2_parser::skilltree::SkillTreeCatalog;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;

pub struct SaveEditor {
    pub load_requested: bool,
    pub save_data: Option<SaveData>,
    pub file_path: Option<PathBuf>,
    pub error_message: Option<String>,
    pub active_tab: Tab,

    pub config: SaveEditorConfig,

    // Loot / monster / skill catalogs and their respective load errors
    pub catalog: Option<LootCatalog>,
    pub catalog_error: Option<String>,
    pub monster_catalog: Option<MonsterCatalog>,
    pub monster_catalog_error: Option<String>,
    pub monster_texture_cache: MonsterTextureCache,
    pub bestiary_search_filter: String,
    pub selected_bestiary_beast: Option<usize>,
    /// Multi-selected beast indices (right-click toggles).
    pub selected_bestiary_beasts: std::collections::HashSet<usize>,
    /// Right-click gesture state for the bestiary grid.
    pub bestiary_grid_sel: crate::tabs::multisel::GridSel<usize>,
    pub skilltree_catalog: Option<SkillTreeCatalog>,
    pub skilltree_catalog_error: Option<String>,

    // Globally loaded item icon atlas (items.xnb), loaded lazily when the equipment tab is first opened.
    pub item_atlas: Option<ItemAtlas>,

    // Skill tree rendering
    pub skilltree_texture: Option<TextureHandle>,
    pub skilltree_texture_error: Option<String>,
    pub skilltree_zoom: f32,
    pub skilltree_scroll: egui::Vec2,
    pub selected_skill_node: Option<usize>,
    /// Multi-selected skill nodes (ctrl+click toggles, shift+click ranges).
    pub selected_skill_nodes: std::collections::HashSet<usize>,
    /// Selection gesture state for the skill tree.
    pub skilltree_grid_sel: crate::tabs::multisel::GridSel<usize>,
    pub skilltree_centered: bool,

    // Whether the skill stats need to be recomputed from the tree
    pub stats_dirty: bool,

    // Equipment tab state
    pub item_search_filter: String,
    pub equipment_subtab: EquipmentSubTab,
    pub selected_equipment_item: Option<usize>,
    /// Multi-selected inventory indices (right-click toggles). Empty when only the single selection is active.
    pub selected_equipment_items: std::collections::HashSet<usize>,
    /// Right-click gesture state for the inventory/stockpile grid.
    pub equipment_grid_sel: crate::tabs::multisel::GridSel<usize>,
    /// When true, the "Remove all by type" picker window is open.
    pub equipment_remove_all_open: bool,
    /// Item type-subtype categories checked in the remove-all picker.
    pub equipment_remove_all_types: std::collections::HashSet<String>,
    pub selected_catalog_item: Option<usize>,
    /// Multi-selected catalog items in the Add Items grid.
    pub selected_catalog_items: std::collections::HashSet<usize>,
    /// Selection gesture state for the Add Items grid.
    pub add_items_grid_sel: crate::tabs::multisel::GridSel<usize>,
    pub add_item_count: i32,
    pub add_item_upgrade: i32,

    // Artifacts tab state
    pub selected_artifact: Option<usize>,
    /// Desired value per artifact field, for the "must match" panel (field id -> desired %).
    pub artifact_desired_values: std::collections::HashMap<i32, f32>,
    /// Desired value per artifact field, for the "can-match" panel (field id -> desired %).
    pub artifact_can_values: std::collections::HashMap<i32, f32>,
    /// Exact matches of the current live search, sorted best first.
    pub artifact_exact_matches: Vec<crate::artifact::ArtifactMatch>,
    /// Partial matches of the current live search, sorted best first.
    pub artifact_partial_matches: Vec<crate::artifact::ArtifactMatch>,
    /// Search text that filters both match lists by seed.
    pub artifact_match_search: String,
    /// When true, the merged result lists also show partial matches.
    pub artifact_show_partial: bool,
    /// When true, the result list shows all artifact stat columns instead of only the filtered ones.
    pub artifact_show_all_stats: bool,
    /// Sort key of the merged result lists (closeness, tier or a field id).
    pub artifact_result_sort_key: crate::artifact::ResultSortKey,
    /// Sort direction of the merged result lists.
    pub artifact_result_sort_desc: bool,
    /// Secondary sort key of the merged result lists (closeness, tier or a field id).
    pub artifact_result_sub_sort_key: crate::artifact::ResultSortKey,
    /// Secondary sort direction of the merged result lists.
    pub artifact_result_sub_sort_desc: bool,
    /// When true, the secondary sort is applied after the primary sort.
    pub artifact_use_sub_sort: bool,
    /// Grouping of the merged result lists (none, by tier, or by a field id).
    pub artifact_result_group_by: crate::artifact::ResultGroupBy,
    /// Group keys (e.g. "Tier 12") that are collapsed in the grouped result list.
    pub artifact_collapsed_groups: std::collections::HashSet<String>,
    /// Tier scope of the live search.
    pub artifact_search_scope: crate::artifact::SearchTierScope,
    /// Minimum tier used when the search scope is MinMax.
    pub artifact_min_tier: i32,
    /// Maximum tier used when the search scope is MinMax.
    pub artifact_max_tier: i32,
    /// User-raised result cap for the "load more" button; None = the default cap.
    pub artifact_result_limit: Option<usize>,
    /// Pending seed to apply to the selected artifact, processed next frame.
    pub artifact_pending_apply: Option<i32>,
    /// Pending seed to add as a new artifact item (Add to Inventory), processed next frame.
    pub artifact_pending_add: Option<i32>,
    /// Cached Resalter artifact_boosts.json contents, keyed by game path.
    pub resalter_boosts_cache: Option<(
        std::path::PathBuf,
        std::collections::HashMap<i32, crate::artifact::ArtifactBoostOverride>,
    )>,

    // XNB exporter
    pub export_tree_loading: bool,
    pub export_tree_receiver: Option<std::sync::mpsc::Receiver<Option<XnbNode>>>,
    pub export_picker: Option<XnbNode>,
    pub export_picker_open: bool,
    pub export_state: Option<ExportState>,
    pub export_overwrite: bool,

    // Settings window
    pub settings_open: bool,

    // Modded -> vanilla conversion
    pub conversion_target_version: i32,
    pub conversion_just_happened: bool,

    // MD5 hash override
    pub hash_edit_string: String,
    pub use_custom_hash: bool,

    // Sidebars timer and previous size
    pub config_save_timer: f32,
    pub prev_canvas_rect: Option<Rect>,
}

impl SaveEditor {
    /// Construct the app with a pre-loaded config (used by main.rs so window position/state can be applied before the window opens).
    pub fn with_config(config: SaveEditorConfig) -> Self {
        // Snapshot the remembered artifact search settings before config is moved into the app.
        let (
            remember_scope,
            remember_sort_key,
            remember_sort_desc,
            remember_sub_sort_key,
            remember_sub_sort_desc,
            remember_use_sub_sort,
            remember_group_by,
        ) = if config.remember_artifact_search {
            (
                config.artifact_search_scope,
                config.artifact_result_sort_key,
                config.artifact_result_sort_desc,
                config.artifact_result_sub_sort_key,
                config.artifact_result_sub_sort_desc,
                config.artifact_use_sub_sort,
                config.artifact_result_group_by,
            )
        } else {
            (
                crate::artifact::SearchTierScope::StaticTier,
                crate::artifact::ResultSortKey::Closeness,
                false,
                crate::artifact::ResultSortKey::Closeness,
                false,
                false,
                crate::artifact::ResultGroupBy::None,
            )
        };
        let mut app = Self {
            load_requested: false,
            save_data: None,
            file_path: None,
            error_message: None,
            active_tab: Tab::Stats,

            config,
            catalog: None,
            catalog_error: None,
            monster_catalog: None,
            monster_catalog_error: None,
            monster_texture_cache: MonsterTextureCache::new(),
            bestiary_search_filter: String::new(),
            selected_bestiary_beast: None,
            selected_bestiary_beasts: std::collections::HashSet::new(),
            bestiary_grid_sel: crate::tabs::multisel::GridSel::default(),
            skilltree_catalog: None,
            skilltree_catalog_error: None,

            item_atlas: None,

            skilltree_texture: None,
            skilltree_texture_error: None,
            skilltree_zoom: 0.5,
            skilltree_scroll: egui::Vec2::ZERO,
            selected_skill_node: None,
            selected_skill_nodes: std::collections::HashSet::new(),
            skilltree_grid_sel: crate::tabs::multisel::GridSel::default(),
            skilltree_centered: false,

            stats_dirty: true,

            item_search_filter: String::new(),
            equipment_subtab: EquipmentSubTab::Inventory,
            selected_equipment_item: None,
            selected_equipment_items: std::collections::HashSet::new(),
            equipment_grid_sel: crate::tabs::multisel::GridSel::default(),
            equipment_remove_all_open: false,
            equipment_remove_all_types: std::collections::HashSet::new(),
            selected_catalog_item: None,
            selected_catalog_items: std::collections::HashSet::new(),
            add_items_grid_sel: crate::tabs::multisel::GridSel::default(),
            add_item_count: 1,
            add_item_upgrade: 0,

            selected_artifact: None,
            artifact_desired_values: std::collections::HashMap::new(),
            artifact_can_values: std::collections::HashMap::new(),
            artifact_exact_matches: Vec::new(),
            artifact_partial_matches: Vec::new(),
            artifact_match_search: String::new(),
            artifact_show_partial: true,
            artifact_show_all_stats: false,
            artifact_result_sort_key: remember_sort_key,
            artifact_result_sort_desc: remember_sort_desc,
            artifact_result_sub_sort_key: remember_sub_sort_key,
            artifact_result_sub_sort_desc: remember_sub_sort_desc,
            artifact_use_sub_sort: remember_use_sub_sort,
            artifact_result_group_by: remember_group_by,
            artifact_collapsed_groups: std::collections::HashSet::new(),
            artifact_search_scope: remember_scope,
            artifact_min_tier: 0,
            artifact_max_tier: 40,
            artifact_result_limit: None,
            artifact_pending_apply: None,
            artifact_pending_add: None,
            resalter_boosts_cache: None,

            export_tree_loading: false,
            export_tree_receiver: None,
            export_picker: None,
            export_picker_open: false,
            export_state: None,
            export_overwrite: false,

            settings_open: false,

            conversion_target_version: 19,
            conversion_just_happened: false,

            hash_edit_string: String::new(),
            use_custom_hash: false,
            config_save_timer: 0.0,
            prev_canvas_rect: None,
        };

        // Load catalogs immediately if we already have a game path stored
        // Textures load in app.ui()
        if let Some(game_path) = &app.config.game_path.clone() {
            app.load_catalogs(game_path);
        }

        app
    }
}

impl Default for SaveEditor {
    fn default() -> Self {
        Self::with_config(SaveEditorConfig::load())
    }
}

impl SaveEditor {
    /// Load (or reload) all three catalogs from `game_path`.
    fn load_catalogs(&mut self, game_path: &Path) {
        match load_loot_catalog(game_path) {
            Ok(cat) => {
                self.catalog = Some(cat);
                self.catalog_error = None;
            }
            Err(e) => {
                self.catalog = None;
                self.catalog_error = Some(e);
            }
        }
        match load_monster_catalog(game_path) {
            Ok(cat) => {
                self.monster_catalog = Some(cat.clone());
                self.monster_catalog_error = None;

                // start background texture loading
                let names: Vec<String> = cat
                    .monsters
                    .iter()
                    .filter(|m| !m.texture.is_empty())
                    .map(|m| m.texture.clone())
                    .collect();
                self.monster_texture_cache.set_game_path(game_path);
                self.monster_texture_cache.start_preload(game_path, names);
            }
            Err(e) => {
                self.monster_catalog = None;
                self.monster_catalog_error = Some(e);
            }
        }
        match load_skilltree_catalog(game_path) {
            Ok(cat) => {
                self.skilltree_catalog = Some(cat);
                self.skilltree_catalog_error = None;
            }
            Err(e) => {
                self.skilltree_catalog = None;
                self.skilltree_catalog_error = Some(e);
            }
        }
    }

    /// Update the stored game path, persist it, and reload everything.
    pub fn set_game_path(&mut self, path: PathBuf) {
        self.config.game_path = Some(path.clone());
        self.config.save();

        self.load_catalogs(&path);

        // Drop the old atlas and texture so they get re-loaded lazily on the next frame that needs them.
        self.item_atlas = None;
        self.skilltree_texture = None;
        self.skilltree_centered = false;
    }

    pub fn choose_game_folder(&mut self) {
        if let Some(folder) = FileDialog::new().pick_folder() {
            self.set_game_path(folder);
        }
    }

    pub fn open_file(&mut self) {
        if let Some(path) = FileDialog::new().pick_file() {
            match fs::read(&path) {
                Ok(data) => match SaveData::from_bytes(&data) {
                    Ok(save) => {
                        self.save_data = Some(save);
                        self.file_path = Some(path);
                        self.error_message = None;
                        self.hash_edit_string.clear();
                        self.use_custom_hash = false;
                        self.conversion_just_happened = false;
                    }
                    Err(e) => self.error_message = Some(e.to_string()),
                },
                Err(e) => self.error_message = Some(e.to_string()),
            }
        }
    }

    pub fn save_file(&mut self) {
        if let (Some(save), Some(path)) = (self.save_data.as_mut(), &self.file_path) {
            SaveEditor::create_backup(path);
            if self.use_custom_hash {
                save.custom_hash_override = save.hash_data;
            } else {
                save.custom_hash_override = None;
            }
            match save.to_bytes() {
                Ok(data) => {
                    if let Err(e) = fs::write(path, data) {
                        self.error_message = Some(e.to_string());
                    } else {
                        self.error_message = None;
                    }
                }
                Err(e) => self.error_message = Some(e.to_string()),
            }
        } else {
            self.error_message = Some("No file loaded".into());
        }
    }

    /// Create a numbered backup of `original_path` (e.g. `file.slv.3.bak`).
    /// Scans the parent directory to find the next unused index.
    fn create_backup(original_path: &Path) -> Option<PathBuf> {
        if !original_path.exists() {
            return None;
        }

        let file_stem = original_path.file_stem()?.to_string_lossy();
        let parent = original_path.parent()?;

        let pattern = format!("{}.", file_stem);
        let mut max_idx = 0u32;

        if let Ok(entries) = fs::read_dir(parent) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if name_str.starts_with(&pattern) && name_str.ends_with(".bak") {
                    // Name is "<stem>.<n>.bak", extract the <n> part
                    let middle = &name_str[pattern.len()..name_str.len() - 4];
                    if let Ok(idx) = middle.parse::<u32>() {
                        max_idx = max_idx.max(idx);
                    }
                }
            }
        }

        let backup_name = format!("{}.slv.{}.bak", file_stem, max_idx + 1);
        let backup_path = parent.join(backup_name);

        match fs::copy(original_path, &backup_path) {
            Ok(_) => Some(backup_path),
            Err(e) => {
                eprintln!("Failed to create backup: {}", e);
                None
            }
        }
    }

    /// Open the XNB file picker and populate `export_picker`.
    pub fn export_assets(&mut self) {
        let game_path = match &self.config.game_path {
            Some(p) => p.clone(),
            None => {
                eprintln!("Game folder not set");
                return;
            }
        };

        // Start scanning in a background thread.
        let (tx, rx) = std::sync::mpsc::channel();
        let scan_root = game_path.join("Content");
        let scan_root = if scan_root.is_dir() {
            scan_root
        } else {
            self.error_message = Some("Game doesn't have Content folder".to_string());
            return;
        };

        std::thread::spawn(move || {
            let tree = build_xnb_tree(&scan_root);
            let _ = tx.send(tree);
        });

        self.export_tree_receiver = Some(rx);
        self.export_tree_loading = true;
    }

    pub fn show_settings_window(&mut self, ctx: &egui::Context) {
        if !self.settings_open {
            return;
        }

        let mut is_open = self.settings_open;

        egui::Window::new("Configure UI")
            .open(&mut is_open)
            .resizable(false)
            .collapsible(false)
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.heading("Item Display Settings");

                    ui.horizontal(|ui| {
                        ui.label("Item Icon Size:");
                        if ui
                            .add(
                                egui::DragValue::new(&mut self.config.item_icon_size)
                                    .range(32.0..=128.0)
                                    .speed(self.config.drag_value_sensitivity)
                                    .suffix("px"),
                            )
                            .changed()
                        {
                            self.config_save_timer = 0.1;
                        }
                        if ui.button("Reset").clicked() {
                            self.config.item_icon_size = default_item_icon_size();
                            self.config_save_timer = 0.1;
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("Upgrade Indicator:");
                        let style = &mut self.config.upgrade_style;
                        let mut changed = egui::ComboBox::from_id_salt("upgrade_style")
                            .selected_text(match style {
                                crate::tabs::multisel::UpgradeStyle::Off => "Off",
                                crate::tabs::multisel::UpgradeStyle::Digits => "Digits",
                                crate::tabs::multisel::UpgradeStyle::Roman => "Roman numerals",
                            })
                            .show_ui(ui, |ui| {
                                let mut changed = false;
                                changed |= ui
                                    .selectable_value(
                                        style,
                                        crate::tabs::multisel::UpgradeStyle::Off,
                                        "Off",
                                    )
                                    .changed();
                                changed |= ui
                                    .selectable_value(
                                        style,
                                        crate::tabs::multisel::UpgradeStyle::Digits,
                                        "Digits",
                                    )
                                    .changed();
                                changed |= ui
                                    .selectable_value(
                                        style,
                                        crate::tabs::multisel::UpgradeStyle::Roman,
                                        "Roman numerals",
                                    )
                                    .changed();
                                changed
                            })
                            .inner
                            .unwrap_or(false);
                        if ui.button("Reset").clicked() {
                            self.config.upgrade_style =
                                crate::tabs::multisel::UpgradeStyle::Digits;
                            changed = true;
                        }
                        if changed {
                            self.config_save_timer = 0.1;
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("Artifact Seed Indicator:");
                        let style = &mut self.config.artifact_seed_style;
                        let mut changed = egui::ComboBox::from_id_salt("artifact_seed_style")
                            .selected_text(match style {
                                crate::tabs::multisel::UpgradeStyle::Off => "Off",
                                _ => "Digits",
                            })
                            .show_ui(ui, |ui| {
                                let mut changed = false;
                                changed |= ui
                                    .selectable_value(
                                        style,
                                        crate::tabs::multisel::UpgradeStyle::Off,
                                        "Off",
                                    )
                                    .changed();
                                changed |= ui
                                    .selectable_value(
                                        style,
                                        crate::tabs::multisel::UpgradeStyle::Digits,
                                        "Digits",
                                    )
                                    .changed();
                                changed
                            })
                            .inner
                            .unwrap_or(false);
                        if ui.button("Reset").clicked() {
                            self.config.artifact_seed_style =
                                crate::tabs::multisel::UpgradeStyle::Digits;
                            changed = true;
                        }
                        if changed {
                            self.config_save_timer = 0.1;
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("Grid Font Size:");
                        if ui
                            .add(
                                egui::DragValue::new(&mut self.config.grid_font_size)
                                    .range(6.0..=24.0)
                                    .speed(self.config.drag_value_sensitivity)
                                    .suffix("pt"),
                            )
                            .changed()
                        {
                            self.config_save_timer = 0.1;
                        }
                        if ui.button("Reset").clicked() {
                            self.config.grid_font_size = default_grid_font_size();
                            self.config_save_timer = 0.1;
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("Sidebar Font Size:");
                        if ui
                            .add(
                                egui::DragValue::new(&mut self.config.sidebar_font_size)
                                    .range(6.0..=24.0)
                                    .speed(self.config.drag_value_sensitivity)
                                    .suffix("pt"),
                            )
                            .changed()
                        {
                            self.config_save_timer = 0.1;
                        }
                        if ui.button("Reset").clicked() {
                            self.config.sidebar_font_size = default_sidebar_font_size();
                            self.config_save_timer = 0.1;
                        }
                    });

                    ui.horizontal(|ui| {
                        if ui
                            .checkbox(
                                &mut self.config.group_by_category,
                                "Group inventory by category",
                            )
                            .on_hover_text(
                                "Group the inventory/stockpile grid by type-subtype. \
                                 When off, items are shown in one flat grid and Move Left/Right \
                                 moves across the whole list instead of within a category.",
                            )
                            .changed()
                        {
                            self.config_save_timer = 0.1;
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("Tabs Font Size:");
                        if ui
                            .add(
                                egui::DragValue::new(&mut self.config.tabs_font_size)
                                    .range(6.0..=24.0)
                                    .speed(self.config.drag_value_sensitivity)
                                    .suffix("pt"),
                            )
                            .changed()
                        {
                            self.config_save_timer = 0.1;
                        }
                        if ui.button("Reset").clicked() {
                            self.config.tabs_font_size = default_tabs_font_size();
                            self.config_save_timer = 0.1;
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("Category Font Size:");
                        if ui
                            .add(
                                egui::DragValue::new(&mut self.config.category_font_size)
                                    .range(6.0..=24.0)
                                    .speed(self.config.drag_value_sensitivity)
                                    .suffix("pt"),
                            )
                            .changed()
                        {
                            self.config_save_timer = 0.1;
                        }
                        if ui.button("Reset").clicked() {
                            self.config.category_font_size = default_category_font_size();
                            self.config_save_timer = 0.1;
                        }
                    });

                    ui.separator();

                    ui.horizontal(|ui| {
                        ui.label("Drag Value Sensitivity:");
                        if ui
                            .add(
                                egui::DragValue::new(&mut self.config.drag_value_sensitivity)
                                    .range(0.005..=1.0)
                                    .speed(0.025)
                                    .suffix("x"),
                            )
                            .changed()
                        {
                            self.config_save_timer = 0.1;
                        }
                        if ui.button("Reset").clicked() {
                            self.config.drag_value_sensitivity = default_drag_sensitivity();
                            self.config_save_timer = 0.1;
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("Test Drag Value Sensitivity:");
                        if ui
                            .add(
                                egui::DragValue::new(&mut self.config.dummy_drag_value)
                                    .range(0.0..=1000.0)
                                    .speed(self.config.drag_value_sensitivity)
                                    .suffix("x"),
                            )
                            .changed()
                        {
                            self.config_save_timer = 0.1;
                        }
                    });

                    ui.separator();
                    ui.heading("Window");
                    if ui
                        .checkbox(
                            &mut self.config.save_window_position,
                            "Save window position",
                        )
                        .changed()
                    {
                        self.config_save_timer = 0.1;
                    }
                    if ui
                        .checkbox(
                            &mut self.config.force_x11_for_position,
                            "Force XWayland for position saving (Wayland only)",
                        )
                        .on_hover_text(
                            "Native Wayland cannot read the window position. \
                             Enabling this runs the app through XWayland so the position can be saved.",
                        )
                        .changed()
                    {
                        self.config_save_timer = 0.1;
                    }
                    if ui
                        .checkbox(
                            &mut self.config.save_window_state,
                            "Save window state (maximized)",
                        )
                        .changed()
                    {
                        self.config_save_timer = 0.1;
                    }

                    ui.separator();
                    ui.heading("Artifacts");
                    if ui
                        .checkbox(
                            &mut self.config.remember_artifact_search,
                            "Remember artifact search settings (tier pick, sorting)",
                        )
                        .on_hover_text(
                            "Remember the Tiers: pick and the result sorting options \
                             across restarts.",
                        )
                        .changed()
                    {
                        // Persist the current settings immediately when enabling.
                        if self.config.remember_artifact_search {
                            self.config.artifact_search_scope = self.artifact_search_scope;
                            self.config.artifact_result_sort_key = self.artifact_result_sort_key;
                            self.config.artifact_result_sort_desc = self.artifact_result_sort_desc;
                        }
                        self.config_save_timer = 0.1;
                    }
                    if ui
                        .checkbox(
                            &mut self.config.always_load_all_results,
                            "Always load all artifact search results",
                        )
                        .on_hover_text(
                            "Show all results instead of only a portion of them; \
                             can lag with very large result sets.",
                        )
                        .changed()
                    {
                        self.config_save_timer = 0.1;
                    }
                });
            });

        self.settings_open = is_open;
    }
}

impl eframe::App for SaveEditor {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut Frame) {
        if self.skilltree_texture.is_none() && self.skilltree_catalog.is_some() {
            if let Some(game_path) = &self.config.game_path {
                match load_skilltree_texture(game_path, ui.ctx()) {
                    Ok(tex) => self.skilltree_texture = Some(tex),
                    Err(e) => self.skilltree_texture_error = Some(e),
                }
            }
        }

        if self.item_atlas.is_none() {
            if let Some(game_path) = self.config.game_path.clone() {
                match ItemAtlas::load(&game_path, ui.ctx()) {
                    Ok(atlas) => self.item_atlas = Some(atlas),
                    Err(e) => eprintln!("Failed to load item atlas: {}", e),
                }
            }
        }

        egui::CentralPanel::default().show_inside(ui, |ui| {
            // Menu bar
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Open").clicked() {
                        self.open_file();
                        ui.close();
                    }
                    if ui.button("Save").clicked() {
                        self.save_file();
                        ui.close();
                    }
                });
                ui.menu_button("Settings", |ui| {
                    if ui.button("Set Game Folder").clicked() {
                        self.choose_game_folder();
                        ui.close();
                    }
                    if ui.button("Export XNB Files").clicked() {
                        self.export_assets();
                        ui.close();
                    }
                    if ui.button("Configure UI").clicked() {
                        self.settings_open = true;
                        ui.close();
                    }
                });
            });

            self.show_settings_window(ui.ctx());

            // Game folder status line
            if let Some(game_path) = &self.config.game_path {
                ui.label(format!("Game folder: {}", game_path.display()));
            } else {
                ui.colored_label(egui::Color32::YELLOW, "Game folder not set (needed for item names/icons, and bestiary textures/names)", );
                if ui.button("Set Game Folder").clicked() {
                    self.choose_game_folder();
                }
            }
            if let Some(err) = &self.catalog_error {
                ui.colored_label(egui::Color32::RED, format!("Loot catalog error: {}", err));
            }
            if let Some(err) = &self.monster_catalog_error {
                ui.colored_label(egui::Color32::RED, format!("Monster catalog error: {}", err));
            }
            if let Some(err) = &self.error_message { ui.colored_label(egui::Color32::RED, err); }

            ui.separator();

            // The borrow checker doesn't let us pass &mut self.save_data to a method that also borrows self, so we briefly take ownership.
            let mut save_taken = self.save_data.take();

            if let Some(save) = &mut save_taken {
                // Tab bar
                egui::Panel::top("tabs")
                    .show_separator_line(false)
                    .show_inside(ui, |ui| {
                        ui.horizontal(|ui| {
                            let tabs = self.config.tabs_font_size;
                            ui.selectable_value(&mut self.active_tab, Tab::Stats, egui::RichText::new("Stats").size(tabs));
                            ui.selectable_value(&mut self.active_tab, Tab::Equipment, egui::RichText::new("Equipment").size(tabs));
                            ui.selectable_value(&mut self.active_tab, Tab::SkillTree, egui::RichText::new("Skill Tree").size(tabs));
                            ui.selectable_value(&mut self.active_tab, Tab::Cosmetics, egui::RichText::new("Cosmetics").size(tabs));
                            ui.selectable_value(&mut self.active_tab, Tab::Flags, egui::RichText::new("Flags").size(tabs));
                            ui.selectable_value(&mut self.active_tab, Tab::Bestiary, egui::RichText::new("Bestiary").size(tabs));
                            ui.selectable_value(&mut self.active_tab, Tab::Faction, egui::RichText::new("Faction").size(tabs));
                            ui.selectable_value(&mut self.active_tab, Tab::Artifacts, egui::RichText::new("Artifacts").size(tabs));
                            ui.selectable_value(&mut self.active_tab, Tab::ConvertSave, egui::RichText::new("Convert modded to vanilla save").size(tabs));

                            ui.vertical(|ui| {
                                // progress bar while textures are loading
                                if let Some((loaded, total)) = self.monster_texture_cache.progress() {
                                    let fraction = loaded as f32 / total as f32;
                                    ui.add(egui::ProgressBar::new(fraction.min(1.0)).show_percentage());
                                    ui.label(format!("{}/{} textures loaded…", loaded, total));
                                }
                            });
                        });
                    });

                ui.separator();

                match self.active_tab {
                    Tab::Stats => self.show_stats_ui(ui, save),
                    Tab::Equipment => self.show_equipment_ui(ui, save),
                    Tab::SkillTree => self.show_skilltree_ui(ui, save),
                    Tab::Cosmetics => self.show_cosmetics_ui(ui, save),
                    Tab::Flags => self.show_flags_ui(ui, save),
                    Tab::Bestiary => self.show_bestiary_ui(ui, save),
                    Tab::Faction => self.show_faction_ui(ui, save),
                    Tab::Artifacts => self.show_artifacts_ui(ui, save),
                    Tab::ConvertSave => self.show_convert_save_ui(ui, save),
                }

                if self.conversion_just_happened {
                    // The convert tab replaced self.save_data, pick it up
                    save_taken = self.save_data.take();
                    self.conversion_just_happened = false;
                }
            } else {
                if ui.button("Open Save File").clicked() {
                    self.load_requested = true;
                }
            }

            self.save_data = save_taken;

            if self.load_requested {
                self.open_file();
                self.load_requested = false;
            }

            if let Some(err) = &self.error_message {
                ui.colored_label(egui::Color32::RED, err);
            }

            // XNB export progress window
            if let Some(state) = self.export_state.as_ref() {
                if show_export_progress(ui, state) {
                    self.export_state = None;
                }
            }

            // XNB file picker window
            if self.export_picker_open {
                if let Some(root) = &mut self.export_picker {
                    match show_export_picker(ui, root, &mut self.export_overwrite) {
                        Some(files) if !files.is_empty() => {
                            let game_path = self.config.game_path.clone().unwrap();
                            self.export_state =
                                Some(start_export_job(game_path, files, self.export_overwrite));
                            self.export_picker_open = false;
                            self.export_picker = None;
                        }
                        Some(_) => {
                            // Empty vec means the user cancelled
                            self.export_picker_open = false;
                            self.export_picker = None;
                        }
                        None => {} // still open
                    }
                }
            }
        });
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        // Process monster texture loading
        //if self.active_tab == Tab::Bestiary {
        self.monster_texture_cache.update(ctx);
        if self.monster_texture_cache.is_loading() {
            ctx.request_repaint();
        }

        // Check for XNB tree completion
        if self.export_tree_loading {
            if let Some(rx) = &self.export_tree_receiver {
                if let Ok(tree) = rx.try_recv() {
                    // Tree ready!
                    self.export_picker = tree;
                    self.export_picker_open = true;
                    self.export_tree_loading = false;
                    self.export_tree_receiver = None;
                } else {
                    // Still scanning, keep refreshing
                    ctx.request_repaint();
                }
            }
        }

        if let Some(state) = &self.export_state {
            if !state.done.load(Ordering::Relaxed) {
                ctx.request_repaint();
            }
        }

        // Persist window position/size/maximized state when enabled.
        // Re-arms the throttled config save only when the window state actually changed.
        if self.config.save_window_position || self.config.save_window_state {
            let info = ctx.input(|i| i.viewport().clone());
            let mut changed = false;
            if self.config.save_window_position {
                // Position is unavailable on Wayland (winit cannot query it), keep the last known value in that case.
                if let Some(rect) = info.outer_rect {
                    let pos = [rect.min.x, rect.min.y];
                    if self.config.window_pos != Some(pos) {
                        self.config.window_pos = Some(pos);
                        changed = true;
                    }
                }
                // Size: prefer inner_rect, fall back to the viewport content rect which is available on all platforms.
                let size = info
                    .inner_rect
                    .map(|r| [r.width(), r.height()])
                    .or_else(|| {
                        let r = ctx.viewport_rect();
                        Some([r.width(), r.height()])
                    });
                if let Some(size) = size {
                    if self.config.window_size != Some(size) {
                        self.config.window_size = Some(size);
                        changed = true;
                    }
                }
            }
            if self.config.save_window_state {
                let maximized = info.maximized.unwrap_or(false);
                if self.config.window_maximized != maximized {
                    self.config.window_maximized = maximized;
                    changed = true;
                }
            }
            if changed {
                self.config_save_timer = 0.1;
            }
        }

        if self.config_save_timer > 0.0 {
            self.config_save_timer -= ctx.input(|i| i.stable_dt);

            if self.config_save_timer <= 0.01 {
                self.config.save();
                eprintln!("Config saved.");
                self.config_save_timer = 0.0;
            }
        }
        //}
    }
}
