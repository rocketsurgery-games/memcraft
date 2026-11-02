//! What a container block holds: the `Items` of its block entity, laid out
//! as the slots its window shows. Opening the window is the job of
//! `container_view`.
//!
//! Chests, furnaces (lit too), dispensers, droppers, hoppers and brewing
//! stands are containers. Each item names the slot it sits in (`Slot`), so
//! empty slots are simply missing from the list. A double chest is two
//! chest blocks, each with its own 27 slots; they show as one 54-slot window.

use bevy::prelude::*;
use mcpe_world::Tag;

use crate::hotbar::Block;
use crate::items::Stack;
use crate::world::WorldMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Chest,
    Furnace,
    Dispenser,
    Dropper,
    Hopper,
    BrewingStand,
}

/// The kind of container `block` is, if it is one: chest (54) and trapped
/// chest (146), furnace (61) and lit furnace (62), dispenser (23), dropper
/// (158), hopper (154), brewing stand (117).
pub fn kind((id, _): Block) -> Option<Kind> {
    Some(match id {
        54 | 146 => Kind::Chest,
        61 | 62 => Kind::Furnace,
        23 => Kind::Dispenser,
        158 => Kind::Dropper,
        154 => Kind::Hopper,
        117 => Kind::BrewingStand,
        _ => return None,
    })
}

/// One slot of the window, at a cell of its grid, and where its stack is
/// kept: slot number `number` of the block entity at `cell` (a double chest's
/// slots are in two block entities).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Slot {
    pub column: u8,
    pub row: u8,
    pub cell: IVec3,
    pub number: u8,
    pub stack: Option<Stack>,
}

/// A container's window: a grid of cells, some of them slots.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Contents {
    pub title: String,
    pub columns: u8,
    pub rows: u8,
    pub slots: Vec<Slot>,
}

/// Where each slot of `kind` is in its window, as `(column, row)` in slot
/// order, and the size of the window. A double chest is [`Kind::Chest`] with
/// the second half's rows added below (see [`contents`]).
fn layout(kind: Kind) -> (u8, u8, Vec<(u8, u8)>) {
    let grid = |columns: u8, rows: u8| (0..rows).flat_map(|r| (0..columns).map(move |c| (c, r))).collect::<Vec<_>>();
    match kind {
        Kind::Chest => (9, 3, grid(9, 3)),
        Kind::Dispenser | Kind::Dropper => (3, 3, grid(3, 3)),
        Kind::Hopper => (5, 1, grid(5, 1)),
        // Input on top, fuel below it, the result to the right.
        Kind::Furnace => (3, 3, vec![(0, 0), (0, 2), (2, 1)]),
        // The ingredient, the three bottles under it, the blaze powder.
        Kind::BrewingStand => (3, 2, vec![(1, 0), (0, 1), (1, 1), (2, 1), (2, 0)]),
    }
}

fn name(kind: Kind) -> &'static str {
    match kind {
        Kind::Chest => "Chest",
        Kind::Furnace => "Furnace",
        Kind::Dispenser => "Dispenser",
        Kind::Dropper => "Dropper",
        Kind::Hopper => "Hopper",
        Kind::BrewingStand => "Brewing Stand",
    }
}

/// The item stack in each slot of the block entity `be`, by slot number.
fn stacks(be: Option<&Tag>) -> Vec<(usize, Stack)> {
    let items = be.and_then(|be| be.get("Items")).and_then(Tag::as_list).unwrap_or_default();
    items
        .iter()
        .filter_map(|item| Some((usize::try_from(item.get("Slot")?.as_i64()?).ok()?, Stack::from_item(item)?)))
        .collect()
}

/// The name a container's block entity has in the world's files.
fn entity_id(kind: Kind) -> &'static str {
    match kind {
        Kind::Chest => "Chest",
        Kind::Furnace => "Furnace",
        Kind::Dispenser => "Dispenser",
        Kind::Dropper => "Dropper",
        Kind::Hopper => "Hopper",
        Kind::BrewingStand => "BrewingStand",
    }
}

/// Puts `stack` (or nothing) in slot `number` of the container at `cell`, by
/// writing the block entity's `Items` back with [`WorldMap::set_block_entity`],
/// so that saving and downloading carry it. Each item is a compound of `Count`
/// (byte), `Damage` (short), `Slot` (byte) and `id` (short), as the game
/// writes them. Everything else the entity has stays as it was, and so do
/// the other slots' items, including any this module did not understand; the
/// slot's own item keeps its other tags (a name, enchantments) while it is the
/// same item. A container without a block entity gets one. Returns false for
/// a cell that is no container or is outside the world.
pub fn set_stack(map: &mut WorldMap, cell: IVec3, number: u8, stack: Option<Stack>) -> bool {
    let Some(kind) = kind(map.block(cell.x, cell.y, cell.z)) else {
        return false;
    };
    let mut entries = match map.block_entity(cell) {
        Some(Tag::Compound(entries)) => entries.clone(),
        _ => [("id", Tag::String(entity_id(kind).into())), ("x", Tag::Int(cell.x)), ("y", Tag::Int(cell.y)), ("z", Tag::Int(cell.z))]
            .map(|(k, v)| (k.to_owned(), v))
            .into(),
    };
    let at = entries.iter().position(|(k, _)| k == "Items");
    let mut items = match at {
        Some(i) => match entries.remove(i).1 {
            Tag::List(_, items) => items,
            _ => Vec::new(),
        },
        None => Vec::new(),
    };
    let slot_of = |item: &Tag| item.get("Slot").and_then(Tag::as_i64);
    let old = items.iter().position(|item| slot_of(item) == Some(i64::from(number)));
    match (stack, old) {
        (None, Some(i)) => {
            items.remove(i);
        }
        (Some(stack), Some(i)) => {
            let same = Stack::from_item(&items[i]).is_some_and(|old| old.same_item(&stack));
            if same {
                let Tag::Compound(fields) = &mut items[i] else { unreachable!("it has a slot") };
                fields.retain(|(k, _)| k != "Count");
                fields.push(("Count".into(), Tag::Byte(stack.count as i8)));
            } else {
                items[i] = item(number, stack);
            }
        }
        (Some(stack), None) => items.push(item(number, stack)),
        (None, None) => {}
    }
    entries.insert(at.unwrap_or(entries.len()), ("Items".to_owned(), Tag::List(10, items)));
    map.set_block_entity(cell, Tag::Compound(entries))
}

/// The compound the game keeps an item stack in.
fn item(number: u8, stack: Stack) -> Tag {
    Tag::Compound(vec![
        ("Count".into(), Tag::Byte(stack.count as i8)),
        ("Damage".into(), Tag::Short(stack.damage)),
        ("Slot".into(), Tag::Byte(number as i8)),
        ("id".into(), Tag::Short(stack.id)),
    ])
}

/// The cell of the other half of the double chest at `cell`, if there is one.
///
/// The game stores it in the chest's block entity as `pairx` and `pairz`. A
/// chest without them is paired the way the game pairs chests as they are
/// placed: with a chest of the same block facing the same way (block data 2
/// to 5: north, south, west, east) next to it, sideways to the way it faces,
/// west or north first.
fn partner(map: &WorldMap, cell: IVec3) -> Option<IVec3> {
    let block = |c: IVec3| map.block(c.x, c.y, c.z);
    let (id, data) = block(cell);
    let paired = |p: IVec3| p != cell && block(p).0 == id;
    let be = map.block_entity(cell);
    let at = |k: &str| be?.get(k)?.as_i64().and_then(|v| i32::try_from(v).ok());
    if let (Some(x), Some(z)) = (at("pairx"), at("pairz")) {
        return Some(IVec3::new(x, cell.y, z)).filter(|&p| paired(p));
    }
    let side = if matches!(data, 2 | 3) { IVec3::X } else { IVec3::Z };
    [cell - side, cell + side].into_iter().find(|&p| paired(p) && block(p).1 == data)
}

/// What the container at `cell` holds, or `None` when the block there is not
/// a container. A chest paired with another one (see [`partner`]) gives both
/// halves, the one that comes first in the world (lower x, then lower z) on
/// top.
pub fn contents(map: &WorldMap, cell: IVec3) -> Option<Contents> {
    let kind = kind(map.block(cell.x, cell.y, cell.z))?;
    let be = map.block_entity(cell);
    let (columns, rows, places) = layout(kind);
    let mut halves = vec![(cell, be)];
    let mut title = name(kind).to_string();
    if kind == Kind::Chest
        && let Some(other) = partner(map, cell)
    {
        halves.push((other, map.block_entity(other)));
        halves.sort_by_key(|(c, _)| (c.x, c.z));
        title = "Large Chest".to_string();
    }
    if let Some(custom) = be.and_then(|be| be.get("CustomName")).and_then(Tag::as_str).filter(|n| !n.is_empty()) {
        title = custom.to_string();
    }
    let mut slots = Vec::new();
    for (half, (cell, be)) in halves.iter().enumerate() {
        let stacks = stacks(*be);
        for (n, &(column, row)) in places.iter().enumerate() {
            let stack = stacks.iter().find(|&&(slot, _)| slot == n).map(|&(_, s)| s);
            slots.push(Slot { column, row: row + half as u8 * rows, cell: *cell, number: n as u8, stack });
        }
    }
    Some(Contents { title, columns, rows: rows * halves.len() as u8, slots })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every container block entity of the sample world: its cell, id, and
    /// the whole entity.
    fn world_containers(map: &WorldMap) -> Vec<(IVec3, &Tag)> {
        let mut found: Vec<_> = map
            .world()
            .columns
            .values()
            .flat_map(|c| &c.block_entities)
            .filter_map(|be| {
                let at = |k: &str| be.get(k).and_then(Tag::as_i64).map(|v| v as i32);
                Some((IVec3::new(at("x")?, at("y")?, at("z")?), be))
            })
            .filter(|(cell, _)| kind(map.block(cell.x, cell.y, cell.z)).is_some())
            .collect();
        found.sort_by_key(|(c, _)| c.to_array());
        found
    }

    fn item(slot: i8, id: i16, damage: i16, count: i8) -> Tag {
        Tag::Compound(vec![
            ("Count".into(), Tag::Byte(count)),
            ("Damage".into(), Tag::Short(damage)),
            ("Slot".into(), Tag::Byte(slot)),
            ("id".into(), Tag::Short(id)),
        ])
    }

    fn entity(name: &str, cell: IVec3, items: Vec<Tag>, extra: Vec<(&str, Tag)>) -> Tag {
        let mut entries = vec![
            ("Items".to_string(), Tag::List(10, items)),
            ("id".to_string(), Tag::String(name.into())),
            ("x".to_string(), Tag::Int(cell.x)),
            ("y".to_string(), Tag::Int(cell.y)),
            ("z".to_string(), Tag::Int(cell.z)),
        ];
        entries.extend(extra.into_iter().map(|(k, v)| (k.to_string(), v)));
        Tag::Compound(entries)
    }

    /// The sample world, with a block at each given cell and, if given, its block entity.
    fn world_with(placed: &[(IVec3, Block, Option<Tag>)]) -> WorldMap {
        let mut map = WorldMap::new(crate::load_world(None).unwrap());
        for (cell, (id, data), be) in placed {
            assert!(map.set_block(cell.x, cell.y, cell.z, *id, *data));
            if let Some(be) = be {
                assert!(map.set_block_entity(*cell, be.clone()));
            }
        }
        map
    }

    /// A stack as `(id, damage, count)`.
    type Plain = (i16, i16, u8);

    /// The stacks of a window by `(column, row)`.
    fn stacks_of(contents: &Contents) -> Vec<((u8, u8), Plain)> {
        contents.slots.iter().filter_map(|s| Some(((s.column, s.row), s.stack.map(|s| (s.id, s.damage, s.count))?))).collect()
    }

    /// Open air in the sample world, in a loaded column.
    const AT: IVec3 = IVec3::new(48, 120, 40);

    #[test]
    fn a_chest_of_the_world_shows_its_stacks_in_their_slots() {
        let map = WorldMap::new(crate::load_world(None).unwrap());
        let c = contents(&map, IVec3::new(93, 77, 110)).expect("a chest");
        assert_eq!((c.title.as_str(), c.columns, c.rows, c.slots.len()), ("Chest", 9, 3, 27));
        // Slots count along the rows: 0 to 8 are the top one.
        assert_eq!(
            stacks_of(&c),
            [
                ((0, 0), (344, 0, 4)),
                ((1, 0), (334, 0, 1)),
                ((2, 0), (329, 0, 1)),
                ((3, 0), (69, 0, 64)),
                ((4, 0), (323, 0, 16)),
                ((5, 0), (54, 0, 64)),
                ((6, 0), (50, 0, 64)),
                ((7, 0), (171, 14, 64)),
                ((8, 0), (171, 14, 36)),
                ((2, 1), (338, 0, 4)),
                ((3, 1), (69, 0, 2)),
            ]
        );
        assert!(c.slots.iter().find(|s| (s.column, s.row) == (0, 1)).is_some_and(|s| s.stack.is_none()), "empty slots are there, and empty");
        // Every container of the world opens; an empty chest shows no stacks.
        for (cell, _) in world_containers(&map) {
            assert!(contents(&map, cell).is_some(), "{cell}");
        }
        assert_eq!(contents(&map, IVec3::new(88, 75, 108)).unwrap().slots.iter().filter(|s| s.stack.is_some()).count(), 0);
        // The dispenser holds name tags.
        assert_eq!(stacks_of(&contents(&map, IVec3::new(106, 67, 114)).unwrap()), [((0, 0), (421, 0, 1)), ((1, 0), (421, 0, 1)), ((2, 0), (421, 0, 1))]);
    }

    #[test]
    fn each_kind_of_container_lays_out_its_slots() {
        let at = |dx| AT + IVec3::X * dx;
        let n = |c: &Contents| (c.columns, c.rows, c.slots.len());
        let stack = |slot, id| item(slot, id, 0, 1);
        let map = world_with(&[
            (at(0), (61, 2), Some(entity("Furnace", at(0), vec![stack(0, 4), stack(1, 263), stack(2, 265)], vec![]))),
            (at(1), (62, 2), Some(entity("Furnace", at(1), vec![stack(2, 266)], vec![]))),
            (at(2), (23, 3), Some(entity("Dispenser", at(2), vec![stack(0, 262), stack(8, 261)], vec![]))),
            (at(3), (158, 3), Some(entity("Dropper", at(3), vec![stack(4, 280)], vec![]))),
            (at(4), (154, 0), Some(entity("Hopper", at(4), vec![stack(0, 1), stack(4, 2)], vec![]))),
            (at(5), (117, 0), Some(entity("BrewingStand", at(5), vec![stack(0, 372), stack(1, 373), stack(3, 373), stack(4, 377)], vec![]))),
            (at(6), (146, 3), None),
        ]);
        let furnace = contents(&map, at(0)).unwrap();
        assert_eq!((furnace.title.as_str(), n(&furnace)), ("Furnace", (3, 3, 3)));
        assert_eq!(stacks_of(&furnace), [((0, 0), (4, 0, 1)), ((0, 2), (263, 0, 1)), ((2, 1), (265, 0, 1))], "input, fuel, output");
        let lit = contents(&map, at(1)).unwrap();
        assert_eq!((lit.title.as_str(), stacks_of(&lit)), ("Furnace", vec![((2, 1), (266, 0, 1))]), "a lit furnace is a furnace");
        for (cell, title, size) in [(at(2), "Dispenser", (3, 3, 9)), (at(3), "Dropper", (3, 3, 9)), (at(4), "Hopper", (5, 1, 5)), (at(5), "Brewing Stand", (3, 2, 5)), (at(6), "Chest", (9, 3, 27))] {
            let c = contents(&map, cell).unwrap();
            assert_eq!((c.title.as_str(), n(&c)), (title, size), "{title}");
        }
        assert_eq!(stacks_of(&contents(&map, at(2)).unwrap()), [((0, 0), (262, 0, 1)), ((2, 2), (261, 0, 1))]);
        assert_eq!(stacks_of(&contents(&map, at(3)).unwrap()), [((1, 1), (280, 0, 1))]);
        assert_eq!(stacks_of(&contents(&map, at(4)).unwrap()), [((0, 0), (1, 0, 1)), ((4, 0), (2, 0, 1))]);
        // The ingredient on top, the bottles under it, the fuel beside it.
        assert_eq!(stacks_of(&contents(&map, at(5)).unwrap()), [((1, 0), (372, 0, 1)), ((0, 1), (373, 0, 1)), ((2, 1), (373, 0, 1)), ((2, 0), (377, 0, 1))]);
        assert!(contents(&map, at(6)).unwrap().slots.iter().all(|s| s.stack.is_none()), "a trapped chest with no block entity is empty");
    }

    #[test]
    fn other_blocks_and_odd_entries_are_not_containers_or_stacks() {
        let junk = vec![item(0, 1, 0, 0), item(-1, 1, 0, 1), item(40, 1, 0, 1), item(2, 1, 0, -3), item(3, 1, 0, 5)];
        let map = world_with(&[(AT, (54, 3), Some(entity("Chest", AT, junk, vec![]))), (AT + IVec3::X, (1, 0), None)]);
        assert_eq!(stacks_of(&contents(&map, AT).unwrap()), [((3, 0), (1, 0, 5))], "no count, no slot, no room: no stack");
        assert_eq!(contents(&map, AT + IVec3::X), None);
        assert_eq!(contents(&map, AT + IVec3::Y * 5), None, "air");
        assert_eq!(kind((130, 0)), None, "an ender chest has no items of its own");
    }

    #[test]
    fn a_custom_name_is_the_title() {
        let be = entity("Furnace", AT, vec![], vec![("CustomName", Tag::String("Smelter".into()))]);
        let map = world_with(&[(AT, (61, 2), Some(be))]);
        assert_eq!(contents(&map, AT).unwrap().title, "Smelter");
    }

    #[test]
    fn the_halves_of_a_double_chest_show_as_one_window_whichever_is_clicked() {
        let (west, east) = (AT, AT + IVec3::X);
        let pair = |own: IVec3, other: IVec3| vec![("pairx", Tag::Int(other.x)), ("pairz", Tag::Int(other.z)), ("pairlead", Tag::Byte((own == west) as i8))];
        let map = world_with(&[
            (west, (54, 3), Some(entity("Chest", west, vec![item(0, 1, 0, 1), item(26, 2, 0, 2)], pair(west, east)))),
            (east, (54, 3), Some(entity("Chest", east, vec![item(0, 3, 0, 3), item(13, 4, 0, 4)], pair(east, west)))),
        ]);
        let expected = [((0, 0), (1, 0, 1)), ((8, 2), (2, 0, 2)), ((0, 3), (3, 0, 3)), ((4, 4), (4, 0, 4))];
        for click in [west, east] {
            let c = contents(&map, click).unwrap();
            assert_eq!((c.title.as_str(), c.columns, c.rows, c.slots.len()), ("Large Chest", 9, 6, 54));
            assert_eq!(stacks_of(&c), expected, "clicked {click}");
        }
    }

    #[test]
    fn a_double_chest_along_z_lists_the_lower_z_first() {
        let (north, south) = (AT, AT + IVec3::Z);
        let be = |own: IVec3, other: IVec3, id| entity("Chest", own, vec![item(1, id, 0, 1)], vec![("pairx", Tag::Int(other.x)), ("pairz", Tag::Int(other.z))]);
        let map = world_with(&[(south, (54, 4), Some(be(south, north, 20))), (north, (54, 4), Some(be(north, south, 10)))]);
        assert_eq!(stacks_of(&contents(&map, south).unwrap()), [((1, 0), (10, 0, 1)), ((1, 3), (20, 0, 1))]);
    }

    #[test]
    fn chests_without_pair_tags_pair_when_they_face_the_same_way_side_by_side() {
        let (a, b) = (AT, AT + IVec3::X);
        let put = |cell: IVec3, data, id| (cell, (54, data), Some(entity("Chest", cell, vec![item(0, id, 0, 1)], vec![])));
        // Facing north or south, they pair along x...
        let map = world_with(&[put(a, 2, 5), put(b, 2, 6)]);
        assert_eq!(stacks_of(&contents(&map, b).unwrap()), [((0, 0), (5, 0, 1)), ((0, 3), (6, 0, 1))]);
        // ...facing east or west, along z, and not along x.
        let (c, d) = (a + IVec3::Z * 3, a + IVec3::Z * 4);
        let map = world_with(&[put(c, 5, 7), put(d, 5, 8), put(a, 5, 9), put(b, 5, 10)]);
        assert_eq!(stacks_of(&contents(&map, c).unwrap()), [((0, 0), (7, 0, 1)), ((0, 3), (8, 0, 1))]);
        assert_eq!(contents(&map, a).unwrap().rows, 3);
        // Facing different ways, or beside another block, or diagonally: single.
        for neighbour in [((54, 3), a + IVec3::X), ((54, 2), a + IVec3::Z), ((146, 2), a + IVec3::X), ((1, 0), a - IVec3::X), ((54, 2), a + IVec3::new(1, 1, 0))] {
            let map = world_with(&[put(a, 2, 5), (neighbour.1, neighbour.0, None)]);
            let c = contents(&map, a).unwrap();
            assert_eq!((c.title.as_str(), c.rows), ("Chest", 3), "{neighbour:?}");
        }
    }

    #[test]
    fn a_pair_tag_that_names_no_chest_leaves_a_single_chest() {
        let be = entity("Chest", AT, vec![item(0, 1, 0, 1)], vec![("pairx", Tag::Int(AT.x + 1)), ("pairz", Tag::Int(AT.z))]);
        let map = world_with(&[(AT, (54, 3), Some(be)), (AT + IVec3::X, (1, 0), None)]);
        let c = contents(&map, AT).unwrap();
        assert_eq!((c.title.as_str(), c.rows, stacks_of(&c)), ("Chest", 3, vec![((0, 0), (1, 0, 1))]));
    }

    /// The sample world with `map`'s edits saved over it, as the zip download
    /// and the world server write them, and loaded again.
    fn saved_and_reloaded(map: &WorldMap) -> WorldMap {
        let mut writer = crate::embedded_writer().unwrap();
        writer.put_columns(&map.edited_columns()).unwrap();
        let saved = writer.finish().unwrap();
        WorldMap::new(mcpe_world::World::from_files(&saved.level_dat, saved.db_files).unwrap())
    }

    const CHEST: IVec3 = IVec3::new(93, 77, 110);

    #[test]
    fn set_stack_edits_one_slot_and_leaves_the_rest_of_the_entity() {
        let mut map = WorldMap::new(crate::load_world(None).unwrap());
        let before = map.block_entity(CHEST).unwrap().clone();
        assert!(map.take_edited_cells().is_empty());
        // Replace the egg in slot 0, add one in empty slot 10, empty slot 1 (a leather).
        assert!(set_stack(&mut map, CHEST, 0, Some(Stack::new(265, 0, 7))));
        assert!(set_stack(&mut map, CHEST, 10, Some(Stack::new(351, 4, 3))));
        assert!(set_stack(&mut map, CHEST, 1, None));
        assert_eq!(map.take_edited_cells(), [CHEST; 3]);
        let c = contents(&map, CHEST).unwrap();
        let stacks = stacks_of(&c);
        assert_eq!(stacks[0], ((0, 0), (265, 0, 7)));
        assert!(!stacks.iter().any(|&(at, _)| at == (1, 0)), "slot 1 is empty");
        assert!(stacks.contains(&((1, 1), (351, 4, 3))));
        assert_eq!(stacks.len(), 11, "one replaced, one added, one gone");
        // The game's own shape: Count a byte, Damage a short, Slot a byte, id a short.
        let added = map.block_entity(CHEST).unwrap().get("Items").unwrap().as_list().unwrap().iter().find(|i| i.get("Slot").and_then(Tag::as_i64) == Some(10)).unwrap();
        assert_eq!(added, &item(10, 351, 4, 3));
        // Nothing else of the entity changed, and the key order is the same.
        let (Tag::Compound(old), Tag::Compound(new)) = (&before, map.block_entity(CHEST).unwrap()) else { panic!() };
        assert_eq!(old.iter().map(|(k, _)| k).collect::<Vec<_>>(), new.iter().map(|(k, _)| k).collect::<Vec<_>>());
        for (k, v) in old.iter().filter(|(k, _)| k != "Items") {
            assert_eq!(Some(v), map.block_entity(CHEST).unwrap().get(k), "{k}");
        }
    }

    #[test]
    fn the_same_item_keeps_its_other_tags_and_unknown_entries_stay() {
        let named = Tag::Compound(vec![
            ("Count".into(), Tag::Byte(2)),
            ("Damage".into(), Tag::Short(0)),
            ("Slot".into(), Tag::Byte(0)),
            ("id".into(), Tag::Short(276)),
            ("tag".into(), Tag::Compound(vec![("display".into(), Tag::Compound(vec![("Name".into(), Tag::String("Sting".into()))]))])),
        ]);
        let odd = item(5, 1, 0, 0);
        let mut map = world_with(&[(AT, (54, 3), Some(entity("Chest", AT, vec![named.clone(), odd.clone()], vec![])))]);
        // More of the same item: only the count changes.
        assert!(set_stack(&mut map, AT, 0, Some(Stack::new(276, 0, 5))));
        let items = map.block_entity(AT).unwrap().get("Items").unwrap().as_list().unwrap();
        assert_eq!((items[0].get("Count"), items[0].get("tag")), (Some(&Tag::Byte(5)), named.get("tag")));
        // Another item in that slot replaces it whole; an entry that was no stack stays.
        assert!(set_stack(&mut map, AT, 0, Some(Stack::new(1, 0, 1))));
        let items = map.block_entity(AT).unwrap().get("Items").unwrap().as_list().unwrap();
        assert_eq!(items, [item(0, 1, 0, 1), odd]);
    }

    #[test]
    fn a_container_without_an_entity_gets_one_and_others_are_refused() {
        let mut map = world_with(&[(AT, (146, 3), None), (AT + IVec3::X, (1, 0), None)]);
        assert!(map.block_entity(AT).is_none());
        assert!(set_stack(&mut map, AT, 4, Some(Stack::new(1, 0, 9))));
        let be = map.block_entity(AT).unwrap();
        assert_eq!(be.get("id").and_then(Tag::as_str), Some("Chest"));
        assert_eq!(["x", "y", "z"].map(|k| be.get(k).and_then(Tag::as_i64).unwrap()), [i64::from(AT.x), i64::from(AT.y), i64::from(AT.z)]);
        assert_eq!(stacks_of(&contents(&map, AT).unwrap()), [((4, 0), (1, 0, 9))]);
        // Not a container, and not in the world.
        assert!(!set_stack(&mut map, AT + IVec3::X, 0, Some(Stack::new(1, 0, 1))));
        assert!(!set_stack(&mut map, AT + IVec3::Y * 500, 0, None));
        assert!(map.block_entity(AT + IVec3::X).is_none());
    }

    #[test]
    fn each_half_of_a_double_chest_is_written_to_its_own_entity() {
        let (west, east) = (AT, AT + IVec3::X);
        let mut map = world_with(&[
            (west, (54, 3), Some(entity("Chest", west, vec![], vec![("pairx", Tag::Int(east.x)), ("pairz", Tag::Int(east.z))]))),
            (east, (54, 3), Some(entity("Chest", east, vec![], vec![("pairx", Tag::Int(west.x)), ("pairz", Tag::Int(west.z))]))),
        ]);
        let c = contents(&map, east).unwrap();
        // Window slot 30 is the second half's slot 3.
        let slot = &c.slots[30];
        assert_eq!((slot.cell, slot.number), (east, 3));
        assert!(set_stack(&mut map, slot.cell, slot.number, Some(Stack::new(5, 0, 2))));
        assert_eq!(stacks_of(&contents(&map, west).unwrap()), [((3, 3), (5, 0, 2))]);
        assert_eq!(map.block_entity(west).unwrap().get("Items").unwrap().as_list().unwrap().len(), 0);
        assert_eq!(map.block_entity(east).unwrap().get("Items").unwrap().as_list().unwrap().len(), 1);
    }

    #[test]
    fn edits_survive_the_world_writer() {
        let mut map = WorldMap::new(crate::load_world(None).unwrap());
        assert!(set_stack(&mut map, CHEST, 0, Some(Stack::new(265, 0, 7))));
        assert!(set_stack(&mut map, CHEST, 3, None));
        assert!(set_stack(&mut map, CHEST, 26, Some(Stack::new(351, 1, 64))));
        let expected = contents(&map, CHEST).unwrap();
        // Only the chest's column is edited, so only it is written.
        assert_eq!(map.edited_columns().len(), 1);
        let reloaded = saved_and_reloaded(&map);
        assert_eq!(contents(&reloaded, CHEST).unwrap(), expected);
        assert_eq!(stacks_of(&expected)[0], ((0, 0), (265, 0, 7)));
        assert_eq!(stacks_of(&expected).last(), Some(&((8, 2), (351, 1, 64))));
        // The rest of the world's containers read as before.
        let original = WorldMap::new(crate::load_world(None).unwrap());
        for (cell, _) in world_containers(&original).into_iter().filter(|&(c, _)| c != CHEST) {
            assert_eq!(contents(&reloaded, cell), contents(&original, cell), "{cell}");
        }
    }

    /// Lists the sample world's containers with their contents, to find one
    /// to aim at: `cargo test -p memcraft-viewer -- --ignored --nocapture list_containers`.
    #[test]
    #[ignore = "prints a list for people, asserts nothing"]
    fn list_containers() {
        let map = WorldMap::new(crate::load_world(None).unwrap());
        for (cell, be) in world_containers(&map) {
            println!("{cell} block {:?}: {be:?}", map.block(cell.x, cell.y, cell.z));
        }
    }
}
