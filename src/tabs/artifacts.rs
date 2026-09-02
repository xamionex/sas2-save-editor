use crate::app::SaveEditor;
use crate::artifact::{
    ARTIFACT_FIELDS, ArtifactBoostOverride, ArtifactMatch, ResultSortKey, SearchTierScope,
    artifact_main_field, artifact_rarity, artifact_seed, artifact_tier, charm_effective,
    charm_unit, charm_vanilla, compute_artifact_values, effective_range_union, find_matches,
    load_resalter_artifact_boosts, search_tier_range, seed_for_tier, CharmUnit,
};
use eframe::egui;
use egui::Ui;
use sas2_parser::{SaveData, loot_names};
use std::collections::HashMap;

/// Talisman slots used by GetCharmVal: 7/8 = ring slots, 9 = amulet, 20 = dagger.
const CHARM_SLOTS: [i32; 4] = [7, 8, 9, 20];

/// Reroll the seed of an artifact within the tier range of the search scope.
fn reroll_seed(
    item: &mut sas2_parser::Item,
    scope: SearchTierScope,
    current_tier: i32,
    min_tier: i32,
    max_tier: i32,
) {
    let (lo, hi) = search_tier_range(scope, current_tier, min_tier, max_tier);
    let first = lo * 2000 + 1;
    let last = hi * 2000 + 2000;
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as i32)
        .unwrap_or(0);
    let new_seed = first + nanos.rem_euclid(last - first + 1);
    item.artifact_seed = new_seed;
    item.upgrade = new_seed;
}

/// The name of an artifact field, for table headers and error messages.
fn field_name(field: i32) -> String {
    ARTIFACT_FIELDS
        .iter()
        .find(|(id, _)| *id == field)
        .map(|(_, n)| n.to_string())
        .unwrap_or_else(|| format!("field {}", field))
}

/// Recompute both match lists from the current settings.
#[allow(clippy::too_many_arguments)]
fn recompute_matches(
    exact: &mut Vec<ArtifactMatch>,
    partial: &mut Vec<ArtifactMatch>,
    must: &HashMap<i32, f32>,
    can: &HashMap<i32, f32>,
    subtype: i32,
    scope: SearchTierScope,
    current_tier: i32,
    min_tier: i32,
    max_tier: i32,
) {
    let (lo, hi) = search_tier_range(scope, current_tier, min_tier, max_tier);
    let results = find_matches(subtype, lo, hi, must, can);
    *exact = results.exact;
    *partial = results.partial;
}

/// Owned snapshot of an artifact item for the list.
struct ArtifactEntry {
    inv_idx: usize,
    seed: i32,
    subtype: i32,
    name: String,
}

/// Header controls: seed filter, the tier drag (static tier), tier scope radios with always-visible min/max drags, then the selected artifact's seed drag and reroll button.
#[allow(clippy::too_many_arguments)]
fn artifact_header(
    ui: &mut Ui,
    match_search: &mut String,
    scope: &mut SearchTierScope,
    min_tier: &mut i32,
    max_tier: &mut i32,
    drag_sensitivity: f32,
    tier: i32,
    seed: i32,
    sel_idx: usize,
    save: &mut SaveData,
    search_changed: &mut bool,
) {
    // Search seed filter and tier scope.
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Search:").strong());
        let resp = ui.add(
            egui::TextEdit::singleline(match_search)
                .hint_text("seed filter")
                .desired_width(110.0),
        );
        if resp.changed() {
            *search_changed = true;
        }
        ui.separator();
        ui.label("Tiers:");
        let resp = ui.selectable_value(scope, SearchTierScope::StaticTier, "Static tier");
        if resp.changed() {
            *search_changed = true;
        }
        // The tier drag is the static tier: it edits the selected artifact and backs the "Static tier" scope.
        let mut tier_edit = tier;
        if ui
            .add(egui::DragValue::new(&mut tier_edit).speed(0.1).range(0..=100))
            .changed()
            && let Some(item) = save.equipment.inventory_items.get_mut(sel_idx)
        {
            let new_seed = seed_for_tier(artifact_seed(item), tier_edit);
            item.artifact_seed = new_seed;
            item.upgrade = new_seed;
        }
        let resp = ui.selectable_value(scope, SearchTierScope::MinMax, "Min/Max");
        if resp.changed() {
            *search_changed = true;
        }
        let resp = ui.selectable_value(scope, SearchTierScope::AllTiers, "All Tiers");
        if resp.changed() {
            *search_changed = true;
        }
        // Min/max drags are always visible so the user does not have to switch the scope just to adjust them.
        ui.label("Min:");
        let resp = ui.add(egui::DragValue::new(min_tier).speed(0.1).range(0..=40));
        if resp.changed() {
            *search_changed = true;
        }
        ui.label("Max:");
        let resp = ui.add(egui::DragValue::new(max_tier).speed(0.1).range(0..=40));
        if resp.changed() {
            *search_changed = true;
        }
    });

    // Selected artifact controls: seed drag and reroll.
    ui.horizontal(|ui| {
        ui.label("Seed:");
        let mut seed_edit = seed;
        if ui
            .add(
                egui::DragValue::new(&mut seed_edit)
                    .speed(drag_sensitivity)
                    .range(0..=i32::MAX),
            )
            .changed()
            && let Some(item) = save.equipment.inventory_items.get_mut(sel_idx)
        {
            item.artifact_seed = seed_edit;
            item.upgrade = seed_edit;
        }
        if ui
            .button("Reroll")
            .on_hover_text("Roll a new seed within the search tier range")
            .clicked()
            && let Some(item) = save.equipment.inventory_items.get_mut(sel_idx)
        {
            reroll_seed(item, *scope, tier, *min_tier, *max_tier);
        }
    });
}

/// The editor panel for one match kind: one row per artifact field with current value, desired input and per-row QoL buttons.
/// Possibility warnings and the min/max buttons use the achievable range across the search tier range, so min/max and all-tiers searches are not limited by the artifact's own tier.
/// The panel is capped at `max_height`.
#[allow(clippy::too_many_arguments)]
fn artifact_editor_panel(
    ui: &mut Ui,
    grid_id: &str,
    title: &str,
    desired_map: &mut HashMap<i32, f32>,
    drag_sensitivity: f32,
    subtype: i32,
    search_min_tier: i32,
    search_max_tier: i32,
    values: &[f32; 35],
    resalter_boosts: &HashMap<i32, ArtifactBoostOverride>,
    max_height: f32,
    search_changed: &mut bool,
) {
    let main_field = artifact_main_field(subtype);

    egui::Frame::group(ui.style())
        .inner_margin(egui::Margin::symmetric(8, 6))
        .show(ui, |ui| {
            ui.label(egui::RichText::new(title).strong());
            ui.add_space(2.0);
            ui.horizontal(|ui| {
                if ui
                    .button("Reset desired to 0")
                    .on_hover_text("Clear all desired values")
                    .clicked()
                {
                    desired_map.clear();
                    *search_changed = true;
                }
                if ui
                    .button("Set desired to current")
                    .on_hover_text("Set desired to the current value of each possible boost")
                    .clicked()
                {
                    for (field_id, _) in ARTIFACT_FIELDS {
                        let v = values[*field_id as usize];
                        if v > 0.0
                            && effective_range_union(
                                subtype,
                                search_min_tier,
                                search_max_tier,
                                *field_id,
                                resalter_boosts.get(field_id),
                            )
                            .is_some()
                        {
                            desired_map.insert(*field_id, v);
                        }
                    }
                    *search_changed = true;
                }
                if ui
                    .button("Set desired to max")
                    .on_hover_text("Set desired to the max of each possible boost")
                    .clicked()
                {
                    for (field_id, _) in ARTIFACT_FIELDS {
                        let v = values[*field_id as usize];
                        if v > 0.0
                            && let Some((_, max)) = effective_range_union(
                                subtype,
                                search_min_tier,
                                search_max_tier,
                                *field_id,
                                resalter_boosts.get(field_id),
                            )
                        {
                            desired_map.insert(*field_id, max);
                        }
                    }
                    *search_changed = true;
                }
            });
            ui.separator();

            // Fields this subtype can never roll (across any tier, and not enabled by a Resalter override) are pruned: hidden from the grid and cleared from the desired map so they do not silently filter the search.
            let mut pruned: Vec<i32> = Vec::new();
            for (field_id, _) in ARTIFACT_FIELDS {
                if effective_range_union(subtype, 0, 40, *field_id, resalter_boosts.get(field_id))
                    .is_none()
                {
                    pruned.push(*field_id);
                }
            }
            for field_id in &pruned {
                if desired_map.remove(field_id).is_some() {
                    *search_changed = true;
                }
            }

            egui::ScrollArea::both()
                .id_salt(format!("{}_scroll", grid_id))
                .max_height(max_height)
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    if !pruned.is_empty() {
                        ui.label(
                            egui::RichText::new(format!(
                                "{} stats hidden: impossible for this artifact type",
                                pruned.len()
                            ))
                            .weak(),
                        );
                    }
                    egui::Grid::new(grid_id)
                        .num_columns(4)
                        .spacing([12.0, 4.0])
                        .striped(true)
                        .show(ui, |ui| {
                            ui.label(egui::RichText::new("Field").strong());
                            ui.label(egui::RichText::new("Current").strong());
                            ui.label(egui::RichText::new("Desired").strong());
                            ui.label("");
                            ui.end_row();

                            for (field_id, field_name) in ARTIFACT_FIELDS {
                                if pruned.contains(field_id) {
                                    continue;
                                }
                                let v = values[*field_id as usize];
                                let label = if *field_id == main_field {
                                    format!("{} (main)", field_name)
                                } else {
                                    field_name.to_string()
                                };
                                if v > 0.0 {
                                    ui.label(egui::RichText::new(&label).strong());
                                    ui.label(format!("+{:.1}%", v));
                                } else {
                                    ui.label(egui::RichText::new(&label).weak());
                                    ui.label(egui::RichText::new("-").weak());
                                }

                                // Desired value input.
                                let desired = desired_map.entry(*field_id).or_insert(0.0);
                                let desired_resp = ui.add(
                                    egui::DragValue::new(desired)
                                        .speed(drag_sensitivity)
                                        .range(0.0..=100.0)
                                        .suffix("%"),
                                );
                                if desired_resp.changed() {
                                    *search_changed = true;
                                }

                                // Fourth cell: per-row QoL buttons and the impossibility warning, stacked vertically.
                                ui.vertical(|ui| {
                                    let range = effective_range_union(
                                        subtype,
                                        search_min_tier,
                                        search_max_tier,
                                        *field_id,
                                        resalter_boosts.get(field_id),
                                    );
                                    ui.horizontal(|ui| {
                                        if let Some((rmin, rmax)) = range {
                                            if ui
                                                .small_button("min")
                                                .on_hover_text("Set desired to the minimum")
                                                .clicked()
                                            {
                                                *desired = rmin;
                                                *search_changed = true;
                                            }
                                            if ui
                                                .small_button("max")
                                                .on_hover_text("Set desired to the maximum")
                                                .clicked()
                                            {
                                                *desired = rmax;
                                                *search_changed = true;
                                            }
                                        }
                                        if ui
                                            .small_button("0")
                                            .on_hover_text("Set desired to 0 (clear)")
                                            .clicked()
                                        {
                                            *desired = 0.0;
                                            *search_changed = true;
                                        }
                                    });

                                    // Impossible warning for this field.
                                    if *desired > 0.0 {
                                        let possible = effective_range_union(
                                            subtype,
                                            search_min_tier,
                                            search_max_tier,
                                            *field_id,
                                            resalter_boosts.get(field_id),
                                        )
                                        .map(|(min, max)| {
                                            // The search matches within 0.05.
                                            // The min is achievable, but the max is exclusive (NextDouble() < 1.0), so the upper bound needs a tiny epsilon to catch values like 3.30 when the max is 3.25 (the closest achievable value is ~3.25).
                                            *desired >= min - 0.05 && *desired <= max + 0.05 - 0.0001
                                        })
                                        .unwrap_or(false);
                                        if !possible {
                                            let range_text = effective_range_union(
                                                subtype,
                                                search_min_tier,
                                                search_max_tier,
                                                *field_id,
                                                resalter_boosts.get(field_id),
                                            )
                                            .map(|(min, max)| {
                                                format!("{:.1}-{:.1}%", min, max)
                                            })
                                            .unwrap_or_else(|| "never set".to_string());
                                            ui.colored_label(
                                                egui::Color32::RED,
                                                format!(
                                                    "{:.1}% is outside {}",
                                                    desired, range_text
                                                ),
                                            );
                                        }
                                    }
                                });
                                ui.end_row();
                            }
                        });
                });
        });
}

/// The sort key extractor of a merged result list row.
type SortKeyFn = Box<dyn Fn(&ArtifactMatch) -> f32>;

/// The merged result list: exact matches (gold) and, when enabled, partial matches (green) in one list, sortable by closeness or any filtered field, ascending or descending.
/// Filtered by the seed search text.
/// Huge result sets are capped: 50% above 5k results, 25% above 10k, 12.5% above 20k, 5% above 40k, with "Show less" and "Show more" buttons.
#[allow(clippy::too_many_arguments)]
fn artifact_result_list(
    ui: &mut Ui,
    grid_id: &str,
    title: &str,
    scope_text: &str,
    match_search: &str,
    exact: &[ArtifactMatch],
    partial: &[ArtifactMatch],
    show_partial: &mut bool,
    sort_key: &mut ResultSortKey,
    sort_desc: &mut bool,
    must: &HashMap<i32, f32>,
    can: &HashMap<i32, f32>,
    apply_target: Option<usize>,
    pending_apply: &mut Option<i32>,
    limit: &mut Option<usize>,
    always_all: bool,
    max_height: f32,
) {
    let key = *sort_key;
    let desc = *sort_desc;
    let needle = match_search.to_lowercase();
    let mut rows: Vec<(&ArtifactMatch, bool)> = exact
        .iter()
        .map(|m| (m, true))
        .chain(
            (*show_partial)
                .then(|| partial.iter().map(|m| (m, false)))
                .into_iter()
                .flatten(),
        )
        .filter(|(m, _)| needle.is_empty() || m.seed.to_string().contains(&needle))
        .collect();

    let (sort_key_text, key_fn): (String, SortKeyFn) = match key {
        ResultSortKey::Closeness => ("Closeness".to_string(), Box::new(|m| m.error)),
        ResultSortKey::Field(f) => (
            field_name(f),
            Box::new(move |m: &ArtifactMatch| {
                m.values
                    .iter()
                    .find(|(field, _)| *field == f)
                    .map(|(_, v)| *v)
                    .unwrap_or(0.0)
            }),
        ),
    };
    if desc {
        rows.sort_by(|a, b| {
            key_fn(b.0)
                .total_cmp(&key_fn(a.0))
                .then(b.0.seed.cmp(&a.0.seed))
        });
    } else {
        rows.sort_by(|a, b| {
            key_fn(a.0)
                .total_cmp(&key_fn(b.0))
                .then(a.0.seed.cmp(&b.0.seed))
        });
    }

    // Union of the filtered fields, sorted: the display columns.
    let mut fields: Vec<i32> = must
        .iter()
        .chain(can.iter())
        .filter(|(_, v)| **v > 0.0)
        .map(|(f, _)| *f)
        .collect();
    fields.sort_unstable();
    fields.dedup();

    // Cap huge result sets: 50% above 5k, 25% above 10k, 12.5% above 20k, 5% above 40k, keeping the shown count around the 5k comfort limit.
    // "Show less" halves and "Show more" doubles the shown amount; both can go below the initial percentage, down to a single row.
    let total = rows.len();
    let fraction = if total > 40_000 {
        0.05
    } else if total > 20_000 {
        0.125
    } else if total > 10_000 {
        0.25
    } else if total > 5_000 {
        0.5
    } else {
        1.0
    };
    let initial = (total as f32 * fraction) as usize;
    let capped = !always_all && fraction < 1.0;
    let shown = if capped {
        limit.unwrap_or(initial).clamp(1, total)
    } else {
        total
    };

    egui::Frame::group(ui.style())
        .inner_margin(egui::Margin::symmetric(8, 6))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(format!("{} ({})", title, scope_text)).strong(),
                );
                if rows.is_empty() {
                    ui.label(egui::RichText::new("No matches.").weak());
                } else if capped {
                    ui.label(format!("Showing {} of {} result(s)", shown, total));
                } else {
                    ui.label(format!("{} result(s)", rows.len()));
                }
            });
            ui.horizontal(|ui| {
                ui.checkbox(show_partial, "Show partial matches");
                ui.label("Sort by:");
                egui::ComboBox::from_id_salt(format!("{}_sort", grid_id))
                    .selected_text(&sort_key_text)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(sort_key, ResultSortKey::Closeness, "Closeness");
                        for f in &fields {
                            let name = field_name(*f);
                            ui.selectable_value(sort_key, ResultSortKey::Field(*f), name);
                        }
                    });
                ui.selectable_value(sort_desc, true, "Desc");
                ui.selectable_value(sort_desc, false, "Asc");
                if capped {
                    if ui.button("Show less").clicked() {
                        *limit = Some(shown / 2);
                    }
                    if ui.button("Show more").clicked() {
                        *limit = Some(shown * 2);
                    }
                }
            });
            ui.separator();

            egui::ScrollArea::both()
                .id_salt(format!("{}_scroll", grid_id))
                .max_height(max_height)
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    egui::Grid::new(grid_id)
                        .num_columns(fields.len() + 4)
                        .spacing([12.0, 4.0])
                        .striped(true)
                        .show(ui, |ui| {
                            ui.label(egui::RichText::new("Seed").strong());
                            ui.label(egui::RichText::new("Tier").strong());
                            for f in &fields {
                                ui.label(egui::RichText::new(field_name(*f)).strong());
                            }
                            ui.label(egui::RichText::new("Closeness").strong());
                            ui.label("");
                            ui.end_row();

                            for (m, is_exact) in rows.iter().take(shown) {
                                let row_color = if *is_exact {
                                    egui::Color32::GOLD
                                } else {
                                    egui::Color32::LIGHT_GREEN
                                };
                                ui.colored_label(row_color, m.seed.to_string());
                                ui.colored_label(row_color, m.tier.to_string());
                                for f in &fields {
                                    let actual = m
                                        .values
                                        .iter()
                                        .find(|(field, _)| field == f)
                                        .map(|(_, v)| *v)
                                        .unwrap_or(0.0);
                                    let text = if actual > 0.0 {
                                        format!("{:.1}%", actual)
                                    } else {
                                        "-".to_string()
                                    };
                                    ui.colored_label(row_color, text);
                                }
                                ui.colored_label(row_color, format!("{:.2}", m.error));
                                if ui.button("Apply").clicked()
                                    && apply_target.is_some()
                                {
                                    *pending_apply = Some(m.seed);
                                }
                                ui.end_row();
                            }
                        });
                });
        });
}

impl SaveEditor {
    pub fn show_artifacts_ui(&mut self, ui: &mut Ui, save: &mut SaveData) {
        ui.add(
            egui::Label::new(
                "Artifact values are not stored in the save: the game rolls them from a seed on load. Edit the seed or reroll it to change the values. \
                 Talisman boosts come from the equipped talismans' flags, shown below for reference.",
            )
            .wrap(),
        );

        // Collect artifact items (type 6, subtype 3/4/5) as owned data.
        let catalog = self.catalog.as_ref();
        let mut artifacts: Vec<ArtifactEntry> = Vec::new();
        for (idx, item) in save.equipment.inventory_items.iter().enumerate() {
            let Some(def) = catalog.and_then(|c| c.loot_defs.get(item.loot_idx as usize)) else {
                continue;
            };
            if def.type_ == 6 && (3..=5).contains(&def.sub_type) {
                let name = def
                    .title
                    .first()
                    .filter(|t| !t.is_empty())
                    .cloned()
                    .unwrap_or_else(|| format!("Item {}", item.loot_idx));
                artifacts.push(ArtifactEntry {
                    inv_idx: idx,
                    seed: artifact_seed(item),
                    subtype: def.sub_type,
                    name,
                });
            }
        }

        if artifacts.is_empty() {
            ui.label("No artifacts (talisman subtype Attack/Defense/Utility) in the inventory.");
            return;
        }

        let mut selected: Option<usize> = self.selected_artifact;
        let mut selected_changed = false;

        // Resalter override info for the impossibility check, loaded once.
        let resalter_boosts = self
            .config
            .game_path
            .as_deref()
            .map(load_resalter_artifact_boosts)
            .unwrap_or_default();
        let drag_sensitivity = self.config.drag_value_sensitivity;

        // Left panel: artifact list
        egui::Panel::left("artifact_list")
            .resizable(true)
            .default_size(280.0)
            .min_size(180.0)
            .frame(egui::Frame::NONE)
            .show_inside(ui, |ui| {
                ui.label(egui::RichText::new("Artifacts").strong());
                egui::ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        for entry in &artifacts {
                            let tier = artifact_tier(entry.seed);
                            let values = compute_artifact_values(entry.seed, entry.subtype, tier);
                            let rarity = artifact_rarity(&values);
                            let label = format!(
                                "{} (T{}{})",
                                entry.name,
                                tier,
                                if rarity > 0 {
                                    format!(", R{}", rarity)
                                } else {
                                    String::new()
                                }
                            );
                            if ui
                                .selectable_label(selected == Some(entry.inv_idx), label)
                                .clicked()
                            {
                                if selected != Some(entry.inv_idx) {
                                    selected_changed = true;
                                }
                                selected = Some(entry.inv_idx);
                            }
                        }
                    });
            });

        // Central panel: search grid
        egui::CentralPanel::default().show_inside(ui, |ui| {
            let Some(sel_idx) = selected else {
                ui.label("Select an artifact.");
                return;
            };
            let Some(entry) = artifacts.iter().find(|e| e.inv_idx == sel_idx) else {
                ui.label("Selected artifact not found.");
                return;
            };
            let seed = entry.seed;
            let subtype = entry.subtype;
            let tier = artifact_tier(seed);
            let values = compute_artifact_values(seed, subtype, tier);
            let subtype_name = loot_names::get_subtype_name(6, subtype);

            let mut search_changed = selected_changed;

            ui.heading(&entry.name);
            ui.label(format!("Type: Charm - {}", subtype_name));
            ui.label(format!(
                "Rarity: {}",
                match artifact_rarity(&values) {
                    0 => "Common".to_string(),
                    1 => "Rare".to_string(),
                    2 => "Very Rare".to_string(),
                    3 => "Legendary".to_string(),
                    _ => "Epic".to_string(),
                }
            ));
            ui.separator();

            artifact_header(
                ui,
                &mut self.artifact_match_search,
                &mut self.artifact_search_scope,
                &mut self.artifact_min_tier,
                &mut self.artifact_max_tier,
                drag_sensitivity,
                tier,
                seed,
                sel_idx,
                save,
                &mut search_changed,
            );
            // Remember the tier scope and sort settings when the config option is enabled.
            if self.config.remember_artifact_search {
                let changed = self.config.artifact_search_scope != self.artifact_search_scope
                    || self.config.artifact_result_sort_key != self.artifact_result_sort_key
                    || self.config.artifact_result_sort_desc != self.artifact_result_sort_desc;
                self.config.artifact_search_scope = self.artifact_search_scope;
                self.config.artifact_result_sort_key = self.artifact_result_sort_key;
                self.config.artifact_result_sort_desc = self.artifact_result_sort_desc;
                if changed {
                    self.config_save_timer = 0.1;
                }
            }
            ui.separator();

            // The header can change the seed (tier/seed drags, reroll). Re-read it and the tier so the search follows the new seed/tier.
            let seed = save
                .equipment
                .inventory_items
                .get(sel_idx)
                .map(artifact_seed)
                .unwrap_or(seed);
            let tier = artifact_tier(seed);
            if seed != entry.seed {
                search_changed = true;
            }
            let values = compute_artifact_values(seed, subtype, tier);

            // Live search: recompute both match lists whenever a setting changed.
            if search_changed {
                recompute_matches(
                    &mut self.artifact_exact_matches,
                    &mut self.artifact_partial_matches,
                    &self.artifact_desired_values,
                    &self.artifact_can_values,
                    subtype,
                    self.artifact_search_scope,
                    tier,
                    self.artifact_min_tier,
                    self.artifact_max_tier,
                );
                // A new result set starts at the size-based cap again.
                self.artifact_result_limit = None;
                search_changed = false;
            }

            // Apply a chosen seed from a match list (pending state set by the Apply button click, processed here so the lists can update).
            if let Some(new_seed) = self.artifact_pending_apply.take()
                && let Some(item) = save.equipment.inventory_items.get_mut(sel_idx)
            {
                item.artifact_seed = new_seed;
                item.upgrade = new_seed;
            }

            // The tier range the current search scope covers, used by the editor panels for possibility checks and min/max buttons.
            let (search_min_tier, search_max_tier) = search_tier_range(
                self.artifact_search_scope,
                tier,
                self.artifact_min_tier,
                self.artifact_max_tier,
            );

            let scope_text = match self.artifact_search_scope {
                SearchTierScope::StaticTier => format!("tier {}", tier),
                SearchTierScope::MinMax => format!(
                    "tiers {}-{}",
                    self.artifact_min_tier, self.artifact_max_tier
                ),
                SearchTierScope::AllTiers => "all tiers".to_string(),
            };

            // Responsive layout: the merged results list on the left, the two editor panels on the right when wide enough, stacked otherwise.
            // Each editor panel is allocated exactly half the column (minus the 8px gap), so the two boxes always have the same height.
            // The scroll area inside a panel fills whatever its header leaves.
            let min_column_width = 420.0;
            // Each box gets half the column minus the gap, then 1px less per box and 1px less gap, so the column's own item spacing never pushes the total over and the outer scroll bar stays hidden.
            let panel_total = ((ui.available_height() - 10.0) / 2.0).max(120.0);
            if ui.available_width() >= min_column_width * 2.0 {
                ui.columns(2, |columns| {
                    egui::ScrollArea::vertical()
                        .id_salt("results_col")
                        .auto_shrink([false; 2])
                        .show(&mut columns[0], |ui| {
                            artifact_result_list(
                                ui,
                                "results_list",
                                "Results",
                                &scope_text,
                                &self.artifact_match_search,
                                &self.artifact_exact_matches,
                                &self.artifact_partial_matches,
                                &mut self.artifact_show_partial,
                                &mut self.artifact_result_sort_key,
                                &mut self.artifact_result_sort_desc,
                                &self.artifact_desired_values,
                                &self.artifact_can_values,
                                Some(sel_idx),
                                &mut self.artifact_pending_apply,
                                &mut self.artifact_result_limit,
                                self.config.always_load_all_results,
                                ui.available_height().max(120.0),
                            );
                        });
                    egui::ScrollArea::vertical()
                        .id_salt("editors_col")
                        .auto_shrink([false; 2])
                        .show(&mut columns[1], |ui| {
                            ui.allocate_ui_with_layout(
                                egui::vec2(ui.available_width(), panel_total),
                                egui::Layout::top_down(egui::Align::Min),
                                |ui| {
                                    artifact_editor_panel(
                                        ui,
                                        "can_editor",
                                        "Partial match",
                                        &mut self.artifact_can_values,
                                        drag_sensitivity,
                                        subtype,
                                        search_min_tier,
                                        search_max_tier,
                                        &values,
                                        &resalter_boosts,
                                        panel_total,
                                        &mut search_changed,
                                    );
                                },
                            );
                            ui.add_space(7.0);
                            ui.allocate_ui_with_layout(
                                egui::vec2(ui.available_width(), panel_total),
                                egui::Layout::top_down(egui::Align::Min),
                                |ui| {
                                    artifact_editor_panel(
                                        ui,
                                        "must_editor",
                                        "Must match",
                                        &mut self.artifact_desired_values,
                                        drag_sensitivity,
                                        subtype,
                                        search_min_tier,
                                        search_max_tier,
                                        &values,
                                        &resalter_boosts,
                                        panel_total,
                                        &mut search_changed,
                                    );
                                },
                            );
                        });
                });
            } else {
                let stacked_max = (ui.available_height() * 0.45).max(160.0);
                egui::ScrollArea::vertical()
                    .id_salt("stacked_grid")
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        artifact_result_list(
                            ui,
                            "results_list",
                            "Results",
                            &scope_text,
                            &self.artifact_match_search,
                            &self.artifact_exact_matches,
                            &self.artifact_partial_matches,
                            &mut self.artifact_show_partial,
                            &mut self.artifact_result_sort_key,
                            &mut self.artifact_result_sort_desc,
                            &self.artifact_desired_values,
                            &self.artifact_can_values,
                            Some(sel_idx),
                            &mut self.artifact_pending_apply,
                            &mut self.artifact_result_limit,
                            self.config.always_load_all_results,
                            stacked_max,
                        );
                        ui.add_space(8.0);
                        artifact_editor_panel(
                            ui,
                            "can_editor",
                            "Partial match",
                            &mut self.artifact_can_values,
                            drag_sensitivity,
                            subtype,
                            search_min_tier,
                            search_max_tier,
                            &values,
                            &resalter_boosts,
                            stacked_max,
                            &mut search_changed,
                        );
                        ui.add_space(8.0);
                        artifact_editor_panel(
                            ui,
                            "must_editor",
                            "Must match",
                            &mut self.artifact_desired_values,
                            drag_sensitivity,
                            subtype,
                            search_min_tier,
                            search_max_tier,
                            &values,
                            &resalter_boosts,
                            stacked_max,
                            &mut search_changed,
                        );
                    });
            }

            // A search change triggered inside the panels must still recompute.
            if search_changed {
                recompute_matches(
                    &mut self.artifact_exact_matches,
                    &mut self.artifact_partial_matches,
                    &self.artifact_desired_values,
                    &self.artifact_can_values,
                    subtype,
                    self.artifact_search_scope,
                    tier,
                    self.artifact_min_tier,
                    self.artifact_max_tier,
                );
                // A new result set starts at the size-based cap again.
                self.artifact_result_limit = None;
            }
        });

        self.selected_artifact = selected;

        ui.add_space(16.0);
        ui.separator();
        self.show_talismans_ui(ui, save);
    }

    /// Show equipped talismans and the effective charm values they produce.
    fn show_talismans_ui(&self, ui: &mut Ui, save: &SaveData) {
        ui.heading("Equipped Talismans");
        ui.add(
            egui::Label::new(
                "Charm boosts are computed from the flags of equipped talismans: each flag counts how many equipped talismans share it, giving a tier of 1.0 / 1.25 / 1.35 / 1.4. \
                 The effective value is that tier times the flag's vanilla magnitude. \
                 Nothing here is stored in the save.",
            )
            .wrap(),
        );
        ui.add_space(8.0);

        let catalog = self.catalog.as_ref();
        let items = &save.equipment.inventory_items;
        let equipped = &save.equipment.equipped_items;

        // Collect equipped talismans (slots 7/8/9/20) with their flags.
        let mut talisman_flags: Vec<(String, Vec<i32>)> = Vec::new();
        for slot in CHARM_SLOTS {
            let Some(&inv_idx) = equipped.get(slot as usize) else {
                continue;
            };
            if inv_idx < 0 || inv_idx as usize >= items.len() {
                continue;
            }
            let item = &items[inv_idx as usize];
            let Some(def) = catalog.and_then(|c| c.loot_defs.get(item.loot_idx as usize)) else {
                continue;
            };
            if def.type_ != 6 || def.sub_type > 2 {
                continue;
            }
            let name = def
                .title
                .first()
                .filter(|t| !t.is_empty())
                .cloned()
                .unwrap_or_else(|| format!("Item {}", item.loot_idx));
            talisman_flags.push((name, def.flags.clone()));
        }

        if talisman_flags.is_empty() {
            ui.label("No talismans equipped (ring/amulet/dagger slots).");
            return;
        }

        // Count flags across equipped talismans.
        let mut flag_counts: std::collections::BTreeMap<i32, i32> = std::collections::BTreeMap::new();
        for (_, flags) in &talisman_flags {
            for flag in flags {
                *flag_counts.entry(*flag).or_insert(0) += 1;
            }
        }

        // Equipped talismans list
        ui.label(egui::RichText::new("Equipped:").strong());
        for (name, flags) in &talisman_flags {
            let flag_names: Vec<&str> = flags
                .iter()
                .map(|f| loot_names::get_flag_name(6, *f))
                .collect();
            ui.label(format!("{}: {}", name, flag_names.join(", ")));
        }

        ui.add_space(8.0);
        ui.label(egui::RichText::new("Effective charm values:").strong());

        egui::ScrollArea::vertical()
            .max_height(300.0)
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                egui::Grid::new("charm_values")
                    .num_columns(3)
                    .spacing([16.0, 4.0])
                    .striped(true)
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new("Boost").strong());
                        ui.label(egui::RichText::new("Count").strong());
                        ui.label(egui::RichText::new("Effective").strong());
                        ui.end_row();

                        for (flag, count) in &flag_counts {
                            let name = loot_names::get_flag_name(6, *flag);
                            let unit = charm_unit(*flag);
                            let suffix = match unit {
                                CharmUnit::Percent => "%",
                                CharmUnit::Flat => "",
                            };
                            let effective = charm_effective(*flag, *count);
                            let vanilla = charm_vanilla(*flag);
                            ui.label(format!("[{}] {}", flag, name));
                            ui.label(count.to_string());
                            ui.label(format!(
                                "{:.1}{} (vanilla {:.1}{})",
                                effective, suffix, vanilla, suffix
                            ));
                            ui.end_row();
                        }
                    });
            });
    }
}
