use crate::atlas::ItemAtlas;
use crate::catalog::{
    load_loot_catalog, load_monster_catalog, load_skilltree_catalog, load_skilltree_texture,
};
use crate::config::{
    default_drag_sensitivity, default_item_font_size, default_item_icon_size, AppConfig,
};
use crate::export::{
    build_xnb_tree, show_export_picker, show_export_progress, start_export_job, ExportState,
    XnbNode,
};
use crate::tabs::{EquipmentSubTab, Tab};
use eframe::{egui, Frame};
use egui::{Rect, TextureHandle, Ui};
use rfd::FileDialog;
use sas2_save::loot_catalog::LootCatalog;
use sas2_save::monster_catalog::MonsterCatalog;
use sas2_save::skilltree::SkillTreeCatalog;
use sas2_save::SaveData;
use std::fs;
use std::path::{Path, PathBuf};

pub struct SaveEditorApp {
    pub load_requested: bool,
    pub save_data: Option<SaveData>,
    pub file_path: Option<PathBuf>,
    pub error_message: Option<String>,
    pub active_tab: Tab,

    pub config: AppConfig,

    // Loot / monster / skill catalogs and their respective load errors
    pub catalog: Option<LootCatalog>,
    pub catalog_error: Option<String>,
    pub monster_catalog: Option<MonsterCatalog>,
    pub monster_catalog_error: Option<String>,
    pub skilltree_catalog: Option<SkillTreeCatalog>,
    pub skilltree_catalog_error: Option<String>,

    // Globally loaded item icon atlas (items.xnb) — loaded lazily when the equipment tab is first opened.
    pub item_atlas: Option<ItemAtlas>,

    // Skill tree rendering
    pub skilltree_texture: Option<TextureHandle>,
    pub skilltree_texture_error: Option<String>,
    pub skilltree_zoom: f32,
    pub skilltree_scroll: egui::Vec2,
    pub selected_skill_node: Option<usize>,
    pub skilltree_centered: bool,

    // Whether the skill stats need to be recomputed from the tree
    pub stats_dirty: bool,

    // Equipment tab state
    pub item_search_filter: String,
    pub equipment_subtab: EquipmentSubTab,
    pub selected_equipment_item: Option<usize>,
    pub selected_catalog_item: Option<usize>,
    pub add_item_count: i32,
    pub add_item_upgrade: i32,

    // XNB exporter
    pub export_picker: Option<XnbNode>,
    pub export_picker_open: bool,
    pub export_state: Option<ExportState>,
    pub export_overwrite: bool,

    // Settings window
    pub settings_open: bool,

    // Modded → vanilla conversion
    pub conversion_target_version: i32,
    pub conversion_just_happened: bool,

    // MD5 hash override
    pub hash_edit_string: String,
    pub use_custom_hash: bool,

    // Sidebars timer and previous size
    pub config_save_timer: f32,
    pub prev_canvas_rect: Option<Rect>,
}

impl Default for SaveEditorApp {
    fn default() -> Self {
        let config = AppConfig::load();

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
            skilltree_catalog: None,
            skilltree_catalog_error: None,

            item_atlas: None,

            skilltree_texture: None,
            skilltree_texture_error: None,
            skilltree_zoom: 0.5,
            skilltree_scroll: egui::Vec2::ZERO,
            selected_skill_node: None,
            skilltree_centered: false,

            stats_dirty: true,

            item_search_filter: String::new(),
            equipment_subtab: EquipmentSubTab::Inventory,
            selected_equipment_item: None,
            selected_catalog_item: None,
            add_item_count: 1,
            add_item_upgrade: 0,

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

impl SaveEditorApp {
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
                self.monster_catalog = Some(cat);
                self.monster_catalog_error = None;
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
            SaveEditorApp::create_backup(path);
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
                    // Name is "<stem>.<n>.bak" — extract the <n> part
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

        self.export_picker = build_xnb_tree(&game_path);
        self.export_picker_open = true;
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
                            self.config.save();
                        }
                        if ui.button("Reset").clicked() {
                            self.config.item_icon_size = default_item_icon_size();
                            self.config.save();
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("Item Font Size:");
                        if ui
                            .add(
                                egui::DragValue::new(&mut self.config.item_font_size)
                                    .range(6.0..=24.0)
                                    .speed(self.config.drag_value_sensitivity)
                                    .suffix("pt"),
                            )
                            .changed()
                        {
                            self.config.save();
                        }
                        if ui.button("Reset").clicked() {
                            self.config.item_font_size = default_item_font_size();
                            self.config.save();
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
                            self.config.save();
                        }
                        if ui.button("Reset").clicked() {
                            self.config.drag_value_sensitivity = default_drag_sensitivity();
                            self.config.save();
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
                            self.config.save();
                        }
                    });
                });
            });

        self.settings_open = is_open;
    }
}

impl eframe::App for SaveEditorApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut Frame) {
        if self.config_save_timer > 0.0 {
            self.config_save_timer -= ui.ctx().input(|i| i.stable_dt);

            if self.config_save_timer <= 0.01 {
                self.config.save();
                eprintln!("Config saved.");
                self.config_save_timer = 0.0;
            }
        }

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
                ui.colored_label(
                    egui::Color32::YELLOW,
                    "Game folder not set (needed for item names, icons, and bestiary names)",
                );
                if ui.button("Set Game Folder").clicked() {
                    self.choose_game_folder();
                }
            }
            if let Some(err) = &self.catalog_error {
                ui.colored_label(egui::Color32::RED, format!("Loot catalog error: {}", err));
            }
            if let Some(err) = &self.monster_catalog_error {
                ui.colored_label(
                    egui::Color32::RED,
                    format!("Monster catalog error: {}", err),
                );
            }

            ui.separator();

            // The borrow checker doesn't let us pass &mut self.save_data to
            // a method that also borrows self, so we briefly take ownership.
            let mut save_taken = self.save_data.take();

            if let Some(save) = &mut save_taken {
                // Tab bar
                egui::Panel::top("tabs")
                    .show_separator_line(false)
                    .show_inside(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.selectable_value(&mut self.active_tab, Tab::Stats, "Stats");
                            ui.selectable_value(&mut self.active_tab, Tab::Equipment, "Equipment");
                            ui.selectable_value(&mut self.active_tab, Tab::SkillTree, "Skill Tree");
                            ui.selectable_value(&mut self.active_tab, Tab::Cosmetics, "Cosmetics");
                            ui.selectable_value(&mut self.active_tab, Tab::Flags, "Flags");
                            ui.selectable_value(&mut self.active_tab, Tab::Bestiary, "Bestiary");
                            ui.selectable_value(&mut self.active_tab, Tab::Faction, "Faction");
                            ui.selectable_value(
                                &mut self.active_tab,
                                Tab::ConvertSave,
                                "Convert modded to vanilla save",
                            );
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
}
