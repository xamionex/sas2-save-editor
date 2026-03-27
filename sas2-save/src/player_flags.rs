pub struct PlayerFlags {
    pub flags: Vec<String>,
    pub bounty_seed: i32,
    pub bounties_complete: i32,
    pub ng_level: i32, // derived from flags, but we can store it anyway
}
