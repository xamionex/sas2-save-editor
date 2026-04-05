use crate::app::SaveEditorApp;
use eframe::egui;
use egui::{pos2, Rect, Stroke, Ui};
use sas2_save::skilltree::{SkillTreeCatalog, SKILL_IMG};
use sas2_save::SaveData;

impl SaveEditorApp {
    /// Sum of (node.cost * unlock_level) across the entire tree.
    /// Used to check whether the player has spent more points than their level allows.
    pub fn calculate_total_spent_starstones(save: &SaveData, catalog: &SkillTreeCatalog) -> i32 {
        let mut total = 0;
        for node in &catalog.nodes {
            if let Some(&level) = save.stats.tree_unlocks.get(node.id) {
                total += node.cost * level;
            }
        }
        total
    }

    pub fn show_skilltree_ui(&mut self, ui: &mut Ui, save: &mut SaveData) {
        // Need both catalog and texture to render the tree
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

        // Controls
        ui.horizontal(|ui| {
            ui.label("Zoom:");
            ui.add(egui::Slider::new(&mut self.skilltree_zoom, 0.05..=4.0).logarithmic(true));
            if ui.button("Reset View").clicked() {
                self.skilltree_zoom = 0.5;
                self.skilltree_centered = false; // will re-center next frame
            }
            ui.label(egui::RichText::new("Shift+Click=Quick +1").weak());
            ui.label(egui::RichText::new("Right Click=Quick -1").weak());
            ui.label(egui::RichText::new("Middle Click=Toggle Max").weak());
        });

        ui.horizontal(|ui| {
            if ui
                .checkbox(
                    &mut self.config.sync_black_starstones,
                    "Sync Black Starstones",
                )
                .changed()
            {
                self.config.save();
            }
            if ui
                .checkbox(
                    &mut self.config.add_gray_starstones,
                    "Add Gray Starstones on upgrade",
                )
                .changed()
            {
                self.config.save();
            }
            if ui
                .checkbox(
                    &mut self.config.remove_gray_starstones,
                    "Remove Gray Starstones on downgrade",
                )
                .changed()
            {
                self.config.save();
            }
        });

        // Black/gray starstone indices from the loot catalog (used for starstone sync)
        let black_idx = self.catalog.as_ref().and_then(|c| c.black_starstone_index);
        let gray_idx = self.catalog.as_ref().and_then(|c| c.gray_starstone_index);
        let mut black_starstone_count = 0;
        let mut gray_starstone_count = 0;

        ui.horizontal(|ui| {
            let total_spent = Self::calculate_total_spent_starstones(save, catalog);
            let color = if total_spent > save.stats.level - 1 {
                egui::Color32::RED
            } else {
                egui::Color32::GREEN
            };

            ui.checkbox(&mut self.config.account_for_level, "Account for level");

            ui.checkbox(
                &mut self.config.account_for_starstones,
                "Account for starstones",
            );
            ui.label(
                egui::RichText::new(format!(
                    "Points Spent: {} / {}",
                    total_spent,
                    save.stats.level - 1
                ))
                .color(color),
            );

            // Show small starstone icons with counts
            let icon_size = self.config.item_icon_size * 0.5;
            for (idx_opt, label) in [(black_idx, "Black"), (gray_idx, "Gray")] {
                if let (Some(idx), Some(atlas), Some(cat)) =
                    (idx_opt, self.item_atlas.as_ref(), self.catalog.as_ref())
                {
                    if let Some(def) = cat.loot_defs.get(idx) {
                        if let Some(uv) = atlas.icon_uv(def) {
                            ui.separator();
                            ui.add(
                                egui::Image::from_texture(&atlas.texture)
                                    .uv(uv)
                                    .fit_to_exact_size(egui::vec2(icon_size, icon_size)),
                            )
                            .on_hover_text(label);

                            let count = save
                                .equipment
                                .inventory_items
                                .iter()
                                .find(|i| i.loot_idx == idx as i32)
                                .map(|i| i.count)
                                .unwrap_or(0);

                            if idx == gray_idx.unwrap() {
                                gray_starstone_count = count;
                            } else if idx == black_idx.unwrap() {
                                black_starstone_count = count;
                            }

                            ui.label(egui::RichText::new(count.to_string()).strong());
                        }
                    }
                }
            }
        });

        ui.separator();

        let full_width = ui.available_width();
        let min_size = 250.0;
        let panel_width = if self.config.skilltree_panel_width > 0.0 {
            self.config.skilltree_panel_width.max(min_size)
        } else {
            full_width * 0.5
        };

        // Side panel (node details / class unlocks)
        let right_panel = egui::Panel::right("item_details")
            .resizable(true)
            .default_size(panel_width)
            .min_size(min_size)
            .max_size(full_width * 0.8)
            .size_range(min_size..=full_width * 0.8)
            .show_inside(ui, |ui| {
                ui.set_min_width(ui.available_width());

                if let Some(id) = self.selected_skill_node {
                    if let Some(node) = catalog.nodes.get(id) {
                        ui.heading(&node.titles[0]);
                        ui.add_space(4.0);
                        ui.label(&node.descriptions[0]);
                        ui.separator();

                        ui.label(format!(
                            "Type: {}",
                            node.stat_name().unwrap_or("Weapon/Glyph unlock")
                        ));
                        ui.label(format!("Value: {}", node.value));
                        ui.label(format!("Cost (starstones): {}", node.cost));

                        let mut val = save.stats.tree_unlocks[node.id];
                        ui.horizontal(|ui| {
                            ui.label("Unlock level:");
                            if ui
                                .add(
                                    egui::DragValue::new(&mut val)
                                        .range(0..=node.max_unlock())
                                        .speed(0.01),
                                )
                                .changed()
                            {
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
                        for (slot, label) in [
                            (0, "Set as Class Unlock 1"),
                            (1, "Set as Class Unlock 2"),
                            (2, "Set as Class Unlock 3"),
                        ] {
                            ui.horizontal(|ui| {
                                if ui.button(label).clicked() {
                                    save.stats.class_unlocks[slot] = node.id as i32;
                                    // Slot 3 doesn't trigger recalc in the original code, kept as-is
                                    if slot < 2 {
                                        SaveEditorApp::recalc_player_stats(save, catalog);
                                    }
                                }
                            });
                        }

                        ui.add_space(8.0);
                        if ui.button("Close Details").clicked() {
                            self.selected_skill_node = None;
                        }
                    }
                } else {
                    ui.vertical(|ui| {
                        ui.add_space(20.0);
                        ui.label(egui::RichText::new("Select a node to edit").weak());
                        ui.add_space(8.0);
                        ui.label(egui::RichText::new("Class Unlocks (always active)").strong());

                        for i in 0..3 {
                            let class_id = save.stats.class_unlocks[i];
                            let name = if class_id >= 0 && (class_id as usize) < catalog.nodes.len()
                            {
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
            });

        let actual_width = right_panel.response.rect.width();
        if (actual_width - self.config.skilltree_panel_width).abs() > 0.1 {
            self.config.skilltree_panel_width = actual_width;
            self.config_save_timer = 0.5;
        }

        egui::CentralPanel::default().show_inside(ui, |ui| {
            let canvas_rect = ui.available_rect_before_wrap();

            if self.skilltree_centered {
                if let Some(prev_rect) = self.prev_canvas_rect {
                    if prev_rect != canvas_rect {
                        let old_center = prev_rect.center();
                        let new_center = canvas_rect.center();
                        let delta_screen = new_center - old_center;
                        let delta_world = delta_screen / self.skilltree_zoom;
                        self.skilltree_scroll -= delta_world;
                    }
                }
            }
            self.prev_canvas_rect = Some(canvas_rect);

            let (response, painter) =
                ui.allocate_painter(canvas_rect.size(), egui::Sense::click_and_drag());

            // Auto-center on first render by computing the bounding box of all nodes
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
                // Rearranged from: screen = (world - scroll) * zoom + canvas.min
                self.skilltree_scroll =
                    world_center - (canvas_center - canvas_rect.min) / self.skilltree_zoom;
                self.skilltree_centered = true;
            }

            // Panning
            if response.dragged() {
                self.skilltree_scroll -= response.drag_delta() / self.skilltree_zoom;
            }

            // Zoom around the mouse cursor
            if response.hovered() {
                let scroll = ui.input(|i| i.smooth_scroll_delta.y);
                if scroll != 0.0 {
                    let old_zoom = self.skilltree_zoom;
                    self.skilltree_zoom =
                        (self.skilltree_zoom * (1.0 + scroll * 0.001)).clamp(0.05, 4.0);
                    let mouse = response.hover_pos().unwrap_or(canvas_rect.center());
                    let world_before = (mouse - canvas_rect.min) / old_zoom + self.skilltree_scroll;
                    let world_after =
                        (mouse - canvas_rect.min) / self.skilltree_zoom + self.skilltree_scroll;
                    self.skilltree_scroll += world_before - world_after;
                }
            }

            let to_screen = |x: f32, y: f32| {
                pos2(
                    canvas_rect.min.x + (x - self.skilltree_scroll.x) * self.skilltree_zoom,
                    canvas_rect.min.y + (y - self.skilltree_scroll.y) * self.skilltree_zoom,
                )
            };

            // Connection lines
            for node in &catalog.nodes {
                let start = to_screen(node.loc_x, node.loc_y);
                for &parent_id in &node.parents {
                    if parent_id < 0 {
                        continue;
                    }
                    if let Some(parent) = catalog.nodes.get(parent_id as usize) {
                        let end = to_screen(parent.loc_x, parent.loc_y);

                        let node_unlocked = save.stats.tree_unlocks[node.id] > 0
                            || save.stats.class_unlocks.contains(&(node.id as i32));
                        let parent_unlocked = save.stats.tree_unlocks[parent_id as usize] > 0
                            || save.stats.class_unlocks.contains(&(parent_id));

                        let line_color = if node_unlocked && parent_unlocked {
                            egui::Color32::from_rgb(255, 215, 0) // both unlocked — gold
                        } else if node_unlocked || parent_unlocked {
                            egui::Color32::from_rgb(100, 100, 200) // one side — blue-ish
                        } else {
                            egui::Color32::from_gray(80) // neither — dark grey
                        };

                        painter.line_segment([start, end], (2.0 * self.skilltree_zoom, line_color));
                    }
                }
            }

            let tex_size = texture.size_vec2();
            let tile_size = 128.0;
            let tiles_per_row = (tex_size.x / tile_size) as i32;

            // Nodes
            for node in &catalog.nodes {
                let screen_pos = to_screen(node.loc_x, node.loc_y);

                // Icon size scales with zoom but grows a bit extra when zoomed out
                let base_size = 64.0 * self.skilltree_zoom;
                let zoom_out_bonus = 1.0 + (0.5 - self.skilltree_zoom).max(0.0) * 0.8333;
                let icon_display_size = base_size * zoom_out_bonus;

                let rect = Rect::from_center_size(
                    screen_pos,
                    egui::vec2(icon_display_size, icon_display_size),
                );

                // Skip nodes that are completely off-screen
                if !canvas_rect.intersects(rect) {
                    continue;
                }

                let icon_idx = SKILL_IMG.get(node.node_type as usize).copied().unwrap_or(0);

                let tile_x = (icon_idx / tiles_per_row) as f32 * tile_size;
                let tile_y = (icon_idx % tiles_per_row) as f32 * tile_size;
                let uv = Rect::from_min_max(
                    pos2(tile_x / tex_size.x, tile_y / tex_size.y),
                    pos2(
                        (tile_x + tile_size) / tex_size.x,
                        (tile_y + tile_size) / tex_size.y,
                    ),
                );

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

                // Class unlock nodes get decorative markers (line, X, asterisk)
                if is_class_unlock {
                    let w = 2.0;
                    painter.circle_stroke(rect.center(), icon_display_size, Stroke::new(w, tint));

                    let c = rect.center();
                    let r = icon_display_size * std::f32::consts::FRAC_1_SQRT_2;

                    let tl = pos2(c.x - r, c.y - r);
                    let br = pos2(c.x + r, c.y + r);
                    let tr = pos2(c.x + r, c.y - r);
                    let bl = pos2(c.x - r, c.y + r);

                    if node.id as i32 == save.stats.class_unlocks[0] {
                        // Horizontal line for the first class unlock
                        painter.line_segment(
                            [
                                pos2(c.x - icon_display_size, c.y),
                                pos2(c.x + icon_display_size, c.y),
                            ],
                            Stroke::new(w, tint),
                        );
                    } else if node.id as i32 == save.stats.class_unlocks[1] {
                        painter.line_segment([tr, bl], Stroke::new(w, tint));
                        painter.line_segment([tl, br], Stroke::new(w, tint));
                    } else if node.id as i32 == save.stats.class_unlocks[2] {
                        painter.line_segment([tr, bl], Stroke::new(w, tint));
                        painter.line_segment([tl, br], Stroke::new(w, tint));
                        painter.line_segment(
                            [
                                pos2(c.x - icon_display_size, c.y),
                                pos2(c.x + icon_display_size, c.y),
                            ],
                            Stroke::new(w, tint),
                        );
                    }
                }

                painter.image(texture.id(), rect, uv, tint);

                // Interaction
                let node_response = ui.interact(rect, egui::Id::new(node.id), egui::Sense::click());
                let single_click = node_response.clicked();
                let middle_click = node_response.middle_clicked();
                let modifiers = node_response.ctx.input(|i| i.modifiers);

                let total_spent = Self::calculate_total_spent_starstones(save, catalog);
                let level_limit = save.stats.level - 1;

                let level_ok = !self.config.account_for_level || total_spent < level_limit;
                let starstone_ok =
                    !self.config.account_for_starstones || black_starstone_count >= node.cost;

                // SHIFT+LMB add 1 level (costs node.cost black stones)
                if single_click && modifiers.shift {
                    if current_level < max_level && level_ok && starstone_ok {
                        save.stats.tree_unlocks[node.id] = current_level + 1;
                        self.stats_dirty = true;

                        if self.config.sync_black_starstones {
                            Self::adjust_startstone(save, black_idx, -node.cost);
                        }
                        if self.config.add_gray_starstones {
                            Self::adjust_startstone(save, gray_idx, node.cost);
                        }
                    }
                }
                // RMB: remove 1 level (refunds node.cost black stones)
                else if node_response.secondary_clicked() {
                    if current_level > 0 {
                        save.stats.tree_unlocks[node.id] = current_level - 1;
                        self.stats_dirty = true;

                        if self.config.sync_black_starstones {
                            Self::adjust_startstone(save, black_idx, node.cost);
                        }
                        if self.config.remove_gray_starstones {
                            Self::adjust_startstone(save, gray_idx, -node.cost);
                        }
                    }
                }
                // MMB: toggle between 0 and maximum affordable levels
                else if middle_click {
                    if current_level > 0 {
                        // Downgrade to 0: refund all spent stones
                        let total_cost = current_level * node.cost;
                        if self.config.sync_black_starstones {
                            Self::adjust_startstone(save, black_idx, total_cost);
                        }
                        if self.config.remove_gray_starstones {
                            Self::adjust_startstone(save, gray_idx, -total_cost);
                        }
                        save.stats.tree_unlocks[node.id] = 0;
                        self.stats_dirty = true;
                    } else {
                        // Upgrade to as many levels as we can afford (level limit + black stones)
                        let max_by_level = if self.config.account_for_level {
                            (level_limit - total_spent).max(0)
                        } else {
                            max_level
                        };
                        let max_by_stones = if self.config.account_for_starstones && node.cost > 0 {
                            (black_starstone_count / node.cost).min(max_level)
                        } else {
                            max_level
                        };
                        let points_to_add = max_level.min(max_by_level).min(max_by_stones);

                        if points_to_add > 0 {
                            let total_cost = points_to_add * node.cost;
                            save.stats.tree_unlocks[node.id] = points_to_add;
                            self.stats_dirty = true;

                            if self.config.sync_black_starstones {
                                Self::adjust_startstone(save, black_idx, -total_cost);
                            }
                            if self.config.add_gray_starstones {
                                Self::adjust_startstone(save, gray_idx, total_cost);
                            }
                        }
                    }
                }
                // LMB: select node for details panel
                else if single_click && !modifiers.any() {
                    self.selected_skill_node = Some(node.id);
                }

                // Progress dots for multi-level nodes
                if max_level > 1 {
                    let max_allowed = icon_display_size;
                    let mut radius = (icon_display_size * 0.08).max(3.0);
                    let mut spacing = radius * 2.5;
                    let mut total_w = (max_level - 1) as f32 * spacing;

                    if total_w > max_allowed {
                        spacing = max_allowed / (max_level - 1) as f32;
                        radius = (spacing * 0.4).max(1.5);
                        total_w = (max_level - 1) as f32 * spacing;
                    }

                    let start_x = screen_pos.x - total_w / 2.0;
                    let dot_y = screen_pos.y + icon_display_size * 0.55;

                    for i in 0..max_level {
                        let center = pos2(start_x + i as f32 * spacing, dot_y);
                        let fill = if is_max_level || i < current_level {
                            tint
                        } else {
                            egui::Color32::DARK_GRAY
                        };
                        painter.circle_filled(center, radius, fill);
                        painter.circle_stroke(center, radius, (1.0, tint.gamma_multiply(0.6)));
                    }
                }
            }
        });
    }
}
