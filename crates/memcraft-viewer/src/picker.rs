//! The block picker: B opens a panel with a search box and a grid of every
//! catalog block (see `catalog`), names on hover; clicking a block puts it in
//! the selected hotbar slot. Esc closes it.
//!
//! The panel needs the free pointer, so opening it releases the lock, and
//! [`Modal`] is set while it is up: the menu stays down. Closing it leaves the
//! pointer free (the menu comes up, as for Esc anywhere), and the next click
//! locks it again, like the one that starts the game. B is not a closing key:
//! the search box takes every typable key, and "birch" has a b in it.
//!
//! The player's controls act only while the pointer is locked (see
//! `controls`), so typing never moves the player and the number keys don't
//! change the slot. The hotbar stays visible under the panel.

use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::mouse::{AccumulatedMouseScroll, MouseScrollUnit};
use bevy::input::ButtonState;
use bevy::prelude::*;
use bevy::window::{CursorOptions, PrimaryWindow};

use crate::catalog::{self, Entry};
use crate::controls::{grab_pointer, pointer_locked, set_locked, Modal};
use crate::font::is_typable;
use crate::hotbar::{tile_rect, Hotbar};
use crate::AtlasImage;

/// Opens the picker, while the pointer is locked.
const OPEN_KEY: KeyCode = KeyCode::KeyB;
/// The longest search.
const QUERY_CHARS: usize = 24;

/// Whether `entry` answers `query`: every word of the query is part of the
/// block's name or category, in any case. An empty query matches everything.
fn matches(entry: &Entry, query: &str) -> bool {
    let (name, category) = (entry.name.to_lowercase(), entry.category.name().to_lowercase());
    query.split_whitespace().map(str::to_lowercase).all(|word| name.contains(&word) || category.contains(&word))
}

#[derive(Debug, PartialEq)]
enum Outcome {
    Keep,
    Close,
}

/// The picker: whether it is open, and what is typed in its search box.
#[derive(Resource, Default, Debug)]
struct Picker {
    open: bool,
    query: String,
}

impl Picker {
    /// Applies a key press: `text` is what it types, if anything.
    fn press(&mut self, key: &Key, text: Option<&str>) -> Outcome {
        match key {
            Key::Escape => return Outcome::Close,
            Key::Backspace => {
                self.query.pop();
            }
            _ => {
                for c in text.unwrap_or_default().chars().filter(|&c| is_typable(c)) {
                    if self.query.chars().count() < QUERY_CHARS {
                        self.query.push(c);
                    }
                }
            }
        }
        Outcome::Keep
    }
}

#[derive(Component)]
struct PickerRoot;

/// A grid cell showing catalog entry `n` (an index into [`catalog::all`]).
#[derive(Component)]
struct Cell(usize);

#[derive(Component)]
struct Grid;

#[derive(Component)]
struct SearchLabel;

#[derive(Component)]
struct CountLabel;

#[derive(Component)]
struct Tooltip;

/// The cell's size: a 32 px icon (2x the 16 px tile) in a 3 px border.
const CELL: f32 = 38.0;
const BORDER: f32 = 3.0;
const ICON: f32 = 32.0;
const COLUMNS: f32 = 10.0;
const GAP: f32 = 4.0;
/// How far one wheel line scrolls the grid.
const LINE_PX: f32 = 40.0;

const PANEL: Color = Color::srgba(0.07, 0.07, 0.07, 0.92);
const FIELD: Color = Color::srgb(0.16, 0.16, 0.16);
const CELL_FILL: Color = Color::srgb(0.22, 0.22, 0.22);
const EDGE_HOVER: Color = Color::WHITE;
const EDGE_HELD: Color = Color::srgb(0.95, 0.80, 0.25);
const DIM: Color = Color::srgb(0.6, 0.6, 0.6);

pub struct PickerPlugin;

impl Plugin for PickerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Picker>()
            // After `Startup`, so the atlas `setup` inserts is there.
            .add_systems(PostStartup, spawn_ui)
            .add_systems(
                Update,
                // `type_text` before `open`, so the press of B that opens the
                // picker is not typed into it.
                (type_text, open.run_if(pointer_locked), click_cells, show, style_cells, scroll, tooltip)
                    .chain()
                    .after(grab_pointer),
            );
    }
}

fn spawn_ui(mut commands: Commands, atlas: Res<AtlasImage>) {
    let text = |size: f32| (TextFont { font_size: FontSize::Px(size), ..default() }, TextColor(Color::WHITE));
    let width = COLUMNS * CELL + (COLUMNS - 1.0) * GAP;
    commands
        .spawn((
            PickerRoot,
            Node {
                display: Display::None,
                position_type: PositionType::Absolute,
                width: percent(100),
                // Leaves the hotbar along the bottom showing.
                height: percent(80),
                justify_content: JustifyContent::Center,
                padding: UiRect::vertical(px(16)),
                ..default()
            },
            // Over the menu and the hotbar.
            GlobalZIndex(20),
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    width: px(width + 32.0 + 8.0),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(px(16)),
                    row_gap: px(10),
                    ..default()
                },
                BackgroundColor(PANEL),
            ))
            .with_children(|panel| {
                panel.spawn((Text::new("Blocks"), text(24.0)));
                panel
                    .spawn((
                        Node { height: px(34), padding: UiRect::horizontal(px(10)), align_items: AlignItems::Center, ..default() },
                        BackgroundColor(FIELD),
                    ))
                    .with_child((SearchLabel, Text::new(""), text(18.0)));
                panel.spawn((CountLabel, Text::new(""), TextFont { font_size: FontSize::Px(13.0), ..default() }, TextColor(DIM)));
                panel
                    .spawn((
                        Grid,
                        Node {
                            flex_grow: 1.0,
                            // A scroll area is as tall as its content unless
                            // the flex box may shrink it.
                            min_height: px(0),
                            // The scrollbar-less scroll area is a little wider than the cells.
                            width: px(width + 8.0),
                            flex_direction: FlexDirection::Row,
                            flex_wrap: FlexWrap::Wrap,
                            align_content: AlignContent::FlexStart,
                            column_gap: px(GAP),
                            row_gap: px(GAP),
                            overflow: Overflow::scroll_y(),
                            ..default()
                        },
                        ScrollPosition::default(),
                    ))
                    .with_children(|grid| {
                        for (n, entry) in catalog::all().iter().enumerate() {
                            let [r, g, b] = entry.look().color;
                            grid.spawn((
                                Button,
                                Cell(n),
                                Node {
                                    width: px(CELL),
                                    height: px(CELL),
                                    flex_shrink: 0.0,
                                    border: UiRect::all(px(BORDER)),
                                    ..default()
                                },
                                BackgroundColor(CELL_FILL),
                                BorderColor::all(Color::NONE),
                            ))
                            .with_child((
                                Node { width: px(ICON), height: px(ICON), ..default() },
                                {
                                    let mut image = ImageNode::new(atlas.0.clone()).with_color(Color::srgb(r, g, b));
                                    image.rect = Some(tile_rect(entry.icon()));
                                    image
                                },
                            ));
                        }
                    });
                panel.spawn((Text::new("Type to search - click a block to put it in the selected slot - Esc: close"), TextFont { font_size: FontSize::Px(13.0), ..default() }, TextColor(DIM)));
            });
        });

    commands.spawn((
        Tooltip,
        Node { display: Display::None, position_type: PositionType::Absolute, padding: UiRect::axes(px(8), px(4)), ..default() },
        BackgroundColor(Color::srgba(0.1, 0.0, 0.1, 0.95)),
        Text::new(""),
        text(16.0),
        GlobalZIndex(30),
    ));
}

/// B opens the picker, freeing the pointer for it.
fn open(
    keys: Res<ButtonInput<KeyCode>>,
    mut picker: ResMut<Picker>,
    mut modal: ResMut<Modal>,
    mut cursor: Single<&mut CursorOptions, With<PrimaryWindow>>,
) {
    if !keys.just_pressed(OPEN_KEY) {
        return;
    }
    *picker = Picker { open: true, query: String::new() };
    modal.0 = true;
    set_locked(&mut cursor, false);
}

/// Typing into the search box. Every key event is read, so that keys
/// pressed before the picker opened do not show up in it. Closing leaves
/// the pointer free.
fn type_text(mut keys: MessageReader<KeyboardInput>, mut picker: ResMut<Picker>, mut modal: ResMut<Modal>) {
    for key in keys.read().filter(|k| k.state == ButtonState::Pressed) {
        if !picker.open {
            continue;
        }
        if picker.press(&key.logical_key, key.text.as_deref()) == Outcome::Close {
            picker.open = false;
            modal.0 = false;
        }
    }
}

/// A click on a block puts it in the selected slot.
fn click_cells(cells: Query<(&Interaction, &Cell), Changed<Interaction>>, picker: Res<Picker>, mut hotbar: ResMut<Hotbar>) {
    if !picker.open {
        return;
    }
    for (_, Cell(n)) in cells.iter().filter(|(i, _)| **i == Interaction::Pressed) {
        let entry = &catalog::all()[*n];
        hotbar.set_selected_block((entry.id, entry.data));
        info!("picked {} for slot {}", entry.name, hotbar.selected + 1);
    }
}

/// Shows the panel while the picker is open, with the search text and only
/// the blocks that match it.
fn show(
    picker: Res<Picker>,
    mut root: Single<&mut Node, With<PickerRoot>>,
    mut cells: Query<(&Cell, &mut Node), Without<PickerRoot>>,
    mut search: Single<&mut Text, (With<SearchLabel>, Without<CountLabel>)>,
    mut count: Single<&mut Text, (With<CountLabel>, Without<SearchLabel>)>,
    mut scroll: Single<&mut ScrollPosition, With<Grid>>,
) {
    if !picker.is_changed() {
        return;
    }
    let display = if picker.open { Display::Flex } else { Display::None };
    if root.display != display {
        root.display = display;
    }
    search.0 = format!("{}|", picker.query);
    let mut shown = 0;
    for (Cell(n), mut node) in &mut cells {
        let show = matches(&catalog::all()[*n], &picker.query);
        shown += show as usize;
        node.display = if show { Display::Flex } else { Display::None };
    }
    count.0 = if shown == 0 { "No block matches".to_string() } else { format!("{shown} blocks") };
    scroll.0 = Vec2::ZERO;
}

/// The border of a block: white under the pointer, gold for the one in the
/// selected slot.
fn style_cells(picker: Res<Picker>, hotbar: Res<Hotbar>, mut cells: Query<(&Cell, &Interaction, &mut BorderColor)>) {
    if !picker.open {
        return;
    }
    for (Cell(n), interaction, mut border) in &mut cells {
        let entry = &catalog::all()[*n];
        let edge = if *interaction != Interaction::None {
            EDGE_HOVER
        } else if hotbar.selected_block() == Some((entry.id, entry.data)) {
            EDGE_HELD
        } else {
            Color::NONE
        };
        border.set_if_neq(BorderColor::all(edge));
    }
}

/// The wheel scrolls the grid (it is free: the speed is the locked pointer's).
fn scroll(picker: Res<Picker>, wheel: Res<AccumulatedMouseScroll>, mut grid: Single<(&mut ScrollPosition, &ComputedNode), With<Grid>>) {
    if !picker.open || wheel.delta.y == 0.0 {
        return;
    }
    let (position, node) = &mut *grid;
    let pixels = match wheel.unit {
        MouseScrollUnit::Line => wheel.delta.y * LINE_PX,
        MouseScrollUnit::Pixel => wheel.delta.y,
    };
    // Not past the end: the layout only clamps what it draws.
    let max = ((node.content_size() - node.size()) * node.inverse_scale_factor()).y.max(0.0);
    position.y = (position.y - pixels).clamp(0.0, max);
}

/// A label with the name of the block under the pointer, next to it.
fn tooltip(
    picker: Res<Picker>,
    cells: Query<(&Cell, &Interaction)>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut tip: Single<(&mut Node, &mut Text), With<Tooltip>>,
) {
    let hovered = cells.iter().find(|(_, i)| **i != Interaction::None).map(|(Cell(n), _)| catalog::all()[*n].name);
    let (node, text) = &mut *tip;
    match (hovered, window.cursor_position()) {
        (Some(name), Some(at)) if picker.open => {
            if text.0 != name {
                text.0 = name.to_string();
            }
            node.display = Display::Flex;
            node.left = px(at.x + 14.0);
            node.top = px(at.y + 18.0);
        }
        _ => {
            if node.display != Display::None {
                node.display = Display::None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The catalog entries that answer `query`, in catalog order.
    fn filter(query: &str) -> Vec<&'static Entry> {
        catalog::all().iter().filter(|e| matches(e, query)).collect()
    }

    fn names(query: &str) -> Vec<&'static str> {
        filter(query).into_iter().map(|e| e.name).collect()
    }

    #[test]
    fn an_empty_search_lists_everything_in_catalog_order() {
        assert_eq!(filter("").len(), catalog::all().len());
        assert_eq!(filter("  ").len(), catalog::all().len());
        assert_eq!(filter("")[0].name, catalog::all()[0].name);
    }

    #[test]
    fn search_is_case_insensitive_and_matches_inside_names() {
        assert_eq!(names("birch").len(), 9, "{:?}", names("birch"));
        assert_eq!(names("BIRCH"), names("birch"));
        assert!(names("birch").contains(&"Birch Planks"));
        assert!(names("stone sl").contains(&"Stone Slab"), "a partial last word");
        assert!(names("lazuli").iter().all(|n| n.contains("Lazuli")));
        assert_eq!(names("tnt"), ["TNT"]);
    }

    #[test]
    fn every_word_must_match_in_any_order() {
        assert_eq!(names("oak slab"), ["Oak Slab", "Dark Oak Slab"]);
        assert_eq!(names("slab oak"), names("oak slab"));
        assert_eq!(names("  oak   slab "), names("oak slab"));
        assert!(names("oak diamond").is_empty());
    }

    #[test]
    fn a_category_name_matches_its_blocks() {
        let redstone = names("redstone");
        assert!(redstone.contains(&"Lever"), "a Redstone category block with no redstone in its name");
        assert!(redstone.contains(&"Redstone Torch"));
        assert!(names("zzz").is_empty());
    }

    #[test]
    fn typing_edits_the_query_and_only_what_the_font_draws() {
        let mut p = Picker::default();
        let chars = |p: &mut Picker, text: &str| {
            for c in text.chars() {
                let s = c.to_string();
                assert_eq!(p.press(&Key::Character(s.as_str().into()), Some(&s)), Outcome::Keep);
            }
        };
        chars(&mut p, "Brick x");
        assert_eq!(p.press(&Key::Backspace, None), Outcome::Keep);
        assert_eq!(p.query, "Brick ");
        // Control characters (a Ctrl+key chord) and what the font lacks are dropped.
        assert_eq!(p.press(&Key::Character("v".into()), Some("\u{16}")), Outcome::Keep);
        assert_eq!(p.press(&Key::Character("é".into()), Some("é")), Outcome::Keep);
        assert_eq!(p.press(&Key::Shift, None), Outcome::Keep);
        assert_eq!(p.query, "Brick ");
        chars(&mut p, &"x".repeat(40));
        assert_eq!(p.query.chars().count(), QUERY_CHARS);
        assert_eq!(p.press(&Key::Escape, None), Outcome::Close);
        // Backspace on an empty query is fine.
        let mut p = Picker::default();
        assert_eq!(p.press(&Key::Backspace, None), Outcome::Keep);
    }

    /// A locked window, the picker, the hotbar, and the pointer's systems.
    fn app() -> App {
        let mut app = App::new();
        app.init_resource::<Modal>()
            .init_resource::<Hotbar>()
            .init_resource::<ButtonInput<MouseButton>>()
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<AccumulatedMouseScroll>()
            .insert_resource(AtlasImage(Handle::default()))
            .add_message::<KeyboardInput>()
            .add_plugins(PickerPlugin)
            .add_systems(Update, grab_pointer.before(type_text));
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

    fn press_b(app: &mut App) {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.press(OPEN_KEY);
        // The browser reports the key as an event, too.
        type_key(app, Key::Character("b".into()), Some("b"));
        // Let go, as the player does: the press is a one-frame thing.
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.release(OPEN_KEY);
        keys.clear();
    }

    fn type_key(app: &mut App, key: Key, text: Option<&str>) {
        let window = app.world_mut().query_filtered::<Entity, With<Window>>().single(app.world()).unwrap();
        app.world_mut().write_message(KeyboardInput { key_code: KeyCode::KeyA, logical_key: key, state: ButtonState::Pressed, text: text.map(Into::into), repeat: false, window });
        app.update();
    }

    fn type_str(app: &mut App, text: &str) {
        for c in text.chars() {
            let s = c.to_string();
            type_key(app, Key::Character(s.as_str().into()), Some(&s));
        }
    }

    /// The catalog entries whose cells are showing.
    fn shown(app: &mut App) -> Vec<&'static str> {
        let mut cells = app.world_mut().query::<(&Cell, &Node)>();
        cells.iter(app.world()).filter(|(_, n)| n.display != Display::None).map(|(Cell(n), _)| catalog::all()[*n].name).collect()
    }

    fn click_cell(app: &mut App, name: &str) {
        let mut cells = app.world_mut().query::<(&Cell, &mut Interaction)>();
        for (Cell(n), mut interaction) in cells.iter_mut(app.world_mut()) {
            if catalog::all()[*n].name == name {
                *interaction = Interaction::Pressed;
            }
        }
        app.update();
    }

    fn root_display(app: &mut App) -> Display {
        app.world_mut().query_filtered::<&Node, With<PickerRoot>>().single(app.world()).unwrap().display
    }

    #[test]
    fn b_opens_the_picker_with_the_pointer_free_and_the_key_not_typed() {
        let mut app = app();
        assert_eq!(root_display(&mut app), Display::None);
        press_b(&mut app);
        assert!(!locked(&mut app));
        assert!(app.world().resource::<Modal>().0);
        assert_eq!(app.world().resource::<Picker>().query, "", "the b that opened it is not typed");
        app.update();
        assert_eq!(root_display(&mut app), Display::Flex);
        assert_eq!(shown(&mut app).len(), catalog::all().len());

        // B is text from now on.
        type_str(&mut app, "b");
        assert_eq!(app.world().resource::<Picker>().query, "b");
        assert!(app.world().resource::<Picker>().open);
    }

    #[test]
    fn typing_filters_the_grid() {
        let mut app = app();
        press_b(&mut app);
        type_str(&mut app, "oak slab");
        app.update();
        assert_eq!(shown(&mut app).len(), 2);
        let mut got = shown(&mut app);
        got.sort();
        assert_eq!(got, ["Dark Oak Slab", "Oak Slab"]);
        for _ in 0..4 {
            type_key(&mut app, Key::Backspace, None);
        }
        assert_eq!(shown(&mut app).len(), filter("oak").len());
    }

    #[test]
    fn clicking_a_block_fills_the_selected_slot_and_leaves_the_rest() {
        let mut app = app();
        app.world_mut().resource_mut::<Hotbar>().selected = 4;
        press_b(&mut app);
        type_str(&mut app, "diamond");
        click_cell(&mut app, "Block of Diamond");
        let hotbar = app.world().resource::<Hotbar>();
        assert_eq!(hotbar.slots[4], Some((57, 0)));
        assert_eq!(hotbar.slots[3], Some((5, 0)), "neighbours are as they were");
        assert_eq!(hotbar.selected, 4);
        assert!(app.world().resource::<Picker>().open, "the picker stays open for the next pick");
    }

    #[test]
    fn esc_closes_it_and_the_next_click_locks_the_pointer() {
        let mut app = app();
        press_b(&mut app);
        type_str(&mut app, "gl");
        type_key(&mut app, Key::Escape, None);
        assert!(!app.world().resource::<Picker>().open);
        assert!(!app.world().resource::<Modal>().0);
        assert!(!locked(&mut app), "the pointer stays free until a click");
        app.update();
        assert_eq!(root_display(&mut app), Display::None);

        // Clicks on a closed picker's cells (they are hidden) pick nothing.
        let before = app.world().resource::<Hotbar>().clone();
        click_cell(&mut app, "Glass");
        assert_eq!(*app.world().resource::<Hotbar>(), before);

        // The pointer is not over a button: it is on the scene.
        let mut cells = app.world_mut().query::<&mut Interaction>();
        for mut i in cells.iter_mut(app.world_mut()) {
            *i = Interaction::None;
        }
        app.world_mut().resource_mut::<ButtonInput<MouseButton>>().press(MouseButton::Left);
        app.update();
        assert!(locked(&mut app));
    }

    #[test]
    fn the_picker_opens_afresh_each_time() {
        let mut app = app();
        press_b(&mut app);
        type_str(&mut app, "stone");
        type_key(&mut app, Key::Escape, None);
        // Lock again, as a click does, and reopen.
        let mut cursors = app.world_mut().query::<&mut CursorOptions>();
        set_locked(&mut cursors.single_mut(app.world_mut()).unwrap(), true);
        press_b(&mut app);
        assert_eq!(app.world().resource::<Picker>().query, "");
        app.update();
        assert_eq!(shown(&mut app).len(), catalog::all().len());
    }

    #[test]
    fn b_does_nothing_while_the_pointer_is_free() {
        let mut app = app();
        let mut cursors = app.world_mut().query::<&mut CursorOptions>();
        set_locked(&mut cursors.single_mut(app.world_mut()).unwrap(), false);
        press_b(&mut app);
        assert!(!app.world().resource::<Picker>().open, "the menu is up, or another dialog");
        assert!(!app.world().resource::<Modal>().0);
    }
}
