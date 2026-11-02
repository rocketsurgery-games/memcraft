//! Item stacks: what a container slot or a hotbar slot holds, how each shows
//! (icon and name) and how stacks move between slots.
//!
//! An item is a Pocket Edition item id with a damage value; ids below 256 are
//! blocks (damage is then the block's data), the rest are items, which have
//! the icons and names of `item_icons`.

use mcpe_world::Tag;

use crate::blocks::{self, Shape, Tile};
use crate::catalog;
use crate::item_icons;

/// The most a stack holds when stacks merge.
pub const MAX_STACK: u8 = 64;

/// A stack of items: Pocket Edition item `id` (a block's id below 256), its
/// damage or variant, and how many.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stack {
    pub id: i16,
    pub damage: i16,
    pub count: u8,
}

impl Stack {
    pub fn new(id: i16, damage: i16, count: u8) -> Self {
        Self { id, damage, count }
    }

    /// The stack in an `Items` entry of a block entity (`id`, `Damage`,
    /// `Count`). An entry without an id, or with no items, is none.
    pub fn from_item(item: &Tag) -> Option<Stack> {
        let number = |k: &str| item.get(k).and_then(Tag::as_i64);
        let count = u8::try_from(number("Count")?).ok().filter(|&c| c > 0)?;
        let damage = number("Damage").and_then(|d| i16::try_from(d).ok()).unwrap_or(0);
        Some(Stack { id: i16::try_from(number("id")?).ok()?, damage, count })
    }

    /// The same kind of item (a count apart).
    pub fn same_item(&self, other: &Stack) -> bool {
        (self.id, self.damage) == (other.id, other.damage)
    }

    /// How many more fit in this stack.
    pub fn room(&self) -> u8 {
        MAX_STACK.saturating_sub(self.count)
    }

    /// The block `(id, data)` that placing this puts down, if it is a block
    /// the viewer draws.
    pub fn block(&self) -> Option<(u8, u8)> {
        let id = u8::try_from(self.id).ok().filter(|&id| id > 0)?;
        let data = u8::try_from(self.damage).unwrap_or(0);
        (blocks::look(id, data).shape != Shape::Air).then_some((id, data))
    }
}

/// The atlas tile and tint (the colour of a flat-coloured block, else white)
/// that show `stack`: a block's catalog icon (the nearest entry, see
/// [`catalog::nearest`]; a block the catalog lacks gets its side) or the
/// item's icon. `None` for an item without one.
pub fn icon(stack: Stack) -> Option<(Tile, [f32; 3])> {
    match u8::try_from(stack.id) {
        Ok(id @ 1..=255) => {
            let data = u8::try_from(stack.damage).unwrap_or(0);
            match catalog::nearest(id, data) {
                Some(entry) => Some((entry.icon(), entry.look().color)),
                None => {
                    let look = blocks::look(id, data);
                    (look.shape != Shape::Air).then_some((look.tiles[4], look.color))
                }
            }
        }
        _ => Some((item_icons::icon(stack.id, stack.damage)?, [1.0; 3])),
    }
}

/// What `stack` is called: a block by its catalog entry, an item by its name
/// in `item_icons`; anything else by its numbers.
pub fn name(stack: Stack) -> String {
    let named = match u8::try_from(stack.id) {
        Ok(id @ 1..=255) => catalog::nearest(id, u8::try_from(stack.damage).unwrap_or(0)).map(|e| e.name),
        _ => item_icons::name(stack.id, stack.damage),
    };
    match named {
        Some(name) => name.to_string(),
        None if (1..256).contains(&stack.id) => format!("Block {}:{}", stack.id, stack.damage),
        None => format!("Item {}:{}", stack.id, stack.damage),
    }
}

/// A click on slot `a` while `b` is the slot to trade with: the stacks swap,
/// except that two of the same item merge into `b` as far as it has room
/// (`a` keeps the rest).
pub fn exchange(a: &mut Option<Stack>, b: &mut Option<Stack>) {
    if let (Some(from), Some(to)) = (a.as_mut(), b.as_mut())
        && from.same_item(to)
    {
        let n = from.count.min(to.room());
        to.count += n;
        from.count -= n;
        if from.count == 0 {
            *a = None;
        }
    } else {
        std::mem::swap(a, b);
    }
}

/// Moves the stack in `from` into `to`: onto stacks of the same item first,
/// as far as they have room, then into the first empty slot. What does not
/// fit stays in `from`.
pub fn send(from: &mut Option<Stack>, to: &mut [Option<Stack>]) {
    let Some(stack) = from.as_mut() else {
        return;
    };
    let item = *stack;
    for slot in to.iter_mut().flatten().filter(|s| s.same_item(&item)) {
        let n = stack.count.min(slot.room());
        slot.count += n;
        stack.count -= n;
    }
    if stack.count > 0
        && let Some(empty) = to.iter_mut().find(|s| s.is_none())
    {
        *empty = Some(*stack);
        stack.count = 0;
    }
    if stack.count == 0 {
        *from = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(id: i16, count: u8) -> Option<Stack> {
        Some(Stack::new(id, 0, count))
    }

    #[test]
    fn exchange_swaps_different_items_and_fills_empty_slots() {
        // Take: a stack goes into an empty slot.
        let (mut a, mut b) = (s(265, 3), None);
        exchange(&mut a, &mut b);
        assert_eq!((a, b), (None, s(265, 3)));
        // Put the other way, and swap.
        let (mut a, mut b) = (s(265, 3), s(1, 1));
        exchange(&mut a, &mut b);
        assert_eq!((a, b), (s(1, 1), s(265, 3)));
        // Damage tells items apart: red and green dye swap.
        let (mut a, mut b) = (Some(Stack::new(351, 1, 2)), Some(Stack::new(351, 2, 5)));
        exchange(&mut a, &mut b);
        assert_eq!((a, b), (Some(Stack::new(351, 2, 5)), Some(Stack::new(351, 1, 2))));
        // Nothing with nothing.
        let (mut a, mut b) = (None, None);
        exchange(&mut a, &mut b);
        assert_eq!((a, b), (None, None));
    }

    #[test]
    fn exchange_merges_the_same_item_up_to_a_full_stack() {
        let (mut a, mut b) = (s(1, 10), s(1, 20));
        exchange(&mut a, &mut b);
        assert_eq!((a, b), (None, s(1, 30)));
        let (mut a, mut b) = (s(1, 50), s(1, 40));
        exchange(&mut a, &mut b);
        assert_eq!((a, b), (s(1, 26), s(1, 64)));
        let (mut a, mut b) = (s(1, 5), s(1, 64));
        exchange(&mut a, &mut b);
        assert_eq!((a, b), (s(1, 5), s(1, 64)), "no room: nothing moves");
    }

    #[test]
    fn send_fills_stacks_of_the_item_then_the_first_empty_slot() {
        let mut to = [s(2, 1), s(1, 60), s(1, 64), None, None];
        let mut from = s(1, 30);
        send(&mut from, &mut to);
        assert_eq!(from, None);
        assert_eq!(to, [s(2, 1), s(1, 64), s(1, 64), s(1, 26), None]);
        // Room in slot 3 first, and the rest in the first empty slot after it.
        let mut from = s(1, 60);
        send(&mut from, &mut to);
        assert_eq!((from, to[3], to[4]), (None, s(1, 64), s(1, 22)));
    }

    #[test]
    fn send_leaves_what_does_not_fit() {
        let mut to = [s(1, 64), s(2, 1)];
        let mut from = s(1, 3);
        send(&mut from, &mut to);
        assert_eq!((from, to), (s(1, 3), [s(1, 64), s(2, 1)]));
        let mut to = [s(1, 62), s(2, 1)];
        send(&mut from, &mut to);
        assert_eq!((from, to), (s(1, 1), [s(1, 64), s(2, 1)]), "two fit, one stays");
        let mut none: Option<Stack> = None;
        send(&mut none, &mut to);
        assert_eq!(none, None);
    }

    #[test]
    fn blocks_and_items_have_icons_and_names() {
        assert_eq!(name(Stack::new(4, 0, 1)), "Cobblestone");
        assert_eq!(name(Stack::new(265, 0, 1)), "Iron Ingot");
        assert_eq!(name(Stack::new(351, 4, 1)), "Lapis Lazuli");
        assert_eq!(name(Stack::new(95, 0, 1)), "Block 95:0");
        assert_eq!(name(Stack::new(342, 0, 1)), "Item 342:0");
        assert!(icon(Stack::new(4, 0, 1)).is_some());
        assert!(icon(Stack::new(265, 0, 1)).is_some());
        // A lit furnace is not in the catalog but is drawn.
        assert!(icon(Stack::new(62, 0, 1)).is_some());
        for stack in [Stack::new(0, 0, 1), Stack::new(342, 0, 1), Stack::new(-5, 0, 1), Stack::new(30000, 0, 1)] {
            assert_eq!(icon(stack), None, "{stack:?}");
        }
    }

    #[test]
    fn only_blocks_place() {
        assert_eq!(Stack::new(4, 0, 1).block(), Some((4, 0)));
        assert_eq!(Stack::new(17, 1, 9).block(), Some((17, 1)));
        assert_eq!(Stack::new(265, 0, 1).block(), None);
        assert_eq!(Stack::new(0, 0, 1).block(), None);
        assert_eq!(Stack::new(-1, 0, 1).block(), None);
    }
}
