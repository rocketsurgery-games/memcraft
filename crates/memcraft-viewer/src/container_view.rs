//! The container window: right click on a chest, furnace, dispenser,
//! dropper, hopper or brewing stand (see `targeting`) opens a panel with the
//! slots of its block entity (see `containers`), each with the item's icon
//! and count. It only looks: moving items is not possible.
//!
//! The panel needs the free pointer, so opening it releases the lock, and
//! [`Modal`] is set while it is up: the menu stays down and a click on the
//! scene does not lock the pointer. Esc closes it and leaves the pointer free
//! (the menu comes up, as for Esc anywhere; a browser does not take a lock
//! request made in answer to Esc anyway). E and the Close button close it and
//! lock the pointer again, so the player is back in the game.

use bevy::prelude::*;
use bevy::window::{CursorOptions, PrimaryWindow};

use crate::blocks::Tile;
use crate::catalog;
use crate::containers::{self, Contents, Stack};
use crate::controls::{grab_pointer, set_locked, Modal};
use crate::hotbar::tile_rect;
use crate::item_icons;
use crate::world::WorldMap;
use crate::AtlasImage;

/// Closes the window and locks the pointer again, while it is open.
const CLOSE_KEY: KeyCode = KeyCode::KeyE;

/// Right click on the container at this cell: open its window.
#[derive(Message)]
pub struct OpenContainer(pub IVec3);

/// The window's root entity, while it is open.
#[derive(Resource, Default)]
struct Open(Option<Entity>);

#[derive(Component)]
struct CloseButton;

/// A slot is a 40 px box holding a 32 px icon (2x the 16 px tile, so the
/// pixels stay even), in a 4 px gap.
const SLOT: f32 = 40.0;
const GAP: f32 = 4.0;
const ICON: f32 = 32.0;

const BACKDROP: Color = Color::srgba(0.0, 0.0, 0.0, 0.6);
const PANEL: Color = Color::srgba(0.07, 0.07, 0.07, 0.95);
const SLOT_FILL: Color = Color::srgb(0.22, 0.22, 0.22);
const BUTTON: Color = Color::srgb(0.30, 0.30, 0.30);
const BUTTON_HOVER: Color = Color::srgb(0.42, 0.45, 0.62);
const DIM: Color = Color::srgb(0.6, 0.6, 0.6);

pub struct ContainerViewPlugin;

impl Plugin for ContainerViewPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Open>().add_message::<OpenContainer>().add_systems(
            Update,
            // After `grab_pointer`, so the key that closes the window (it
            // locks the pointer again) is not also taken by it as a release.
            (close, open).chain().after(grab_pointer),
        );
    }
}

/// The atlas tile and tint that show `stack`: the block's catalog icon, or the
/// item's icon. `None` for an item without one.
fn icon(stack: Stack) -> Option<(Tile, Color)> {
    match u8::try_from(stack.id) {
        Ok(id @ 1..=255) => {
            let entry = catalog::nearest(id, u8::try_from(stack.damage).unwrap_or(0))?;
            let [r, g, b] = entry.look().color;
            Some((entry.icon(), Color::srgb(r, g, b)))
        }
        _ => Some((item_icons::icon(stack.id, stack.damage)?, Color::WHITE)),
    }
}

/// Spawns the window for `contents`; returns its root.
fn spawn_window(commands: &mut Commands, atlas: &AtlasImage, contents: &Contents) -> Entity {
    let text = |size: f32| (TextFont { font_size: FontSize::Px(size), ..default() }, TextColor(Color::WHITE));
    let extent = |n: u8| f32::from(n) * (SLOT + GAP) - GAP;
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(BACKDROP),
            // Over the menu and the hotbar.
            GlobalZIndex(20),
        ))
        .with_children(|root| {
            root.spawn((Node { flex_direction: FlexDirection::Column, padding: UiRect::all(px(16)), row_gap: px(10), ..default() }, BackgroundColor(PANEL)))
                .with_children(|panel| {
                    panel.spawn((Text::new(contents.title.clone()), text(24.0)));
                    panel
                        .spawn(Node { width: px(extent(contents.columns)), height: px(extent(contents.rows)), ..default() })
                        .with_children(|grid| {
                            for slot in &contents.slots {
                                spawn_slot(grid, atlas, slot.column, slot.row, slot.stack);
                            }
                        });
                    panel
                        .spawn((
                            Button,
                            CloseButton,
                            Node { height: px(34), justify_content: JustifyContent::Center, align_items: AlignItems::Center, ..default() },
                            BackgroundColor(BUTTON),
                        ))
                        .with_child((Text::new("Close"), text(18.0)));
                    panel.spawn((Text::new("E: close and go on - Esc: close"), TextFont { font_size: FontSize::Px(13.0), ..default() }, TextColor(DIM)));
                });
        })
        .id()
}

/// One slot of the grid: its box, the item's icon (or its id, for an item with
/// none) and the count when it is more than one.
fn spawn_slot(grid: &mut ChildSpawnerCommands, atlas: &AtlasImage, column: u8, row: u8, stack: Option<Stack>) {
    let corner = |text: String, size: f32| {
        (
            Node { position_type: PositionType::Absolute, right: px(2), bottom: px(0), ..default() },
            Text::new(text),
            TextFont { font_size: FontSize::Px(size), ..default() },
            TextColor(Color::WHITE),
        )
    };
    let mut slot = grid.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: px(f32::from(column) * (SLOT + GAP)),
            top: px(f32::from(row) * (SLOT + GAP)),
            width: px(SLOT),
            height: px(SLOT),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(SLOT_FILL),
    ));
    let Some(stack) = stack else {
        return;
    };
    slot.with_children(|slot| {
        match icon(stack) {
            Some((tile, tint)) => {
                let mut image = ImageNode::new(atlas.0.clone()).with_color(tint);
                image.rect = Some(tile_rect(tile));
                slot.spawn((Node { width: px(ICON), height: px(ICON), ..default() }, image));
            }
            None => {
                slot.spawn((Text::new(stack.id.to_string()), TextFont { font_size: FontSize::Px(12.0), ..default() }, TextColor(DIM)));
            }
        }
        if stack.count > 1 {
            slot.spawn(corner(stack.count.to_string(), 14.0));
        }
    });
}

/// Opens the window on a click on a container, freeing the pointer for it.
fn open(
    mut clicks: MessageReader<OpenContainer>,
    mut commands: Commands,
    map: Res<WorldMap>,
    atlas: Res<AtlasImage>,
    mut window: ResMut<Open>,
    mut modal: ResMut<Modal>,
    mut cursor: Single<&mut CursorOptions, With<PrimaryWindow>>,
) {
    let Some(&OpenContainer(cell)) = clicks.read().last() else {
        return;
    };
    let Some(contents) = containers::contents(&map, cell) else {
        return;
    };
    if let Some(old) = window.0.take() {
        commands.entity(old).despawn();
    }
    window.0 = Some(spawn_window(&mut commands, &atlas, &contents));
    modal.0 = true;
    set_locked(&mut cursor, false);
}

/// Esc, E and the Close button. Esc leaves the pointer free; the others give
/// it back to the player.
fn close(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    mut button: Query<(&Interaction, &mut BackgroundColor), With<CloseButton>>,
    mut window: ResMut<Open>,
    mut modal: ResMut<Modal>,
    mut cursor: Single<&mut CursorOptions, With<PrimaryWindow>>,
) {
    let Some(root) = window.0 else {
        return;
    };
    let mut lock_again = keys.just_pressed(CLOSE_KEY);
    for (interaction, mut background) in &mut button {
        background.set_if_neq(BackgroundColor(if *interaction == Interaction::None { BUTTON } else { BUTTON_HOVER }));
        lock_again |= *interaction == Interaction::Pressed;
    }
    if lock_again || keys.just_pressed(KeyCode::Escape) {
        commands.entity(root).despawn();
        window.0 = None;
        modal.0 = false;
        set_locked(&mut cursor, lock_again);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::controls::grab_pointer;

    fn stack(id: i16, damage: i16) -> Stack {
        Stack { id, damage, count: 1 }
    }

    #[test]
    fn blocks_show_their_catalog_icon_and_items_theirs() {
        let (stone, tint) = icon(stack(1, 0)).unwrap();
        assert_eq!(Some(stone), catalog::find(1, 0).map(|e| e.icon()));
        assert_eq!(tint, Color::srgb(catalog::find(1, 0).unwrap().look().color[0], catalog::find(1, 0).unwrap().look().color[1], catalog::find(1, 0).unwrap().look().color[2]));
        // A block outside the catalog's exact entries stands for its nearest one: carpet, damage 14.
        assert!(icon(stack(171, 14)).is_some());
        // An item (id 265 is an iron ingot) and a dye by its damage.
        assert_eq!(icon(stack(265, 0)), Some((item_icons::icon(265, 0).unwrap(), Color::WHITE)));
        assert_ne!(icon(stack(351, 1)), icon(stack(351, 2)));
    }

    #[test]
    fn a_stack_without_an_icon_has_none_to_fall_back_from() {
        // Not an item the pack draws, nor a block the viewer places, nor an id in range.
        assert_eq!(icon(stack(30000, 0)), None);
        assert_eq!(icon(stack(342, 0)), None);
        assert_eq!(icon(stack(-5, 0)), None);
        assert_eq!(icon(stack(0, 0)), None);
        // Block 0 is air; the data of a block is a byte.
        assert_eq!(icon(stack(1, 300)), icon(stack(1, 0)));
    }

    /// A locked window, the world, and the container window's plugin.
    fn app() -> App {
        let mut app = App::new();
        app.insert_resource(WorldMap::new(crate::load_world(None).unwrap()))
            .insert_resource(AtlasImage(Handle::default()))
            .init_resource::<Modal>()
            .init_resource::<ButtonInput<MouseButton>>()
            .init_resource::<ButtonInput<KeyCode>>()
            .add_plugins(ContainerViewPlugin)
            .add_systems(Update, grab_pointer.before(open));
        let mut cursor = CursorOptions::default();
        set_locked(&mut cursor, true);
        app.world_mut().spawn((Window { focused: true, ..default() }, cursor, PrimaryWindow));
        app.update();
        app
    }

    fn locked(app: &mut App) -> bool {
        let mut cursors = app.world_mut().query::<&CursorOptions>();
        crate::controls::is_locked(cursors.single(app.world()).unwrap())
    }

    fn tap(app: &mut App, key: KeyCode) {
        app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(key);
        app.update();
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.release(key);
        keys.clear();
    }

    const CHEST: IVec3 = IVec3::new(93, 77, 110);

    #[test]
    fn opening_frees_the_pointer_and_e_or_esc_closes() {
        let mut app = app();
        app.world_mut().write_message(OpenContainer(CHEST));
        app.update();
        assert!(!locked(&mut app), "the pointer is free for the window");
        assert!(app.world().resource::<Modal>().0);
        let root = app.world().resource::<Open>().0.expect("the window is open");
        assert!(app.world().get_entity(root).is_ok());
        // Eleven stacks, 27 slots, and a close button are somewhere under the root.
        let nodes = app.world_mut().query::<&Node>().iter(app.world()).count();
        assert!(nodes > 27);

        // A click on the scene must not lock it while the window is up.
        app.world_mut().resource_mut::<ButtonInput<MouseButton>>().press(MouseButton::Left);
        app.update();
        assert!(!locked(&mut app));
        app.world_mut().resource_mut::<ButtonInput<MouseButton>>().release(MouseButton::Left);
        app.world_mut().resource_mut::<ButtonInput<MouseButton>>().clear();

        // E closes it and the player is back in the game.
        tap(&mut app, CLOSE_KEY);
        assert!(locked(&mut app));
        assert!(!app.world().resource::<Modal>().0);
        assert!(app.world().resource::<Open>().0.is_none());
        assert!(app.world().get_entity(root).is_err(), "the window is gone");

        // Esc closes it and leaves the pointer free (the menu comes up).
        app.world_mut().write_message(OpenContainer(CHEST));
        app.update();
        assert!(!locked(&mut app));
        tap(&mut app, KeyCode::Escape);
        assert!(!locked(&mut app));
        assert!(!app.world().resource::<Modal>().0);
        assert!(app.world().resource::<Open>().0.is_none());
    }

    #[test]
    fn a_second_click_replaces_the_window_and_a_non_container_opens_nothing() {
        let mut app = app();
        app.world_mut().write_message(OpenContainer(CHEST));
        app.update();
        let first = app.world().resource::<Open>().0.unwrap();
        app.world_mut().write_message(OpenContainer(CHEST));
        app.update();
        let second = app.world().resource::<Open>().0.unwrap();
        assert!(app.world().get_entity(first).is_err());
        assert!(app.world().get_entity(second).is_ok());

        let mut app = self::app();
        app.world_mut().write_message(OpenContainer(IVec3::new(48, 120, 40)));
        app.update();
        assert!(locked(&mut app), "air is no container");
        assert!(app.world().resource::<Open>().0.is_none());
    }
}
