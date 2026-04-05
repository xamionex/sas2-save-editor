use eframe::egui;
use egui::{ScrollArea, Ui};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc,
};

/// A node in the XNB file picker tree (directory or leaf file).
#[derive(Clone)]
pub struct XnbNode {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub children: Vec<XnbNode>,
    pub selected: bool,
}

/// Shared state for the background export thread.
pub struct ExportState {
    pub progress: Arc<AtomicUsize>,
    pub total: Arc<AtomicUsize>,
    pub cancel: Arc<AtomicBool>,
    pub done: Arc<AtomicBool>,
}

/// Recursively build an XnbNode tree rooted at `path`. Directories without any .xnb descendants are pruned.
pub fn build_xnb_tree(path: &Path) -> Option<XnbNode> {
    let mut node = XnbNode {
        name: path
            .file_name()
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
                    // Only keep dirs that contain at least one xnb
                    if let Some(child) = build_xnb_tree(&p) {
                        if !child.children.is_empty() {
                            node.children.push(child);
                        }
                    }
                } else if p
                    .extension()
                    .map(|e| e.eq_ignore_ascii_case("xnb"))
                    .unwrap_or(false)
                {
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

/// Walk the tree and collect all leaf paths where the node (and all its ancestors) are selected. `parent_selected` starts as `true` at the root.
pub fn collect_selected(node: &XnbNode, out: &mut Vec<PathBuf>, parent_selected: bool) {
    let effective = parent_selected && node.selected;

    if node.is_dir {
        for child in &node.children {
            collect_selected(child, out, effective);
        }
    } else if effective {
        out.push(node.path.clone());
    }
}

/// Draw the interactive checkbox tree for the XNB picker.
pub fn draw_xnb_tree(ui: &mut Ui, node: &mut XnbNode) {
    if node.is_dir {
        let id = ui.make_persistent_id(&node.name);
        let state =
            egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, false);

        state
            .show_header(ui, |ui| {
                if ui.checkbox(&mut node.selected, &node.name).changed() {
                    let val = node.selected;
                    propagate_selection(node, val);
                }
            })
            .body(|ui| {
                for child in &mut node.children {
                    draw_xnb_tree(ui, child);
                }
            });
    } else {
        ui.checkbox(&mut node.selected, &node.name);
    }
}

/// Recursively set `selected` on a node and all its descendants.
fn propagate_selection(node: &mut XnbNode, val: bool) {
    node.selected = val;
    for child in &mut node.children {
        propagate_selection(child, val);
    }
}

/// Spawn a background thread that exports the given XNB files to an `exports/` folder, preserving the directory structure relative to `game_path`.
pub fn start_export_job(game_path: PathBuf, files: Vec<PathBuf>, overwrite: bool) -> ExportState {
    use sas2_save::xnb_loader::{asset_extension, export_asset_to_file, load_asset_from_xnb};

    let cancel = Arc::new(AtomicBool::new(false));
    let progress = Arc::new(AtomicUsize::new(0));
    let total = Arc::new(AtomicUsize::new(files.len()));
    let done = Arc::new(AtomicBool::new(false));

    let state = ExportState {
        progress: progress.clone(),
        total: total.clone(),
        cancel: cancel.clone(),
        done: done.clone(),
    };

    std::thread::spawn(move || {
        let export_root = PathBuf::from("exports");
        let _ = fs::create_dir_all(&export_root);

        for path in files {
            if cancel.load(Ordering::Relaxed) {
                break;
            }

            // Preserve the game folder's directory structure under exports/
            let relative = path.strip_prefix(&game_path).unwrap_or(&path);
            let mut out_path = export_root.join(relative);
            out_path.set_extension(""); // drop .xnb

            let asset = match load_asset_from_xnb(path.to_str().unwrap()) {
                Ok(a) => a,
                Err(e) => {
                    eprintln!("Failed to load {:?}: {}", path, e);
                    progress.fetch_add(1, Ordering::Relaxed);
                    continue;
                }
            };

            let ext = asset_extension(&asset);
            out_path.set_extension(ext);

            if !overwrite && out_path.exists() {
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

    state
}

/// Draw the export-progress window. Returns `true` when the user closes it.
pub fn show_export_progress(ui: &mut Ui, state: &ExportState) -> bool {
    let progress = state.progress.load(Ordering::Relaxed);
    let total = state.total.load(Ordering::Relaxed);
    let done = state.done.load(Ordering::Relaxed);

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
        state.cancel.store(true, Ordering::Relaxed);
    }

    should_close
}

/// Draw the file-picker window. Returns the list of files to export when the user clicks "Start Export", or `None` if still open / cancelled.
pub fn show_export_picker(
    ui: &mut Ui,
    root: &mut XnbNode,
    overwrite: &mut bool,
) -> Option<Vec<PathBuf>> {
    let mut result = None;
    let mut cancelled = false;

    egui::Window::new("Select XNB files to export")
        .resizable(true)
        .vscroll(true)
        .show(ui.ctx(), |ui| {
            ScrollArea::vertical().show(ui, |ui| {
                draw_xnb_tree(ui, root);
            });

            ui.separator();

            ui.horizontal(|ui| {
                if ui.button("Start Export").clicked() {
                    let mut files = Vec::new();
                    collect_selected(root, &mut files, true);
                    result = Some(files);
                }
                if ui.button("Cancel").clicked() {
                    cancelled = true;
                }
            });

            ui.separator();
            ui.checkbox(overwrite, "Overwrite existing PNGs");
        });

    if cancelled {
        Some(Vec::new()) // empty vec signals "closed without exporting"
    } else {
        result
    }
}
