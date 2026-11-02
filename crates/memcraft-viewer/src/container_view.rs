//! The container window: right click on a chest, furnace, dispenser,
//! dropper, hopper or brewing stand (see `targeting`) opens a panel with the
//! slots of its block entity (see `containers`), each with the item's icon
//! and count, and the hotbar below them. Hovering a slot names its item.
//!
//! Clicks move stacks between the two (see `items` for how stacks merge and
//! swap; the counts are kept, nothing is split):
//!
//! - a click on a container slot takes its stack into the selected hotbar
//!   slot, swapping with what is there;
//! - shift and click sends it to the hotbar's first free slot instead;
//! - a click on a hotbar slot puts its stack into the container's first free
//!   slot;
//! - the number keys 1-9 choose the selected hotbar slot, as in the game.
//!
//! Every change is written to the container's block entity (see
//! [`containers::set_stack`]), so it is saved with the world.
//!
//! The panel needs the free pointer, so opening it releases the lock, and
//! [`Modal`] is set while it is up: the menu stays down and a click on the
//! scene does not lock the pointer. Esc closes it and leaves the pointer free
//! (the menu comes up, as for Esc anywhere; a browser does not take a lock
//! request made in answer to Esc anyway). E and the Close button close it and
//! lock the pointer again, so the player is back in the game.

use bevy::prelude::*;
use bevy::window::{CursorOptions, PrimaryWindow};

use crate::containers::{self, Contents};
use crate::controls::{grab_pointer, set_locked, Modal};
use crate::hotbar::{pressed_slot, tile_rect, Hotbar};
use crate::items::{self, Stack};
use crate::world::WorldMap;
use crate::AtlasImage;

/// Closes the window and locks the pointer again, while it is open.
const CLOSE_KEY: KeyCode = KeyCode::KeyE;

/// Right click on the container at this cell: open its window.
#[derive(Message)]
pub struct OpenContainer(pub IVec3);

/// The window's root entity and the container's cell, while it is open.
#[derive(Resource, Default)]
struct Open(Option<(Entity, IVec3)>);

#[derive(Component)]
struct CloseButton;

/// Where a slot of the window is: the container's, or the hotbar's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Place {
    /// An index into [`Contents::slots`].
    Container(usize),
    Hotbar(usize),
}

/// A slot's button, with the stack it shows.
#[derive(Component)]
struct SlotButton(Place, Option<Stack>);

/// The line under the container that names the hovered slot's item.
#[derive(Component)]
struct HoverName;

/// A slot is a 40 px box holding a 32 px icon (2x the 16 px tile, so the
/// pixels stay even), in a 4 px gap.
const SLOT: f32 = 40.0;
const GAP: f32 = 4.0;
const ICON: f32 = 32.0;

const BACKDROP: Color = Color::srgba(0.0, 0.0, 0.0, 0.6);
const PANEL: Color = Color::srgba(0.07, 0.07, 0.07, 0.95);
const SLOT_FILL: Color = Color::srgb(0.22, 0.22, 0.22);
const SLOT_HOVER: Color = Color::srgb(0.34, 0.34, 0.40);
const SELECTED: Color = Color::WHITE;
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
            (close, open, click_slots, hover_name).chain().after(grab_pointer),
        );
    }
}

/// Spawns the window for `contents`; returns its root.
fn spawn_window(commands: &mut Commands, atlas: &AtlasImage, contents: &Contents, hotbar: &Hotbar) -> Entity {
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
                            for (i, slot) in contents.slots.iter().enumerate() {
                                spawn_slot(grid, atlas, Place::Container(i), (slot.column, slot.row), slot.stack, false);
                            }
                        });
                    panel.spawn((Node { height: px(18), ..default() }, Text::new(""), text(16.0), HoverName));
                    panel
                        .spawn(Node { width: px(extent(9)), height: px(extent(1)), ..default() })
                        .with_children(|row| {
                            for (i, stack) in hotbar.slots.iter().enumerate() {
                                spawn_slot(row, atlas, Place::Hotbar(i), (i as u8, 0), *stack, i == hotbar.selected);
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

/// One slot of the grid, a button: its box (with a white border if
/// `selected`), the item's icon (or its id, for an item with none) and the
/// count when it is more than one.
fn spawn_slot(grid: &mut ChildSpawnerCommands, atlas: &AtlasImage, place: Place, (column, row): (u8, u8), stack: Option<Stack>, selected: bool) {
    let corner = |text: String, size: f32| {
        (
            Node { position_type: PositionType::Absolute, right: px(2), bottom: px(0), ..default() },
            Text::new(text),
            TextFont { font_size: FontSize::Px(size), ..default() },
            TextColor(Color::WHITE),
        )
    };
    let mut slot = grid.spawn((
        Button,
        SlotButton(place, stack),
        BorderColor::all(if selected { SELECTED } else { Color::NONE }),
        Node {
            border: UiRect::all(px(2)),
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
        match items::icon(stack) {
            Some((tile, [r, g, b])) => {
                let mut image = ImageNode::new(atlas.0.clone()).with_color(Color::srgb(r, g, b));
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
#[allow(clippy::too_many_arguments)] // a Bevy system: each parameter is a resource or message it uses
fn open(
    mut clicks: MessageReader<OpenContainer>,
    mut commands: Commands,
    map: Res<WorldMap>,
    hotbar: Res<Hotbar>,
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
    if let Some((old, _)) = window.0.take() {
        commands.entity(old).despawn();
    }
    window.0 = Some((spawn_window(&mut commands, &atlas, &contents, &hotbar), cell));
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
    let Some((root, _)) = window.0 else {
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

/// A click on `place` in the window of the container whose slots are
/// `contents`, with the hotbar's `slots` and its `selected` one: moves the
/// stacks as the module doc says and writes the container's changes to the
/// world. Returns whether anything moved.
fn move_stack(map: &mut WorldMap, contents: &Contents, slots: &mut [Option<Stack>; 9], selected: usize, place: Place, shift: bool) -> bool {
    let mut stacks: Vec<Option<Stack>> = contents.slots.iter().map(|s| s.stack).collect();
    let hotbar = *slots;
    match place {
        Place::Container(i) if shift => items::send(&mut stacks[i], slots),
        Place::Container(i) => items::exchange(&mut stacks[i], &mut slots[selected]),
        Place::Hotbar(i) => items::send(&mut slots[i], &mut stacks),
    }
    let mut moved = *slots != hotbar;
    for (slot, stack) in contents.slots.iter().zip(stacks) {
        if slot.stack != stack && containers::set_stack(map, slot.cell, slot.number, stack) {
            info!("moved: slot {} of the container at {} now holds {:?}", slot.number, slot.cell, stack);
            moved = true;
        }
    }
    moved
}

/// Clicks on the slots and the number keys. Whatever changed, the window is
/// drawn again.
#[allow(clippy::too_many_arguments)] // a Bevy system: each parameter is a resource or query it uses
fn click_slots(
    slots: Query<(&Interaction, &SlotButton), Changed<Interaction>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    atlas: Res<AtlasImage>,
    mut window: ResMut<Open>,
    mut map: ResMut<WorldMap>,
    mut hotbar: ResMut<Hotbar>,
) {
    let Some((root, cell)) = window.0 else {
        return;
    };
    let mut changed = false;
    if let Some(slot) = pressed_slot(&keys).filter(|&slot| slot != hotbar.selected) {
        hotbar.selected = slot;
        changed = true;
    }
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if let Some((_, &SlotButton(place, _))) = slots.iter().find(|(interaction, _)| **interaction == Interaction::Pressed)
        && let Some(contents) = containers::contents(&map, cell)
    {
        let mut moved = hotbar.slots;
        if move_stack(&mut map, &contents, &mut moved, hotbar.selected, place, shift) {
            // Not through `deref_mut` unless it changes, see `Hotbar`.
            if moved != hotbar.slots {
                hotbar.slots = moved;
            }
            changed = true;
        }
    }
    if changed && let Some(contents) = containers::contents(&map, cell) {
        commands.entity(root).despawn();
        window.0 = Some((spawn_window(&mut commands, &atlas, &contents, &hotbar), cell));
    }
}

/// Names the hovered slot's item under the container.
fn hover_name(slots: Query<(&Interaction, &SlotButton)>, mut buttons: Query<(&Interaction, &mut BackgroundColor), With<SlotButton>>, mut label: Query<&mut Text, With<HoverName>>) {
    let Ok(mut label) = label.single_mut() else {
        return;
    };
    let hovered = slots.iter().find(|(interaction, _)| **interaction != Interaction::None);
    let text = hovered.and_then(|(_, SlotButton(_, stack))| *stack).map(items::name).unwrap_or_default();
    label.set_if_neq(Text::new(text));
    for (interaction, mut background) in &mut buttons {
        background.set_if_neq(BackgroundColor(if *interaction == Interaction::None { SLOT_FILL } else { SLOT_HOVER }));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::controls::grab_pointer;
    use mcpe_world::Tag;

    /// A locked window, the world, and the container window's plugin.
    fn app() -> App {
        let mut app = App::new();
        app.insert_resource(WorldMap::new(crate::load_world(None).unwrap()))
            .insert_resource(AtlasImage(Handle::default()))
            .init_resource::<Modal>()
            .init_resource::<Hotbar>()
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
        let (root, _) = app.world().resource::<Open>().0.expect("the window is open");
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
        let (first, _) = app.world().resource::<Open>().0.unwrap();
        app.world_mut().write_message(OpenContainer(CHEST));
        app.update();
        let (second, _) = app.world().resource::<Open>().0.unwrap();
        assert!(app.world().get_entity(first).is_err());
        assert!(app.world().get_entity(second).is_ok());

        let mut app = self::app();
        app.world_mut().write_message(OpenContainer(IVec3::new(48, 120, 40)));
        app.update();
        assert!(locked(&mut app), "air is no container");
        assert!(app.world().resource::<Open>().0.is_none());
    }

    /// The chest's window slot at `(column, row)` as a place.
    fn at(app: &App, column: u8, row: u8) -> Place {
        let contents = containers::contents(app.world().resource::<WorldMap>(), CHEST).unwrap();
        Place::Container(contents.slots.iter().position(|s| (s.column, s.row) == (column, row)).unwrap())
    }

    /// Presses the button of `place` in the open window, for one update.
    fn press(app: &mut App, place: Place) {
        let mut buttons = app.world_mut().query::<(&SlotButton, &mut Interaction)>();
        let (_, mut interaction) = buttons.iter_mut(app.world_mut()).find(|(b, _)| b.0 == place).expect("a slot button");
        *interaction = Interaction::Pressed;
        app.update();
    }

    fn chest(app: &App) -> Vec<(u8, u8, i16, u8)> {
        let contents = containers::contents(app.world().resource::<WorldMap>(), CHEST).unwrap();
        contents.slots.iter().filter_map(|s| s.stack.map(|k| (s.column, s.row, k.id, k.count))).collect()
    }

    fn hotbar(app: &App) -> [Option<Stack>; 9] {
        app.world().resource::<Hotbar>().slots
    }

    fn open_chest() -> App {
        let mut app = app();
        app.world_mut().write_message(OpenContainer(CHEST));
        app.update();
        app
    }

    #[test]
    fn a_click_on_a_container_slot_takes_its_stack_into_the_selected_slot_and_swaps() {
        let mut app = open_chest();
        let before = app.world().resource::<Open>().0.unwrap().0;
        // The egg stack (344, four of them) at (0, 0); the selected slot 0 holds stone.
        let place = at(&app, 0, 0);
        press(&mut app, place);
        assert_eq!(hotbar(&app)[0], Some(Stack::new(344, 0, 4)));
        assert!(chest(&app).contains(&(0, 0, 1, 1)), "the stone is in the chest now");
        assert!(!chest(&app).contains(&(0, 0, 344, 4)));
        let (after, _) = app.world().resource::<Open>().0.unwrap();
        assert!(app.world().get_entity(before).is_err() && app.world().get_entity(after).is_ok(), "the window is drawn again");
        assert_eq!(app.world().resource::<WorldMap>().edited_columns().len(), 1, "the chest's column is edited");
        // No block was edited, only the block entity.
        assert_eq!(app.world().resource::<Hotbar>().selected_block(), None, "an egg places nothing");
    }

    #[test]
    fn number_keys_choose_the_slot_a_take_goes_to() {
        let mut app = open_chest();
        tap(&mut app, KeyCode::Digit4);
        assert_eq!(app.world().resource::<Hotbar>().selected, 3);
        let place = at(&app, 0, 0);
        press(&mut app, place);
        assert_eq!(hotbar(&app)[3], Some(Stack::new(344, 0, 4)));
        assert_eq!(hotbar(&app)[0], Some(Stack::new(1, 0, 1)), "the others are as they were");
        // Choosing it again changes nothing and draws nothing new.
        let (root, _) = app.world().resource::<Open>().0.unwrap();
        tap(&mut app, KeyCode::Digit4);
        assert_eq!(app.world().resource::<Open>().0.unwrap().0, root);
    }

    #[test]
    fn shift_click_goes_to_the_first_free_slot_and_a_hotbar_click_puts_into_the_chest() {
        let mut app = open_chest();
        // The hotbar is full: nothing happens.
        app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::ShiftLeft);
        let lever = at(&app, 3, 0);
        press(&mut app, lever);
        assert_eq!(hotbar(&app), Hotbar::default().slots);
        assert!(chest(&app).contains(&(3, 0, 69, 64)));
        // With a free slot, it goes there and the chest's slot empties.
        app.world_mut().resource_mut::<Hotbar>().slots[4] = None;
        press(&mut app, lever);
        assert_eq!(hotbar(&app)[4], Some(Stack::new(69, 0, 64)));
        assert!(!chest(&app).iter().any(|&(c, r, ..)| (c, r) == (3, 0)));
        app.world_mut().resource_mut::<ButtonInput<KeyCode>>().release(KeyCode::ShiftLeft);

        // A click on the hotbar slot puts the stack into the chest: onto the
        // other lever stack (2 of 69 at (3, 1)) first, as far as it fits, then on.
        press(&mut app, Place::Hotbar(4));
        assert_eq!(hotbar(&app)[4], None);
        let chest = chest(&app);
        assert!(chest.contains(&(3, 1, 69, 64)), "{chest:?}");
        assert!(chest.contains(&(3, 0, 69, 2)), "62 filled, 2 in the first free slot: {chest:?}");
    }

    #[test]
    fn what_was_moved_is_in_the_world_when_it_is_saved() {
        let mut app = open_chest();
        let place = at(&app, 5, 0);
        press(&mut app, place);
        let map = app.world().resource::<WorldMap>();
        let be = map.block_entity(CHEST).unwrap();
        let items = be.get("Items").and_then(Tag::as_list).unwrap();
        // The chest's slot 5 now holds the stone, written as the game does.
        let slot5 = items.iter().find(|i| i.get("Slot").and_then(Tag::as_i64) == Some(5)).unwrap();
        assert_eq!(slot5.get("id"), Some(&Tag::Short(1)));
        assert_eq!(slot5.get("Count"), Some(&Tag::Byte(1)));
        assert_eq!(slot5.get("Damage"), Some(&Tag::Short(0)));
    }
}
