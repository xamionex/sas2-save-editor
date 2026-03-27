use crate::item::Item;

pub struct Equipment {
    pub inventory_items: Vec<Item>,
    pub equipped_items: [i32; 31], // TOTAL_EQUIPMENT_SLOTS (from enum)
}

impl Default for Equipment {
    fn default() -> Self {
        Self {
            inventory_items: Vec::new(),
            equipped_items: [-1; 31],
        }
    }
}
