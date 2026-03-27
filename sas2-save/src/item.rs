#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    pub loot_idx: i32,
    pub count: i32,
    pub upgrade: i32,
    pub stock_piled: bool,
    // artifact_data is not stored in the save file; it's derived.
}
