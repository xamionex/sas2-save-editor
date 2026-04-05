use crate::app::SaveEditorApp;
use eframe::egui;
use egui::{Grid, Ui};
use sas2_save::skilltree::SkillTreeCatalog;
use sas2_save::types::ng_level;
use sas2_save::{Item, SaveData};

impl SaveEditorApp {
    /// Recalculates the nine primary stats from the skill tree unlocks.
    /// This mirrors the game's `PlayerStats.UpdateStats()` for all node types.
    pub fn recalc_player_stats(save: &mut SaveData, catalog: &SkillTreeCatalog) {
        // Reset all stats to the game's baseline of 5
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
                // Direct stat nodes (types 0–8 map 1:1 to the stat array)
                0..=8 => {
                    let stat_idx = node.node_type as usize;
                    if node.value > 1 {
                        // Fixed-value node (e.g. always +2 or +3)
                        save.stats.stats[stat_idx] += node.value;
                    } else {
                        // Multi-level node: add the number of times it's been unlocked
                        let add = if save.stats.tree_unlocks[node.id] > 0 {
                            save.stats.tree_unlocks[node.id]
                        } else {
                            1
                        };
                        save.stats.stats[stat_idx] += add;
                    }
                }
                // Weapon/glyph unlock nodes, they grant `cost` points to a specific stat.
                // Mapping comes from the decompiled C# switch statement:
                // 9,20,23,29 -> Strength
                // 10,22,30 -> Will
                // 11,16 -> Vitality
                // 12,13,15,19 -> Dexterity
                // 14,28 -> Conviction
                // 17,27 -> Arcana
                // 18,25,26 -> Endurance
                // 21 -> Resolve
                // 24,31 -> Luck
                9 | 20 | 23 | 29 => save.stats.stats[0] += node.cost,
                10 | 22 | 30 => save.stats.stats[3] += node.cost,
                11 | 16 => save.stats.stats[2] += node.cost,
                12 | 13 | 15 | 19 => save.stats.stats[1] += node.cost,
                14 | 28 => save.stats.stats[6] += node.cost,
                17 | 27 => save.stats.stats[5] += node.cost,
                18 | 25 | 26 => save.stats.stats[4] += node.cost,
                21 => save.stats.stats[7] += node.cost,
                24 | 31 => save.stats.stats[8] += node.cost,
                _ => {} // Unknown node type, ignore silently
            }
        }
    }

    /// Small helper drawn both here and on the flags tab.
    pub fn add_ng_level_label(&self, ui: &mut Ui, save: &mut SaveData) {
        ui.horizontal(|ui| {
            ui.label("NG Level:");
            let mut ng = save.flags.ng_level;
            if ui
                .add(
                    egui::DragValue::new(&mut ng)
                        .speed(self.config.drag_value_sensitivity)
                        .range(0..=999999),
                )
                .changed()
            {
                ng_level::set_ng_level(&mut save.flags, ng);
            }
            ui.label("(This adds/removes the $&ng_X flag)");
        });
    }

    pub fn show_stats_ui(&mut self, ui: &mut Ui, save: &mut SaveData) {
        // Recalculate whenever the skill tree changes (stats_dirty is set there)
        if self.stats_dirty {
            if let Some(catalog) = &self.skilltree_catalog {
                SaveEditorApp::recalc_player_stats(save, catalog);
            }
            self.stats_dirty = false;
        }

        ui.horizontal(|ui| {
            ui.label("Player Name:");
            ui.text_edit_singleline(&mut save.name);
        });

        ui.horizontal(|ui| {
            let old_level = save.stats.level;
            ui.label("Level:");
            if ui
                .add(
                    egui::DragValue::new(&mut save.stats.level)
                        .speed(self.config.drag_value_sensitivity)
                        .range(1..=999999),
                )
                .changed()
            {
                if self.config.adjust_black_pearls_on_level_change {
                    let delta = save.stats.level - old_level;
                    if delta != 0 {
                        let black_idx = self.catalog.as_ref().and_then(|c| c.black_starstone_index);
                        Self::adjust_startstone(save, black_idx, delta);
                    }
                }
                self.stats_dirty = true;
            }
            ui.checkbox(
                &mut self.config.adjust_black_pearls_on_level_change,
                "Sync Black Starstones when changing",
            );
        });

        self.add_ng_level_label(ui, save);

        ui.horizontal(|ui| {
            ui.label("XP:");
            ui.add(
                egui::DragValue::new(&mut save.stats.xp)
                    .speed(100)
                    .range(0..=999999),
            );
        });
        ui.horizontal(|ui| {
            ui.label("Silver:");
            ui.add(
                egui::DragValue::new(&mut save.stats.silver)
                    .speed(100)
                    .range(0..=999999),
            );
        });
        ui.horizontal(|ui| {
            ui.label("Time Played (seconds):");
            ui.add(
                egui::DragValue::new(&mut save.stats.time_played)
                    .speed(1.0)
                    .range(0.0..=1e9),
            );
        });
        ui.horizontal(|ui| {
            ui.label("Hazeburnt:");
            ui.checkbox(&mut save.stats.hazeburnt, "");
        });

        ui.checkbox(
            &mut self.use_custom_hash,
            "Use custom hash (disable auto-recalculation on save)",
        );
        if self.use_custom_hash {
            ui.label(
                "Warning: The game might reject the save if the custom hash doesn't match the actual data.",
            );
        }

        if let Some(hash) = &mut save.hash_data {
            // Populate the edit buffer on first display (or after a reload)
            if self.hash_edit_string.is_empty() {
                self.hash_edit_string = hash.iter().map(|b| format!("{:02x}", b)).collect();
            }

            ui.horizontal(|ui| {
                ui.label("Save Hash (MD5):");
                let response = ui.text_edit_singleline(&mut self.hash_edit_string);

                if response.changed() {
                    let valid = self.hash_edit_string.len() == 32
                        && self.hash_edit_string.chars().all(|c| c.is_ascii_hexdigit());

                    if !valid {
                        ui.colored_label(
                            egui::Color32::RED,
                            "Invalid hash (must be 32 hex characters)",
                        );
                    } else {
                        let mut new_hash = [0u8; 16];
                        for i in 0..16 {
                            new_hash[i] =
                                u8::from_str_radix(&self.hash_edit_string[i * 2..i * 2 + 2], 16)
                                    .unwrap();
                        }
                        *hash = new_hash;
                    }
                }
            });
        } else {
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
                    ui.label(format!("{}: {}", name, save.stats.stats[i]));
                    // Game recomputes these from the skill tree, editing them directly would just get overwritten, so we display them read-only.
                    ui.end_row();
                }
            });
    }

    /// Add or subtract from a starstone stack in the inventory.
    /// Creates a new stack when adding and none exists yet.
    pub fn adjust_startstone(save: &mut SaveData, stone_idx: Option<usize>, delta: i32) {
        let Some(idx) = stone_idx else { return };
        let items = &mut save.equipment.inventory_items;

        if let Some(item) = items.iter_mut().find(|i| i.loot_idx == idx as i32) {
            let new_count = item.count + delta;
            item.count = new_count.max(0);
        } else if delta > 0 {
            items.push(Item {
                loot_idx: idx as i32,
                count: delta,
                upgrade: 0,
                stock_piled: false,
                artifact_seed: -1,
                item_version: 0,
                rarity: 1,
            });
        }
    }
}
