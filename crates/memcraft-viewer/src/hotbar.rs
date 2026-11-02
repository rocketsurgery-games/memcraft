//! The hotbar: nine slots of item stacks, one of them selected. Blocks in
//! them are placed; other items are only carried (moved to and from
//! containers, see `container_view`). The plugin owns the [`Hotbar`]
//! resource, draws it along the bottom of the window, and lets the number
//! keys 1-9 choose a slot (while the pointer is locked).

use bevy::prelude::*;

use crate::AtlasImage;
use crate::blocks::atlas;
use crate::controls::pointer_locked;
use crate::items::{self, Stack};

/// A placeable block: Pocket Edition `(id, data)`.
pub type Block = (u8, u8);

#[derive(Resource, Debug, Clone, PartialEq)]
pub struct Hotbar {
    pub slots: [Option<Stack>; 9],
    pub selected: usize,
}

impl Default for Hotbar {
    fn default() -> Self {
        // Stone, dirt, grass, oak planks, oak log, glass, cobblestone, bricks, torch.
        let blocks: [(i16, i16); 9] = [(1, 0), (3, 0), (2, 0), (5, 0), (17, 0), (20, 0), (4, 0), (45, 0), (50, 5)];
        Self { slots: blocks.map(|(id, data)| Some(Stack::new(id, data, 1))), selected: 0 }
    }
}

impl Hotbar {
    /// The block that placing puts down, if the selected slot holds one (an
    /// item that is not a block places nothing).
    pub fn selected_block(&self) -> Option<Block> {
        self.slots[self.selected]?.block()
    }

    /// Puts one of `block` in the selected slot (the picker and pick-block do
    /// this), unless the slot holds it already.
    pub fn set_selected_block(&mut self, (id, data): Block) {
        // Not through `deref_mut` unless it changes: `refresh_slots` runs on that.
        if self.selected_block() != Some((id, data)) {
            self.slots[self.selected] = Some(Stack::new(id.into(), data.into(), 1));
        }
    }
}

const KEYS: [KeyCode; 9] = [
    KeyCode::Digit1,
    KeyCode::Digit2,
    KeyCode::Digit3,
    KeyCode::Digit4,
    KeyCode::Digit5,
    KeyCode::Digit6,
    KeyCode::Digit7,
    KeyCode::Digit8,
    KeyCode::Digit9,
];

/// Slot box size in logical pixels: a 32 px icon (2x the 16 px tile, so the
/// pixels stay even) inside a 3 px border and a little padding.
const SLOT: f32 = 46.0;
const BORDER: f32 = 3.0;
const ICON: f32 = 32.0;

const SLOT_FILL: Color = Color::srgba(0.08, 0.08, 0.08, 0.6);
const EDGE: Color = Color::srgba(0.0, 0.0, 0.0, 0.7);
const EDGE_SELECTED: Color = Color::srgb(1.0, 1.0, 1.0);

/// How long the selected block's name stays up, and how much of that is fade.
const NAME_SECONDS: f32 = 2.0;
const NAME_FADE: f32 = 0.5;

pub struct HotbarPlugin;

impl Plugin for HotbarPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Hotbar>()
            // After `Startup`, so the atlas `setup` inserts is there.
            .add_systems(PostStartup, spawn_ui)
            .add_systems(Update, (select_by_key.run_if(pointer_locked), refresh_slots, show_name).chain());
    }
}

/// The slot a number key chose this frame, if any (the lowest, if several).
pub fn pressed_slot(keys: &ButtonInput<KeyCode>) -> Option<usize> {
    KEYS.iter().position(|&key| keys.just_pressed(key))
}

fn select_by_key(keys: Res<ButtonInput<KeyCode>>, mut hotbar: ResMut<Hotbar>) {
    if let Some(slot) = pressed_slot(&keys) {
        // Not through `deref_mut` unless it changes: `refresh_slots` runs on that.
        if hotbar.selected != slot {
            hotbar.selected = slot;
        }
    }
}

/// What to show for a stack: its name, and its atlas tile with a tint. An
/// item without an icon shows none.
fn describe(stack: Stack) -> (String, Option<(u16, Color)>) {
    let look = items::icon(stack).map(|(tile, [r, g, b])| (tile, Color::srgb(r, g, b)));
    (items::name(stack), look)
}

/// The pixel rectangle of an atlas tile.
pub fn tile_rect(tile: u16) -> Rect {
    let (col, row) = (u32::from(tile) % atlas::COLS, u32::from(tile) / atlas::COLS);
    let min = Vec2::new((col * atlas::TILE_PX) as f32, (row * atlas::TILE_PX) as f32);
    Rect::from_corners(min, min + Vec2::splat(atlas::TILE_PX as f32))
}

#[derive(Component)]
struct SlotBox(usize);

#[derive(Component)]
struct SlotIcon(usize);

#[derive(Component)]
struct SlotCount(usize);

#[derive(Component)]
struct BlockName;

fn spawn_ui(mut commands: Commands, atlas: Res<AtlasImage>) {
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            bottom: px(12),
            width: percent(100),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: px(6),
            ..default()
        })
        .with_children(|root| {
            root.spawn((
                Node { height: px(20), ..default() },
                Text::new(""),
                TextFont { font_size: FontSize::Px(18.0), ..default() },
                TextColor(Color::NONE),
                BlockName,
            ));
            root.spawn(Node { column_gap: px(4), ..default() }).with_children(|bar| {
                for i in 0..9 {
                    bar.spawn((
                        Node {
                            width: px(SLOT),
                            height: px(SLOT),
                            border: UiRect::all(px(BORDER)),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(SLOT_FILL),
                        BorderColor::all(EDGE),
                        SlotBox(i),
                    ))
                    .with_children(|slot| {
                        slot.spawn((
                            Node { width: px(ICON), height: px(ICON), ..default() },
                            ImageNode::new(atlas.0.clone()).with_color(Color::NONE),
                            SlotIcon(i),
                        ));
                        slot.spawn((
                            Node { position_type: PositionType::Absolute, right: px(1), bottom: px(0), ..default() },
                            Text::new(""),
                            TextFont { font_size: FontSize::Px(14.0), ..default() },
                            TextColor(Color::WHITE),
                            SlotCount(i),
                        ));
                    });
                }
            });
        });
}

/// Keeps the icons and the selection border in line with the [`Hotbar`].
fn refresh_slots(
    hotbar: Res<Hotbar>,
    mut boxes: Query<(&SlotBox, &mut BorderColor)>,
    mut icons: Query<(&SlotIcon, &mut ImageNode)>,
    mut counts: Query<(&SlotCount, &mut Text)>,
) {
    if !hotbar.is_changed() {
        return;
    }
    for (SlotBox(i), mut border) in &mut boxes {
        *border = BorderColor::all(if *i == hotbar.selected { EDGE_SELECTED } else { EDGE });
    }
    for (SlotIcon(i), mut image) in &mut icons {
        match hotbar.slots[*i].and_then(|stack| describe(stack).1) {
            Some((tile, tint)) => {
                image.rect = Some(tile_rect(tile));
                image.color = tint;
            }
            None => image.color = Color::NONE,
        }
    }
    for (SlotCount(i), mut text) in &mut counts {
        text.0 = match hotbar.slots[*i] {
            Some(stack) if stack.count > 1 => stack.count.to_string(),
            _ => String::new(),
        };
    }
}

/// The selected slot and the kind of item in it (id and damage).
type Selection = (usize, Option<(i16, i16)>);

/// Shows the selected item's name above the bar for a moment whenever the
/// selection changes, fading out at the end.
fn show_name(
    hotbar: Res<Hotbar>,
    time: Res<Time>,
    mut shown: Local<Option<Selection>>,
    mut timer: Local<Timer>,
    mut label: Single<(&mut Text, &mut TextColor), With<BlockName>>,
) {
    // The first frame only notes the starting selection. A new item in the
    // selected slot (picked, chosen in the picker or taken from a container)
    // is a new selection too; a different count of it is not.
    let selected = hotbar.slots[hotbar.selected];
    let now = (hotbar.selected, selected.map(|s| (s.id, s.damage)));
    if *shown != Some(now) {
        if shown.is_some() {
            label.0.0 = selected.map(items::name).unwrap_or_default();
            *timer = Timer::from_seconds(NAME_SECONDS, TimerMode::Once);
        }
        *shown = Some(now);
    }
    timer.tick(time.delta());
    let alpha = (timer.remaining_secs() / NAME_FADE).min(1.0);
    label.1.0 = Color::srgba(1.0, 1.0, 1.0, alpha);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> App {
        let mut app = App::new();
        app.init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<Hotbar>()
            .add_systems(Update, select_by_key);
        app
    }

    #[test]
    fn number_keys_select_slots() {
        let mut app = app();
        assert_eq!(app.world().resource::<Hotbar>().selected, 0);
        for (key, slot) in KEYS.into_iter().zip(0..9) {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.clear();
            keys.press(key);
            app.update();
            assert_eq!(app.world().resource::<Hotbar>().selected, slot, "{key:?}");
        }
    }

    #[test]
    fn other_keys_leave_the_selection() {
        let mut app = app();
        app.world_mut().resource_mut::<Hotbar>().selected = 5;
        app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::KeyW);
        app.update();
        assert_eq!(app.world().resource::<Hotbar>().selected, 5);
    }

    #[test]
    fn describes_blocks_and_items() {
        assert_eq!(describe(Stack::new(4, 0, 1)).0, "Cobblestone");
        // Not in the catalog, so it gets a generic name.
        assert_eq!(describe(Stack::new(95, 0, 1)).0, "Block 95:0");
        assert_eq!(describe(Stack::new(265, 0, 1)).0, "Iron Ingot");
        assert!(describe(Stack::new(265, 0, 1)).1.is_some());
        assert!(describe(Stack::new(342, 0, 1)).1.is_none(), "no icon for it");
    }

    #[test]
    fn only_a_block_in_the_selected_slot_places() {
        let mut hotbar = Hotbar::default();
        assert_eq!(hotbar.selected_block(), Some((1, 0)));
        hotbar.slots[0] = Some(Stack::new(265, 0, 5));
        assert_eq!(hotbar.selected_block(), None, "an iron ingot places nothing");
        // Picking puts one block over it; picking what is there changes nothing.
        hotbar.set_selected_block((17, 1));
        assert_eq!(hotbar.slots[0], Some(Stack::new(17, 1, 1)));
        hotbar.slots[0] = Some(Stack::new(17, 1, 40));
        hotbar.set_selected_block((17, 1));
        assert_eq!(hotbar.slots[0], Some(Stack::new(17, 1, 40)));
    }

    #[test]
    fn default_slots_all_have_icons() {
        for stack in Hotbar::default().slots.into_iter().flatten() {
            let (name, look) = describe(stack);
            assert!(!name.starts_with("Block "), "{stack:?} is not in the catalog");
            let rect = tile_rect(look.expect("an icon").0);
            assert!(rect.max.x <= atlas::WIDTH as f32 && rect.max.y <= atlas::HEIGHT as f32);
        }
    }
}
