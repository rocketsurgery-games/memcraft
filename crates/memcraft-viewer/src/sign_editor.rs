//! The sign editor: right click on a sign (see `targeting`) opens a small
//! dialog with the sign's four lines; typing edits the line with the caret,
//! Enter goes on to the next line (and saves from the last), Up/Down/Tab or a
//! click choose a line, Done saves, Esc or Cancel throws the edit away.
//!
//! The dialog needs the free pointer, so opening it releases the lock, and
//! closing it locks again. While it is open [`Modal`] is set: the menu stays
//! down and a click on the scene does not lock the pointer. The player's
//! controls act only while the pointer is locked (see `controls`), so typing
//! never moves the player. Saving stores the lines in the sign's block entity
//! (see [`WorldMap::set_block_entity`]); `signs` redraws the text.

use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::ButtonState;
use bevy::prelude::*;
use bevy::window::{CursorOptions, PrimaryWindow};

use crate::controls::{grab_pointer, set_locked, Modal};
use crate::font::{is_typable, LINE_CHARS};
use crate::signs;
use crate::world::WorldMap;

/// Right click on the sign at this cell: open its editor.
#[derive(Message)]
pub struct EditSign(pub IVec3);

/// The edit in progress.
#[derive(Debug, PartialEq)]
struct Editing {
    cell: IVec3,
    lines: [String; 4],
    /// The line with the caret.
    line: usize,
}

#[derive(Debug, PartialEq)]
enum Outcome {
    Keep,
    Save,
    Cancel,
}

impl Editing {
    /// Applies a key press: `text` is what it types, if anything.
    fn press(&mut self, key: &Key, text: Option<&str>) -> Outcome {
        match key {
            Key::Escape => return Outcome::Cancel,
            Key::Enter if self.line == 3 => return Outcome::Save,
            Key::Enter | Key::ArrowDown | Key::Tab => self.line = (self.line + 1).min(3),
            Key::ArrowUp => self.line = self.line.saturating_sub(1),
            Key::Backspace => {
                self.lines[self.line].pop();
            }
            _ => {
                for c in text.unwrap_or_default().chars().filter(|&c| is_typable(c)) {
                    let line = &mut self.lines[self.line];
                    if line.chars().count() < LINE_CHARS {
                        line.push(c);
                    }
                }
            }
        }
        Outcome::Keep
    }
}

/// The open editor, if any.
#[derive(Resource, Default)]
struct Editor(Option<Editing>);

#[derive(Component)]
struct EditorRoot;

/// A line's button, and its label.
#[derive(Component)]
struct Line(usize);

#[derive(Component)]
struct LineLabel(usize);

#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum Finish {
    Done,
    Cancel,
}

const BACKDROP: Color = Color::srgba(0.0, 0.0, 0.0, 0.6);
const BUTTON: Color = Color::srgb(0.30, 0.30, 0.30);
const BUTTON_HOVER: Color = Color::srgb(0.42, 0.45, 0.62);
const LINE_FILL: Color = Color::srgb(0.12, 0.12, 0.12);
const LINE_CURRENT: Color = Color::srgb(0.30, 0.28, 0.12);

pub struct SignEditorPlugin;

impl Plugin for SignEditorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Editor>().add_message::<EditSign>().add_systems(Startup, spawn_ui).add_systems(
            Update,
            // After `grab_pointer`, so the click that closes the editor (it
            // locks the pointer again) is not also taken for a block click.
            (type_text, press_buttons, open, show).chain().after(grab_pointer),
        );
    }
}

fn spawn_ui(mut commands: Commands) {
    let text = |size: f32| (TextFont { font_size: FontSize::Px(size), ..default() }, TextColor(Color::WHITE));
    commands
        .spawn((
            EditorRoot,
            Node {
                display: Display::None,
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                padding: UiRect::top(px(160)),
                row_gap: px(10),
                ..default()
            },
            BackgroundColor(BACKDROP),
            // Over the menu and the hotbar.
            GlobalZIndex(20),
        ))
        .with_children(|root| {
            root.spawn((Text::new("Edit sign message"), text(24.0)));
            for n in 0..4 {
                root.spawn((
                    Button,
                    Line(n),
                    Node { width: px(360), height: px(34), padding: UiRect::horizontal(px(10)), align_items: AlignItems::Center, ..default() },
                    BackgroundColor(LINE_FILL),
                ))
                .with_child((LineLabel(n), Text::new(""), text(18.0)));
            }
            root.spawn(Node { column_gap: px(10), margin: UiRect::top(px(6)), ..default() }).with_children(|row| {
                for (finish, label) in [(Finish::Done, "Done"), (Finish::Cancel, "Cancel")] {
                    row.spawn((
                        Button,
                        finish,
                        Node { width: px(175), height: px(36), justify_content: JustifyContent::Center, align_items: AlignItems::Center, ..default() },
                        BackgroundColor(BUTTON),
                    ))
                    .with_child((Text::new(label), text(18.0)));
                }
            });
            root.spawn((Text::new("Enter: next line, saves after the last - Up/Down/Tab or click: choose a line - Esc: cancel"), text(13.0)));
        });
}

/// Opens the editor on a click on a sign, freeing the pointer for it.
fn open(
    mut clicks: MessageReader<EditSign>,
    map: Res<WorldMap>,
    mut editor: ResMut<Editor>,
    mut modal: ResMut<Modal>,
    mut cursor: Single<&mut CursorOptions, With<PrimaryWindow>>,
) {
    let Some(&EditSign(cell)) = clicks.read().last() else {
        return;
    };
    let lines = map.block_entity(cell).map(signs::lines).unwrap_or_default();
    editor.0 = Some(Editing { cell, lines, line: 0 });
    modal.0 = true;
    set_locked(&mut cursor, false);
}

/// Ends the edit, saving it or not, and gives the pointer back to the player.
fn finish(outcome: Outcome, editor: &mut Editor, map: &mut WorldMap, modal: &mut Modal, cursor: &mut CursorOptions) {
    if outcome == Outcome::Keep {
        return;
    }
    let Some(Editing { cell, lines, .. }) = editor.0.take() else {
        return;
    };
    if outcome == Outcome::Save {
        let old = map.block_entity(cell);
        if old.map(signs::lines).unwrap_or_default() != lines {
            let sign = signs::with_lines(old, cell, &lines);
            map.set_block_entity(cell, sign);
            info!("edited the sign at {cell}: {lines:?}");
        }
    }
    modal.0 = false;
    set_locked(cursor, true);
}

/// Typing and the editing keys. Every key event is read, so that keys
/// pressed before the editor opened do not show up in it.
fn type_text(
    mut keys: MessageReader<KeyboardInput>,
    mut editor: ResMut<Editor>,
    mut map: ResMut<WorldMap>,
    mut modal: ResMut<Modal>,
    mut cursor: Single<&mut CursorOptions, With<PrimaryWindow>>,
) {
    for key in keys.read().filter(|k| k.state == ButtonState::Pressed) {
        let Some(editing) = &mut editor.0 else {
            continue;
        };
        let outcome = editing.press(&key.logical_key, key.text.as_deref());
        finish(outcome, &mut editor, &mut map, &mut modal, &mut cursor);
    }
}

/// Clicks on a line, Done and Cancel, and the buttons' hover colour.
fn press_buttons(
    lines: Query<(&Interaction, &Line), Changed<Interaction>>,
    finishers: Query<(&Interaction, &Finish, &mut BackgroundColor)>,
    mut editor: ResMut<Editor>,
    mut map: ResMut<WorldMap>,
    mut modal: ResMut<Modal>,
    mut cursor: Single<&mut CursorOptions, With<PrimaryWindow>>,
) {
    let mut outcome = Outcome::Keep;
    if let Some(editing) = &mut editor.0
        && let Some((_, Line(n))) = lines.iter().find(|(i, _)| **i == Interaction::Pressed)
    {
        editing.line = *n;
    }
    for (interaction, finish, mut background) in finishers {
        background.set_if_neq(BackgroundColor(if *interaction == Interaction::None { BUTTON } else { BUTTON_HOVER }));
        if editor.0.is_some() && *interaction == Interaction::Pressed {
            outcome = if *finish == Finish::Done { Outcome::Save } else { Outcome::Cancel };
        }
    }
    finish(outcome, &mut editor, &mut map, &mut modal, &mut cursor);
}

/// Shows the dialog while an edit is open, with the caret `|` in the current
/// line.
fn show(
    editor: Res<Editor>,
    mut root: Single<&mut Node, With<EditorRoot>>,
    mut buttons: Query<(&Line, &mut BackgroundColor)>,
    mut labels: Query<(&LineLabel, &mut Text)>,
) {
    let display = if editor.0.is_some() { Display::Flex } else { Display::None };
    if root.display != display {
        root.display = display;
    }
    let Some(editing) = &editor.0 else {
        return;
    };
    for (Line(n), mut background) in &mut buttons {
        background.set_if_neq(BackgroundColor(if *n == editing.line { LINE_CURRENT } else { LINE_FILL }));
    }
    for (LineLabel(n), mut text) in &mut labels {
        let label = if *n == editing.line { format!("{}|", editing.lines[*n]) } else { editing.lines[*n].clone() };
        if text.0 != label {
            text.0 = label;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn editing(lines: [&str; 4]) -> Editing {
        Editing { cell: IVec3::ZERO, lines: lines.map(String::from), line: 0 }
    }

    fn chars(e: &mut Editing, text: &str) {
        for c in text.chars() {
            let s = c.to_string();
            assert_eq!(e.press(&Key::Character(s.as_str().into()), Some(&s)), Outcome::Keep);
        }
    }

    #[test]
    fn typing_fills_the_line_and_enter_moves_on_until_the_last_saves() {
        let mut e = editing(["", "", "", ""]);
        chars(&mut e, "Hello, world!");
        assert_eq!(e.press(&Key::Backspace, None), Outcome::Keep);
        assert_eq!(e.lines[0], "Hello, world");
        assert_eq!(e.press(&Key::Enter, Some("\r")), Outcome::Keep);
        chars(&mut e, "line two");
        assert_eq!((e.line, e.lines[0].as_str(), e.lines[1].as_str()), (1, "Hello, world", "line two"));
        e.press(&Key::ArrowDown, None);
        e.press(&Key::Tab, None);
        assert_eq!(e.line, 3);
        assert_eq!(e.press(&Key::ArrowDown, None), Outcome::Keep);
        assert_eq!(e.line, 3, "stays on the last line");
        assert_eq!(e.press(&Key::Enter, Some("\r")), Outcome::Save);
        e.line = 0;
        assert_eq!(e.press(&Key::ArrowUp, None), Outcome::Keep);
        assert_eq!(e.line, 0);
        assert_eq!(e.press(&Key::Escape, None), Outcome::Cancel);
    }

    #[test]
    fn a_line_holds_what_fits_the_board_and_only_what_the_font_draws() {
        let mut e = editing(["", "", "", ""]);
        chars(&mut e, &"x".repeat(40));
        assert_eq!(e.lines[0].len(), LINE_CHARS);
        let mut e = editing(["", "", "", ""]);
        // Control characters (a Ctrl+key chord) and what the font lacks are dropped.
        assert_eq!(e.press(&Key::Character("v".into()), Some("\u{16}")), Outcome::Keep);
        assert_eq!(e.press(&Key::Character("é".into()), Some("é")), Outcome::Keep);
        assert_eq!(e.press(&Key::Shift, None), Outcome::Keep);
        assert_eq!(e.lines[0], "");
        assert_eq!(e.press(&Key::Backspace, None), Outcome::Keep, "backspace on an empty line is fine");
    }

    /// A window with the pointer locked, the sign editor, and the world.
    fn app() -> (App, IVec3) {
        let map = WorldMap::new(crate::load_world(None).unwrap());
        let cell = map.world().columns.values().flat_map(|c| &c.block_entities).find(|be| be.get("id").and_then(mcpe_world::Tag::as_str) == Some("Sign")).map(|be| IVec3::new(be.get("x").unwrap().as_i64().unwrap() as i32, be.get("y").unwrap().as_i64().unwrap() as i32, be.get("z").unwrap().as_i64().unwrap() as i32)).unwrap();
        let mut app = App::new();
        app.insert_resource(map)
            .init_resource::<Modal>()
            .init_resource::<ButtonInput<MouseButton>>()
            .init_resource::<ButtonInput<KeyCode>>()
            .add_message::<KeyboardInput>()
            .add_plugins(SignEditorPlugin)
            .add_systems(Update, grab_pointer.before(open));
        let mut cursor = CursorOptions::default();
        set_locked(&mut cursor, true);
        app.world_mut().spawn((Window { focused: true, ..default() }, cursor, PrimaryWindow));
        app.update();
        (app, cell)
    }

    fn locked(app: &mut App) -> bool {
        let mut cursors = app.world_mut().query::<&CursorOptions>();
        crate::controls::is_locked(cursors.single(app.world()).unwrap())
    }

    fn type_key(app: &mut App, key: Key, text: Option<&str>) {
        let window = app.world_mut().query_filtered::<Entity, With<Window>>().single(app.world()).unwrap();
        app.world_mut().write_message(KeyboardInput { key_code: KeyCode::KeyA, logical_key: key, state: ButtonState::Pressed, text: text.map(Into::into), repeat: false, window });
        app.update();
    }

    #[test]
    fn opening_frees_the_pointer_and_saving_stores_the_lines_and_locks_again() {
        let (mut app, cell) = app();
        assert!(locked(&mut app));
        app.world_mut().write_message(EditSign(cell));
        app.update();
        assert!(!locked(&mut app), "the pointer is free for the dialog");
        assert!(app.world().resource::<Modal>().0);
        assert!(app.world().resource::<Editor>().0.is_some());

        // A click on the scene must not lock it while the dialog is up.
        app.world_mut().resource_mut::<ButtonInput<MouseButton>>().press(MouseButton::Left);
        app.update();
        assert!(!locked(&mut app));
        app.world_mut().resource_mut::<ButtonInput<MouseButton>>().release(MouseButton::Left);
        app.world_mut().resource_mut::<ButtonInput<MouseButton>>().clear();

        for c in ["h", "i"] {
            type_key(&mut app, Key::Character(c.into()), Some(c));
        }
        let before = app.world().resource::<WorldMap>().block_entity(cell).map(signs::lines).unwrap();
        // Esc cancels: nothing changes, the pointer is locked.
        type_key(&mut app, Key::Escape, None);
        assert!(locked(&mut app));
        assert!(!app.world().resource::<Modal>().0);
        assert_eq!(app.world().resource::<WorldMap>().block_entity(cell).map(signs::lines).unwrap(), before);

        // Again, this time saved from the last line with Enter.
        app.world_mut().write_message(EditSign(cell));
        app.update();
        for _ in 0..4 {
            type_key(&mut app, Key::Backspace, None);
        }
        type_key(&mut app, Key::Character("o".into()), Some("o"));
        type_key(&mut app, Key::Character("k".into()), Some("k"));
        for _ in 0..3 {
            type_key(&mut app, Key::Enter, Some("\r"));
        }
        assert!(app.world().resource::<Editor>().0.is_some());
        type_key(&mut app, Key::Enter, Some("\r"));
        assert!(app.world().resource::<Editor>().0.is_none());
        assert!(locked(&mut app));
        let saved = app.world().resource::<WorldMap>().block_entity(cell).map(signs::lines).unwrap();
        assert!(saved[0].ends_with("ok"), "{saved:?}");
        assert_eq!(saved[1..], before[1..]);
    }
}
