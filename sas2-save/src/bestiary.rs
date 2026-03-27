pub const TOTAL_DROPS: usize = 5;

pub struct BestiaryBeast {
    pub kills: i32,
    pub deaths: i32,
    pub drops: [bool; TOTAL_DROPS],
}

pub struct Bestiary {
    pub beasts: Vec<BestiaryBeast>,
}

impl Default for Bestiary {
    fn default() -> Self {
        Self {
            beasts: vec![BestiaryBeast {
                kills: 0,
                deaths: 0,
                drops: [false; TOTAL_DROPS],
            }], // as in C# constructor
        }
    }
}
