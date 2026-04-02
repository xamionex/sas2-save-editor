use crate::catalog::{load_loot_catalog, load_monster_catalog, load_skilltree_catalog, load_skilltree_texture};
use crate::config::{default_drag_sensitivity, default_item_font_size, default_item_icon_size, AppConfig};
use eframe::egui::{Grid, ScrollArea, TextureHandle};
use eframe::{egui, Frame};
use egui::{pos2, Rect, Response, Stroke, Ui};
use rfd::FileDialog;
use sas2_save::cosmetics::{AncestryCatalog, BeardCatalog, ClassCatalog, ColorCatalog, CrimeCatalog, EyeCatalog, HairCatalog, SexCatalog};
use sas2_save::loot_catalog::{LootCatalog, LootDef};
use sas2_save::monster_catalog::MonsterCatalog;
use sas2_save::skilltree::{SkillTreeCatalog, SKILL_IMG};
use sas2_save::{loot_names, BestiaryBeast, Item, SaveData};
use std::fs;
use std::path::{Path, PathBuf};
use sas2_save::types::faction::PlayerFaction;
use sas2_save::types::ng_level;

#[derive(Clone)]
pub struct XnbNode {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub children: Vec<XnbNode>,
    pub selected: bool,
}

pub struct ExportState {
    pub progress: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    pub total: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    pub cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
    pub done: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

#[derive(PartialEq)]
pub enum Tab {
    Stats,
    Equipment,
    Flags,
    Bestiary,
    Cosmetics,
    SkillTree,
    Faction,
    ConvertSave,
}

#[derive(PartialEq)]
pub enum EquipmentSubTab {
    Inventory,
    Stockpile,
    AddItems,
}

pub struct SaveEditorApp {
    pub load_requested: bool,
    pub save_data: Option<SaveData>,
    pub file_path: Option<PathBuf>,
    pub error_message: Option<String>,
    pub active_tab: Tab,

    pub config: AppConfig,
    pub catalog: Option<LootCatalog>,
    pub catalog_error: Option<String>,
    pub monster_catalog: Option<MonsterCatalog>,
    pub monster_catalog_error: Option<String>,
    pub item_atlas: Option<TextureHandle>,
    pub atlas_width: u32,
    pub atlas_height: u32,
    pub item_search_filter: String,
    pub equipment_subtab: EquipmentSubTab,
    pub selected_equipment_item: Option<usize>,
    pub selected_catalog_item: Option<usize>,
    pub add_item_count: i32,
    pub add_item_upgrade: i32,

    // Skill tree
    pub skilltree_catalog: Option<SkillTreeCatalog>,
    pub skilltree_texture: Option<TextureHandle>,
    pub skilltree_zoom: f32,
    pub skilltree_scroll: egui::Vec2,
    pub selected_skill_node: Option<usize>,
    pub skilltree_catalog_error: Option<String>,
    pub skilltree_texture_error: Option<String>,
    pub skilltree_centered: bool,
    pub stats_dirty: bool,

    // exporter
    pub export_picker: Option<XnbNode>,
    pub export_picker_open: bool,
    pub export_state: Option<ExportState>,
    pub export_overwrite: bool,
    pub settings_open: bool,

    // conversion
    pub conversion_target_version: i32,

    // hash
    pub hash_edit_string: String,
    pub use_custom_hash: bool,
}

impl Default for SaveEditorApp {
    fn default() -> Self {
        let mut app = Self {
            load_requested: false,
            save_data: None,
            file_path: None,
            error_message: None,
            active_tab: Tab::Stats,
            config: AppConfig::load(),
            catalog: None,
            catalog_error: None,
            monster_catalog: None,
            monster_catalog_error: None,
            item_atlas: None,
            atlas_width: 0,
            atlas_height: 0,
            item_search_filter: String::new(),
            equipment_subtab: EquipmentSubTab::Inventory,
            selected_equipment_item: None,
            selected_catalog_item: None,
            add_item_count: 1,
            add_item_upgrade: 0,
            skilltree_catalog: None,
            skilltree_texture: None,
            skilltree_zoom: 0.5,
            skilltree_scroll: egui::Vec2::ZERO,
            selected_skill_node: None,
            skilltree_catalog_error: None,
            skilltree_texture_error: None,
            skilltree_centered: false,
            stats_dirty: true,
            export_state: None,
            export_picker: None,
            export_picker_open: false,
            export_overwrite: false,
            settings_open: false,
            conversion_target_version: 19,
            hash_edit_string: String::new(),
            use_custom_hash: false,
        };

        if let Some(game_path) = &app.config.game_path {
            match load_loot_catalog(game_path) {
                Ok(cat) => app.catalog = Some(cat),
                Err(e) => app.catalog_error = Some(e),
            }
            match load_monster_catalog(game_path) {
                Ok(cat) => app.monster_catalog = Some(cat),
                Err(e) => app.monster_catalog_error = Some(e),
            }
            match load_skilltree_catalog(game_path) {
                Ok(cat) => app.skilltree_catalog = Some(cat),
                Err(e) => app.skilltree_catalog_error = Some(e),
            }
            // Textures will be loaded later when needed or when user sets folder again
        }
        app
    }
}

impl SaveEditorApp {
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
                    }
                    Err(e) => self.error_message = Some(e.to_string()),
                },
                Err(e) => self.error_message = Some(e.to_string()),
            }
        }
    }

    fn create_backup(original_path: &Path) -> Option<PathBuf> {
        if !original_path.exists() {
            return None;
        }

        let file_stem = original_path.file_stem()?.to_string_lossy();
        let parent = original_path.parent()?;

        // Find the highest existing backup index
        let pattern = format!("{}.", file_stem);
        let mut max_idx = 0;
        if let Ok(entries) = fs::read_dir(parent) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if name_str.starts_with(&pattern) && name_str.ends_with(".bak") {
                    // Extract the number between the stem and ".bak"
                    let middle = &name_str[pattern.len()..name_str.len() - 4];
                    if let Ok(idx) = middle.parse::<u32>() {
                        max_idx = max_idx.max(idx);
                    }
                }
            }
        }

        let new_idx = max_idx + 1;
        let backup_name = format!("{}.{}.bak", file_stem, new_idx);
        let backup_path = parent.join(backup_name);

        match fs::copy(original_path, &backup_path) {
            Ok(_) => Some(backup_path),
            Err(e) => {
                eprintln!("Failed to create backup: {}", e);
                None
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

    pub fn set_game_path(&mut self, path: PathBuf) {
        self.config.game_path = Some(path.clone());
        self.config.save();
        // Reload loot catalog
        match load_loot_catalog(&path) {
            Ok(cat) => {
                self.catalog = Some(cat);
                self.catalog_error = None;
            }
            Err(e) => {
                self.catalog = None;
                self.catalog_error = Some(e);
            }
        }
        // Reload monster catalog
        match load_monster_catalog(&path) {
            Ok(cat) => {
                self.monster_catalog = Some(cat);
                self.monster_catalog_error = None;
            }
            Err(e) => {
                self.monster_catalog = None;
                self.monster_catalog_error = Some(e);
            }
        }
        // Load skill tree catalog
        match load_skilltree_catalog(&path) {
            Ok(cat) => {
                self.skilltree_catalog = Some(cat);
                self.skilltree_catalog_error = None;
            }
            Err(e) => {
                self.skilltree_catalog = None;
                self.skilltree_catalog_error = Some(e);
            }
        }
        // Load skill tree texture
        // Happens now when opening the ui if not loaded
        //match load_skilltree_texture(&path, ctx) {
        //    Ok(tex) => {
        //        self.skilltree_texture = Some(tex);
        //        self.skilltree_texture_error = None;
        //    }
        //    Err(e) => {
        //        self.skilltree_texture = None;
        //        self.skilltree_texture_error = Some(e);
        //    }
        //}
        // Atlas will be loaded lazily in update
        self.item_atlas = None;
        self.skilltree_centered = false;
    }

    pub fn choose_game_folder(&mut self) {
        if let Some(folder) = FileDialog::new().pick_folder() {
            self.set_game_path(folder);
        }
    }

    pub fn load_atlas(&mut self, game_path: &Path, ctx: &egui::Context) {
        let items_xnb = game_path.join("Content").join("gfx").join("items.xnb");
        if !items_xnb.exists() {
            eprintln!("items.xnb not found at {:?}", items_xnb);
            return;
        }
        match sas2_save::xnb_loader::load_texture_from_path(items_xnb.to_str().unwrap()) {
            Ok(img) => {
                let width = img.width();
                let height = img.height();
                let pixels = img.into_vec();
                let size = [width as usize, height as usize];
                let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &pixels);
                let texture = ctx.load_texture("items_atlas", color_image, Default::default());
                self.item_atlas = Some(texture);
                self.atlas_width = width;
                self.atlas_height = height;
            }
            Err(e) => eprintln!("Failed to load items.xnb: {}", e),
        }
    }

    /// Recalculates the nine primary stats from the skill tree unlocks.
    /// This exactly mimics the game's PlayerStats.UpdateStats() for all node types.
    fn recalc_player_stats(save: &mut SaveData, catalog: &SkillTreeCatalog) {
        // Reset all stats to base 5 (as in UpdateStats)
        for stat in &mut save.stats.stats {
            *stat = 5;
        }

        for node in &catalog.nodes {
            let unlocked = save.stats.tree_unlocks[node.id] > 0
                || save.stats.class_unlocks.contains(&(node.id as i32));
            if !unlocked {
                continue;
            }

            match node.node_type {
                // Stat nodes (0..8)
                0..=8 => {
                    let stat_idx = node.node_type as usize;
                    if node.value > 1 {
                        // Fixed‑value node (e.g., +2 or +3)
                        save.stats.stats[stat_idx] += node.value;
                    } else {
                        // Multi‑level node: add the unlock count (at least 1)
                        let add = if save.stats.tree_unlocks[node.id] > 0 {
                            save.stats.tree_unlocks[node.id]
                        } else {
                            1
                        };
                        save.stats.stats[stat_idx] += add;
                    }
                }
                // Weapon/glyph nodes – they add `cost` to specific stats
                // Based on the decompiled C# switch:
                // case 9,20,23,29 -> Strength
                // case 10,22,30 -> Will
                // case 11,16 -> Vitality
                // case 12,13,15,19 -> Dexterity
                // case 14,28 -> Conviction
                // case 17,27 -> Arcana
                // case 18,25,26 -> Endurance
                // case 21 -> Resolve
                // case 24,31 -> Luck
                9 | 20 | 23 | 29 => save.stats.stats[0] += node.cost,
                10 | 22 | 30 => save.stats.stats[3] += node.cost,
                11 | 16 => save.stats.stats[2] += node.cost,
                12 | 13 | 15 | 19 => save.stats.stats[1] += node.cost,
                14 | 28 => save.stats.stats[6] += node.cost,
                17 | 27 => save.stats.stats[5] += node.cost,
                18 | 25 | 26 => save.stats.stats[4] += node.cost,
                21 => save.stats.stats[7] += node.cost,
                24 | 31 => save.stats.stats[8] += node.cost,
                _ => {} // Other types (should not exist) ignored
            }
        }
    }

    pub fn show_stats_ui(&mut self, ui: &mut Ui, save: &mut SaveData) {
        if self.stats_dirty {
            if let Some(catalog) = &self.skilltree_catalog {
                SaveEditorApp::recalc_player_stats(save, catalog);
            }
            self.stats_dirty = false;
        }

        ui.heading("Player Stats");
        ui.separator();
        ui.horizontal(|ui| {
            ui.label("Player Name:");
            ui.text_edit_singleline(&mut save.name);
        });
        ui.horizontal(|ui| {
            ui.label("Level:");
            ui.add(egui::DragValue::new(&mut save.stats.level).speed(self.config.drag_value_sensitivity).range(1..=999999));
        });
        self.add_ng_level_label(ui, save);
        ui.horizontal(|ui| {
            ui.label("XP:");
            ui.add(egui::DragValue::new(&mut save.stats.xp).speed(100).range(0..=999999));
        });
        ui.horizontal(|ui| {
            ui.label("Silver:");
            ui.add(egui::DragValue::new(&mut save.stats.silver).speed(100).range(0..=999999));
        });
        ui.horizontal(|ui| {
            ui.label("Time Played (seconds):");
            ui.add(egui::DragValue::new(&mut save.stats.time_played).speed(1.0).range(0.0..=1e9));
        });
        ui.horizontal(|ui| {
            ui.label("Hazeburnt:");
            ui.checkbox(&mut save.stats.hazeburnt, "");
        });
        ui.checkbox(&mut self.use_custom_hash, "Use custom hash (disable auto-recalculation on save)");
        if self.use_custom_hash {
            ui.label("Warning: The game might reject the save if the custom hash doesn't match the actual data.");
        }
        if let Some(hash) = &mut save.hash_data {
            // Initialize the edit string if it's empty (first time)
            if self.hash_edit_string.is_empty() {
                self.hash_edit_string = hash.iter().map(|b| format!("{:02x}", b)).collect();
            }

            ui.horizontal(|ui| {
                ui.label("Save Hash (MD5):");
                let response = ui.text_edit_singleline(&mut self.hash_edit_string);

                if response.changed() {
                    // Validate the hex string
                    if self.hash_edit_string.len() != 32 || !self.hash_edit_string.chars().all(|c| c.is_ascii_hexdigit()) {
                        ui.colored_label(egui::Color32::RED, "Invalid hash (must be 32 hex characters)");
                    } else {
                        let mut new_hash = [0; 16];
                        for i in 0..16 {
                            let byte = u8::from_str_radix(&self.hash_edit_string[i*2..i*2+2], 16).unwrap();
                            new_hash[i] = byte;
                        }
                        *hash = new_hash;
                    }
                }
            });
        } else {
            // Reset the edit string when there is no hash
            self.hash_edit_string.clear();
        }

        ui.separator();
        ui.heading("Attributes (from skill tree)");
        ui.label("Visit skill tree tab to edit stats");

        let stat_names = [
            "Strength",
            "Dexterity",
            "Vitality",
            "Will",
            "Endurance",
            "Arcana",
            "Conviction",
            "Resolve",
            "Luck",
        ];

        Grid::new("stats_grid")
            .num_columns(2)
            .spacing([40.0, 8.0])
            .striped(true)
            .show(ui, |ui| {
                for (i, name) in stat_names.iter().enumerate() {
                    let value = &mut save.stats.stats[i];
                    ui.label(format!("{}: {}", name, value));
                    // game recalcs based on skill tree
                    //ui.add(egui::DragValue::new(value).speed(1).range(0..=100));
                    ui.end_row();
                }
            });
    }

    pub fn show_equipment_ui(&mut self, ui: &mut Ui, save: &mut SaveData) {
        ui.heading("Equipment");
        ui.separator();

        // Ensure atlas is loaded if we have a game folder
        if self.item_atlas.is_none() {
            if let Some(game_path) = self.config.game_path.clone() {
                self.load_atlas(&game_path, ui.ctx());
            }
        }

        // Sub-tab bar
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.equipment_subtab, EquipmentSubTab::Inventory, "Inventory");
            ui.selectable_value(&mut self.equipment_subtab, EquipmentSubTab::Stockpile, "Stockpile");
            ui.selectable_value(&mut self.equipment_subtab, EquipmentSubTab::AddItems, "Add Items");
        });
        ui.add_space(8.0);

        match self.equipment_subtab {
            EquipmentSubTab::Inventory | EquipmentSubTab::Stockpile => {
                self.show_inventory_or_stockpile(ui, save);
            }
            EquipmentSubTab::AddItems => {
                self.show_add_items_tab(ui, save);
            }
        }
    }

    fn get_icon_uv(&self, def: &LootDef, atlas: Option<&TextureHandle>, atlas_width: f32, atlas_height: f32) -> Option<Rect> {
        let icon_uv = if def.img >= 0 && atlas.is_some() {
            let x = (def.img as u32 % 32) * 128;
            let y = (def.img as u32 / 32) * 128;

            Some(Rect::from_min_max(
                pos2(
                    x as f32 / atlas_width,
                    y as f32 / atlas_height,
                ),
                pos2(
                    (x + 128) as f32 / atlas_width,
                    (y + 128) as f32 / atlas_height,
                ),
            ))
        } else {
            None
        };
        icon_uv
    }

    pub fn draw_image_button(&self, ui: &mut Ui, icon_uv: Option<Rect>, atlas: Option<&TextureHandle>) -> Response {
        let response = if let Some(uv) = icon_uv {
            ui.add(
                egui::Button::image(
                    egui::Image::from_texture(atlas.unwrap())
                        .fit_to_exact_size(egui::vec2(self.config.item_icon_size, self.config.item_icon_size))
                        .uv(uv),
                )
            )
        } else {
            ui.add_space(self.config.item_icon_size);
            ui.allocate_response(egui::vec2(self.config.item_icon_size, self.config.item_icon_size), egui::Sense::click())
        };
        response
    }

    pub fn add_item_label(ui: &mut Ui, title: String, font_size: f32) {
        for text in title.split_whitespace() {
            ui.add(
                egui::Label::new(egui::RichText::new(text).size(font_size))
                    .wrap_mode(egui::TextWrapMode::Truncate)
                    .halign(egui::Align::Center)
                    .show_tooltip_when_elided(false),
            );
        }
    }

    fn show_inventory_or_stockpile(&mut self, ui: &mut Ui, save: &mut SaveData) {
        let stockpile_mode = self.equipment_subtab == EquipmentSubTab::Stockpile;
        let items = &mut save.equipment.inventory_items;

        let filtered_indices: Vec<usize> = items
            .iter()
            .enumerate()
            .filter_map(|(idx, item)| {
                if item.stock_piled == stockpile_mode {
                    Some(idx)
                } else {
                    None
                }
            })
            .collect();

        let catalog = self.catalog.as_ref();
        let atlas = self.item_atlas.as_ref();
        let atlas_width = self.atlas_width;
        let atlas_height = self.atlas_height;

        let mut selected_equipment_item_local = self.selected_equipment_item;

        let full_width = ui.available_width();
        let height = ui.available_height();

        ui.horizontal(|ui| {
            // LEFT
            ui.allocate_ui_with_layout(
                egui::vec2(full_width * 0.6, height),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    let mut clicked_item = None;

                    ScrollArea::both()
                        .max_height(ui.available_height())
                        .auto_shrink([false; 2])
                        .show(ui, |ui| {
                            let mut grouped: std::collections::HashMap<String, Vec<usize>> =
                                std::collections::HashMap::new();

                            for &orig_idx in &filtered_indices {
                                let item = &items[orig_idx];

                                let cat = if let Some(catalog) = catalog {
                                    if let Some(def) =
                                        catalog.loot_defs.get(item.loot_idx as usize)
                                    {
                                        let type_name = loot_names::get_type_name(def.type_);
                                        let subtype_name =
                                            loot_names::get_subtype_name(def.type_, def.sub_type);
                                        format!("{} - {}", type_name, subtype_name)
                                    } else {
                                        "Other".to_string()
                                    }
                                } else {
                                    "Other".to_string()
                                };

                                grouped.entry(cat).or_default().push(orig_idx);
                            }

                            //let mut to_remove: Vec<usize> = Vec::new();

                            let mut categories: Vec<_> = grouped.keys().cloned().collect();
                            categories.sort();

                            for cat in categories {
                                let orig_indices = grouped.get(&cat).unwrap();

                                ui.style_mut().interaction.selectable_labels = false;
                                ui.label(egui::RichText::new(&cat).strong());

                                Grid::new(&cat)
                                    .spacing([8.0, 8.0])
                                    .show(ui, |ui| {
                                        for &orig_idx in orig_indices {
                                            let item = &mut items[orig_idx];

                                            let (item_name, icon_uv) = if let Some(catalog) = catalog {
                                                if let Some(def) =
                                                    catalog.loot_defs.get(item.loot_idx as usize)
                                                {
                                                    let name = def.title[0].clone();

                                                    let uv = self.get_icon_uv(def, atlas, atlas_width as f32, atlas_height as f32);

                                                    (name, uv)
                                                } else {
                                                    (format!("Unknown ({})", item.loot_idx), None)
                                                }
                                            } else {
                                                (format!("Item {}", item.loot_idx), None)
                                            };

                                            ui.vertical(|ui| {
                                                let response = self.draw_image_button(ui, icon_uv, atlas);
                                                let button_width = response.rect.width();

                                                if response.clicked() {
                                                    clicked_item = Some(orig_idx);
                                                }

                                                ui.set_max_width(button_width);

                                                // Name under the button
                                                Self::add_item_label(ui, item_name, self.config.item_font_size);

                                                //ui.add(
                                                //    egui::DragValue::new(&mut item.count)
                                                //        .speed(1)
                                                //        .range(0..=999),
                                                //);

                                                //if ui.button("X").clicked() {
                                                //    to_remove.push(orig_idx);
                                                //}
                                            });
                                        }
                                    });

                                ui.add_space(8.0);
                            }

                            //if !to_remove.is_empty() {
                            //    to_remove.sort_unstable_by(|a, b| b.cmp(a));
                            //    for idx in to_remove {
                            //        items.remove(idx);
                            //        if let Some(sel) = selected_equipment_item_local {
                            //            if sel == idx {
                            //                selected_equipment_item_local = None;
                            //            } else if sel > idx {
                            //                selected_equipment_item_local = Some(sel - 1);
                            //            }
                            //        }
                            //    }
                            //}
                        });

                    if let Some(idx) = clicked_item {
                        selected_equipment_item_local = Some(idx);
                    }
                },
            );

            // RIGHT
            ui.allocate_ui_with_layout(
                egui::vec2(full_width * 0.4, height),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    if let Some(orig_idx) = selected_equipment_item_local {
                        if orig_idx < items.len() {
                            let item = &mut items[orig_idx];

                            if let Some(catalog) = catalog {
                                if let Some(def) =
                                    catalog.loot_defs.get(item.loot_idx as usize)
                                {
                                    self.draw_item_details(ui, def, item);

                                    if ui.button("Remove Item").clicked() {
                                        items.remove(orig_idx);
                                        selected_equipment_item_local = None;
                                    }
                                } else {
                                    ui.label("Item definition not found.");
                                }
                            } else {
                                ui.label("Catalog not loaded.");
                            }
                        }
                    } else {
                        ui.label("No item selected.");
                    }
                },
            );
        });

        self.selected_equipment_item = selected_equipment_item_local;
    }

    fn show_add_items_tab(&mut self, ui: &mut Ui, save: &mut SaveData) {
        let catalog = self.catalog.as_ref();
        let atlas = self.item_atlas.as_ref();
        let atlas_width = self.atlas_width;
        let atlas_height = self.atlas_height;

        let item_search_filter = &mut self.item_search_filter;

        let mut selected_catalog_item_local = self.selected_catalog_item;
        let mut add_item_count_local = self.add_item_count;
        let mut add_item_upgrade_local = self.add_item_upgrade;

        if let Some(catalog) = catalog {
            ui.horizontal(|ui| {
                ui.label("Search:");
                ui.text_edit_singleline(item_search_filter);
            });

            let full_width = ui.available_width();
            let height = ui.available_height();

            // Pre-group
            let mut grouped: std::collections::HashMap<
                String,
                Vec<(usize, &LootDef)>,
            > = std::collections::HashMap::new();

            for (idx, def) in catalog.loot_defs.iter().enumerate() {
                if !item_search_filter.is_empty()
                    && !def
                    .name
                    .to_lowercase()
                    .contains(&item_search_filter.to_lowercase())
                {
                    continue;
                }

                let type_name = loot_names::get_type_name(def.type_);
                let subtype_name = loot_names::get_subtype_name(def.type_, def.sub_type);
                let cat = format!("{} - {}", type_name, subtype_name);

                grouped.entry(cat).or_default().push((idx, def));
            }

            ui.horizontal(|ui| {
                // LEFT
                ui.allocate_ui_with_layout(
                    egui::vec2(full_width * 0.6, height),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        let mut clicked_item = None;

                        ScrollArea::both().show(ui, |ui| {
                            let mut categories: Vec<_> = grouped.keys().cloned().collect();
                            categories.sort();

                            for cat in categories {
                                let items = grouped.get(&cat).unwrap();

                                ui.style_mut().interaction.selectable_labels = false;
                                ui.label(egui::RichText::new(&cat).strong());

                                Grid::new(&cat)
                                    .spacing([8.0, 8.0])
                                    .show(ui, |ui| {
                                        for (idx, def) in items {
                                            let icon_uv = self.get_icon_uv(def, atlas, atlas_width as f32, atlas_height as f32);

                                            ui.vertical(|ui| {
                                                let response = self.draw_image_button(ui, icon_uv, atlas);
                                                let button_width = response.rect.width();

                                                if response.clicked() {
                                                    clicked_item = Some(*idx);
                                                }

                                                ui.set_max_width(button_width);

                                                Self::add_item_label(ui, def.title[0].clone(), self.config.item_font_size);
                                            });
                                        }
                                    });
                            }
                        });

                        if let Some(idx) = clicked_item {
                            selected_catalog_item_local = Some(idx);
                        }
                    },
                );

                // RIGHT
                ui.allocate_ui_with_layout(
                    egui::vec2(full_width * 0.4, height),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        if let Some(idx) = selected_catalog_item_local {
                            if let Some(def) = catalog.loot_defs.get(idx) {
                                let mut dummy_item = Item {
                                    loot_idx: idx as i32,
                                    count: add_item_count_local,
                                    upgrade: add_item_upgrade_local,
                                    stock_piled: false,
                                    artifact_seed: -1,
                                    item_version: 0,
                                    rarity: 1,
                                };

                                self.draw_item_details(ui, def, &mut dummy_item);

                                add_item_count_local = dummy_item.count;
                                add_item_upgrade_local = dummy_item.upgrade;

                                ui.horizontal(|ui| {
                                    if ui.button("Add to Inventory").clicked() {
                                        save.equipment.inventory_items.push(Item {
                                            loot_idx: idx as i32,
                                            count: add_item_count_local,
                                            upgrade: add_item_upgrade_local,
                                            stock_piled: false,
                                            artifact_seed: -1,
                                            item_version: 0,
                                            rarity: 1,
                                        });
                                    }

                                    if ui.button("Add to Stockpile").clicked() {
                                        save.equipment.inventory_items.push(Item {
                                            loot_idx: idx as i32,
                                            count: add_item_count_local,
                                            upgrade: add_item_upgrade_local,
                                            stock_piled: true,
                                            artifact_seed: -1,
                                            item_version: 0,
                                            rarity: 1,
                                        });
                                    }
                                });
                            }
                        } else {
                            ui.label("Select an item.");
                        }
                    },
                );
            });
        } else {
            ui.label("Catalog not loaded.");
        }

        self.selected_catalog_item = selected_catalog_item_local;
        self.add_item_count = add_item_count_local;
        self.add_item_upgrade = add_item_upgrade_local;
    }

    /// Draw detailed information about an item (name, title, description, fields, etc.)
    fn draw_item_details(&self,
                         ui: &mut Ui,
                         def: &LootDef,
                         item: &mut Item,
    ) {
        ui.heading("Item Details");
        ui.separator();

        ui.label(format!("Name: {}", def.name));

        if let Some(title) = def.title.first() {
            if !title.is_empty() {
                ui.label(format!("Title: {}", title));
            }
        }

        if let Some(desc) = def.description.first() {
            if !desc.is_empty() {
                ui.label(format!("Description: {}", desc));
            }
        }

        let type_name = loot_names::get_type_name(def.type_);
        let subtype_name = loot_names::get_subtype_name(def.type_, def.sub_type);
        ui.label(format!("Type: {} - {}", type_name, subtype_name));
        ui.label(format!("Cost: {:.0}", def.cost));

        ui.separator();

        ui.horizontal(|ui| {
            ui.label("Count:");
            ui.add(egui::DragValue::new(&mut item.count).speed(self.config.drag_value_sensitivity).range(0..=999));

            ui.label("Upgrade:");
            ui.add(egui::DragValue::new(&mut item.upgrade).speed(self.config.drag_value_sensitivity).range(0..=10));
        });

        ui.add_space(8.0);

        ui.separator();

        ui.collapsing(format!("Fields ({})", def.fields.len()), |ui| {
            ScrollArea::vertical()
                .max_height(150.0)
                .show(ui, |ui| {
                    for field in &def.fields {
                        let field_name = loot_names::get_field_name(def.type_, field.id);

                        let field_value = match &field.value {
                            sas2_save::loot_catalog::LootFieldValue::Float(v) => format!("{:.2}", v),
                            sas2_save::loot_catalog::LootFieldValue::Int(v) => v.to_string(),
                            sas2_save::loot_catalog::LootFieldValue::Bool(v) => v.to_string(),
                            sas2_save::loot_catalog::LootFieldValue::String(v) => v.clone(),
                        };

                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(field_name)
                                    .weak()
                                    .size(12.0),
                            );
                            ui.label(field_value);
                        });
                    }
                });
        });
    }

    pub fn add_ng_level_label(&self, ui: &mut Ui, save: &mut SaveData) {
        ui.horizontal(|ui| {
            ui.label("NG Level:");
            let mut ng = save.flags.ng_level;
            if ui.add(egui::DragValue::new(&mut ng).speed(self.config.drag_value_sensitivity).range(0..=999999)).changed() {
                ng_level::set_ng_level(&mut save.flags, ng);
            }
            ui.label("(This adds/removes the $&ng_X flag)");
        });
    }

    pub fn show_flags_ui(&mut self, ui: &mut Ui, save: &mut SaveData) {
        ui.heading("Player Flags");
        ui.separator();

        // Editable flags list
        ui.label("Flags:");
        ScrollArea::vertical()
            .max_height(300.0)
            .show(ui, |ui| {
                let mut to_remove = None;
                for (i, flag) in save.flags.flags.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        ui.text_edit_singleline(flag);
                        if ui.button("🗑").clicked() {
                            to_remove = Some(i);
                        }
                    });
                }
                if let Some(i) = to_remove {
                    save.flags.flags.remove(i);
                    ng_level::update_ng_level(&mut save.flags);
                }
            });

        // Add new flag
        ui.horizontal(|ui| {
            if ui.button("Add Flag").clicked() {
                save.flags.flags.push(String::new());
            }
        });

        ui.separator();

        // Editable bounty data
        ui.horizontal(|ui| {
            ui.label("Bounty Seed:");
            ui.add(egui::DragValue::new(&mut save.flags.bounty_seed).speed(self.config.drag_value_sensitivity).range(0..=999999));
        });
        ui.horizontal(|ui| {
            ui.label("Bounties Complete (bitmask):");
            ui.add(egui::DragValue::new(&mut save.flags.bounties_complete).speed(self.config.drag_value_sensitivity).range(0..=999999));
        });

        self.add_ng_level_label(ui, save);
        ui.label("Note: NG level is derived from flags.");
        ui.label("Flags that preserve across NG cycles: v$t_AREA_NOWHERE, dawnlight_saved, shroud_saved, blueheart_saved, oath_saved, sheriff_saved, chaos_saved. The flag \"$1ntr0\" is automatically added if missing.");
    }

    pub fn add_bestiary_details(&self, ui: &mut Ui, beast: &mut BestiaryBeast) {
        ui.horizontal(|ui| {
            ui.label("Kills:");
            ui.add(egui::DragValue::new(&mut beast.kills).speed(self.config.drag_value_sensitivity).range(0..=9999));
        });
        ui.horizontal(|ui| {
            ui.label("Deaths:");
            ui.add(egui::DragValue::new(&mut beast.deaths).speed(self.config.drag_value_sensitivity).range(0..=9999));
        });
        ui.label("Drops:");
        for (drop_idx, drop) in beast.drops.iter_mut().enumerate() {
            ui.checkbox(drop, format!("Drop {}", drop_idx));
        }
    }

    pub fn show_bestiary_ui(&mut self, ui: &mut Ui, save: &mut SaveData) {
        ui.heading("Bestiary");
        ui.separator();

        ScrollArea::vertical()
            .max_height(400.0)
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                if let Some(catalog) = &self.monster_catalog {
                    for (idx, beast) in save.bestiary.beasts.iter_mut().enumerate() {
                        let name = catalog
                            .monsters
                            .get(idx)
                            .map(|m| m.name.clone())
                            .unwrap_or_else(|| format!("Beast {}", idx));
                        ui.collapsing(format!("{} (ID: {})", name, idx), |ui| self.add_bestiary_details(ui, beast));
                    }
                } else {
                    for (idx, beast) in save.bestiary.beasts.iter_mut().enumerate() {
                        ui.collapsing(format!("Beast {}", idx), |ui| self.add_bestiary_details(ui, beast));
                    }
                    if let Some(err) = &self.monster_catalog_error {
                        ui.colored_label(egui::Color32::RED, format!("Monster catalog error: {}", err));
                    }
                }
            });
    }

    pub fn show_cosmetics_ui(&mut self, ui: &mut Ui, save: &mut SaveData) {
        ui.heading("Cosmetics");
        ui.separator();

        type NameFn = fn(usize) -> Option<&'static str>;

        let hair_choices: Vec<(usize, &'static str)> = {
            let indices = HairCatalog::get_ordered_indices();
            indices.into_iter().map(|idx| (idx, HairCatalog::name(idx).unwrap())).collect()
        };

        for slot_idx in 0..save.cosmetics.len() {
            let value = &mut save.cosmetics[slot_idx];
            let (label, name_fn, choices) = match slot_idx {
                0 => ("Sex", SexCatalog::name as NameFn, (0..SexCatalog::len()).collect()),
                1 => ("Ancestry", AncestryCatalog::name as NameFn, (0..AncestryCatalog::len()).collect()),
                2 => ("Eye Color", EyeCatalog::name as NameFn, (0..EyeCatalog::len()).collect()),
                3 => ("Hair", HairCatalog::name as NameFn, hair_choices.iter().map(|&(orig, _)| orig).collect()),
                4 => ("Hair Color", ColorCatalog::name as NameFn, (0..ColorCatalog::len()).collect()),
                5 => ("Beard", BeardCatalog::name as NameFn, (0..BeardCatalog::len()).collect()),
                6 => ("Beard Color", ColorCatalog::name as NameFn, (0..ColorCatalog::len()).collect()),
                7 => ("Eyebrow Color", ColorCatalog::name as NameFn, (0..ColorCatalog::len()).collect()),
                8 => ("Class", ClassCatalog::name as NameFn, (0..ClassCatalog::len()).collect()),
                9 => ("Crime", CrimeCatalog::name as NameFn, (0..CrimeCatalog::len()).collect()),
                10 => ("Unused", (|_| None) as NameFn, Vec::new()),
                _ => continue,
            };

            ui.horizontal(|ui| {
                ui.label(format!("{}:", label));

                if !choices.is_empty() {
                    // Unique ID per slot to avoid clashing
                    ui.push_id(slot_idx, |ui| {
                        let selected_text = name_fn(*value as usize)
                            .map(|s| s.to_string())
                            .unwrap_or_else(|| format!("{}", *value));

                        egui::ComboBox::from_label("")
                            .selected_text(selected_text)
                            .show_ui(ui, |ui| {
                                for &choice_idx in &choices {
                                    let choice_text = name_fn(choice_idx)
                                        .map(|s| s.to_string())
                                        .unwrap_or_else(|| format!("{}", choice_idx));
                                    ui.selectable_value(value, choice_idx as i32, choice_text);
                                }
                            });
                    });
                    // Small label showing the numeric value (instead of a large icon)
                    ui.add_space(8.0);
                    ui.colored_label(egui::Color32::GRAY, format!("{}", *value));
                } else {
                    // Fallback for unused slot
                    ui.add(egui::DragValue::new(value).speed(self.config.drag_value_sensitivity).range(0..=999));
                }
            });
        }
    }

    pub fn start_export_job(&mut self, files: Vec<PathBuf>, overwrite: bool) {
        use sas2_save::xnb_loader::{load_asset_from_xnb, export_asset_to_file, asset_extension};
        use std::sync::{
            atomic::{AtomicBool, AtomicUsize, Ordering},
            Arc,
        };

        let game_path = self.config.game_path.clone().unwrap();
        let cancel_flag = Arc::new(AtomicBool::new(false));
        let progress = Arc::new(AtomicUsize::new(0));
        let total = Arc::new(AtomicUsize::new(files.len()));
        let done = Arc::new(AtomicBool::new(false));

        self.export_state = Some(ExportState {
            progress: progress.clone(),
            total: total.clone(),
            cancel: cancel_flag.clone(),
            done: done.clone(),
        });

        let overwrite_flag = overwrite;

        std::thread::spawn(move || {
            let export_root = PathBuf::from("exports");
            let _ = fs::create_dir_all(&export_root);

            for path in files {
                if cancel_flag.load(Ordering::Relaxed) {
                    break;
                }

                // Compute relative path and output file
                let relative = path.strip_prefix(&game_path).unwrap_or(&path);
                let mut out_path = export_root.join(relative);
                //let original_ext = out_path.extension().unwrap_or_default().to_str().unwrap_or("xnb");
                out_path.set_extension(""); // remove .xnb

                // Load the asset and determine its type
                let asset = match load_asset_from_xnb(path.to_str().unwrap()) {
                    Ok(a) => a,
                    Err(e) => {
                        eprintln!("Failed to load {:?}: {}", path, e);
                        progress.fetch_add(1, Ordering::Relaxed);
                        continue;
                    }
                };

                // Determine final extension based on asset type
                let ext = asset_extension(&asset);
                out_path.set_extension(ext);

                if !overwrite_flag && out_path.exists() {
                    progress.fetch_add(1, Ordering::Relaxed);
                    continue;
                }

                if let Some(parent) = out_path.parent() {
                    let _ = fs::create_dir_all(parent);
                }

                if let Err(e) = export_asset_to_file(asset, &out_path) {
                    eprintln!("Failed to export {:?}: {}", out_path, e);
                }

                progress.fetch_add(1, Ordering::Relaxed);
            }

            done.store(true, Ordering::Relaxed);
        });
    }

    fn collect_selected(node: &XnbNode, out: &mut Vec<PathBuf>, parent_selected: bool) {
        let effective_selected = parent_selected && node.selected;

        if node.is_dir {
            for c in &node.children {
                Self::collect_selected(c, out, effective_selected);
            }
        } else if effective_selected {
            out.push(node.path.clone());
        }
    }

    fn draw_xnb_tree(ui: &mut Ui, node: &mut XnbNode) {
        if node.is_dir {
            // Create a unique ID for this header's state
            let id = ui.make_persistent_id(&node.name);
            let state = egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, false);

            // Build the header UI
            state.show_header(ui, |ui| {
                if ui.checkbox(&mut node.selected, &node.name).changed() {
                    let val = node.selected;
                    fn propagate(n: &mut XnbNode, val: bool) {
                        n.selected = val;
                        for c in &mut n.children {
                            propagate(c, val);
                        }
                    }
                    propagate(node, val);
                }
            })
                .body(|ui| {
                    // Draw children inside the body
                    for child in &mut node.children {
                        Self::draw_xnb_tree(ui, child);
                    }
                });
        } else {
            ui.checkbox(&mut node.selected, &node.name);
        }
    }

    pub fn export_assets(&mut self) {
        let game_path = match &self.config.game_path {
            Some(p) => p.clone(),
            None => {
                eprintln!("Game folder not set");
                return;
            }
        };

        fn build_tree(path: &Path) -> Option<XnbNode> {
            let mut node = XnbNode {
                name: path.file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.display().to_string()),
                path: path.to_path_buf(),
                is_dir: path.is_dir(),
                children: Vec::new(),
                selected: true,
            };

            if path.is_dir() {
                if let Ok(read) = fs::read_dir(path) {
                    for entry in read.flatten() {
                        let p = entry.path();
                        if p.is_dir() {
                            if let Some(child) = build_tree(&p) {
                                if !child.children.is_empty() {
                                    node.children.push(child);
                                }
                            }
                        } else if p.extension().map(|e| e.eq_ignore_ascii_case("xnb")).unwrap_or(false) {
                            node.children.push(XnbNode {
                                name: p.file_name().unwrap().to_string_lossy().to_string(),
                                path: p,
                                is_dir: false,
                                children: vec![],
                                selected: true,
                            });
                        }
                    }
                }
            }

            if node.is_dir && node.children.is_empty() {
                None
            } else {
                Some(node)
            }
        }

        self.export_picker = build_tree(&game_path);
        self.export_picker_open = true;
    }

    pub fn show_skilltree_ui(&mut self, ui: &mut Ui, save: &mut SaveData) {
        ui.heading("Skill Tree");
        ui.separator();

        // Ensure texture/catalog are loaded
        if self.skilltree_texture.is_none() && self.skilltree_catalog.is_some() {
            if let Some(game_path) = &self.config.game_path {
                if let Ok(tex) = load_skilltree_texture(game_path, ui.ctx()) {
                    self.skilltree_texture = Some(tex);
                }
            }
        }

        let catalog = match &self.skilltree_catalog {
            Some(c) => c,
            None => {
                ui.label("Skill tree catalog not loaded.");
                if let Some(err) = &self.skilltree_catalog_error {
                    ui.colored_label(egui::Color32::RED, format!("Error: {}", err));
                }
                return;
            }
        };

        let texture = match &self.skilltree_texture {
            Some(t) => t,
            None => {
                ui.label("Skill tree texture not loaded.");
                if let Some(err) = &self.skilltree_texture_error {
                    ui.colored_label(egui::Color32::RED, format!("Error: {}", err));
                }
                return;
            }
        };

        let total_height = ui.available_height();

        // Zoom controls
        ui.horizontal(|ui| {
            ui.label("Zoom:");
            ui.add(egui::Slider::new(&mut self.skilltree_zoom, 0.05..=4.0).logarithmic(true));
            if ui.button("Reset View").clicked() {
                self.skilltree_zoom = 0.5;
                self.skilltree_centered = false; // Force re-center on next frame
            }
        });
        ui.separator();

        ui.horizontal(|ui| {
            let panel_width = 280.0;
            let canvas_width = (ui.available_width() - panel_width - 10.0).max(0.0);

            let (response, painter) = ui.allocate_painter(
                egui::vec2(canvas_width, total_height),
                egui::Sense::click_and_drag(),
            );
            let canvas_rect = response.rect;

            // Auto-center the view on first view
            if !self.skilltree_centered {
                let mut min_x = f32::MAX;
                let mut max_x = f32::MIN;
                let mut min_y = f32::MAX;
                let mut max_y = f32::MIN;
                for node in &catalog.nodes {
                    min_x = min_x.min(node.loc_x);
                    max_x = max_x.max(node.loc_x);
                    min_y = min_y.min(node.loc_y);
                    max_y = max_y.max(node.loc_y);
                }
                let world_center = egui::vec2((min_x + max_x) / 2.0, (min_y + max_y) / 2.0);
                let canvas_center = canvas_rect.center();
                // screen = (world - scroll) * zoom + canvas.min
                // solve for scroll: canvas_center = (world_center - scroll) * zoom + canvas.min
                // => world_center - scroll = (canvas_center - canvas.min) / zoom
                // => scroll = world_center - (canvas_center - canvas.min) / zoom
                self.skilltree_scroll = world_center - (canvas_center - canvas_rect.min) / self.skilltree_zoom;
                self.skilltree_centered = true;
            }

            // Panning
            if response.dragged() {
                self.skilltree_scroll -= response.drag_delta() / self.skilltree_zoom;
            }

            // Zoom Logic
            if response.hovered() {
                let scroll = ui.input(|i| i.smooth_scroll_delta.y);
                if scroll != 0.0 {
                    let old_zoom = self.skilltree_zoom;
                    self.skilltree_zoom = (self.skilltree_zoom * (1.0 + scroll * 0.001)).clamp(0.05, 4.0);
                    let mouse = response.hover_pos().unwrap_or(canvas_rect.center());
                    let world_before = (mouse - canvas_rect.min) / old_zoom + self.skilltree_scroll;
                    let world_after = (mouse - canvas_rect.min) / self.skilltree_zoom + self.skilltree_scroll;
                    self.skilltree_scroll += world_before - world_after;
                }
            }

            let to_screen = |x: f32, y: f32| {
                pos2(
                    canvas_rect.min.x + (x - self.skilltree_scroll.x) * self.skilltree_zoom,
                    canvas_rect.min.y + (y - self.skilltree_scroll.y) * self.skilltree_zoom,
                )
            };

            // Draw connections
            for node in &catalog.nodes {
                let start = to_screen(node.loc_x, node.loc_y);
                for &parent_id in &node.parents {
                    if parent_id >= 0 {
                        if let Some(parent) = catalog.nodes.get(parent_id as usize) {
                            let end = to_screen(parent.loc_x, parent.loc_y);
                            let node_unlocked = save.stats.tree_unlocks[node.id] > 0 || save.stats.class_unlocks.contains(&(node.id as i32));
                            let parent_unlocked = save.stats.tree_unlocks[parent_id as usize] > 0 || save.stats.class_unlocks.contains(&(parent_id));
                            let line_color = if node_unlocked && parent_unlocked {
                                egui::Color32::from_rgb(255, 215, 0)
                            } else if node_unlocked || parent_unlocked {
                                egui::Color32::from_rgb(100, 100, 200)
                            } else {
                                egui::Color32::from_gray(80)
                            };
                            painter.line_segment([start, end], (2.0 * self.skilltree_zoom, line_color));
                        }
                    }
                }
            }

            let tex_size = texture.size_vec2();
            let tile_size = 128.0;
            let tiles_per_row = (tex_size.x / tile_size) as i32;

            // Draw nodes
            for node in &catalog.nodes {
                let screen_pos = to_screen(node.loc_x, node.loc_y);
                let base_icon_size = 64.0 * self.skilltree_zoom;
                let zoom_out_factor = 1.0 + (0.5 - self.skilltree_zoom).max(0.0) * 0.8333;
                let icon_display_size = base_icon_size * zoom_out_factor;
                let rect = Rect::from_center_size(screen_pos, egui::vec2(icon_display_size, icon_display_size));

                if !canvas_rect.intersects(rect) {
                    continue;
                }

                let icon_idx = if node.node_type >= 0 && (node.node_type as usize) < SKILL_IMG.len() {
                    SKILL_IMG[node.node_type as usize]
                } else { 0 };

                let tile_x = (icon_idx / tiles_per_row) as f32 * tile_size;
                let tile_y = (icon_idx % tiles_per_row) as f32 * tile_size;
                let uv = Rect::from_min_max(
                    pos2(tile_x / tex_size.x, tile_y / tex_size.y),
                    pos2((tile_x + tile_size) / tex_size.x, (tile_y + tile_size) / tex_size.y),
                );

                // Determine tint
                let is_selected = self.selected_skill_node == Some(node.id);
                let is_class_unlock = save.stats.class_unlocks.contains(&(node.id as i32));
                let current_level = save.stats.tree_unlocks[node.id];
                let max_level = node.max_unlock();
                let is_max_level = current_level >= max_level;

                let tint = if is_selected {
                    egui::Color32::CYAN
                } else if is_class_unlock {
                    egui::Color32::from_rgb(255, 200, 50)
                } else if is_max_level {
                    egui::Color32::YELLOW
                } else if current_level > 0 {
                    egui::Color32::WHITE
                } else {
                    egui::Color32::DARK_GRAY
                };

                if is_class_unlock {
                    let width = 2.0;
                    painter.circle_stroke(rect.center(), icon_display_size, Stroke::new(width, tint));

                    let rect_cent = rect.center();
                    let offset = icon_display_size * std::f32::consts::FRAC_1_SQRT_2;

                    let rect_rad_tl = pos2(rect_cent.x - offset, rect_cent.y - offset);
                    let rect_rad_br = pos2(rect_cent.x + offset, rect_cent.y + offset);

                    let rect_rad_tr = pos2(rect_cent.x + offset, rect_cent.y - offset);
                    let rect_rad_bl = pos2(rect_cent.x - offset, rect_cent.y + offset);

                    if node.id as i32 == save.stats.class_unlocks[0] {
                        painter.line_segment([pos2(rect_cent.x - icon_display_size, rect_cent.y), pos2(rect_cent.x + icon_display_size, rect_cent.y)], Stroke::new(width, tint));
                    } else if node.id as i32 == save.stats.class_unlocks[1] {
                        painter.line_segment([rect_rad_tr, rect_rad_bl], Stroke::new(width, tint));
                        painter.line_segment([rect_rad_tl, rect_rad_br], Stroke::new(width, tint));
                    } else if node.id as i32 == save.stats.class_unlocks[2] {
                        painter.line_segment([rect_rad_tr, rect_rad_bl], Stroke::new(width, tint));
                        painter.line_segment([rect_rad_tl, rect_rad_br], Stroke::new(width, tint));
                        painter.line_segment([pos2(rect_cent.x - icon_display_size, rect_cent.y), pos2(rect_cent.x + icon_display_size, rect_cent.y)], Stroke::new(width, tint));
                    }
                }
                painter.image(texture.id(), rect, uv, tint);

                let node_response = ui.interact(rect, egui::Id::new(node.id), egui::Sense::click());
                if node_response.clicked() {
                    self.selected_skill_node = Some(node.id);
                }

                // Draw circles for multi-level nodes
                if max_level > 1 {
                    let max_allowed_width = icon_display_size;
                    let mut circle_radius = (icon_display_size * 0.08).max(3.0);
                    let mut spacing = circle_radius * 2.5;
                    let mut total_width = (max_level - 1) as f32 * spacing;

                    if total_width > max_allowed_width {
                        spacing = max_allowed_width / (max_level - 1) as f32;
                        circle_radius = (spacing * 0.4).max(1.5);
                        total_width = (max_level - 1) as f32 * spacing;
                    }

                    let start_x = screen_pos.x - total_width / 2.0;
                    let circle_y = screen_pos.y + icon_display_size * 0.55;

                    for i in 0..max_level {
                        let circle_x = start_x + i as f32 * spacing;
                        let center = pos2(circle_x, circle_y);

                        let fill_color = if is_max_level || i < current_level {
                            tint
                        } else {
                            egui::Color32::DARK_GRAY
                        };

                        painter.circle_filled(center, circle_radius, fill_color);
                        painter.circle_stroke(center, circle_radius, (1.0, tint.gamma_multiply(0.6)));
                    }
                }
            }

            // Side panel
            ui.allocate_ui_with_layout(
                egui::vec2(panel_width, total_height),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    if let Some(id) = self.selected_skill_node {
                        if let Some(node) = catalog.nodes.get(id) {
                            ui.heading(&node.titles[0]);
                            ui.add_space(4.0);
                            ui.label(&node.descriptions[0]);
                            ui.separator();

                            ui.label(format!("Type: {}", node.stat_name().unwrap_or("Weapon/Glyph unlock")));
                            ui.label(format!("Value: {}", node.value));
                            ui.label(format!("Cost (pearls): {}", node.cost));

                            let mut val = save.stats.tree_unlocks[node.id];
                            ui.horizontal(|ui| {
                                ui.label("Unlock level:");
                                if ui.add(egui::DragValue::new(&mut val).range(0..=node.max_unlock()).speed(0.01)).changed() {
                                    save.stats.tree_unlocks[node.id] = val;
                                    SaveEditorApp::recalc_player_stats(save, catalog);
                                }
                            });

                            ui.add_space(8.0);
                            ui.label("Parents:");
                            for &p in &node.parents {
                                if p >= 0 {
                                    if let Some(parent) = catalog.nodes.get(p as usize) {
                                        ui.label(format!("- {}", parent.titles[0]));
                                    }
                                }
                            }

                            ui.add_space(8.0);
                            ui.horizontal(|ui| {
                                if ui.button("Set as Class Unlock 1").clicked() {
                                    save.stats.class_unlocks[0] = node.id as i32;
                                    SaveEditorApp::recalc_player_stats(save, catalog);
                                }
                            });
                            ui.horizontal(|ui| {
                                if ui.button("Set as Class Unlock 2").clicked() {
                                    save.stats.class_unlocks[1] = node.id as i32;
                                    SaveEditorApp::recalc_player_stats(save, catalog);
                                }
                            });
                            ui.horizontal(|ui| {
                                if ui.button("Set as Class Unlock 3").clicked() {
                                    save.stats.class_unlocks[2] = node.id as i32;
                                }
                            });

                            ui.add_space(8.0);
                            if ui.button("Close Details").clicked() {
                                self.selected_skill_node = None;
                            }
                        }
                    } else {
                        ui.vertical_centered(|ui| {
                            ui.add_space(20.0);
                            ui.label(egui::RichText::new("Select a node to edit").weak());
                            ui.add_space(8.0);
                            ui.label(egui::RichText::new("Class Unlocks (always active)").strong());
                            for i in 0..3 {
                                let class_id = save.stats.class_unlocks[i];
                                let name = if class_id >= 0 && (class_id as usize) < catalog.nodes.len() {
                                    catalog.nodes[class_id as usize].titles[0].clone()
                                } else {
                                    "None".to_string()
                                };
                                ui.horizontal(|ui| {
                                    ui.label(format!("Slot {}: {}", i, name));
                                    if ui.button("Clear").clicked() {
                                        save.stats.class_unlocks[i] = -1;
                                    }
                                });
                            }
                        });
                    }
                },
            );
        });
    }

    pub fn show_faction_ui(&mut self, ui: &mut Ui, save: &mut SaveData) {
        ui.heading("Player Faction");
        ui.separator();

        // Determine current faction from flags
        let current_faction = PlayerFaction::from_flags(&save.flags.flags);
        let mut selected = current_faction;

        ui.label("Faction:");
        egui::ComboBox::from_label("")
            .selected_text(current_faction.name())
            .show_ui(ui, |ui| {
                for faction in PlayerFaction::get_all() {
                    ui.selectable_value(&mut selected, *faction, faction.name());
                }
            });

        if selected != current_faction {
            // Update flags
            selected.apply_to_flags(&mut save.flags.flags);
            // Recompute ng_level after flag changes (just in case)
            ng_level::update_ng_level(&mut save.flags);
        }

        ui.separator();
        ui.label("Faction is determined by the presence of certain flags:");
        ui.label("dawnlight_saved -> Dawnlight Order");
        ui.label("shroud_saved -> Shrouded Alliance");
        ui.label("blueheart_saved -> Blueheart Runners");
        ui.label("sheriff_saved -> Sheriff Inquisitors");
        ui.label("oath_saved -> Oathbound Watchers");
        ui.label("chaos_saved -> Chaos Eaters");
        ui.label("(No flag means No Faction)");
    }

    pub fn show_settings_window(&mut self, ctx: &egui::Context) {
        if !self.settings_open {
            return;
        }

        let mut is_open = self.settings_open;

        let window = egui::Window::new("Configure UI")
            .open(&mut is_open)
            .resizable(false)
            .collapsible(false);

        window.show(ctx, |ui| {
            ui.vertical(|ui| {
                ui.heading("Item Display Settings");
                ui.horizontal(|ui| {
                    ui.label("Item Icon Size:");
                    if ui.add(egui::DragValue::new(&mut self.config.item_icon_size).range(32.0..=128.0).speed(self.config.drag_value_sensitivity).suffix("px")).changed() {
                        self.config.save();
                    }
                    if ui.button("Reset").clicked() {
                        self.config.item_icon_size = default_item_icon_size();
                        self.config.save();
                    };
                });
                ui.horizontal(|ui| {
                    ui.label("Item Font Size:");
                    if ui.add(egui::DragValue::new(&mut self.config.item_font_size).range(6.0..=24.0).speed(self.config.drag_value_sensitivity).suffix("pt")).changed() {
                        self.config.save();
                    }
                    if ui.button("Reset").clicked() {
                        self.config.item_font_size = default_item_font_size();
                        self.config.save();
                    };
                });
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("Drag Value Sensitivity:");
                    if ui.add(egui::DragValue::new(&mut self.config.drag_value_sensitivity).range(0.005..=1.0).speed(0.025).suffix("x")).changed() {
                        self.config.save();
                    }
                    if ui.button("Reset").clicked() {
                        self.config.drag_value_sensitivity = default_drag_sensitivity();
                        self.config.save();
                    };
                });
                ui.horizontal(|ui| {
                    ui.label("Test Drag Value Sensitivity:");
                    if ui.add(egui::DragValue::new(&mut self.config.dummy_drag_value).range(0.0..=1000.0).speed(self.config.drag_value_sensitivity).suffix("x")).changed() {
                        self.config.save();
                    }
                });
            })
        });

        self.settings_open = is_open;
    }

    /// Sanitizes a modded save for vanilla compatibility.
    /// - Removes items with invalid loot_idx (outside vanilla catalog)
    /// - Remaps equipped item indices accordingly
    /// - Clamps upgrade values to 0-10
    /// - Clamps item counts to 999
    fn sanitize_for_vanilla(&self, save: &mut SaveData) {
        let Some(catalog) = &self.catalog else {
            eprintln!("Cannot sanitize: loot catalog not loaded");
            return;
        };
        let max_valid_idx = catalog.loot_defs.len();

        // Build a mapping from old index to new index for items that survive
        let mut old_to_new = std::collections::HashMap::new();
        let mut new_inventory = Vec::new();

        for (old_idx, item) in save.equipment.inventory_items.iter().enumerate() {
            // Keep only items whose loot_idx is within vanilla catalog
            if (item.loot_idx as usize) < max_valid_idx {
                // Clamp upgrade to vanilla max (10)
                let mut sanitized = item.clone();
                if sanitized.upgrade > 10 {
                    sanitized.upgrade = 10;
                }
                // Clamp count to 999
                if sanitized.count > 999 {
                    sanitized.count = 999;
                }
                old_to_new.insert(old_idx, new_inventory.len());
                new_inventory.push(sanitized);
            }
        }

        save.equipment.inventory_items = new_inventory;

        // Remap equipped items
        for slot in &mut save.equipment.equipped_items {
            if *slot >= 0 {
                if let Some(&new_idx) = old_to_new.get(&(*slot as usize)) {
                    *slot = new_idx as i32;
                } else {
                    *slot = -1; // item was removed
                }
            }
        }
    }

    fn convert_to_vanilla(&mut self, save: &mut SaveData, target_version: i32) {
        if save.version <= 100 {
            self.error_message = Some("Save is already vanilla".to_string());
            return;
        }

        let start_dir = self.file_path.as_ref().and_then(|p| p.parent()).map(|p| p.to_path_buf());
        let mut dialog = FileDialog::new()
            .add_filter("Salt and Sacrifice Save", &["slv"])
            .set_file_name("converted.slv");
        if let Some(dir) = start_dir {
            dialog = dialog.set_directory(dir);
        }

        if let Some(path) = dialog.save_file() {
            self.sanitize_for_vanilla(save);
            match save.to_vanilla_bytes(target_version) {
                Ok(data) => {
                    if let Err(e) = fs::write(&path, data) {
                        self.error_message = Some(format!("Failed to write file: {}", e));
                    } else {
                        self.error_message = None;
                        // Reload the newly created vanilla save
                        if let Ok(new_data) = fs::read(&path) {
                            if let Ok(new_save) = SaveData::from_bytes(&new_data) {
                                self.save_data = Some(new_save);
                                self.file_path = Some(path);
                                self.hash_edit_string.clear();
                                self.active_tab = Tab::Stats; // switch to Stats tab
                            }
                        }
                    }
                }
                Err(e) => self.error_message = Some(format!("Conversion failed: {}", e)),
            }
        } else {
            self.error_message = Some("Save dialog cancelled".to_string());
        }
    }

    pub fn show_convert_save_ui(&mut self, ui: &mut Ui, save: &mut SaveData) {
        if save.version <= 100 {
            ui.label("This save is already vanilla. No conversion needed.");
            return;
        }

        ui.heading("Convert Modded Save to Vanilla");
        ui.separator();
        ui.label(format!("Current save version: {} (modded)", save.version));
        ui.label("Select target vanilla version:");
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.conversion_target_version, 18, "Version 18");
            ui.selectable_value(&mut self.conversion_target_version, 19, "Version 19");
        });
        ui.separator();
        ui.label("Warning: Mod only item data will be lost. (artifact seed, rarity, etc.)");
        ui.label("The resulting save should be compatible with the unmodded game.");
        ui.label("This was only tested with Saltguard, backups will be created with the suffix .X.bak where X is a number.");
        ui.separator();
        if ui.button("Convert and Save As...").clicked() {
            self.convert_to_vanilla(save, self.conversion_target_version);
        }
    }
}

impl eframe::App for SaveEditorApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut Frame) {
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

            // Show game folder status
            if let Some(game_path) = &self.config.game_path {
                ui.label(format!("Game folder: {}", game_path.display()));
            } else {
                ui.colored_label(egui::Color32::YELLOW, "Game folder not set (needed for item names, icons, and bestiary names)");
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

            ui.separator();

            // Fix borrow checker: take save_data out of self temporarily
            let mut save_taken = self.save_data.take();

            if let Some(save) = &mut save_taken {
                // Tab bar
                egui::Panel::top("tabs").show_separator_line(false).show_inside(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.selectable_value(&mut self.active_tab, Tab::Stats, "Stats");
                        ui.selectable_value(&mut self.active_tab, Tab::Equipment, "Equipment");
                        ui.selectable_value(&mut self.active_tab, Tab::SkillTree, "Skill Tree");
                        ui.selectable_value(&mut self.active_tab, Tab::Cosmetics, "Cosmetics");
                        ui.selectable_value(&mut self.active_tab, Tab::Flags, "Flags");
                        ui.selectable_value(&mut self.active_tab, Tab::Bestiary, "Bestiary");
                        ui.selectable_value(&mut self.active_tab, Tab::Faction, "Faction");
                        // Only show ConvertSave tab if the save is modded
                        if save.version > 100 {
                            ui.selectable_value(&mut self.active_tab, Tab::ConvertSave, "Convert Save");
                        }
                    });
                });

                ui.separator();

                // Tab content
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

            if let Some(state) = self.export_state.as_ref() {
                let progress = state.progress.load(std::sync::atomic::Ordering::Relaxed);
                let total = state.total.load(std::sync::atomic::Ordering::Relaxed);
                let done = state.done.load(std::sync::atomic::Ordering::Relaxed);

                let cancel = state.cancel.clone();

                let mut should_cancel = false;
                let mut should_close = false;

                egui::Window::new("Exporting XNB Files")
                    .collapsible(false)
                    .resizable(false)
                    .show(ui.ctx(), |ui| {
                        let fraction = if total > 0 {
                            progress as f32 / total as f32
                        } else {
                            0.0
                        };

                        ui.add(egui::ProgressBar::new(fraction).show_percentage());
                        ui.label(format!("{}/{} files exported", progress, total));

                        if !done {
                            if ui.button("Cancel").clicked() {
                                should_cancel = true;
                            }
                        } else {
                            ui.label("Done ✅");
                            if ui.button("Close").clicked() {
                                should_close = true;
                            }
                        }
                    });

                if should_cancel {
                    cancel.store(true, std::sync::atomic::Ordering::Relaxed);
                }

                if should_close {
                    self.export_state = None;
                }
            }

            if self.export_picker_open {
                let mut start_export = false;

                egui::Window::new("Select XNB files to export")
                    .resizable(true)
                    .vscroll(true)
                    .show(ui.ctx(), |ui| {
                        if let Some(root) = &mut self.export_picker {
                            ScrollArea::vertical().show(ui, |ui| {
                                Self::draw_xnb_tree(ui, root);
                            });
                        }

                        ui.separator();

                        ui.horizontal(|ui| {
                            if ui.button("Start Export").clicked() {
                                start_export = true;
                            }
                            if ui.button("Cancel").clicked() {
                                self.export_picker_open = false;
                            }
                        });

                        ui.separator();

                        ui.checkbox(&mut self.export_overwrite, "Overwrite existing PNGs");
                    });

                if start_export {
                    if let Some(root) = &self.export_picker {
                        let mut files = Vec::new();
                        Self::collect_selected(root, &mut files, true);

                        self.start_export_job(files, self.export_overwrite);
                    }

                    self.export_picker_open = false;
                    self.export_picker = None;
                }
            }
        });
    }
}