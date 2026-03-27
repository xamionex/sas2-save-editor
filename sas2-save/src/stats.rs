pub struct Stats {
    pub level: i32,
    pub stats: [i32; 9], // PlayerStat count
    pub xp: i64,
    pub silver: i64,
    pub dropped_xp: i64,
    pub dropped_xp_area: i32,
    pub dropped_xp_vec: (f32, f32), // Vector2
    pub time_played: f64,
    pub hazeburnt: bool,
    pub item_class: [i32; 40],    // TOTAL_ITEM_CLASSES
    pub tree_unlocks: [i32; 500], // MAX_TREE_UNLOCKS
    pub class_unlocks: [i32; 3],  // TOTAL_CLASS_UNLOCKS
}
