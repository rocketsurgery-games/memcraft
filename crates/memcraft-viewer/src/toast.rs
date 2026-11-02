//! A short-lived line of text at the top of the screen, for the result of
//! something the player asked for (today: Save). It sits over the menu and
//! the game alike, because Ctrl+S saves with the pointer locked, and goes
//! away by itself (after four seconds of wall-clock time, however slow the
//! frames are).

use std::time::Duration;

use bevy::prelude::*;

/// Show `text` for a few seconds, in green if `ok` and red if not. A newer
/// toast replaces the current one.
#[derive(Message)]
pub struct Toast {
    pub text: String,
    pub ok: bool,
}

const SHOWN_FOR: Duration = Duration::from_secs(4);
const OK: Color = Color::srgb(0.55, 1.0, 0.55);
const FAILED: Color = Color::srgb(1.0, 0.5, 0.5);

/// Over the menu (`GlobalZIndex(10)`).
const Z: i32 = 20;

#[derive(Component)]
struct ToastLine;

/// How much longer the toast stays up; it is gone at zero.
#[derive(Resource, Default)]
struct Remaining(Duration);

pub struct ToastPlugin;

impl Plugin for ToastPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<Toast>()
            .init_resource::<Remaining>()
            .add_systems(Startup, spawn)
            .add_systems(Update, show);
    }
}

fn spawn(mut commands: Commands) {
    commands.spawn((
        ToastLine,
        Text::new(""),
        TextFont { font_size: FontSize::Px(18.0), ..default() },
        TextColor(OK),
        TextLayout::justify(Justify::Center),
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
        Node {
            display: Display::None,
            position_type: PositionType::Absolute,
            top: px(40),
            // Centred without knowing the text's width.
            left: percent(50),
            padding: UiRect::axes(px(12), px(6)),
            max_width: percent(90),
            ..default()
        },
        UiTransform::from_translation(Val2::new(percent(-50), px(0))),
        GlobalZIndex(Z),
    ));
}

fn show(
    mut toasts: MessageReader<Toast>,
    time: Res<Time<Real>>,
    mut remaining: ResMut<Remaining>,
    mut line: Single<(&mut Node, &mut Text, &mut TextColor), With<ToastLine>>,
) {
    let (node, text, color) = &mut *line;
    if let Some(toast) = toasts.read().last() {
        text.0.clone_from(&toast.text);
        color.0 = if toast.ok { OK } else { FAILED };
        remaining.0 = SHOWN_FOR;
    } else {
        remaining.0 = remaining.0.saturating_sub(time.delta());
    }
    let display = if remaining.0.is_zero() { Display::None } else { Display::Flex };
    if node.display != display {
        node.display = display;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> App {
        let mut app = App::new();
        app.init_resource::<Time<Real>>().add_plugins(ToastPlugin);
        app.update(); // spawns the line
        app
    }

    fn line(app: &mut App) -> (Display, String, Color) {
        let mut q = app.world_mut().query_filtered::<(&Node, &Text, &TextColor), With<ToastLine>>();
        let (node, text, color) = q.single(app.world()).unwrap();
        (node.display, text.0.clone(), color.0)
    }

    fn pass(app: &mut App, secs: u64) {
        app.world_mut().resource_mut::<Time<Real>>().advance_by(Duration::from_secs(secs));
        app.update();
    }

    #[test]
    fn a_toast_shows_for_a_few_seconds_and_goes() {
        let mut app = app();
        assert_eq!(line(&mut app).0, Display::None);

        app.world_mut().write_message(Toast { text: "Saved".into(), ok: true });
        pass(&mut app, 0);
        assert_eq!(line(&mut app), (Display::Flex, "Saved".into(), OK));
        pass(&mut app, 3);
        assert_eq!(line(&mut app).0, Display::Flex);
        pass(&mut app, 2);
        assert_eq!(line(&mut app).0, Display::None);
    }

    #[test]
    fn a_newer_toast_replaces_the_text_and_restarts_the_clock() {
        let mut app = app();
        app.world_mut().write_message(Toast { text: "first".into(), ok: true });
        pass(&mut app, 0);
        pass(&mut app, 3);
        app.world_mut().write_message(Toast { text: "second".into(), ok: false });
        pass(&mut app, 0);
        assert_eq!(line(&mut app), (Display::Flex, "second".into(), FAILED));
        pass(&mut app, 3);
        assert_eq!(line(&mut app).0, Display::Flex);
        pass(&mut app, 2);
        assert_eq!(line(&mut app).0, Display::None);
    }
}
