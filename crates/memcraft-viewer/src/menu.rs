//! The game menu: an overlay that is up whenever the pointer is free (Esc,
//! the window losing focus, the browser dropping the lock, or before the
//! first click) and gone while it is locked, or while a [`Modal`] dialog
//! (the sign editor) has the free pointer. It is the home of Save and,
//! later, settings.
//!
//! Resume locks the pointer again, as does a click anywhere else on the
//! scene (see [`grab_pointer`]). Neither click edits a block: `targeting`
//! ignores clicks while the pointer is free, and runs before this one.
//!
//! Save is a hook for the saver (see `save`): it is greyed out until something
//! sets [`SaveAvailable`], and then pressing it sends a [`SaveRequested`]
//! message; the saver may also rename the button ([`SaveLabel`]). Nothing here
//! writes a world.

use bevy::prelude::*;
use bevy::window::{CursorOptions, PrimaryWindow};

use crate::controls::{grab_pointer, is_locked, set_locked, Modal};

/// Whether the running viewer can save its world. Off until a saver exists
/// (and for good in read-only builds); the menu greys out Save while it is.
#[derive(Resource, Default)]
pub struct SaveAvailable(pub bool);

/// What the Save button says while saving is available ("Download world
/// (.zip)" in the browser).
#[derive(Resource)]
pub struct SaveLabel(pub &'static str);

impl Default for SaveLabel {
    fn default() -> Self {
        Self("Save")
    }
}

/// The player pressed Save while [`SaveAvailable`] was on.
#[derive(Message)]
pub struct SaveRequested;

/// What a menu button does.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
enum MenuAction {
    Resume,
    Save,
    /// Shows or hides the help text.
    Controls,
}

impl MenuAction {
    fn label(self, save_available: bool, save_label: &'static str) -> &'static str {
        match self {
            Self::Resume => "Resume",
            Self::Save if save_available => save_label,
            Self::Save => "Save (not available)",
            Self::Controls => "Controls",
        }
    }
}

#[derive(Component)]
struct MenuRoot;

#[derive(Component)]
struct ControlsHelp;

const HELP: &str = "Click or Resume to look around (Esc releases the mouse)\n\
WASD move - Q/E down/up - Shift fast - scroll: speed\n\
F: fly/walk (walk: Space jumps) - 1-9: choose block - B: block picker (Esc closes)\n\
Left click: break block - Middle click: pick block\n\
Right click: use a door, lever or button, edit a sign, or place block";

const BACKDROP: Color = Color::srgba(0.0, 0.0, 0.0, 0.55);
const BUTTON: Color = Color::srgb(0.30, 0.30, 0.30);
const BUTTON_HOVER: Color = Color::srgb(0.42, 0.45, 0.62);
const BUTTON_DISABLED: Color = Color::srgb(0.18, 0.18, 0.18);
const TEXT: Color = Color::WHITE;
const TEXT_DISABLED: Color = Color::srgb(0.5, 0.5, 0.5);

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SaveAvailable>().init_resource::<SaveLabel>().add_message::<SaveRequested>().add_systems(Startup, spawn_menu).add_systems(
            Update,
            // After `grab_pointer`, so a press on Resume doesn't lock the
            // pointer before `targeting` has seen the click.
            (show_menu, press_buttons, style_buttons).after(grab_pointer),
        );
    }
}

fn spawn_menu(mut commands: Commands, save: Res<SaveAvailable>, label: Res<SaveLabel>) {
    commands
        .spawn((
            MenuRoot,
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                // From a fixed height, so that showing the help moves nothing else.
                padding: UiRect::top(px(200)),
                row_gap: px(10),
                ..default()
            },
            BackgroundColor(BACKDROP),
            // Over the hotbar, whatever order they were spawned in.
            GlobalZIndex(10),
        ))
        .with_children(|menu| {
            menu.spawn((Text::new("MemCraft"), TextFont { font_size: FontSize::Px(32.0), ..default() }, TextColor(TEXT)));
            for action in [MenuAction::Resume, MenuAction::Save, MenuAction::Controls] {
                menu.spawn((
                    Button,
                    action,
                    Node { width: px(260), height: px(40), justify_content: JustifyContent::Center, align_items: AlignItems::Center, ..default() },
                    BackgroundColor(BUTTON),
                ))
                .with_child((Text::new(action.label(save.0, label.0)), TextFont { font_size: FontSize::Px(18.0), ..default() }, TextColor(TEXT)));
            }
            menu.spawn((
                ControlsHelp,
                Node { display: Display::None, margin: UiRect::top(px(10)), ..default() },
                Text::new(HELP),
                TextFont { font_size: FontSize::Px(14.0), ..default() },
                TextColor(TEXT),
            ));
        });
}

/// The menu is up exactly while the pointer is free, and no dialog has it.
fn show_menu(cursor: Single<&CursorOptions, With<PrimaryWindow>>, modal: Res<Modal>, mut menu: Single<&mut Node, With<MenuRoot>>) {
    let display = if is_locked(&cursor) || modal.0 { Display::None } else { Display::Flex };
    if menu.display != display {
        menu.display = display;
    }
}

fn press_buttons(
    buttons: Query<(&Interaction, &MenuAction), Changed<Interaction>>,
    save: Res<SaveAvailable>,
    mut saves: MessageWriter<SaveRequested>,
    mut cursor: Single<&mut CursorOptions, With<PrimaryWindow>>,
    mut help: Single<&mut Node, With<ControlsHelp>>,
) {
    for (_, action) in buttons.iter().filter(|(i, _)| **i == Interaction::Pressed) {
        match action {
            MenuAction::Resume => set_locked(&mut cursor, true),
            MenuAction::Save if save.0 => {
                saves.write(SaveRequested);
            }
            MenuAction::Save => {}
            MenuAction::Controls => help.display = if help.display == Display::None { Display::Flex } else { Display::None },
        }
    }
}

/// Hover highlight, and Save's label and greying while it is unavailable.
fn style_buttons(
    save: Res<SaveAvailable>,
    save_label: Res<SaveLabel>,
    mut buttons: Query<(&Interaction, &MenuAction, &mut BackgroundColor, &Children)>,
    mut labels: Query<(&mut Text, &mut TextColor)>,
) {
    for (interaction, action, mut background, children) in &mut buttons {
        let disabled = *action == MenuAction::Save && !save.0;
        let color = match (disabled, interaction) {
            (true, _) => BUTTON_DISABLED,
            (false, Interaction::None) => BUTTON,
            (false, _) => BUTTON_HOVER,
        };
        background.set_if_neq(BackgroundColor(color));
        for &child in children {
            if let Ok((mut text, mut text_color)) = labels.get_mut(child) {
                let label = action.label(save.0, save_label.0);
                if text.0 != label {
                    text.0 = label.to_string();
                }
                text_color.set_if_neq(TextColor(if disabled { TEXT_DISABLED } else { TEXT }));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A window, the menu and the pointer's systems, without rendering. The menu
    /// is spawned by the first `update`.
    fn app() -> App {
        let mut app = App::new();
        app.init_resource::<ButtonInput<MouseButton>>()
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<Modal>()
            .add_plugins(MenuPlugin)
            .add_systems(Update, grab_pointer);
        app.world_mut().spawn((Window { focused: true, ..default() }, CursorOptions::default(), PrimaryWindow));
        app
    }

    fn locked(app: &mut App) -> bool {
        let mut cursors = app.world_mut().query::<&CursorOptions>();
        is_locked(cursors.single(app.world()).unwrap())
    }

    fn menu_display(app: &mut App) -> Display {
        let mut menu = app.world_mut().query_filtered::<&Node, With<MenuRoot>>();
        menu.single(app.world()).unwrap().display
    }

    /// Presses the button of `action` (as `ui_focus_system` would).
    fn press(app: &mut App, action: MenuAction) {
        let mut buttons = app.world_mut().query::<(&MenuAction, &mut Interaction)>();
        for (a, mut interaction) in buttons.iter_mut(app.world_mut()) {
            if *a == action {
                *interaction = Interaction::Pressed;
            }
        }
        app.update();
    }

    fn lock(app: &mut App) {
        let mut cursors = app.world_mut().query::<&mut CursorOptions>();
        set_locked(&mut cursors.single_mut(app.world_mut()).unwrap(), true);
    }

    #[test]
    fn the_menu_is_up_exactly_while_the_pointer_is_free() {
        let mut app = app();
        app.update();
        assert_eq!((locked(&mut app), menu_display(&mut app)), (false, Display::Flex));
        lock(&mut app);
        app.update();
        assert_eq!(menu_display(&mut app), Display::None);
        app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::Escape);
        app.update();
        assert_eq!((locked(&mut app), menu_display(&mut app)), (false, Display::Flex));
    }

    #[test]
    fn resume_locks_the_pointer_and_hides_the_menu() {
        let mut app = app();
        app.update();
        press(&mut app, MenuAction::Resume);
        assert!(locked(&mut app));
        app.update();
        assert_eq!(menu_display(&mut app), Display::None);
    }

    #[test]
    fn a_click_on_a_button_does_not_lock_but_one_on_the_scene_does() {
        let mut app = app();
        app.update();
        let mut buttons = app.world_mut().query::<&mut Interaction>();
        for mut i in buttons.iter_mut(app.world_mut()) {
            *i = Interaction::Hovered;
        }
        app.world_mut().resource_mut::<ButtonInput<MouseButton>>().press(MouseButton::Left);
        app.update();
        assert!(!locked(&mut app), "the click was on a button");

        for mut i in buttons.iter_mut(app.world_mut()) {
            *i = Interaction::None;
        }
        let mut mouse = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
        mouse.release(MouseButton::Left);
        mouse.clear();
        mouse.press(MouseButton::Left);
        app.update();
        assert!(locked(&mut app));
    }

    #[test]
    fn save_sends_a_request_only_once_it_is_available() {
        let mut app = app();
        app.update();
        let requests = |app: &mut App| app.world_mut().resource_mut::<Messages<SaveRequested>>().drain().count();
        press(&mut app, MenuAction::Save);
        assert_eq!(requests(&mut app), 0, "Save is not available yet");

        app.world_mut().resource_mut::<SaveAvailable>().0 = true;
        // A fresh press: the earlier one is no longer a change.
        let mut buttons = app.world_mut().query::<&mut Interaction>();
        for mut i in buttons.iter_mut(app.world_mut()) {
            *i = Interaction::None;
        }
        app.update();
        press(&mut app, MenuAction::Save);
        assert_eq!(requests(&mut app), 1);
        assert!(!locked(&mut app), "Save leaves the menu up");
    }

    #[test]
    fn controls_toggles_the_help() {
        let mut app = app();
        app.update();
        let mut help = app.world_mut().query_filtered::<&Node, With<ControlsHelp>>();
        assert_eq!(help.single(app.world()).unwrap().display, Display::None);
        press(&mut app, MenuAction::Controls);
        assert_eq!(help.single(app.world()).unwrap().display, Display::Flex);
        assert!(!locked(&mut app));
    }
}
