//! The hotbar: nine slots of blocks to place, one of them selected.

use bevy::prelude::*;

/// A placeable block: Pocket Edition `(id, data)`.
pub type Block = (u8, u8);

#[derive(Resource, Debug, Clone, PartialEq)]
pub struct Hotbar {
    pub slots: [Option<Block>; 9],
    pub selected: usize,
}

impl Default for Hotbar {
    fn default() -> Self {
        // Stone, dirt, grass, oak planks, oak log, glass, cobblestone, bricks, torch.
        let blocks = [(1, 0), (3, 0), (2, 0), (5, 0), (17, 0), (20, 0), (4, 0), (45, 0), (50, 5)];
        Self { slots: blocks.map(Some), selected: 0 }
    }
}

impl Hotbar {
    #[allow(dead_code, reason = "used by block targeting and placing, next")]
    /// The block that placing puts down, if the selected slot holds one.
    pub fn selected_block(&self) -> Option<Block> {
        self.slots[self.selected]
    }
}
