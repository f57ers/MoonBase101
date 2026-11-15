use crate::types::{AppState, GameSettings};
use bevy::prelude::*;

pub const ACTIONS: &[(KeyCode, &str)] = &[
    (KeyCode::KeyW, "Do przodu"),
    (KeyCode::KeyS, "Do tyłu"),
    (KeyCode::KeyA, "W lewo"),
    (KeyCode::KeyD, "W prawo"),
    (KeyCode::Space, "Skok / wzlot"),
    (KeyCode::ShiftLeft, "Napęd plecaka"),
    (KeyCode::ControlLeft, "Lądowanie"),
    (KeyCode::KeyE, "Wejdź / wysiądź"),
    (KeyCode::KeyG, "Pilotuj stację"),
    (KeyCode::KeyP, "Badanie"),
    (KeyCode::KeyI, "Plecak"),
    (KeyCode::KeyM, "Mapa"),
    (KeyCode::KeyN, "Dziennik"),
    (KeyCode::KeyK, "Wygląd"),
    (KeyCode::KeyB, "Budowanie"),
    (KeyCode::KeyR, "Jedzenie"),
    (KeyCode::KeyT, "Picie"),
    (KeyCode::KeyF, "Naprawa"),
    (KeyCode::KeyC, "Ulepszenie narzędzia"),
    (KeyCode::KeyH, "Flaga"),
    (KeyCode::KeyX, "Apteczka"),
    (KeyCode::KeyZ, "Zmiana astronauty"),
];

#[derive(Resource, Default)]
pub struct ControlsMenu {
    pub open: bool,
    capture: Option<usize>,
    status: String,
}
#[derive(Component)]
pub(crate) enum ControlButton {
    Bind(usize),
    Reset,
    Close,
}

pub fn key_name(key: KeyCode) -> String {
    format!("{key:?}").replace("Key", "").replace("Digit", "")
}
pub fn key_code(name: &str) -> Option<KeyCode> {
    let letters = [
        KeyCode::KeyA,
        KeyCode::KeyB,
        KeyCode::KeyC,
        KeyCode::KeyD,
        KeyCode::KeyE,
        KeyCode::KeyF,
        KeyCode::KeyG,
        KeyCode::KeyH,
        KeyCode::KeyI,
        KeyCode::KeyJ,
        KeyCode::KeyK,
        KeyCode::KeyL,
        KeyCode::KeyM,
        KeyCode::KeyN,
        KeyCode::KeyO,
        KeyCode::KeyP,
        KeyCode::KeyQ,
        KeyCode::KeyR,
        KeyCode::KeyS,
        KeyCode::KeyT,
        KeyCode::KeyU,
        KeyCode::KeyV,
        KeyCode::KeyW,
        KeyCode::KeyX,
        KeyCode::KeyY,
        KeyCode::KeyZ,
        KeyCode::Space,
        KeyCode::ShiftLeft,
        KeyCode::ShiftRight,
        KeyCode::ControlLeft,
        KeyCode::ControlRight,
        KeyCode::AltLeft,
        KeyCode::AltRight,
        KeyCode::ArrowUp,
        KeyCode::ArrowDown,
        KeyCode::ArrowLeft,
        KeyCode::ArrowRight,
        KeyCode::Digit0,
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
    letters.into_iter().find(|key| format!("{key:?}") == name)
}

pub fn input(
    mut events: EventReader<bevy::input::keyboard::KeyboardInput>,
    mut focus: EventReader<bevy::window::WindowFocused>,
    mut physical: Local<ButtonInput<KeyCode>>,
    mut logical: Local<ButtonInput<KeyCode>>,
    mut effective: ResMut<ButtonInput<KeyCode>>,
    mut settings: ResMut<GameSettings>,
    mut menu: ResMut<ControlsMenu>,
    state: Res<State<AppState>>,
    lan: Res<crate::net_lan::LanMultiplayer>,
) {
    use bevy::input::ButtonState;
    physical.clear();
    if focus.read().any(|event| !event.focused) {
        physical.reset_all();
    }
    for event in events.read() {
        if event.state == ButtonState::Pressed {
            physical.press(event.key_code);
            if let Some(index) = menu.capture {
                if event.key_code == KeyCode::Escape {
                    menu.capture = None;
                    continue;
                }
                if key_code(&format!("{:?}", event.key_code)).is_some() {
                    let old = settings.binding(index);
                    for other in 0..ACTIONS.len() {
                        if other != index && settings.binding(other) == event.key_code {
                            settings.bindings.insert(other, format!("{old:?}"));
                        }
                    }
                    settings
                        .bindings
                        .insert(index, format!("{:?}", event.key_code));
                    menu.capture = None;
                    menu.status = crate::locale::text("Zapisano sterowanie.").into();
                    settings.save();
                    physical.reset_all();
                } else {
                    menu.status = crate::locale::text(
                        "Wybierz literę, cyfrę, strzałkę, Spację, Shift lub Ctrl. Esc anuluje.",
                    )
                    .into();
                }
            }
        } else {
            physical.release(event.key_code);
        }
    }
    let remap = matches!(
        state.get(),
        AppState::LunarSurface | AppState::InterplanetaryFlight
    ) && !lan.chat_open;
    let desired: std::collections::HashSet<_> = physical
        .get_pressed()
        .copied()
        .filter_map(|key| {
            if !remap {
                return Some(key);
            }
            if let Some(index) = (0..ACTIONS.len()).find(|&i| settings.binding(i) == key) {
                return Some(ACTIONS[index].0);
            }
            (!ACTIONS.iter().any(|(canonical, _)| *canonical == key)).then_some(key)
        })
        .collect();
    // Bevy's PreUpdate already wrote the raw key events to `effective`.
    // Compare with the previous logical frame to preserve fresh key presses.
    logical.clear();
    let released: Vec<_> = logical
        .get_pressed()
        .copied()
        .filter(|key| !desired.contains(key))
        .collect();
    for key in released {
        logical.release(key);
    }
    for key in desired {
        logical.press(key);
    }
    effective.clone_from(&logical);
}

pub fn menu(
    mut commands: Commands,
    state: Res<State<AppState>>,
    mut menu: ResMut<ControlsMenu>,
    mut settings: ResMut<GameSettings>,
    buttons: Query<(&Interaction, &ControlButton), Changed<Interaction>>,
    mut shown: Local<Option<Entity>>,
    mut signature: Local<String>,
) {
    for (interaction, button) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match button {
            ControlButton::Bind(index) => {
                menu.capture = Some(*index);
                menu.status = crate::locale::text("Naciśnij nowy klawisz. Esc anuluje.").into();
            }
            ControlButton::Reset => {
                settings.bindings.clear();
                settings.save();
                menu.status = crate::locale::text("Przywrócono domyślne klawisze.").into();
            }
            ControlButton::Close => {
                menu.open = false;
                menu.capture = None;
            }
        }
    }
    let show = menu.open && *state.get() == AppState::Settings;
    let current = format!(
        "{show}{:?}{:?}{}",
        menu.capture, settings.bindings, menu.status
    );
    if *signature == current {
        return;
    }
    *signature = current;
    if let Some(entity) = shown.take() {
        commands.entity(entity).despawn_recursive();
    }
    if !show {
        return;
    }
    let entity = commands
        .spawn((
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    left: Val::Percent(15.0),
                    top: Val::Px(55.0),
                    width: Val::Percent(70.0),
                    padding: UiRect::all(Val::Px(20.0)),
                    flex_direction: FlexDirection::Column,
                    ..default()
                },
                background_color: Color::rgb(0.025, 0.05, 0.08).into(),
                z_index: ZIndex::Global(2000),
                ..default()
            },
            crate::ui_screens::WorldInputBlocker,
        ))
        .with_children(|panel| {
            panel.spawn(TextBundle::from_section(
                format!("STEROWANIE\n{}", menu.status),
                TextStyle {
                    font_size: 20.0,
                    color: Color::WHITE,
                    ..default()
                },
            ));
            panel
                .spawn(NodeBundle {
                    style: Style {
                        flex_wrap: FlexWrap::Wrap,
                        ..default()
                    },
                    ..default()
                })
                .with_children(|rows| {
                    for (index, (_, label)) in ACTIONS.iter().enumerate() {
                        button(
                            rows,
                            &format!(
                                "{} · {}{}",
                                crate::locale::text(label),
                                key_name(settings.binding(index)),
                                if menu.capture == Some(index) {
                                    " ←"
                                } else {
                                    ""
                                }
                            ),
                            ControlButton::Bind(index),
                        );
                    }
                    button(rows, crate::locale::text("Domyślne"), ControlButton::Reset);
                    button(rows, crate::locale::text("Zamknij"), ControlButton::Close);
                });
        })
        .id();
    *shown = Some(entity);
}
fn button(parent: &mut ChildBuilder, label: &str, action: ControlButton) {
    parent
        .spawn((
            action,
            ButtonBundle {
                style: Style {
                    width: Val::Percent(48.0),
                    min_height: Val::Px(36.0),
                    margin: UiRect::all(Val::Px(3.0)),
                    padding: UiRect::all(Val::Px(7.0)),
                    ..default()
                },
                background_color: Color::rgb(0.08, 0.18, 0.24).into(),
                ..default()
            },
        ))
        .with_children(|button| {
            button.spawn(TextBundle::from_section(
                label,
                TextStyle {
                    font_size: 14.0,
                    color: Color::WHITE,
                    ..default()
                },
            ));
        });
}

pub fn limit_frames(settings: Res<GameSettings>, mut previous: Local<Option<std::time::Instant>>) {
    if let (Some(limit), Some(started)) = (settings.fps_limit, *previous) {
        let budget = std::time::Duration::from_secs_f64(1.0 / f64::from(limit.max(30)));
        if let Some(wait) = budget.checked_sub(started.elapsed()) {
            std::thread::sleep(wait);
        }
    }
    *previous = Some(std::time::Instant::now());
}
