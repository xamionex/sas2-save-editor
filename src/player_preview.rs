//! Player character preview for the Cosmetics tab.
//!
//! Reproduces the game's character composition for a right-facing idle hero:
//!   - the body sheet is `{ancestry}_{sex}` (or `herox_male` / `herox_female` when hazeburnt),
//!   - parts come from the "hero" char def's idle frame,
//!   - "Face" sub-flags of the part cells draw the eyes, pupils, eyebrows, mouth, nose and ears,
//!   - hair and beard sheets are overlaid on the body cells they are mapped to (char_ref / flags pairing, exactly like CharClothesMap),
//!   - hair/beard/pupil/eyebrow pixels are tinted with the matching cosmetic color.
//!
//! Reference: Skellingtons/character/draw/CharDraw.cs, CharDrawPhysics.DrawClothes, CharDrawAttached.DrawFace and CharClothesMap.SetTex in the game decompilation.

use egui::{ColorImage, TextureHandle};
use image::RgbaImage;
use sas2_parser::char_def::CharDef;
use sas2_parser::cosmetics::{AncestryCatalog, BeardCatalog, ColorCatalog, EyeCatalog, HairCatalog, SexCatalog};
use sas2_parser::subflags::SubFlagDefCatalog;
use sas2_parser::xnb_loader::load_texture_from_path;
use sas2_parser::xtexture::{MasterTextures, XSpriteRaw};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// The monster def every player character spawns as (Character.Init(.., "hero", ..)).
pub const PLAYER_MONSTER_DEF: &str = "hero";

/// Char def the engine falls back to when a monster def's def name is not in CharDefMgr's list
/// (GetDefIdx returns 0, which is "base").
pub const FALLBACK_CHAR_DEF: &str = "base";

/// Part cells below this index belong to the body sheet (above it are weapon/consumable cells).
const BODY_MAX_PARTS: i32 = 384;

/// Which part of the character a preview shows.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum PreviewView {
    /// Whole body.
    Full,
    /// Head and shoulders (hair, beard).
    Head,
    /// Face close-up (eyes, eyebrows).
    Face,
}

/// A cosmetic loadout, in save.cosmetics slot order.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct CosmeticSelection {
    pub sex: usize,
    pub ancestry: usize,
    pub eye: usize,
    pub hair: usize,
    pub hair_color: usize,
    pub beard: usize,
    pub beard_color: usize,
    pub eyebrow_color: usize,
    pub hazeburnt: bool,
}

impl CosmeticSelection {
    /// Build a selection from the save's cosmetic indices (slots 0..=7 are read, missing or negative slots fall back to 0).
    pub fn from_cosmetics(cosmetics: &[i32], hazeburnt: bool) -> Self {
        let at = |i: usize| cosmetics.get(i).copied().unwrap_or(0).max(0) as usize;
        let mut sel = Self {
            sex: at(0),
            ancestry: at(1),
            eye: at(2),
            hair: at(3),
            hair_color: at(4),
            beard: at(5),
            beard_color: at(6),
            eyebrow_color: at(7),
            hazeburnt,
        };
        // Out of range values in a save file must not break the preview.
        sel.sex = sel.sex.min(SexCatalog::len().saturating_sub(1));
        sel.ancestry = sel.ancestry.min(AncestryCatalog::len().saturating_sub(1));
        sel.eye = sel.eye.min(EyeCatalog::len().saturating_sub(1));
        sel.hair = sel.hair.min(HairCatalog::len().saturating_sub(1));
        sel.hair_color = sel.hair_color.min(ColorCatalog::len().saturating_sub(1));
        sel.beard = sel.beard.min(BeardCatalog::len().saturating_sub(1));
        sel.beard_color = sel.beard_color.min(ColorCatalog::len().saturating_sub(1));
        sel.eyebrow_color = sel.eyebrow_color.min(ColorCatalog::len().saturating_sub(1));
        sel
    }

    /// Change one slot (0..=7), used to render candidate tiles.
    pub fn set_slot(&mut self, slot: usize, value: usize) {
        match slot {
            0 => self.sex = value,
            1 => self.ancestry = value,
            2 => self.eye = value,
            3 => self.hair = value,
            4 => self.hair_color = value,
            5 => self.beard = value,
            6 => self.beard_color = value,
            7 => self.eyebrow_color = value,
            _ => {}
        }
    }

    /// True when every index points at a real catalog entry.
    pub fn is_valid(&self) -> bool {
        self.sex < SexCatalog::len()
            && self.ancestry < AncestryCatalog::len()
            && self.eye < EyeCatalog::len()
            && self.hair < HairCatalog::len()
            && self.hair_color < ColorCatalog::len()
            && self.beard < BeardCatalog::len()
            && self.beard_color < ColorCatalog::len()
            && self.eyebrow_color < ColorCatalog::len()
    }

    pub fn body_texture(&self) -> Option<String> {
        let sex = SexCatalog::get_all().get(self.sex)?.path.as_str();
        if self.hazeburnt {
            return Some(format!("herox_{}", sex));
        }
        let ancestry = AncestryCatalog::get_all().get(self.ancestry)?.path.clone();
        Some(format!("{}_{}", ancestry, sex))
    }

    pub fn hair_texture(&self) -> Option<String> {
        HairCatalog::get_all()
            .get(self.hair)?
            .img
            .first()
            .and_then(|n| n.clone())
    }

    pub fn beard_texture(&self) -> Option<String> {
        BeardCatalog::get_all()
            .get(self.beard)?
            .img
            .first()
            .and_then(|n| n.clone())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum SheetKind {
    Base,
    Hair,
    Beard,
}

/// One drawable layer: a sheet cell placed with a part's transform.
struct Layer {
    src: (i32, i32, i32, i32),
    /// Anchor in cell-local coordinates (the game mirrors it for mirrored parts).
    anchor: (f32, f32),
    /// The part's art is mirrored (right-facing character with part.flip set).
    mirror: bool,
    /// RGB tint (1.0 = unchanged).
    tint: (f32, f32, f32),
    sheet: SheetKind,
    cx: f32,
    cy: f32,
    rot: f32,
    scale_x: f32,
    scale_y: f32,
}

impl Layer {
    /// Axis aligned bounding box in canvas space: (min_x, min_y, max_x, max_y).
    fn bounds(&self) -> (f32, f32, f32, f32) {
        let left = -self.anchor.0 * self.scale_x;
        let right = (self.src.2 as f32 - self.anchor.0) * self.scale_x;
        let top = -self.anchor.1 * self.scale_y;
        let bottom = (self.src.3 as f32 - self.anchor.1) * self.scale_y;
        let (sin, cos) = self.rot.sin_cos();
        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;
        for (dx, dy) in [(left, top), (right, top), (right, bottom), (left, bottom)] {
            let rx = dx * cos - dy * sin;
            let ry = dx * sin + dy * cos;
            min_x = min_x.min(self.cx + rx);
            min_y = min_y.min(self.cy + ry);
            max_x = max_x.max(self.cx + rx);
            max_y = max_y.max(self.cy + ry);
        }
        (min_x, min_y, max_x, max_y)
    }

    fn tint_of(&self, channel: usize) -> f32 {
        match channel {
            0 => self.tint.0,
            1 => self.tint.1,
            _ => self.tint.2,
        }
    }
}

/// The resolved layer list of one character, plus the layer ranges used for the crop windows.
struct Composition {
    layers: Vec<Layer>,
    /// Layers of the head part (the part carrying face sub-flags), if any.
    head: Vec<usize>,
    /// Face sub-flag layers only (eyes, eyebrows, mouth, ...).
    face: Vec<usize>,
}

/// Loads and composes player previews. One instance lives on the editor while a game folder is set.
pub struct PlayerPreview {
    game_path: PathBuf,
    flag_defs: SubFlagDefCatalog,
    master: MasterTextures,
    char_def: CharDef,
    /// Decoded texture sheets, keyed by texture name.
    sheets: HashMap<String, RgbaImage>,
    /// Composed images, keyed by (selection, view).
    cache: HashMap<(CosmeticSelection, PreviewView), RgbaImage>,
    /// Uploaded textures, keyed the same way.
    textures: HashMap<(CosmeticSelection, PreviewView), TextureHandle>,
}

impl PlayerPreview {
    /// Loads the shared metadata (flag defs, master.zcm) and the player's char def.
    /// `char_def_candidates` are tried in order: the game resolves the player's monster def ("hero") through CharDefMgr, which falls back to the first entry ("base") when the def name in the monster entry is unknown, so "base" belongs last in the list.
    pub fn load(game_path: &Path, char_def_candidates: &[String]) -> Result<Self, String> {
        let gfx = game_path.join("Content").join("gfx");
        let flag_defs = SubFlagDefCatalog::load_from_path(&gfx.join("flagdefs.zfd"))
            .map_err(|e| format!("Failed to load flagdefs.zfd: {}", e))?;
        let master = MasterTextures::load_from_path(&gfx.join("master.zcm"), &flag_defs)
            .map_err(|e| format!("Failed to load master.zcm: {}", e))?;

        let dir = game_path.join("Character").join("data");
        let mut char_def = None;
        let mut tried: Vec<String> = Vec::new();
        for candidate in char_def_candidates {
            if candidate.is_empty() {
                continue;
            }
            let path = dir.join(format!("{}.zsx", candidate));
            if !path.exists() {
                tried.push(candidate.clone());
                continue;
            }
            match CharDef::load_from_path(&path) {
                Ok(def) => {
                    char_def = Some(def);
                    break;
                }
                Err(e) => {
                    tried.push(format!("{} ({})", candidate, e));
                }
            }
        }
        let char_def = char_def.ok_or_else(|| {
            format!(
                "Failed to load the player char def (tried: {})",
                if tried.is_empty() {
                    "none".to_string()
                } else {
                    tried.join(", ")
                }
            )
        })?;

        Ok(Self {
            game_path: game_path.to_path_buf(),
            flag_defs,
            master,
            char_def,
            sheets: HashMap::new(),
            cache: HashMap::new(),
            textures: HashMap::new(),
        })
    }

    /// Ensure a texture sheet is decoded (mutating the cache), without holding a borrow.
    fn ensure_sheet(&mut self, name: &str) {
        if self.sheets.contains_key(name) {
            return;
        }
        let path = self
            .game_path
            .join("Content")
            .join("gfx")
            .join(format!("{}.xnb", name));
        let Some(path_str) = path.to_str() else {
            return;
        };
        match load_texture_from_path(path_str) {
            Ok(img) => {
                self.sheets.insert(name.to_string(), img);
            }
            Err(e) => {
                eprintln!("[player_preview] Failed to load texture {}: {}", name, e);
            }
        }
    }

    /// Texture handle for the Cosmetics tab, composed and uploaded on demand.
    pub fn texture(
        &mut self,
        ctx: &egui::Context,
        sel: &CosmeticSelection,
        view: PreviewView,
    ) -> Option<TextureHandle> {
        let key = (sel.clone(), view);
        if !self.textures.contains_key(&key) {
            // Tile browsing creates a distinct entry per candidate; keep the caches bounded.
            if self.cache.len() > 512 || self.textures.len() > 512 {
                self.cache.clear();
                self.textures.clear();
            }
            if !self.cache.contains_key(&key) {
                let img = self.compose(sel, view)?;
                self.cache.insert(key.clone(), img);
            }
            let img = self.cache.get(&key)?;
            let (w, h) = (img.width() as usize, img.height() as usize);
            let pixels = img.as_raw().clone();
            let color_image = ColorImage::from_rgba_unmultiplied([w, h], &pixels);
            let handle = ctx.load_texture(
                format!("player_preview_{:?}_{}", view, self.textures.len()),
                color_image,
                Default::default(),
            );
            self.textures.insert(key.clone(), handle);
        }
        self.textures.get(&key).cloned()
    }

    /// Compose the character and crop it to the requested view.
    fn compose(&mut self, sel: &CosmeticSelection, view: PreviewView) -> Option<RgbaImage> {
        if !sel.is_valid() {
            return None;
        }

        let body_name = sel.body_texture()?;
        let hair_name = sel.hair_texture();
        let beard_name = sel.beard_texture();

        // Cell metadata (src rects, origins, char_ref/flags and sub-flags) from master.zcm.
        let body_cells = self.master.get(&body_name)?.cells.clone();
        let hair_cells = hair_name
            .as_deref()
            .and_then(|n| self.master.get(n))
            .map(|t| t.cells.clone());
        let beard_cells = beard_name
            .as_deref()
            .and_then(|n| self.master.get(n))
            .map(|t| t.cells.clone());

        // Decode the sheets this loadout needs (cheap when they are already cached).
        self.ensure_sheet(&body_name);
        if let Some(n) = &hair_name {
            self.ensure_sheet(n);
        }
        if let Some(n) = &beard_name {
            self.ensure_sheet(n);
        }

        let composition = self.build_layers(
            sel,
            &body_cells,
            hair_cells.as_ref(),
            beard_cells.as_ref(),
        );
        if composition.layers.is_empty() {
            return None;
        }

        let bounds_of = |indices: &[usize]| -> Option<(f32, f32, f32, f32)> {
            union_of(&composition.layers, indices)
        };
        let full = union_of_all(&composition.layers);

        // Crop window for the requested view.
        let (min_x, min_y, max_x, max_y) = match view {
            PreviewView::Full => full?,
            PreviewView::Head => {
                let head = bounds_of(&composition.head).or(full)?;
                pad_rect(head, 6.0)
            }
            PreviewView::Face => {
                let face = bounds_of(&composition.face).or(full)?;
                pad_rect(face, 5.0)
            }
        };

        let min_x = min_x.floor();
        let min_y = min_y.floor();
        let max_x = max_x.ceil();
        let max_y = max_y.ceil();
        let w = (max_x - min_x).max(1.0) as u32;
        let h = (max_y - min_y).max(1.0) as u32;
        if w == 0 || h == 0 || w > 4096 || h > 4096 {
            return None;
        }

        let mut canvas = RgbaImage::new(w, h);
        let body_sheet = self.sheets.get(&body_name);
        let hair_sheet = hair_name.as_deref().and_then(|n| self.sheets.get(n));
        let beard_sheet = beard_name.as_deref().and_then(|n| self.sheets.get(n));
        for layer in &composition.layers {
            let sheet = match layer.sheet {
                SheetKind::Base => match body_sheet {
                    Some(s) => s,
                    None => continue,
                },
                SheetKind::Hair => match hair_sheet {
                    Some(s) => s,
                    None => continue,
                },
                SheetKind::Beard => match beard_sheet {
                    Some(s) => s,
                    None => continue,
                },
            };
            paint_layer(&mut canvas, sheet, layer, min_x, min_y);
        }

        // Pixel-art friendly upscale so the small crops are readable at tile size.
        let zoom = match view {
            PreviewView::Full => 2,
            PreviewView::Head => 2,
            PreviewView::Face => 3,
        };
        Some(if zoom > 1 {
            upscale_nearest(&canvas, zoom)
        } else {
            canvas
        })
    }

    /// Resolve the idle frame into a flat paint list (base cells, hair/beard overlays and the "Face" sub-flags of every part, in the game's draw order).
    fn build_layers(
        &self,
        sel: &CosmeticSelection,
        body_cells: &[Option<XSpriteRaw>],
        hair_cells: Option<&Vec<Option<XSpriteRaw>>>,
        beard_cells: Option<&Vec<Option<XSpriteRaw>>>,
    ) -> Composition {
        let mut composition = Composition {
            layers: Vec::new(),
            head: Vec::new(),
            face: Vec::new(),
        };
        let Some(frame) = self.char_def.idle_frame() else {
            return composition;
        };

        // Absolute part transforms (parent chain), as in the game's UpdateLerpedSkeleton.
        let mut transforms: Vec<Option<(f32, f32, f32)>> = vec![None; frame.parts.len()];
        fn compute(
            idx: usize,
            parts: &[sas2_parser::char_def::Part],
            transforms: &mut Vec<Option<(f32, f32, f32)>>,
        ) -> (f32, f32, f32) {
            if let Some(t) = transforms[idx] {
                return t;
            }
            let part = &parts[idx];
            let t = if part.parent > -1 && (part.parent as usize) < parts.len() {
                let (px, py, prot) = compute(part.parent as usize, parts, transforms);
                let ox = part.parent_loc_offset.0;
                let oy = part.parent_loc_offset.1;
                (
                    px + prot.cos() * ox + (prot + std::f32::consts::FRAC_PI_2).cos() * oy,
                    py + prot.sin() * ox + (prot + std::f32::consts::FRAC_PI_2).sin() * oy,
                    prot + part.parent_rotation_offset,
                )
            } else {
                (part.location.0, part.location.1, part.rotation)
            };
            transforms[idx] = Some(t);
            t
        }
        for i in 0..frame.parts.len() {
            compute(i, &frame.parts, &mut transforms);
        }

        let white = (1.0, 1.0, 1.0);
        let eye_color = EyeCatalog::get_all().get(sel.eye).map(|c| {
            (
                c.r as f32 / 255.0,
                c.g as f32 / 255.0,
                c.b as f32 / 255.0,
            )
        });
        let hair_color = ColorCatalog::get_all()
            .get(sel.hair_color)
            .map(|c| color_triplet(c, sel.hazeburnt));
        let beard_color = ColorCatalog::get_all()
            .get(sel.beard_color)
            .map(|c| color_triplet(c, sel.hazeburnt));
        let eyebrow_color = ColorCatalog::get_all()
            .get(sel.eyebrow_color)
            .map(|c| color_triplet(c, sel.hazeburnt));

        for (i, part) in frame.parts.iter().enumerate() {
            // Cells at 384+ belong to the weapon sheet, not the body sheet.
            if part.idx < 0 || part.idx >= BODY_MAX_PARTS {
                continue;
            }
            let (cx, cy, rot) = transforms[i].unwrap_or((0.0, 0.0, 0.0));
            let idx = part.idx as usize;
            let Some(Some(cell)) = body_cells.get(idx) else {
                continue;
            };
            // Right-facing idle: the game mirrors the art (and its anchor) when part.flip is set.
            let mirror = part.flip != 0;
            // The game draws part sprites at half the part scale (CharDraw: vector5 *= 0.5f) while locations stay in the authoring space, so limbs line up with their slots.
            let scale_x = part.scaling.0 * 0.5;
            let scale_y = part.scaling.1 * 0.5;

            let part_start = composition.layers.len();
            composition.layers.push(Layer {
                src: cell.src_rect,
                anchor: local_anchor(cell, mirror),
                mirror,
                tint: white,
                sheet: SheetKind::Base,
                cx,
                cy,
                rot,
                scale_x,
                scale_y,
            });

            // Beard (slot 3) and hair (slot 4) cells mapped onto this body cell, in the game's clothes order (beard before hair).
            for (cells, kind, tint) in [
                (beard_cells, SheetKind::Beard, beard_color),
                (hair_cells, SheetKind::Hair, hair_color),
            ] {
                let Some(cells) = cells else { continue };
                let Some(cloth_cell) = clothes_cell_for(cells, idx) else {
                    continue;
                };
                let Some(Some(c)) = cells.get(cloth_cell) else {
                    continue;
                };
                composition.layers.push(Layer {
                    src: c.src_rect,
                    anchor: local_anchor(c, mirror),
                    mirror,
                    tint: tint.unwrap_or(white),
                    sheet: kind,
                    cx,
                    cy,
                    rot,
                    scale_x,
                    scale_y,
                });
            }

            // Face sub-flags: eyes, pupils, eyebrows, mouth, nose and ears.
            let mut has_face = false;
            for sub in &cell.subflags {
                let Some(def) = self.flag_defs.defs.get(sub.flag_def_idx as usize) else {
                    continue;
                };
                if def.name != "Face" {
                    continue;
                }
                has_face = true;
                let item = def
                    .item_list
                    .get(sub.index0.max(0) as usize)
                    .map(|(name, _)| name.as_str())
                    .unwrap_or("");
                if !face_item_is_drawn(item) || sub.index1 < 0 {
                    continue;
                }
                let Some(Some(sub_sprite)) = body_cells.get(sub.index1 as usize) else {
                    continue;
                };
                let tint = if item.starts_with("Pupils") {
                    eye_color.unwrap_or(white)
                } else if item.starts_with("Eyebrows") {
                    eyebrow_color.unwrap_or(white)
                } else {
                    white
                };
                let anchor = face_anchor(sub_sprite, cell, sub.vector, mirror);
                composition.face.push(composition.layers.len());
                composition.layers.push(Layer {
                    src: sub_sprite.src_rect,
                    anchor,
                    mirror,
                    tint,
                    sheet: SheetKind::Base,
                    cx,
                    cy,
                    rot,
                    scale_x,
                    scale_y,
                });
            }

            // Beard cells attached to a mouth sub-flag (moustaches and the like) are drawn from that sub-flag's position, as DrawFace does.
            if let Some(beard) = beard_cells {
                for sub in &cell.subflags {
                    let Some(def) = self.flag_defs.defs.get(sub.flag_def_idx as usize) else {
                        continue;
                    };
                    if def.name != "Face" {
                        continue;
                    }
                    let item = def
                        .item_list
                        .get(sub.index0.max(0) as usize)
                        .map(|(name, _)| name.as_str())
                        .unwrap_or("");
                    if !item.starts_with("Mouth") {
                        continue;
                    }
                    let Some(cloth_cell) = clothes_cell_for(beard, sub.index1.max(0) as usize)
                    else {
                        continue;
                    };
                    let Some(Some(c)) = beard.get(cloth_cell) else {
                        continue;
                    };
                    let mut anchor = local_anchor(c, mirror);
                    let (mx, my) = sub_flag_offset_sign(mirror);
                    anchor.0 -= (sub.vector.0 - cell.origin.0) * mx;
                    anchor.1 -= (sub.vector.1 - cell.origin.1) * my;
                    composition.layers.push(Layer {
                        src: c.src_rect,
                        anchor,
                        mirror,
                        tint: beard_color.unwrap_or(white),
                        sheet: SheetKind::Beard,
                        cx,
                        cy,
                        rot,
                        scale_x,
                        scale_y,
                    });
                }
            }

            if has_face {
                composition.head.extend(part_start..composition.layers.len());
            }
        }

        composition
    }
}

/// The DrawFace offset multipliers: `-(vector - origin) * (flip ? (1,1) : (-1,1))`, where the game's `flip` is `face != part.flip` (the hero idles facing right).
fn sub_flag_offset_sign(mirror: bool) -> (f32, f32) {
    // mirror == true  -> flip == false -> (-1, 1)
    // mirror == false -> flip == true  -> ( 1, 1)
    if mirror { (-1.0, 1.0) } else { (1.0, 1.0) }
}

/// DrawFace anchor: the sub-cell's local origin, mirrored when the art is mirrored, shifted by the sub-flag offset.
fn face_anchor(
    sub_sprite: &XSpriteRaw,
    part_cell: &XSpriteRaw,
    vector: (f32, f32),
    mirror: bool,
) -> (f32, f32) {
    let mut anchor = local_anchor(sub_sprite, mirror);
    let (mx, my) = sub_flag_offset_sign(mirror);
    anchor.0 -= (vector.0 - part_cell.origin.0) * mx;
    anchor.1 -= (vector.1 - part_cell.origin.1) * my;
    anchor
}

/// Cell-local anchor (game convention): the origin relative to the source rect, mirrored when the part is drawn mirrored.
fn local_anchor(cell: &XSpriteRaw, mirror: bool) -> (f32, f32) {
    let mut anchor = (
        cell.origin.0 - cell.src_rect.0 as f32,
        cell.origin.1 - cell.src_rect.1 as f32,
    );
    if mirror {
        anchor.0 = cell.src_rect.2 as f32 - anchor.0;
    }
    anchor
}

/// Index of the clothes cell mapped to a body cell (char_ref > 0 and flags == body cell index), mirroring CharClothesMap.SetTex.
/// When several cells map to the same body cell the last one wins, exactly like the game's loop.
fn clothes_cell_for(cells: &[Option<XSpriteRaw>], body_cell: usize) -> Option<usize> {
    cells.iter().rposition(|cell| {
        cell.as_ref()
            .map(|c| c.char_ref > 0 && c.flags >= 0 && c.flags as usize == body_cell)
            .unwrap_or(false)
    })
}

fn color_triplet(c: &sas2_parser::cosmetics::color::CosmeticColor, hazeburnt: bool) -> (f32, f32, f32) {
    let (r, g, b) = if hazeburnt {
        (c.burnt_r, c.burnt_g, c.burnt_b)
    } else {
        (c.r, c.g, c.b)
    };
    (r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0)
}

/// Which face sub-flag item is drawn on a static, open-eyed, neutral idle frame.
/// The game picks one variant per feature (pupils wander, eyelids blink, the mouth emotes), so only the neutral entries are painted here.
fn face_item_is_drawn(item: &str) -> bool {
    matches!(
        item,
        "Nose"
            | "Eye Whites"
            | "Ears"
            | "Pupils Mid"
            | "Eyebrows Down"
            | "Eyelids Rest"
            | "Mouth Closed"
    )
}

/// Paint one layer with inverse-transform sampling and alpha blending.
fn paint_layer(canvas: &mut RgbaImage, sheet: &RgbaImage, layer: &Layer, off_x: f32, off_y: f32) {
    if layer.scale_x.abs() < 0.001 || layer.scale_y.abs() < 0.001 {
        return;
    }
    let (min_x, min_y, max_x, max_y) = layer.bounds();
    let start_x = (min_x - off_x).floor().max(0.0) as u32;
    let start_y = (min_y - off_y).floor().max(0.0) as u32;
    let end_x = ((max_x - off_x).ceil() as i64).clamp(0, canvas.width() as i64) as u32;
    let end_y = ((max_y - off_y).ceil() as i64).clamp(0, canvas.height() as i64) as u32;
    if start_x >= end_x || start_y >= end_y {
        return;
    }

    let (sin_i, cos_i) = (-layer.rot).sin_cos();
    let w = layer.src.2 as f32;
    let h = layer.src.3 as f32;

    for dy in start_y..end_y {
        for dx in start_x..end_x {
            let px = dx as f32 + off_x + 0.5;
            let py = dy as f32 + off_y + 0.5;
            let vx = px - layer.cx;
            let vy = py - layer.cy;
            let rx = vx * cos_i - vy * sin_i;
            let ry = vx * sin_i + vy * cos_i;
            let mut lx = rx / layer.scale_x;
            let ly = ry / layer.scale_y;
            if layer.mirror {
                lx = -lx;
            }
            let sx = lx + layer.anchor.0;
            let sy = ly + layer.anchor.1;
            if sx < 0.0 || sx >= w || sy < 0.0 || sy >= h {
                continue;
            }
            let src_x = layer.src.0 + sx.floor() as i32;
            let src_y = layer.src.1 + sy.floor() as i32;
            if src_x < 0
                || src_y < 0
                || src_x as u32 >= sheet.width()
                || src_y as u32 >= sheet.height()
            {
                continue;
            }
            let pixel = sheet.get_pixel(src_x as u32, src_y as u32);
            if pixel[3] == 0 {
                continue;
            }
            let alpha = pixel[3] as f32 / 255.0;
            let mut bg = *canvas.get_pixel(dx, dy);
            let bg_a = bg[3] as f32 / 255.0;
            let out_a = alpha + bg_a * (1.0 - alpha);
            if out_a <= 0.0 {
                continue;
            }
            for c in 0..3 {
                let src_c = (pixel[c] as f32 / 255.0) * layer.tint_of(c);
                bg[c] = (((src_c * alpha + (bg[c] as f32 / 255.0) * bg_a * (1.0 - alpha)) / out_a)
                    * 255.0)
                    .round()
                    .clamp(0.0, 255.0) as u8;
            }
            bg[3] = (out_a * 255.0).round().clamp(0.0, 255.0) as u8;
            canvas.put_pixel(dx, dy, bg);
        }
    }
}

fn union_of(layers: &[Layer], indices: &[usize]) -> Option<(f32, f32, f32, f32)> {
    let mut rect: Option<(f32, f32, f32, f32)> = None;
    for &i in indices {
        let Some(layer) = layers.get(i) else { continue };
        let b = layer.bounds();
        rect = Some(match rect {
            Some((min_x, min_y, max_x, max_y)) => (
                min_x.min(b.0),
                min_y.min(b.1),
                max_x.max(b.2),
                max_y.max(b.3),
            ),
            None => b,
        });
    }
    rect
}

fn union_of_all(layers: &[Layer]) -> Option<(f32, f32, f32, f32)> {
    let all: Vec<usize> = (0..layers.len()).collect();
    union_of(layers, &all)
}

fn pad_rect(rect: (f32, f32, f32, f32), pad: f32) -> (f32, f32, f32, f32) {
    (
        rect.0 - pad,
        rect.1 - pad,
        rect.2 + pad,
        rect.3 + pad,
    )
}

fn upscale_nearest(img: &RgbaImage, zoom: u32) -> RgbaImage {
    let (w, h) = (img.width(), img.height());
    let mut out = RgbaImage::new(w * zoom, h * zoom);
    for y in 0..h {
        for x in 0..w {
            let px = *img.get_pixel(x, y);
            for oy in 0..zoom {
                for ox in 0..zoom {
                    out.put_pixel(x * zoom + ox, y * zoom + oy, px);
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    fn cell(src: (i32, i32, i32, i32), origin: (f32, f32), flags: i32, char_ref: i32) -> XSpriteRaw {
        XSpriteRaw {
            name_bytes: Vec::new(),
            src_rect: src,
            origin,
            subflags: Vec::new(),
            char_ref,
            flags,
        }
    }

    #[test]
    fn local_anchor_mirrors_about_the_source_rect() {
        let c = cell((10, 20, 8, 8), (14.0, 24.0), 0, 0);
        assert_eq!(local_anchor(&c, false), (4.0, 4.0));
        assert_eq!(local_anchor(&c, true), (4.0, 4.0));
        let c = cell((10, 20, 8, 8), (12.0, 26.0), 0, 0);
        assert_eq!(local_anchor(&c, false), (2.0, 6.0));
        assert_eq!(local_anchor(&c, true), (6.0, 6.0));
    }

    #[test]
    fn face_items_only_draw_the_neutral_variants() {
        assert!(face_item_is_drawn("Pupils Mid"));
        assert!(face_item_is_drawn("Eyebrows Down"));
        assert!(face_item_is_drawn("Mouth Closed"));
        assert!(face_item_is_drawn("Eyelids Rest"));
        assert!(!face_item_is_drawn("Pupils Left"));
        assert!(!face_item_is_drawn("Mouth Grin"));
        assert!(!face_item_is_drawn("Eyelids Closed"));
    }

    #[test]
    fn clothes_mapping_matches_char_ref_and_flags() {
        let cells = vec![
            Some(cell((0, 0, 4, 4), (2.0, 2.0), -1, 0)),
            Some(cell((4, 0, 4, 4), (6.0, 2.0), 5, 9)),
            Some(cell((8, 0, 4, 4), (10.0, 2.0), 5, 3)),
        ];
        // Last matching cell wins, char_ref must be positive.
        assert_eq!(clothes_cell_for(&cells, 5), Some(2));
        assert_eq!(clothes_cell_for(&cells, 1), None);
    }

    #[test]
    fn paint_layer_blits_at_the_anchor() {
        let mut sheet = RgbaImage::new(2, 1);
        sheet.put_pixel(0, 0, Rgba([255, 0, 0, 255]));
        sheet.put_pixel(1, 0, Rgba([0, 255, 0, 255]));
        let layer = Layer {
            src: (0, 0, 2, 1),
            anchor: (0.5, 0.5),
            mirror: false,
            tint: (1.0, 1.0, 1.0),
            sheet: SheetKind::Base,
            cx: 4.0,
            cy: 4.0,
            rot: 0.0,
            scale_x: 1.0,
            scale_y: 1.0,
        };
        let mut canvas = RgbaImage::new(8, 8);
        paint_layer(&mut canvas, &sheet, &layer, 0.0, 0.0);
        // Anchor is between the two pixels: the red one sits left of it, the green one right.
        assert_eq!(*canvas.get_pixel(3, 3), Rgba([255, 0, 0, 255]));
        assert_eq!(*canvas.get_pixel(4, 3), Rgba([0, 255, 0, 255]));
        assert_eq!(*canvas.get_pixel(2, 3), Rgba([0, 0, 0, 0]));
    }

    #[test]
    fn paint_layer_mirrors_about_the_anchor() {
        let mut sheet = RgbaImage::new(2, 1);
        sheet.put_pixel(0, 0, Rgba([255, 0, 0, 255]));
        sheet.put_pixel(1, 0, Rgba([0, 255, 0, 255]));
        let layer = Layer {
            src: (0, 0, 2, 1),
            anchor: (1.0, 0.5),
            mirror: true,
            tint: (1.0, 1.0, 1.0),
            sheet: SheetKind::Base,
            cx: 4.0,
            cy: 4.0,
            rot: 0.0,
            scale_x: 1.0,
            scale_y: 1.0,
        };
        let mut canvas = RgbaImage::new(8, 8);
        paint_layer(&mut canvas, &sheet, &layer, 0.0, 0.0);
        // Reading right to left: the mirrored cell starts at the anchor pixel.
        assert_eq!(*canvas.get_pixel(3, 3), Rgba([0, 255, 0, 255]));
        assert_eq!(*canvas.get_pixel(4, 3), Rgba([255, 0, 0, 255]));
    }

    #[test]
    fn paint_layer_applies_the_tint() {
        let mut sheet = RgbaImage::new(1, 1);
        sheet.put_pixel(0, 0, Rgba([200, 100, 50, 255]));
        let layer = Layer {
            src: (0, 0, 1, 1),
            anchor: (0.0, 0.0),
            mirror: false,
            tint: (0.5, 0.5, 0.5),
            sheet: SheetKind::Hair,
            cx: 2.0,
            cy: 2.0,
            rot: 0.0,
            scale_x: 1.0,
            scale_y: 1.0,
        };
        let mut canvas = RgbaImage::new(4, 4);
        paint_layer(&mut canvas, &sheet, &layer, 0.0, 0.0);
        assert_eq!(*canvas.get_pixel(2, 2), Rgba([100, 50, 25, 255]));
    }
}
