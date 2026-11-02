//! The hotbar: nine slots of blocks to place, one of them selected. The
//! plugin owns the [`Hotbar`] resource, draws it along the bottom of the
//! window, and lets the number keys 1-9 choose a slot (while the pointer is locked).

use bevy::prelude::*;

use crate::AtlasImage;
use crate::blocks::{self, atlas};
use crate::catalog;
use crate::controls::pointer_locked;

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
    /// The block that placing puts down, if the selected slot holds one.
    pub fn selected_block(&self) -> Option<Block> {
        self.slots[self.selected]
    }

    /// Puts `block` in the selected slot (the picker and pick-block do this).
    pub fn set_selected_block(&mut self, block: Block) {
        // Not through `deref_mut` unless it changes: `refresh_slots` runs on that.
        if self.slots[self.selected] != Some(block) {
            self.slots[self.selected] = Some(block);
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
fn pressed_slot(keys: &ButtonInput<KeyCode>) -> Option<usize> {
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

/// What to show for a block: its name, and its atlas tile with a tint. Blocks
/// outside the catalog fall back to their +z face.
fn describe((id, data): Block) -> (String, u16, Color) {
    let (name, tile, look) = match catalog::find(id, data) {
        Some(entry) => (entry.name.to_string(), entry.icon(), entry.look()),
        None => {
            let look = blocks::look(id, data);
            (format!("Block {id}:{data}"), look.tiles[4], look)
        }
    };
    let [r, g, b] = look.color;
    (name, tile, Color::srgb(r, g, b))
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
) {
    if !hotbar.is_changed() {
        return;
    }
    for (SlotBox(i), mut border) in &mut boxes {
        *border = BorderColor::all(if *i == hotbar.selected { EDGE_SELECTED } else { EDGE });
    }
    for (SlotIcon(i), mut image) in &mut icons {
        match hotbar.slots[*i] {
            Some(block) => {
                let (_, tile, tint) = describe(block);
                image.rect = Some(tile_rect(tile));
                image.color = tint;
            }
            None => image.color = Color::NONE,
        }
    }
}

/// Shows the selected block's name above the bar for a moment whenever the
/// selection changes, fading out at the end.
fn show_name(
    hotbar: Res<Hotbar>,
    time: Res<Time>,
    mut shown: Local<Option<(usize, Option<Block>)>>,
    mut timer: Local<Timer>,
    mut label: Single<(&mut Text, &mut TextColor), With<BlockName>>,
) {
    // The first frame only notes the starting selection. A new block in the
    // selected slot (picked, or chosen in the picker) is a new selection too.
    let now = (hotbar.selected, hotbar.selected_block());
    if *shown != Some(now) {
        if shown.is_some() {
            label.0.0 = hotbar.selected_block().map(|b| describe(b).0).unwrap_or_default();
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
    fn describes_catalog_and_other_blocks() {
        assert_eq!(describe((4, 0)).0, "Cobblestone");
        // Not in the catalog, so it gets a generic name.
        assert_eq!(describe((95, 0)).0, "Block 95:0");
    }

    #[test]
    fn default_slots_all_have_icons() {
        for block in Hotbar::default().slots.into_iter().flatten() {
            let (name, tile, _) = describe(block);
            assert!(!name.starts_with("Block "), "{block:?} is not in the catalog");
            let rect = tile_rect(tile);
            assert!(rect.max.x <= atlas::WIDTH as f32 && rect.max.y <= atlas::HEIGHT as f32);
        }
    }
}
