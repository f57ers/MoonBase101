use bevy::app::AppExit;
use bevy::prelude::*;
use bevy::window::{PrimaryWindow, WindowMode};
use std::fs;

use crate::audio::{GameAudio, SfxKind};
use crate::campaign::{self, CampaignLaunch, Quest};
use crate::isometric::{iso_to_world, ProceduralAssets};
use crate::net_lan::LanMultiplayer;
use crate::physics_gameplay::LunarEventManager;
use crate::planets::{Expedition, WorldBody};
use crate::types::{
    AppState, DifficultyLevel, GameSettings, LunarEventKind, MdlOption, MissionConfig,
    SciInstrument, StructureKind, GAME_CREATORS_LINE,
};
use crate::world::{AstronautPlayer, LunarMap, SurfaceUiCmd, SurvivalStats, TouchAndUiInput};

#[derive(Component)]
pub struct ScreenUiRoot;

#[derive(Component)]
pub struct WorldInputBlocker;

#[derive(Resource, Default)]
pub struct SettingsEdit {
    pub name: bool,
}
pub fn settings_text_input(
    mut events: EventReader<bevy::input::keyboard::KeyboardInput>,
    state: Res<State<AppState>>,
    mut edit: ResMut<SettingsEdit>,
    mut config: ResMut<MissionConfig>,
    mut settings: ResMut<GameSettings>,
) {
    use bevy::input::{keyboard::Key, ButtonState};
    if *state.get() != AppState::Settings {
        edit.name = false;
        events.clear();
        return;
    }
    if !edit.name {
        events.clear();
        return;
    }
    for event in events
        .read()
        .filter(|event| event.state == ButtonState::Pressed)
    {
        match &event.logical_key {
            Key::Enter | Key::Escape => {
                edit.name = false;
                settings.astronaut_name = config.callsign.trim().to_string();
                settings.save();
            }
            Key::Backspace => {
                config.callsign.pop();
            }
            Key::Space => {
                if config.callsign.chars().count() < 24 {
                    config.callsign.push(' ');
                }
            }
            Key::Character(value) => {
                for ch in value.chars().filter(|c| !c.is_control()) {
                    if config.callsign.chars().count() < 24 {
                        config.callsign.push(ch);
                    }
                }
            }
            _ => {}
        }
    }
}
pub fn hud_visibility(
    keys: Res<ButtonInput<KeyCode>>,
    state: Res<State<AppState>>,
    mut settings: ResMut<GameSettings>,
    mut roots: Query<
        &mut Visibility,
        Or<(With<ScreenUiRoot>, With<crate::mobile_host::PhonePanel>)>,
    >,
) {
    if keys.just_pressed(KeyCode::F4) && *state.get() == AppState::LunarSurface {
        settings.hide_hud = !settings.hide_hud;
    }
    for mut visibility in &mut roots {
        *visibility = if settings.hide_hud && *state.get() == AppState::LunarSurface {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
    }
}

#[derive(Component)]
pub struct ChatPanel;
#[derive(Component)]
pub struct ChatText;

pub fn chat_keyboard(
    mut events: EventReader<bevy::input::keyboard::KeyboardInput>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mut mouse: ResMut<ButtonInput<MouseButton>>,
    mut lan: ResMut<LanMultiplayer>,
    config: Res<MissionConfig>,
    time: Res<Time>,
    state: Res<State<AppState>>,
    mut wheel: EventReader<bevy::input::mouse::MouseWheel>,
) {
    use bevy::input::{keyboard::Key, ButtonState};
    if !lan.is_active || *state.get() != AppState::LunarSurface {
        lan.chat_open = false;
        events.clear();
        return;
    }
    let was_open = lan.chat_open;
    let control = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    for event in events
        .read()
        .filter(|event| event.state == ButtonState::Pressed)
    {
        match &event.logical_key {
            Key::Enter if !lan.chat_open => lan.chat_open = true,
            Key::Enter => {
                let text = std::mem::take(&mut lan.chat_input);
                lan.send_chat(config.callsign.clone(), text, time.elapsed_seconds());
                lan.chat_open = false;
            }
            Key::Escape if lan.chat_open => lan.chat_open = false,
            Key::Backspace if lan.chat_open => {
                lan.chat_input.pop();
            }
            Key::Space if lan.chat_open => {
                if lan.chat_input.chars().count() < 160 {
                    lan.chat_input.push(' ');
                }
            }
            Key::PageUp if lan.chat_open => {
                lan.chat_scroll = (lan.chat_scroll + 6).min(lan.chat.len().saturating_sub(18))
            }
            Key::PageDown if lan.chat_open => lan.chat_scroll = lan.chat_scroll.saturating_sub(6),
            Key::Character(text) if lan.chat_open && !control => {
                for ch in text.chars().filter(|c| !c.is_control()) {
                    if lan.chat_input.chars().count() < 160 {
                        lan.chat_input.push(ch);
                    }
                }
            }
            _ => {}
        }
    }
    if was_open || lan.chat_open {
        for event in wheel.read() {
            if event.y > 0.0 {
                lan.chat_scroll = (lan.chat_scroll + 3).min(lan.chat.len().saturating_sub(18));
            } else if event.y < 0.0 {
                lan.chat_scroll = lan.chat_scroll.saturating_sub(3);
            }
        }
        keys.reset_all();
        mouse.reset_all();
    }
}

pub fn update_chat_panel(
    lan: Res<LanMultiplayer>,
    mut panels: Query<&mut Style, With<ChatPanel>>,
    mut texts: Query<&mut Text, With<ChatText>>,
) {
    let visible = lan.is_active && (lan.chat_open || !lan.chat.is_empty());
    for mut panel in &mut panels {
        panel.display = if visible {
            Display::Flex
        } else {
            Display::None
        };
    }
    if !visible {
        return;
    }
    let count = if lan.chat_open { 18 } else { 4 };
    let end = lan
        .chat
        .len()
        .saturating_sub(if lan.chat_open { lan.chat_scroll } else { 0 });
    let lines: Vec<_> = lan
        .chat
        .iter()
        .take(end)
        .skip(end.saturating_sub(count))
        .map(|line| format!("{}  {}", line.name, line.text))
        .collect();
    let mut content = lines.join("\n");
    if lan.chat_open {
        content.push_str(&moon_base_localize::localized_format!(
            "\n> {}|\nEnter — wyślij · rolka / PgUp / PgDn — historia · Esc — zamknij",
            lan.chat_input
        ));
    }
    for mut text in &mut texts {
        text.sections[0].value = content.clone();
    }
}

fn toggle_fullscreen(window: &mut Window, settings: &mut GameSettings) {
    if window.mode == WindowMode::Windowed {
        settings.windowed_placement = Some((window.resolution.clone(), window.position));
        window.mode = WindowMode::BorderlessFullscreen;
    } else {
        window.mode = WindowMode::Windowed;
        if let Some((resolution, position)) = settings.windowed_placement.take() {
            window.resolution = resolution;
            window.position = position;
        }
    }
}

pub fn fullscreen_shortcut(
    keys: Res<ButtonInput<KeyCode>>,
    mut settings: ResMut<GameSettings>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    if keys.just_pressed(KeyCode::F11) {
        if let Ok(mut window) = windows.get_single_mut() {
            toggle_fullscreen(&mut window, &mut settings);
        }
    }
}

#[derive(Resource, Default)]
pub struct MenuNavigation {
    return_to_pause: bool,
    pub file_status: String,
}

pub fn update_responsive_ui(
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut scale: ResMut<UiScale>,
) {
    if let Ok(window) = windows.get_single() {
        scale.0 = (window.width() / 1440.0)
            .min(window.height() / 900.0)
            .clamp(0.45, 1.5);
    }
}

pub fn update_menu_navigation(
    keys: Res<ButtonInput<KeyCode>>,
    state: Res<State<AppState>>,
    navigation: Res<MenuNavigation>,
    mut next: ResMut<NextState<AppState>>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        match state.get() {
            AppState::NasaApod => next.set(AppState::MainMenu),
            AppState::PlanetAtlas => next.set(AppState::LunarSurface),
            AppState::Discovery => next.set(AppState::LunarSurface),
            AppState::CampaignBriefing => next.set(AppState::MainMenu),
            AppState::Paused => next.set(AppState::LunarSurface),
            AppState::Settings | AppState::Controls => next.set(if navigation.return_to_pause {
                AppState::Paused
            } else {
                AppState::MainMenu
            }),
            _ => {}
        }
    }
}

#[cfg(test)]
mod navigation_tests {
    use super::*;

    #[test]
    fn settings_toggle_fullscreen_and_return_to_pause_without_ending_mission() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_state::<AppState>()
            .init_resource::<MissionConfig>()
            .init_resource::<GameSettings>()
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<LanMultiplayer>()
            .init_resource::<LunarMap>()
            .init_resource::<TouchAndUiInput>()
            .init_resource::<MenuNavigation>()
            .init_resource::<Expedition>()
            .init_resource::<CampaignLaunch>()
            .insert_resource(GameAudio { tx: None })
            .add_event::<AppExit>()
            .add_systems(
                Update,
                (fullscreen_shortcut, handle_ui_button_clicks).chain(),
            );
        app.world
            .resource_mut::<NextState<AppState>>()
            .set(AppState::Paused);
        app.update();
        let button = app
            .world
            .spawn((
                Button,
                Interaction::Pressed,
                UiAction::GotoState(AppState::Settings),
            ))
            .id();
        app.update();
        app.update();
        assert_eq!(
            *app.world.resource::<State<AppState>>().get(),
            AppState::Settings
        );
        app.world.entity_mut(button).despawn();
        let window = app.world.spawn((Window::default(), PrimaryWindow)).id();
        let original_size = app.world.get::<Window>(window).unwrap().resolution.clone();
        let fullscreen = app
            .world
            .spawn((Button, Interaction::Pressed, UiAction::ToggleFullscreen))
            .id();
        app.update();
        assert_eq!(
            app.world.get::<Window>(window).unwrap().mode,
            WindowMode::BorderlessFullscreen
        );
        app.world
            .get_mut::<Window>(window)
            .unwrap()
            .resolution
            .set(1920.0, 1080.0);
        app.world.entity_mut(fullscreen).insert(Interaction::None);
        app.update();
        app.world
            .entity_mut(fullscreen)
            .insert(Interaction::Pressed);
        app.update();
        assert_eq!(
            app.world.get::<Window>(window).unwrap().mode,
            WindowMode::Windowed
        );
        assert_eq!(
            app.world.get::<Window>(window).unwrap().resolution,
            original_size
        );
        app.world.entity_mut(fullscreen).despawn();
        app.world
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::F11);
        app.update();
        assert_eq!(
            app.world.get::<Window>(window).unwrap().mode,
            WindowMode::BorderlessFullscreen
        );
        app.world.resource_mut::<ButtonInput<KeyCode>>().reset_all();
        app.update();
        app.world
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::F11);
        app.update();
        assert_eq!(
            app.world.get::<Window>(window).unwrap().mode,
            WindowMode::Windowed
        );
        app.world.resource_mut::<ButtonInput<KeyCode>>().reset_all();
        app.world
            .spawn((Button, Interaction::Pressed, UiAction::Back));
        app.update();
        app.update();
        assert_eq!(
            *app.world.resource::<State<AppState>>().get(),
            AppState::Paused
        );
    }

    #[test]
    fn escape_resumes_paused_game() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_state::<AppState>()
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<MenuNavigation>()
            .add_systems(Update, update_menu_navigation);
        app.world
            .resource_mut::<NextState<AppState>>()
            .set(AppState::Paused);
        app.update();
        app.world
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Escape);
        app.update();
        app.update();
        assert_eq!(
            *app.world.resource::<State<AppState>>().get(),
            AppState::LunarSurface
        );
    }
}

#[derive(Component, Clone, Copy, Debug)]
pub struct ButtonVisualStyle {
    pub normal_bg: Color,
    pub hover_bg: Color,
    pub press_bg: Color,
    pub border_col: Color,
}

impl ButtonVisualStyle {
    pub const fn new(
        normal_bg: Color,
        hover_bg: Color,
        press_bg: Color,
        border_col: Color,
    ) -> Self {
        Self {
            normal_bg,
            hover_bg,
            press_bg,
            border_col,
        }
    }
}

// Restrained mission-control palette; bright accents indicate actionable controls.
pub const STYLE_EMERALD: ButtonVisualStyle = ButtonVisualStyle::new(
    Color::rgb(0.07, 0.29, 0.27),
    Color::rgb(0.10, 0.40, 0.36),
    Color::rgb(0.12, 0.48, 0.41),
    Color::rgb(0.35, 0.77, 0.66),
);
pub const STYLE_AMBER: ButtonVisualStyle = ButtonVisualStyle::new(
    Color::rgb(0.20, 0.17, 0.12),
    Color::rgb(0.31, 0.25, 0.15),
    Color::rgb(0.39, 0.30, 0.16),
    Color::rgb(0.65, 0.51, 0.29),
);
pub const STYLE_BLUEPRINT: ButtonVisualStyle = ButtonVisualStyle::new(
    Color::rgb(0.07, 0.12, 0.17),
    Color::rgb(0.12, 0.21, 0.27),
    Color::rgb(0.14, 0.29, 0.34),
    Color::rgb(0.24, 0.38, 0.45),
);
pub const STYLE_HUSARIA: ButtonVisualStyle = STYLE_BLUEPRINT;
pub const STYLE_VITAL: ButtonVisualStyle = STYLE_EMERALD;
pub const STYLE_SLATE: ButtonVisualStyle = ButtonVisualStyle::new(
    Color::rgb(0.07, 0.10, 0.14),
    Color::rgb(0.14, 0.18, 0.23),
    Color::rgb(0.18, 0.24, 0.29),
    Color::rgb(0.24, 0.30, 0.36),
);
pub const STYLE_DANGER: ButtonVisualStyle = ButtonVisualStyle::new(
    Color::rgb(0.18, 0.09, 0.11),
    Color::rgb(0.29, 0.13, 0.15),
    Color::rgb(0.39, 0.16, 0.18),
    Color::rgb(0.50, 0.28, 0.29),
);

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct HotbarSlotButton(pub Option<usize>);

#[derive(Component, Clone, Debug)]
pub enum UiAction {
    ToggleLanguage,
    EditAstronautName,
    OpenControls,
    CycleFps,
    ToggleSandboxQuests,
    GotoState(AppState),
    Back,
    QuickLaunch,
    CampaignAction,
    TravelTo(WorldBody),
    MapMark(IVec2),
    RespawnInLander,
    QuitApp,
    // Game Mode & Character Lab actions
    MdlApplyPreset(usize),
    MdlCycle(MdlOption, i32),
    MdlToggleInst(SciInstrument),
    MdlNegotiateDilemma,
    MdlSaveJson,
    MdlLoadJson,
    MdlLaunchIfReady,
    // Settings & LAN actions
    AdjustMasterVol(f32),
    AdjustMusicVol(f32),
    AdjustSfxVol(f32),
    AdjustZoom(f32),
    ToggleCornerRadar,
    ToggleHorizonPlanets,
    ToggleTouchControls,
    ToggleFullscreen,
    ToggleChat,
    ToggleAiCrew,
    StartLanHost,
    #[cfg(windows)]
    AllowLanInFirewall,
    JoinLanClient,
    EditLanAddress,
    // Surface HUD, Modals & Waypoints actions
    ToggleInventoryModal,
    ToggleQuestPanel,
    ToggleCustomizerModal,
    ToggleMapModal,
    ToggleProgressionModal,
    SurfaceCmd(SurfaceUiCmd),
    TouchMove(Vec2),
    TouchJetpack,
}

#[derive(Component)]
pub struct BarFillKind(pub BarMetric);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BarMetric {
    MdlMass,
    MdlPower,
    MdlBudget,
    MdlComms,
    HudHealth,
    HudO2,
    HudFood,
    HudWater,
    HudBattery,
    HudSuit,
    HudMiningProgress,
}

#[derive(Component)]
pub struct DynamicLabel(pub LabelId);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LabelId {
    SettingsName,
    SettingsFps,
    SandboxQuests,
    MdlScenario,
    MdlScenarioDesc,
    MdlDifficulty,
    MdlTeamSeat,
    MdlLaunch,
    MdlOrbit,
    MdlPower,
    MdlComms,
    MdlRover,
    MdlLifetime,
    MdlCallsign,
    MdlSuit,
    MdlVisor,
    MdlAccessory,
    MdlTrail,
    MdlPatch,
    MdlBackpack,
    MdlTrait,
    MdlMetricsSummary,
    MdlStatusBanner,
    MdlInst(usize),
    SettingsSummary,
    SettingsFullscreen,
    HudTopMission,
    HudQuestInstruction,
    HudQuestToggle,
    HudTopEvent,
    HudResourcePill(usize),
    HudVitalsHeader,
    HudVitalValue(BarMetric),
    HudOxygenHint,
    HudFoodAction,
    HudWaterAction,
    HudMakeWater,
    HudMakeMeal,
    HudToolAndRover,
    HudBigSelectionTitle,
    HudBigTileAndMiningInfo,
    InvSlotCount(usize),
    CargoWeight,
    BuildSlotBtn(usize),
    PickaxeSlotBtn,
    ProgModalHeader,
    ProgStepRow(usize),
    AchievementRow(usize),
}

#[derive(Component)]
pub struct InventoryModalNode;

#[derive(Component)]
pub struct QuestDetails;

#[derive(Component)]
pub struct QuestPanel;

#[derive(Component)]
pub struct CustomizerModalNode;

#[derive(Component)]
pub struct MapModalNode;

#[derive(Component)]
pub struct ProgressionModalNode;

#[derive(Component)]
pub struct TouchControlsNode;

#[derive(Component)]
pub struct BuildingDock;

#[derive(Component)]
pub struct SupplyControls;

const MAP_RADIUS: i32 = 8;

#[derive(Component)]
pub struct MapSquare(IVec2);

fn spawn_map_grid(parent: &mut ChildBuilder) {
    parent
        .spawn(NodeBundle {
            style: Style {
                flex_direction: FlexDirection::Column,
                margin: UiRect::vertical(Val::Px(12.0)),
                ..default()
            },
            ..default()
        })
        .with_children(|grid| {
            for y in -MAP_RADIUS..=MAP_RADIUS {
                grid.spawn(NodeBundle::default()).with_children(|row| {
                    for x in -MAP_RADIUS..=MAP_RADIUS {
                        let offset = IVec2::new(x, y);
                        row.spawn((
                            MapSquare(offset),
                            UiAction::MapMark(offset),
                            ButtonBundle {
                                style: Style {
                                    width: Val::Px(18.0),
                                    height: Val::Px(18.0),
                                    margin: UiRect::all(Val::Px(0.5)),
                                    ..default()
                                },
                                ..default()
                            },
                        ));
                    }
                });
            }
        });
}

pub fn update_map_grid(
    map: Res<LunarMap>,
    stats: Res<SurvivalStats>,
    player: Query<&AstronautPlayer>,
    mut squares: Query<(&MapSquare, &Interaction, &mut BackgroundColor)>,
) {
    if !map.show_map_window {
        return;
    }
    let Ok(player) = player.get_single() else {
        return;
    };
    let center = player.iso_pos.truncate().round().as_ivec2();
    let station = map.station_position.truncate().round().as_ivec2();
    let surveys = map.survey_points();
    let target = stats
        .campaign
        .as_ref()
        .and_then(|c| campaign::current_target(c.quest, &map))
        .map(|p| p.round().as_ivec2());
    for (square, interaction, mut background) in &mut squares {
        let point = center + square.0;
        let cell = map
            .cells
            .get(&(point.x, point.y))
            .filter(|cell| cell.scanned);
        let mut color = if point == center {
            Color::WHITE
        } else if point == station {
            Color::rgb(0.27, 0.91, 0.84)
        } else if target == Some(point) {
            Color::rgb(1.0, 0.79, 0.32)
        } else if surveys
            .iter()
            .enumerate()
            .any(|(i, survey)| survey.pos == point && map.surveyed & (1 << i) == 0)
        {
            Color::rgb(0.72, 0.52, 1.0)
        } else if map
            .waypoints
            .iter()
            .any(|wp| (wp.gx, wp.gy) == (point.x, point.y))
        {
            Color::rgb(1.0, 0.75, 0.27)
        } else if let Some(cell) = cell {
            if cell.structure.is_some() {
                Color::rgb(0.55, 0.67, 0.86)
            } else {
                match cell.kind {
                    crate::types::TileKind::SubsurfaceIceVein => Color::rgb(0.24, 0.53, 0.69),
                    crate::types::TileKind::CraterFloor => Color::rgb(0.14, 0.17, 0.23),
                    _ => Color::rgb(
                        0.31 + cell.elevation * 0.1,
                        0.36 + cell.elevation * 0.1,
                        0.44 + cell.elevation * 0.1,
                    ),
                }
            }
        } else {
            Color::rgb(0.05, 0.07, 0.11)
        };
        if cell.is_some() && *interaction != Interaction::None {
            color = Color::rgb(
                color.r() * 0.55 + 0.45,
                color.g() * 0.55 + 0.45,
                color.b() * 0.55 + 0.45,
            );
        }
        *background = color.into();
    }
}

pub fn update_action_buttons(
    lan: Option<Res<LanMultiplayer>>,
    time: Res<Time>,
    map: Res<LunarMap>,
    stats: Res<SurvivalStats>,
    player: Query<&AstronautPlayer>,
    mut buttons: Query<(&UiAction, &Children, &mut Style), With<Button>>,
    mut supplies: Query<&mut Style, (With<SupplyControls>, Without<Button>)>,
    mut labels: Query<&mut Text>,
) {
    let Ok(player) = player.get_single() else {
        return;
    };
    for mut style in &mut supplies {
        let tile = player.iso_pos.truncate().round().as_ivec2();
        style.display = if player.inside_building
            && player.airlock_anim_timer <= 0.0
            && !map.piloting_station
            && map.networks.contains_key(&(tile.x, tile.y))
        {
            Display::Flex
        } else {
            Display::None
        };
    }
    let nearby = player
        .iso_pos
        .truncate()
        .distance(map.station_position.truncate())
        <= 3.0;
    for (action, children, mut style) in &mut buttons {
        match action {
            UiAction::CampaignAction => {
                let title = stats
                    .campaign
                    .as_ref()
                    .and_then(|c| campaign::available_action(c, &map, player));
                style.display = if title.is_some() {
                    Display::Flex
                } else {
                    Display::None
                };
                if let (Some(title), Some(&child)) = (title, children.first()) {
                    if let Ok(mut text) = labels.get_mut(child) {
                        if text.sections[0].value != title {
                            text.sections[0].value = title.into();
                        }
                    }
                }
            }
            UiAction::SurfaceCmd(SurfaceUiCmd::GrantCargo(..)) => {
                style.display = if stats.creative_cargo {
                    Display::Flex
                } else {
                    Display::None
                };
            }
            UiAction::SurfaceCmd(SurfaceUiCmd::ThrowSurveyFlare) => {
                if let Some(&child) = children.first() {
                    if let Ok(mut text) = labels.get_mut(child) {
                        let left = (stats.flare_ready_at - time.elapsed_seconds()).max(0.0);
                        text.sections[0].value = if left > 0.0 {
                            moon_base_localize::localized_format!("Flara · poczekaj {:.1} s", left)
                        } else {
                            crate::locale::text("Rzuć flarę · co 6 s").into()
                        };
                    }
                }
            }
            UiAction::SurfaceCmd(SurfaceUiCmd::SetDemolitionMode) => {
                style.display = if map.piloting_station {
                    Display::None
                } else {
                    Display::Flex
                };
                if let Some(&child) = children.first() {
                    if let Ok(mut text) = labels.get_mut(child) {
                        text.sections[0].value = if map.demolition_mode {
                            crate::locale::text("Rozbiórka ✓")
                        } else {
                            crate::locale::text("Rozbiórka")
                        }
                        .into();
                    }
                }
            }
            UiAction::SurfaceCmd(SurfaceUiCmd::PilotStation) => {
                style.display =
                    if map.piloting_station || (nearby && player.mounted_rover.is_none()) {
                        Display::Flex
                    } else {
                        Display::None
                    };
                if let Some(&child) = children.first() {
                    if let Ok(mut label) = labels.get_mut(child) {
                        let message = if lan.as_ref().is_some_and(|l| l.is_active && !l.is_host) {
                            crate::locale::text("Podróżuj [G / N]")
                        } else if map.piloting_station {
                            crate::locale::text("Wysiądź [G]")
                        } else {
                            crate::locale::text("Pilotuj stację [G]")
                        };
                        if label.sections[0].value != message {
                            label.sections[0].value = message.into();
                        }
                    }
                }
            }
            UiAction::SurfaceCmd(
                SurfaceUiCmd::EnterOrExitBuilding | SurfaceUiCmd::SetBuildMode(true),
            ) => {
                style.display = if map.piloting_station {
                    Display::None
                } else {
                    Display::Flex
                };
            }
            UiAction::SurfaceCmd(SurfaceUiCmd::PatchSuit) => {
                style.display = if map.campaign_route
                    && player.inside_building
                    && !map.piloting_station
                    && stats.local_powered
                    && stats.suit_integrity < 99.0
                {
                    Display::Flex
                } else {
                    Display::None
                };
            }
            _ => {}
        }
    }
}

#[derive(Component)]
pub struct SuitPreviewSwatch;

#[derive(Component)]
pub struct VisorPreviewSwatch;

#[derive(Component)]
pub struct LaunchSeqText;

#[derive(Resource, Default)]
pub struct LaunchTimer {
    pub elapsed: f32,
}

const BG_DARK: Color = Color::rgba(0.025, 0.038, 0.055, 0.95);
const CARD_BG: Color = Color::rgba(0.035, 0.055, 0.075, 0.96);
const CARD_BORDER: Color = Color::rgba(0.30, 0.44, 0.50, 0.55);

pub const PROGRESSION_STEPS_META: [(&str, &str); 8] = [
    (
        "KROK 1: Wydobądź pierwsze surowce Kilofem [Q/B] (min. 2 bloki)",
        "Wybierz Kilof [Q/B], najedź kursorem na skałę lub niebieski Lód H2O i przytrzymaj LPM aż pasek kopania dojdzie do 100%.",
    ),
    (
        "KROK 2: Wejdź przez Śluzę Powietrzną do wnętrza bazy klawiszem [E]",
        "Podejdź do Śluzy Powietrznej lub Habitatu (np. w odkrytej bazie) i naciśnij [E], aby uruchomić animację śluzy i wejść do środka!",
    ),
    (
        "KROK 3: Zbuduj własny Habitat [2] lub Śluzę Powietrzną [1]",
        "Wybierz Śluzę [1] lub Habitat [2] na dolnym pasku i kliknij LPM na wolnym polu, aby postawić własny dom na Księżycu.",
    ),
    (
        "KROK 4: Zbuduj zasilanie (Panel [3] / RTG [4]) oraz Elektrolizer Tlenu [5]",
        "Zapewnij swojej bazie prąd oraz automatyczną produkcję tlenu O2 ze zbieranego lodu wodnego.",
    ),
    (
        "KROK 5: Ulepsz narzędzie górnicze w warsztacie klawiszem [C]",
        "Zbierz Tytan oraz Krzem i naciśnij [C] (lub użyj przycisku w Plecaku [I]), aby stworzyć znacznie szybsze wiertło.",
    ),
    (
        "KROK 6: Otwórz Mapę [M], postaw Waypoint lub zasiedl strukturę w świecie",
        "Naciśnij [M] aby postawić własny punkt orientacyjny (Waypoint) lub wejdź [E] do bazy napotkanej w otwartym świecie!",
    ),
    (
        "KROK 7: Wsiądź do Łazika Księżycowego LRV klawiszem [E]",
        "Podejdź do łazika i naciśnij E. W lekkim łaziku tlen zużywa się wolniej.",
    ),
    (
        "KROK 8: Samowystarczalna Kolonia (7 modułów bazy i Przecinak Plazmowy)",
        "Rozbuduj połączone moduły swojej bazy księżycowej i ulepsz narzędzie do poziomu Przecinaka Plazmowego [C]!",
    ),
];

pub const ACHIEVEMENTS_META: [(&str, &str); 8] = [
    (
        "1. Pierwszy Urobek",
        "Wykop przynajmniej 10 bloków surowców z widocznym paskiem postępu kopania.",
    ),
    (
        "2. Dom na Księżycu",
        "Wejdź do wnętrza Habitatu lub Śluzy Powietrznej klawiszem [E] (animacja dekompresji).",
    ),
    (
        "3. Mistrz Wiertła",
        "Ulepsz swoje narzędzie górnicze [C], aby kopać skały znacznie szybciej.",
    ),
    (
        "4. Architekt Modułowy",
        "Połącz ze sobą sąsiadujące budynki (min. 4 moduły bazy) w jedną strukturę.",
    ),
    (
        "5. Kartograf i Osadnik",
        "Odkryj strukturę w otwartym świecie (np. Baza Kopernik-0) lub dodaj Waypoint na Mapie [M].",
    ),
    (
        "6. Polacy w Kosmosie",
        "Zatkaj biało-czerwoną flagę [H] lub załóż Skrzydła Husarskie w Kreatorze Postaci [K].",
    ),
    (
        "7. Kierowca LRV Bekkera",
        "Wsiądź do łazika klawiszem E.",
    ),
    (
        "8. Łowca Helu-3",
        "Wydobądź przynajmniej 3 jednostki rzadkiego izotopu Hel-3 (3He) do reaktorów fuzyjnych.",
    ),
];

pub fn cleanup_ui_root(mut commands: Commands, q: Query<Entity, With<ScreenUiRoot>>) {
    for ent in &q {
        commands.entity(ent).despawn_recursive();
    }
}

fn spawn_styled_button(
    parent: &mut ChildBuilder,
    label: &str,
    action: UiAction,
    width: Val,
    height: Val,
    style_theme: ButtonVisualStyle,
    font_size: f32,
) {
    let dynamic = match action {
        UiAction::ToggleFullscreen => Some(LabelId::SettingsFullscreen),
        UiAction::EditAstronautName => Some(LabelId::SettingsName),
        UiAction::CycleFps => Some(LabelId::SettingsFps),
        UiAction::ToggleSandboxQuests => Some(LabelId::SandboxQuests),
        _ => None,
    };
    parent
        .spawn((
            ButtonBundle {
                style: Style {
                    width,
                    height,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    padding: UiRect::axes(Val::Px(10.0), Val::Px(4.0)),
                    margin: UiRect::all(Val::Px(3.0)),
                    border: UiRect::all(Val::Px(1.0)),
                    flex_shrink: 0.0,
                    ..default()
                },
                background_color: style_theme.normal_bg.into(),
                border_color: style_theme.border_col.into(),
                ..default()
            },
            style_theme,
            action,
        ))
        .with_children(|b| {
            let mut text = b.spawn(
                TextBundle::from_section(
                    label,
                    TextStyle {
                        font_size,
                        color: Color::rgb(0.97, 0.99, 1.0),
                        ..default()
                    },
                )
                .with_text_justify(JustifyText::Center),
            );
            if let Some(label) = dynamic {
                text.insert(DynamicLabel(label));
            }
        });
}

#[cfg(windows)]
pub(crate) fn spawn_lan_firewall_control(parent: &mut ChildBuilder) {
    spawn_styled_button(
        parent,
        crate::locale::text("Zezwól na LAN"),
        UiAction::AllowLanInFirewall,
        Val::Px(220.0),
        Val::Px(36.0),
        STYLE_BLUEPRINT,
        15.0,
    );
    parent.spawn((
        crate::firewall::FirewallStatus,
        TextBundle::from_section(
            "",
            TextStyle {
                font_size: 13.0,
                color: Color::rgb(0.77, 0.88, 0.94),
                ..default()
            },
        ),
    ));
}

fn spawn_labeled_progress_bar(
    parent: &mut ChildBuilder,
    title: &str,
    metric: BarMetric,
    fill_color: Color,
) {
    parent
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                margin: UiRect::vertical(Val::Px(2.5)),
                ..default()
            },
            ..default()
        })
        .with_children(|col| {
            col.spawn(TextBundle::from_section(
                title,
                TextStyle {
                    font_size: 12.0,
                    color: Color::rgb(0.86, 0.94, 1.0),
                    ..default()
                },
            ));
            col.spawn(NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Px(12.0),
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                background_color: Color::rgb(0.03, 0.06, 0.12).into(),
                border_color: Color::rgba(0.32, 0.64, 0.90, 0.55).into(),
                ..default()
            })
            .with_children(|bar_bg| {
                bar_bg.spawn((
                    BarFillKind(metric),
                    NodeBundle {
                        style: Style {
                            width: Val::Percent(70.0),
                            height: Val::Percent(100.0),
                            ..default()
                        },
                        background_color: fill_color.into(),
                        ..default()
                    },
                ));
            });
        });
}

const VITALS: [(BarMetric, &str, Color); 6] = [
    (BarMetric::HudO2, "Tlen", Color::rgb(0.33, 0.81, 0.98)),
    (
        BarMetric::HudHealth,
        "Zdrowie",
        Color::rgb(0.97, 0.51, 0.58),
    ),
    (BarMetric::HudFood, "Sytość", Color::rgb(0.58, 0.85, 0.46)),
    (
        BarMetric::HudWater,
        "Nawodnienie",
        Color::rgb(0.46, 0.77, 1.0),
    ),
    (
        BarMetric::HudBattery,
        "Bateria",
        Color::rgb(0.99, 0.76, 0.41),
    ),
    (
        BarMetric::HudSuit,
        "Szczelność",
        Color::rgb(0.73, 0.65, 0.97),
    ),
];

fn vital_percent(metric: BarMetric, stats: &SurvivalStats) -> Option<f32> {
    Some(
        match metric {
            BarMetric::HudO2 => stats.oxygen / stats.max_oxygen * 100.0,
            BarMetric::HudHealth => stats.health,
            BarMetric::HudFood => stats.hunger,
            BarMetric::HudWater => stats.hydration,
            BarMetric::HudBattery => stats.suit_power / stats.max_suit_power * 100.0,
            BarMetric::HudSuit => stats.suit_integrity,
            _ => return None,
        }
        .clamp(0.0, 100.0),
    )
}

fn vital_color(metric: BarMetric, percent: f32) -> Color {
    if percent < 15.0 {
        return Color::rgb(1.0, 0.41, 0.42);
    }
    if percent < 30.0 {
        return Color::rgb(1.0, 0.73, 0.34);
    }
    VITALS
        .iter()
        .find(|(kind, _, _)| *kind == metric)
        .map_or(Color::WHITE, |item| item.2)
}

fn spawn_vital_row(parent: &mut ChildBuilder, metric: BarMetric, title: &str, color: Color) {
    parent
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)),
                ..default()
            },
            background_color: Color::rgb(0.066, 0.10, 0.151).into(),
            ..default()
        })
        .with_children(|row| {
            row.spawn(NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    ..default()
                },
                ..default()
            })
            .with_children(|header| {
                header.spawn(TextBundle::from_section(
                    title,
                    TextStyle {
                        font_size: 14.0,
                        color: Color::rgb(0.91, 0.94, 0.98),
                        ..default()
                    },
                ));
                header.spawn((
                    DynamicLabel(LabelId::HudVitalValue(metric)),
                    TextBundle::from_section(
                        "100%",
                        TextStyle {
                            font_size: 14.0,
                            color,
                            ..default()
                        },
                    ),
                ));
            });
            row.spawn(NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Px(if metric == BarMetric::HudO2 {
                        10.0
                    } else {
                        6.0
                    }),
                    ..default()
                },
                background_color: Color::rgb(0.023, 0.042, 0.074).into(),
                ..default()
            })
            .with_children(|track| {
                track.spawn((
                    BarFillKind(metric),
                    NodeBundle {
                        style: Style {
                            width: Val::Percent(100.0),
                            height: Val::Percent(100.0),
                            ..default()
                        },
                        background_color: color.into(),
                        ..default()
                    },
                ));
            });
        });
}

fn spawn_consumable_button(
    parent: &mut ChildBuilder,
    title: &str,
    command: SurfaceUiCmd,
    label: LabelId,
    theme: ButtonVisualStyle,
) {
    parent
        .spawn((
            theme,
            UiAction::SurfaceCmd(command),
            ButtonBundle {
                style: Style {
                    flex_grow: 1.0,
                    flex_basis: Val::Px(0.0),
                    min_height: Val::Px(34.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                background_color: theme.normal_bg.into(),
                border_color: theme.border_col.into(),
                ..default()
            },
        ))
        .with_children(|button| {
            button.spawn((
                DynamicLabel(label),
                TextBundle::from_section(
                    title,
                    TextStyle {
                        font_size: 13.0,
                        color: Color::WHITE,
                        ..default()
                    },
                )
                .with_text_justify(JustifyText::Center),
            ));
        });
}

// ============================================================================
// GLOBAL INTERACTIVE BUTTON SYSTEM (Preserves custom per-button color themes!)
// ============================================================================

pub fn handle_ui_buttons(
    mut interaction_q: Query<
        (
            &Interaction,
            &UiAction,
            Option<&ButtonVisualStyle>,
            &mut BackgroundColor,
            &mut BorderColor,
        ),
        With<Button>,
    >,
    mut touch_ui: ResMut<TouchAndUiInput>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    blockers: Query<
        (&Node, &GlobalTransform, &ViewVisibility),
        Or<(With<WorldInputBlocker>, With<Button>)>,
    >,
    scale: Res<UiScale>,
) {
    touch_ui.move_dir = Vec2::ZERO;
    touch_ui.jetpack_held = false;
    touch_ui.action_held = false;
    touch_ui.pointer_over_ui = windows
        .get_single()
        .ok()
        .and_then(Window::cursor_position)
        .map(|cursor| {
            blockers.iter().any(|(node, transform, visible)| {
                visible.get() && node.logical_rect(transform).contains(cursor / scale.0)
            })
        })
        .unwrap_or(false);

    for (interaction, action, opt_style, mut bg, mut border) in &mut interaction_q {
        let style = opt_style.copied().unwrap_or(STYLE_BLUEPRINT);
        match *interaction {
            Interaction::Pressed => {
                *bg = style.press_bg.into();
                *border = Color::WHITE.into();

                match action {
                    UiAction::TouchMove(dir) => {
                        touch_ui.move_dir = *dir;
                    }
                    UiAction::TouchJetpack => {
                        touch_ui.jetpack_held = true;
                    }
                    UiAction::SurfaceCmd(SurfaceUiCmd::MineOrBuildAtPlayer) => {
                        touch_ui.action_held = true;
                    }
                    _ => {}
                }
            }
            Interaction::Hovered => {
                *bg = style.hover_bg.into();
                *border = Color::rgb(0.95, 0.98, 1.0).into();
            }
            Interaction::None => {
                *bg = style.normal_bg.into();
                *border = style.border_col.into();
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn handle_ui_button_clicks(
    interaction_q: Query<(&Interaction, &UiAction), (Changed<Interaction>, With<Button>)>,
    mut config: ResMut<MissionConfig>,
    mut settings: ResMut<GameSettings>,
    mut lan: ResMut<LanMultiplayer>,
    mut map: ResMut<LunarMap>,
    mut stats: Option<ResMut<SurvivalStats>>,
    mut player_q: Query<(&mut AstronautPlayer, &mut Transform, &mut Visibility)>,
    mut touch_ui: ResMut<TouchAndUiInput>,
    audio: Res<GameAudio>,
    (mut next_state, mut exit_ev): (ResMut<NextState<AppState>>, EventWriter<AppExit>),
    state: Res<State<AppState>>,
    (mut navigation, mut name_edit, mut controls_menu): (
        ResMut<MenuNavigation>,
        Option<ResMut<SettingsEdit>>,
        Option<ResMut<crate::controls::ControlsMenu>>,
    ),
    mut expedition: ResMut<Expedition>,
    mut campaign_launch: ResMut<CampaignLaunch>,
    mut main_window: Query<&mut Window, With<PrimaryWindow>>,
    #[cfg(windows)] mut firewall: Option<ResMut<crate::firewall::FirewallAccess>>,
) {
    let sfx_vol = settings.sfx_volume * settings.master_volume;

    for (interaction, action) in &interaction_q {
        if *interaction != Interaction::Pressed {
            continue;
        }
        if !matches!(action, UiAction::EditAstronautName) {
            if let Some(edit) = name_edit.as_deref_mut().filter(|edit| edit.name) {
                edit.name = false;
                settings.astronaut_name = config.callsign.trim().to_owned();
                settings.save();
            }
        }
        match action {
            UiAction::ToggleLanguage => {
                use crate::locale::Language;
                settings.language = match settings.language {
                    Language::English => Language::Polish,
                    Language::Polish => Language::English,
                };
                crate::locale::set_language(settings.language);
                settings.save();
            }
            UiAction::EditAstronautName => {
                if let Some(edit) = name_edit.as_deref_mut() {
                    edit.name = true;
                    config.callsign.clear();
                    lan.editing_address = false;
                }
            }
            UiAction::OpenControls => {
                if let Some(menu) = controls_menu.as_deref_mut() {
                    menu.open = true;
                }
            }
            UiAction::CycleFps => {
                let options = [
                    Some(30),
                    Some(60),
                    Some(90),
                    Some(120),
                    Some(144),
                    Some(165),
                    Some(240),
                    None,
                ];
                let index = options
                    .iter()
                    .position(|value| *value == settings.fps_limit)
                    .unwrap_or(0);
                settings.fps_limit = options[(index + 1) % options.len()];
                settings.save();
            }
            UiAction::ToggleSandboxQuests => {
                config.sandbox_quests = !config.sandbox_quests;
            }
            UiAction::GotoState(st) => {
                if matches!(st, AppState::Settings | AppState::Controls) {
                    navigation.return_to_pause =
                        matches!(state.get(), AppState::Paused | AppState::LunarSurface);
                }
                audio.play(SfxKind::UiClick, sfx_vol);
                next_state.set(*st);
            }
            UiAction::Back => {
                settings.save();
                next_state.set(if navigation.return_to_pause {
                    AppState::Paused
                } else {
                    AppState::MainMenu
                });
                audio.play(SfxKind::UiClick, sfx_vol);
            }
            UiAction::QuickLaunch => {
                audio.play(SfxKind::UiConfirm, sfx_vol);
                *config = MissionConfig {
                    difficulty: DifficultyLevel::Cadet,
                    callsign: config.callsign.clone(),
                    ..default()
                };
                campaign_launch.0 = !lan.is_active;
                next_state.set(AppState::LunarSurface);
            }
            UiAction::CampaignAction if lan.is_active && !lan.is_host => {
                lan.send(crate::net_lan::Message::Action(
                    crate::net_world::Action::Mission,
                ));
            }
            UiAction::CampaignAction => {
                if let Some(campaign) = stats
                    .as_deref_mut()
                    .and_then(|stats| stats.campaign.as_mut())
                {
                    campaign.action_requested = true;
                }
            }
            UiAction::TravelTo(destination) => {
                if *state.get() == AppState::PlanetAtlas
                    && (map.piloting_station
                        || (lan.is_active
                            && player_q.get_single().is_ok_and(|(p, _, _)| {
                                p.iso_pos
                                    .truncate()
                                    .distance(map.station_position.truncate())
                                    <= 4.0
                                    && p.mounted_rover.is_none()
                            })))
                    && *destination != map.body
                    && (lan.is_active
                        || campaign::travel_allowed(
                            stats.as_deref().and_then(|s| s.campaign.as_ref()),
                            *destination,
                        ))
                {
                    if map.campaign_route {
                        expedition.request_campaign(*destination);
                    } else {
                        expedition.request(*destination);
                    }
                    next_state.set(AppState::InterplanetaryFlight);
                    audio.play(SfxKind::UiConfirm, sfx_vol);
                }
            }
            UiAction::RespawnInLander => {
                if config.is_hardcore() {
                    audio.play(SfxKind::AlertSiren, sfx_vol);
                    continue;
                }
                audio.play(SfxKind::UiConfirm, sfx_vol);
                if let Some(ref mut st) = stats {
                    st.health = 100.0;
                    st.oxygen = st.max_oxygen;
                    st.oxygen_use_per_sec = 0.0;
                    st.suit_power = st.max_suit_power;
                    st.body_temp_c = 37.0;
                    st.hunger = 85.0;
                    st.hydration = 85.0;
                    st.radiation = 0.0;
                    st.suit_integrity = 100.0;
                    st.death_reason.clear();
                    st.status_log = crate::locale::text(
                        "Jesteś z powrotem przy stacji. Postęp zadań zachowany.",
                    )
                    .into();
                }
                map.piloting_station = false;
                if let Ok((mut p, mut tf, mut visibility)) = player_q.get_single_mut() {
                    let station = map.station_position.truncate();
                    let xy = [
                        Vec2::new(1.5, 1.5),
                        Vec2::new(-1.7, 0.0),
                        Vec2::new(0.0, -1.7),
                    ]
                    .into_iter()
                    .map(|offset| station + offset)
                    .find(|pos| !map.blocks_movement(pos.x, pos.y))
                    .unwrap_or(station);
                    p.iso_pos = xy.extend(map.get_elevation(xy.x, xy.y));
                    *visibility = Visibility::Inherited;
                    p.iso_vel = Vec3::ZERO;
                    p.mounted_rover = None;
                    p.inside_building = false;
                    p.airlock_anim_timer = 0.0;
                    p.airlock_entering = false;
                    p.grounded = false;
                    let w = iso_to_world(p.iso_pos.x, p.iso_pos.y, p.iso_pos.z);
                    tf.translation.x = w.x;
                    tf.translation.y = w.y + 18.0;
                }
                next_state.set(AppState::LunarSurface);
            }
            UiAction::QuitApp => {
                exit_ev.send(AppExit);
            }
            UiAction::MdlApplyPreset(preset_idx) => {
                audio.play(SfxKind::UiConfirm, sfx_vol);
                config.apply_preset(*preset_idx);
            }
            UiAction::MdlCycle(opt, dir) => {
                audio.play(SfxKind::UiClick, sfx_vol);
                config.cycle(*opt, *dir);
            }
            UiAction::MdlToggleInst(inst) => {
                audio.play(SfxKind::UiClick, sfx_vol);
                config.toggle_instrument(*inst);
            }
            UiAction::MdlNegotiateDilemma => {
                audio.play(SfxKind::UiConfirm, sfx_vol);
                config.negotiate_dilemma();
            }
            UiAction::MdlSaveJson => {
                navigation.file_status = match serde_json::to_string_pretty(&*config) {
                    Ok(json) => match fs::write("mission_design.json", json) {
                        Ok(()) => {
                            crate::locale::text("Zapisano projekt misji: mission_design.json")
                                .to_string()
                        }
                        Err(err) => moon_base_localize::localized_format!(
                            "Nie udało się zapisać projektu: {err}"
                        ),
                    },
                    Err(err) => moon_base_localize::localized_format!(
                        "Nie udało się zapisać projektu: {err}"
                    ),
                };
                audio.play(SfxKind::UiClick, sfx_vol);
            }
            UiAction::MdlLoadJson => {
                navigation.file_status = match fs::read_to_string("mission_design.json") {
                    Ok(raw) => match serde_json::from_str::<MissionConfig>(&raw) {
                        Ok(mut loaded) => {
                            loaded.normalize();
                            *config = loaded;
                            crate::locale::text("Wczytano projekt misji.").to_string()
                        }
                        Err(err) => moon_base_localize::localized_format!(
                            "Nieprawidłowy plik projektu: {err}"
                        ),
                    },
                    Err(err) => moon_base_localize::localized_format!(
                        "Nie udało się wczytać projektu: {err}"
                    ),
                };
                audio.play(SfxKind::UiClick, sfx_vol);
            }
            UiAction::MdlLaunchIfReady => {
                if config.validate_launch_readiness().is_ok() {
                    audio.play(SfxKind::UiConfirm, sfx_vol);
                    next_state.set(AppState::LaunchSequence);
                } else {
                    audio.play(SfxKind::AlertSiren, sfx_vol * 0.6);
                }
            }
            UiAction::AdjustMasterVol(d) => {
                settings.master_volume = (settings.master_volume + *d).clamp(0.0, 1.0);
                audio.set_music_volume(settings.music_volume * settings.master_volume);
                audio.play(
                    SfxKind::UiClick,
                    settings.sfx_volume * settings.master_volume,
                );
            }
            UiAction::AdjustMusicVol(d) => {
                settings.music_volume = (settings.music_volume + *d).clamp(0.0, 1.0);
                audio.set_music_volume(settings.music_volume * settings.master_volume);
                audio.play(SfxKind::UiClick, sfx_vol);
            }
            UiAction::AdjustSfxVol(d) => {
                settings.sfx_volume = (settings.sfx_volume + *d).clamp(0.0, 1.0);
                audio.play(
                    SfxKind::UiClick,
                    settings.sfx_volume * settings.master_volume,
                );
            }
            UiAction::AdjustZoom(d) => {
                settings.camera_zoom = (settings.camera_zoom + *d).clamp(0.68, 1.55);
                audio.play(SfxKind::UiClick, sfx_vol);
            }
            UiAction::ToggleCornerRadar => {
                settings.show_corner_radar = !settings.show_corner_radar;
                audio.play(SfxKind::UiClick, sfx_vol);
            }
            UiAction::ToggleHorizonPlanets => {
                settings.show_horizon_planets = !settings.show_horizon_planets;
                audio.play(SfxKind::UiClick, sfx_vol);
            }
            UiAction::ToggleTouchControls => {
                settings.show_touch_controls = !settings.show_touch_controls;
                audio.play(SfxKind::UiClick, sfx_vol);
            }
            UiAction::ToggleFullscreen => {
                if let Ok(mut window) = main_window.get_single_mut() {
                    toggle_fullscreen(&mut window, &mut settings);
                    audio.play(SfxKind::UiClick, sfx_vol);
                }
            }
            UiAction::ToggleChat => {
                if lan.is_active {
                    lan.chat_open = !lan.chat_open;
                }
            }
            UiAction::ToggleAiCrew => {
                settings.co_op_ai_crew = !settings.co_op_ai_crew;
                audio.play(SfxKind::UiClick, sfx_vol);
            }
            UiAction::EditLanAddress => {
                lan.editing_address = !lan.editing_address;
                if lan.editing_address {
                    lan.target_addr.clear();
                }
            }
            UiAction::StartLanHost => {
                lan.start_host();
                if lan.is_active {
                    config.difficulty = DifficultyLevel::Cadet;
                    campaign_launch.0 = true;
                    next_state.set(AppState::LunarSurface);
                } else if *state.get() == AppState::MainMenu {
                    next_state.set(AppState::Settings);
                }
                audio.play(SfxKind::UiConfirm, sfx_vol);
            }
            #[cfg(windows)]
            UiAction::AllowLanInFirewall => {
                if let Some(firewall) = &mut firewall {
                    firewall.request();
                }
            }
            UiAction::JoinLanClient => {
                config.difficulty = DifficultyLevel::Cadet;
                campaign_launch.0 = false;
                lan.join_lan();
                audio.play(SfxKind::UiConfirm, sfx_vol);
            }
            UiAction::ToggleInventoryModal => {
                map.show_inventory_window = !map.show_inventory_window;
                if map.show_inventory_window {
                    map.show_customizer_window = false;
                    map.show_map_window = false;
                    map.show_progression_window = false;
                }
                audio.play(SfxKind::UiClick, sfx_vol);
            }
            UiAction::ToggleQuestPanel => {
                map.quest_panel_collapsed = !map.quest_panel_collapsed;
                audio.play(SfxKind::UiClick, sfx_vol);
            }
            UiAction::ToggleCustomizerModal => {
                map.show_customizer_window = !map.show_customizer_window;
                if map.show_customizer_window {
                    map.show_inventory_window = false;
                    map.show_map_window = false;
                    map.show_progression_window = false;
                }
                audio.play(SfxKind::UiClick, sfx_vol);
            }
            UiAction::ToggleMapModal => {
                map.show_map_window = !map.show_map_window;
                if map.show_map_window {
                    map.show_inventory_window = false;
                    map.show_customizer_window = false;
                    map.show_progression_window = false;
                }
                audio.play(SfxKind::UiClick, sfx_vol);
            }
            UiAction::ToggleProgressionModal => {
                map.show_progression_window = !map.show_progression_window;
                if map.show_progression_window {
                    map.show_inventory_window = false;
                    map.show_customizer_window = false;
                    map.show_map_window = false;
                }
                audio.play(SfxKind::UiClick, sfx_vol);
            }
            UiAction::SurfaceCmd(cmd) => {
                touch_ui.cmds.push(*cmd);
            }
            UiAction::MapMark(offset) => {
                if let Ok((player, _, _)) = player_q.get_single() {
                    let tile = player.iso_pos.truncate().round().as_ivec2() + *offset;
                    if lan.is_active && !lan.is_host {
                        lan.send(crate::net_lan::Message::Action(
                            crate::net_world::Action::Command(SurfaceUiCmd::MarkAt(tile.x, tile.y)),
                        ));
                        continue;
                    }
                    if map
                        .cells
                        .get(&(tile.x, tile.y))
                        .is_some_and(|cell| cell.scanned)
                        && !map
                            .waypoints
                            .iter()
                            .any(|point| (point.gx, point.gy) == (tile.x, tile.y))
                    {
                        map.waypoints.push(crate::world::Waypoint {
                            name: crate::locale::text("Mój znacznik").into(),
                            gx: tile.x,
                            gy: tile.y,
                            color: Color::rgb(1.0, 0.79, 0.36),
                        });
                        if let Some(stats) = stats.as_deref_mut() {
                            stats.placed_waypoint_once = true;
                            stats.status_log =
                                crate::locale::text("Dodano znacznik na mapie.").into();
                        }
                        audio.play(SfxKind::UiConfirm, sfx_vol);
                    }
                }
            }
            UiAction::TouchMove(_) | UiAction::TouchJetpack => {}
        }
    }
}

// ============================================================================
// 1. MAIN MENU SCREEN (3D Black Hole Backdrop, Game Modes & Creators Credits)
// ============================================================================

fn menu_text(parent: &mut ChildBuilder, value: &str, size: f32, color: Color) {
    parent.spawn(
        TextBundle::from_section(
            value,
            TextStyle {
                font_size: size,
                color,
                ..default()
            },
        )
        .with_style(Style {
            margin: UiRect::bottom(Val::Px(12.0)),
            ..default()
        }),
    );
}

fn atlas_root() -> NodeBundle {
    NodeBundle {
        style: Style {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            padding: UiRect::all(Val::Px(24.0)),
            ..default()
        },
        background_color: Color::rgba(0.018, 0.032, 0.056, 0.98).into(),
        ..default()
    }
}

pub fn setup_discovery(mut commands: Commands, map: Res<LunarMap>, assets: Res<ProceduralAssets>) {
    let points = map.survey_points();
    let Some(point) = map.last_discovery.and_then(|index| points.get(index)) else {
        return;
    };
    commands
        .spawn((ScreenUiRoot, atlas_root()))
        .with_children(|root| {
            root.spawn(NodeBundle {
                style: Style {
                    width: Val::Px(780.0),
                    padding: UiRect::all(Val::Px(32.0)),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(12.0),
                    border: UiRect::top(Val::Px(4.0)),
                    ..default()
                },
                background_color: CARD_BG.into(),
                border_color: Color::rgb(0.74, 0.56, 1.0).into(),
                ..default()
            })
            .with_children(|card| {
                card.spawn(ImageBundle {
                    style: Style {
                        width: Val::Px(120.0),
                        height: Val::Px(120.0),
                        align_self: AlignSelf::Center,
                        ..default()
                    },
                    image: UiImage::new(assets.planet_portraits[map.body.portrait_index()].clone()),
                    ..default()
                });
                menu_text(
                    card,
                    &format!("ZBADANO · {}", map.body.name()),
                    17.0,
                    Color::rgb(0.74, 0.56, 1.0),
                );
                menu_text(card, point.name, 36.0, Color::WHITE);
                menu_text(card, point.fact, 23.0, Color::rgb(0.86, 0.92, 0.99));
                menu_text(
                    card,
                    &format!(
                        "Odkrycia · {}/{} · +18 GB danych",
                        map.surveyed.count_ones(),
                        points.len()
                    ),
                    17.0,
                    Color::rgb(0.51, 0.91, 0.78),
                );
                menu_text(
                    card,
                    crate::locale::text("Odkrycie znajdziesz ponownie w dzienniku [N]."),
                    16.0,
                    Color::rgb(0.67, 0.79, 0.89),
                );
                spawn_styled_button(
                    card,
                    crate::locale::text("Dalej [Esc]"),
                    UiAction::GotoState(AppState::LunarSurface),
                    Val::Percent(100.0),
                    Val::Px(46.0),
                    STYLE_EMERALD,
                    19.0,
                );
            });
        });
}

pub fn setup_campaign_briefing(mut commands: Commands) {
    commands.spawn((ScreenUiRoot,atlas_root())).with_children(|root| {
        root.spawn(NodeBundle {
            style: Style {width:Val::Px(700.0),flex_direction:FlexDirection::Column,padding:UiRect::all(Val::Px(32.0)),border:UiRect::top(Val::Px(3.0)),..default()},
            background_color:CARD_BG.into(),border_color:Color::rgb(0.38,0.88,0.76).into(),..default()
        }).with_children(|card| {
            menu_text(card,crate::locale::text("MISJA · OKOŁO 20 MINUT"),16.0,Color::rgb(0.38,0.88,0.76));
            menu_text(card,crate::locale::text("Raport dla Ziemi"),44.0,Color::WHITE);
            menu_text(card,crate::locale::text(campaign::GOAL),25.0,Color::WHITE);
            menu_text(card,crate::locale::text("1. Przygotuj wodę i jedzenie w bazie.\n2. Zbadaj Księżyc, Marsa i Wenus.\n3. Wróć do bazy na Księżycu i wyślij raport."),21.0,Color::rgb(0.82,0.89,0.97));
            menu_text(card,crate::locale::text("Złota strzałka wskaże drogę. Gra będzie pokazywać następne zadanie."),17.0,Color::rgb(0.72,0.81,0.91));
            spawn_styled_button(card,crate::locale::text("Rozpocznij misję"),UiAction::QuickLaunch,Val::Percent(100.0),Val::Px(54.0),STYLE_EMERALD,21.0);
            spawn_styled_button(card,crate::locale::text("Wróć"),UiAction::GotoState(AppState::MainMenu),Val::Percent(100.0),Val::Px(42.0),STYLE_SLATE,16.0);
        });
    });
}

pub fn setup_planet_atlas(
    player: Query<&AstronautPlayer>,
    mut commands: Commands,
    map: Res<LunarMap>,
    lan: Res<LanMultiplayer>,
    expedition: Res<Expedition>,
    assets: Res<ProceduralAssets>,
    stats: Res<SurvivalStats>,
) {
    commands.spawn((ScreenUiRoot, atlas_root())).with_children(|root| {
        menu_text(root, crate::locale::text("Dziennik odkrywcy"), 38.0, Color::WHITE);
        if let Some(mission) = &stats.campaign {
            menu_text(root, &moon_base_localize::localized_format!("Aktualne zadanie · {}",mission.quest.title()),20.0,Color::rgb(1.0,0.82,0.47));
        }
        menu_text(root, if lan.is_active { crate::locale::text("Podróże między planetami są dostępne w trybie solo.") }
            else if lan.is_active { crate::locale::text("Podejdź do stacji i wybierz własny cel. Pozostali gracze zostają na swoich planetach.") }
            else if map.piloting_station { crate::locale::text("Wybierz cel podróży. Zbudowane bazy zaczekają na twój powrót.") }
            else { crate::locale::text("Chcesz zmienić planetę? Wejdź do kokpitu stacji [G] i otwórz dziennik ponownie.") }, 18.0, Color::rgb(0.68,0.78,0.88));
        root.spawn(NodeBundle {style: Style {column_gap:Val::Px(16.0), ..default()}, ..default()})
            .with_children(|row| {
                for body in WorldBody::ALL {
                    let accent = match body { WorldBody::Moon => Color::rgb(0.64,0.83,1.0), WorldBody::Mars => Color::rgb(1.0,0.61,0.42), WorldBody::Venus => Color::rgb(1.0,0.79,0.49) };
                    row.spawn(NodeBundle {
                        style: Style { width: Val::Px(360.0), padding: UiRect::all(Val::Px(23.0)), flex_direction: FlexDirection::Column, border: UiRect::top(Val::Px(3.0)), ..default() },
                        background_color: CARD_BG.into(), border_color: accent.into(), ..default()
                    }).with_children(|card| {
                        card.spawn(ImageBundle {style:Style {width:Val::Px(112.0),height:Val::Px(112.0),align_self:AlignSelf::Center,..default()},image:UiImage::new(assets.planet_portraits[body.portrait_index()].clone()),..default()});
                        menu_text(card, body.name(), 30.0, accent);
                        let gravity = format!("Grawitacja {:.2} m/s² · {:.0}% ziemskiej", body.gravity(), body.gravity()/9.81*100.0);
                        menu_text(card, &gravity, 16.0, Color::WHITE);
                        menu_text(card, body.atmosphere(), 16.0, Color::WHITE);
                        menu_text(card, body.fact(), 17.0, Color::rgb(0.8,0.88,0.94));
                        let condition = match body {
                            WorldBody::Moon => crate::locale::text("Duże różnice temperatur. W bazie generator tlenu napełnia butlę."),
                            WorldBody::Mars => crate::locale::text("Burze pyłowe osłabiają panele słoneczne. Bateria bazy pomaga przetrwać burzę."),
                            WorldBody::Venus => crate::locale::text("Gęste chmury osłabiają panele. W grze używasz fikcyjnego skafandra ochronnego, aby zbadać tę planetę."),
                        };
                        menu_text(card, condition, 16.0, Color::rgb(0.70,0.80,0.9));
                        let surveyed = if body == map.body {map.surveyed} else {expedition.surveyed(body)};
                        let points = campaign::survey_points(body).to_vec();
                        menu_text(card, &moon_base_localize::localized_format!("{}Punkty badań {}/{}",if body == map.body {crate::locale::text("Tu jesteś · ")} else {""},surveyed.count_ones(),points.len()), 16.0, accent);
                        for (i, point) in points.into_iter().enumerate() {
                            if surveyed & (1 << i) != 0 {
                                menu_text(card, &format!("{} — {}",point.name,point.fact), 15.0, Color::WHITE);
                            }
                        }
                        if body != map.body && (map.piloting_station || (lan.is_active && player.get_single().is_ok_and(|p|p.iso_pos.truncate().distance(map.station_position.truncate())<=4.0 && p.mounted_rover.is_none()))) && (lan.is_active || campaign::travel_allowed(stats.campaign.as_ref(),body)) {
                            spawn_styled_button(card, crate::locale::text("Leć tutaj"), UiAction::TravelTo(body), Val::Percent(100.0), Val::Px(42.0), STYLE_EMERALD, 17.0);
                        }
                    });
                }
            });
        menu_text(root, crate::locale::text("Tlen do oddychania zapewnia skafander. Promieniowanie może uszkadzać komórki, dlatego podczas burzy słonecznej szukaj osłony."), 17.0, Color::rgb(0.77,0.86,0.94));
        menu_text(root, crate::locale::text("W grze przelot trwa kilka sekund. Prawdziwe misje wymagają obliczenia trasy i trwają znacznie dłużej."), 15.0, Color::rgb(0.61,0.74,0.85));
        spawn_styled_button(root,crate::locale::text("Wróć do gry [Esc]"),UiAction::GotoState(AppState::LunarSurface),Val::Px(240.0),Val::Px(45.0),STYLE_SLATE,16.0);
    });
}

#[derive(Component)]
pub struct TransferCaption;

#[derive(Component)]
pub struct TransferShip;

pub fn setup_interplanetary_flight(
    mut commands: Commands,
    flight: Res<Expedition>,
    assets: Res<ProceduralAssets>,
) {
    let Some((body, _)) = flight.flight() else {
        return;
    };
    let mut panel = atlas_root();
    panel.background_color = Color::rgba(0.012, 0.023, 0.04, 0.98).into();
    commands.spawn((ScreenUiRoot, panel)).with_children(|root| {
        root.spawn((
            TransferShip,
            ImageBundle {
                style: Style {
                    width: Val::Px(176.0),
                    height: Val::Px(176.0),
                    ..default()
                },
                image: UiImage::new(assets.lander_tex.clone()),
                ..default()
            },
        ));
        menu_text(
            root,
            &moon_base_localize::localized_format!("Cel podróży · {}", body.name()),
            36.0,
            Color::WHITE,
        );
        menu_text(root, body.fact(), 18.0, Color::rgb(0.77, 0.88, 0.96));
        root.spawn((
            TransferCaption,
            TextBundle::from_section(
                "Start",
                TextStyle {
                    font_size: 19.0,
                    color: Color::rgb(0.5, 0.96, 0.85),
                    ..default()
                },
            ),
        ));
    });
}

pub fn update_interplanetary_ui(
    flight: Res<Expedition>,
    mut label: Query<&mut Text, With<TransferCaption>>,
    mut ship: Query<&mut Transform, With<TransferShip>>,
) {
    if let (Some((_, progress)), Ok(mut label)) = (flight.flight(), label.get_single_mut()) {
        let phase = if progress < 0.25 {
            "Start"
        } else if progress < 0.72 {
            crate::locale::text("Przelot")
        } else {
            crate::locale::text("Lądowanie")
        };
        label.sections[0].value = format!("{} · {:.0}%", phase, progress * 100.0);
        if let Ok(mut ship) = ship.get_single_mut() {
            let arc = (progress * std::f32::consts::PI).sin();
            ship.translation = Vec3::new((progress - 0.5) * 260.0, -80.0 * arc, 0.0);
            ship.rotation = Quat::from_rotation_z(-0.26 * arc);
            ship.scale = Vec3::splat(1.0 - 0.26 * arc);
        }
    }
}

pub fn setup_main_menu_ui(
    mut commands: Commands,
    apod: Res<crate::nasa_apod::NasaApod>,
    assets: Res<ProceduralAssets>,
) {
    let muted = Color::rgb(0.57, 0.66, 0.70);
    let accent = Color::rgb(0.49, 0.86, 0.75);
    commands
        .spawn((
            ScreenUiRoot,
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::SpaceBetween,
                    padding: UiRect::axes(Val::Px(64.0), Val::Px(36.0)),
                    ..default()
                },
                background_color: Color::rgba(0.015, 0.025, 0.04, 0.30).into(),
                ..default()
            },
        ))
        .with_children(|root| {
            root.spawn(NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    justify_content: JustifyContent::SpaceBetween,
                    ..default()
                },
                ..default()
            })
            .with_children(|header| {
                menu_text(
                    header,
                    crate::locale::text("WYPRAWA NA KSIĘŻYC"),
                    17.0,
                    Color::WHITE,
                );
                menu_text(header, crate::types::BUILD_LABEL, 15.0, muted);
            });
            root.spawn(NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    ..default()
                },
                ..default()
            })
            .with_children(|body| {
                body.spawn(NodeBundle {
                    style: Style {
                        width: Val::Px(580.0),
                        padding: UiRect::all(Val::Px(30.0)),
                        border: UiRect::left(Val::Px(2.0)),
                        flex_direction: FlexDirection::Column,
                        ..default()
                    },
                    background_color: Color::rgba(0.025, 0.04, 0.06, 0.94).into(),
                    border_color: accent.into(),
                    ..default()
                })
                .with_children(|card| {
                    menu_text(card, "Moon Base 101", 66.0, Color::rgb(0.93, 0.96, 0.95));
                    menu_text(
                        card,
                        crate::locale::text("Zbieraj minerały, zbuduj bazę i poleć stacją."),
                        18.0,
                        muted,
                    );
                    for (label, action, theme, height) in [
                        (
                            crate::locale::text("Graj · Misja na około 20 min"),
                            UiAction::GotoState(AppState::CampaignBriefing),
                            STYLE_EMERALD,
                            56.0,
                        ),
                        (
                            crate::locale::text("Hostuj na PC · LAN i telefon"),
                            UiAction::StartLanHost,
                            STYLE_BLUEPRINT,
                            44.0,
                        ),
                        (
                            crate::locale::text("Zaprojektuj misję"),
                            UiAction::GotoState(AppState::MissionDesignLab),
                            STYLE_SLATE,
                            44.0,
                        ),
                        (
                            crate::locale::text("Jak grać"),
                            UiAction::GotoState(AppState::Controls),
                            STYLE_SLATE,
                            44.0,
                        ),
                        (
                            crate::locale::text("Ustawienia"),
                            UiAction::GotoState(AppState::Settings),
                            STYLE_SLATE,
                            44.0,
                        ),
                        (
                            crate::locale::text("Zakończ grę"),
                            UiAction::QuitApp,
                            STYLE_SLATE,
                            40.0,
                        ),
                    ] {
                        #[cfg(windows)]
                        let hosting = matches!(action, UiAction::StartLanHost);
                        spawn_styled_button(
                            card,
                            label,
                            action,
                            Val::Percent(100.0),
                            Val::Px(height),
                            theme,
                            17.0,
                        );
                        #[cfg(windows)]
                        if hosting {
                            spawn_lan_firewall_control(card);
                        }
                    }
                });
                crate::nasa_apod::menu_card(body, &apod, assets.planet_portraits[0].clone());
            });
            root.spawn(NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    justify_content: JustifyContent::SpaceBetween,
                    border: UiRect::top(Val::Px(1.0)),
                    padding: UiRect::top(Val::Px(16.0)),
                    ..default()
                },
                border_color: CARD_BORDER.into(),
                ..default()
            })
            .with_children(|footer| {
                menu_text(footer, GAME_CREATORS_LINE, 12.0, muted);
                menu_text(
                    footer,
                    crate::locale::text("SURVIVAL   /   BUDOWANIE   /   ODKRYWANIE"),
                    12.0,
                    muted,
                );
            });
        });
}

// ============================================================================
// 2. SETTINGS & LAN MULTIPLAYER SCREEN
// ============================================================================

pub fn setup_settings_ui(mut commands: Commands) {
    commands
        .spawn((
            ScreenUiRoot,
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    padding: UiRect::all(Val::Px(20.0)),
                    ..default()
                },
                background_color: BG_DARK.into(),
                ..default()
            },
        ))
        .with_children(|root| {
            root.spawn(NodeBundle {
                style: Style {
                    width: Val::Px(820.0),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    padding: UiRect::all(Val::Px(24.0)),
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                background_color: CARD_BG.into(),
                border_color: CARD_BORDER.into(),
                ..default()
            })
            .with_children(|card| {
                card.spawn(TextBundle::from_section(
                    crate::locale::text("USTAWIENIA GRY, KAMERY, ZAŁOGI I MULTIPLAYER LAN"),
                    TextStyle {
                        font_size: 24.0,
                        color: Color::rgb(0.38, 0.94, 1.0),
                        ..default()
                    },
                ));

                spawn_styled_button(
                    card,
                    match crate::locale::language() {
                        crate::locale::Language::English => "Language · English → Polski",
                        crate::locale::Language::Polish => "Język · Polski → English",
                    },
                    UiAction::ToggleLanguage,
                    Val::Px(360.0),
                    Val::Px(36.0),
                    STYLE_SLATE,
                    15.0,
                );
                spawn_styled_button(
                    card,
                    crate::locale::text("Pełny ekran · wyłączony [F11]"),
                    UiAction::ToggleFullscreen,
                    Val::Px(360.0),
                    Val::Px(44.0),
                    STYLE_BLUEPRINT,
                    17.0,
                );
                card.spawn(NodeBundle {
                    style: Style {
                        flex_wrap: FlexWrap::Wrap,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    ..default()
                })
                .with_children(|row| {
                    spawn_styled_button(
                        row,
                        crate::locale::text("Imię astronauty"),
                        UiAction::EditAstronautName,
                        Val::Px(300.0),
                        Val::Px(38.0),
                        STYLE_SLATE,
                        14.0,
                    );
                    spawn_styled_button(
                        row,
                        crate::locale::text("Limit FPS"),
                        UiAction::CycleFps,
                        Val::Px(200.0),
                        Val::Px(38.0),
                        STYLE_SLATE,
                        14.0,
                    );
                    spawn_styled_button(
                        row,
                        crate::locale::text("Zmień sterowanie"),
                        UiAction::OpenControls,
                        Val::Px(180.0),
                        Val::Px(38.0),
                        STYLE_SLATE,
                        14.0,
                    );
                });

                card.spawn((
                    DynamicLabel(LabelId::SettingsSummary),
                    TextBundle::from_section(
                        crate::locale::text("Wczytywanie ustawień..."),
                        TextStyle {
                            font_size: 14.5,
                            color: Color::rgb(0.92, 0.96, 1.0),
                            ..default()
                        },
                    )
                    .with_style(Style {
                        margin: UiRect::vertical(Val::Px(12.0)),
                        ..default()
                    }),
                ));

                card.spawn(NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::Row,
                        flex_wrap: FlexWrap::Wrap,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    ..default()
                })
                .with_children(|row| {
                    spawn_styled_button(
                        row,
                        crate::locale::text("Głośność Główna -"),
                        UiAction::AdjustMasterVol(-0.1),
                        Val::Px(175.0),
                        Val::Px(36.0),
                        STYLE_SLATE,
                        13.5,
                    );
                    spawn_styled_button(
                        row,
                        crate::locale::text("Głośność Główna +"),
                        UiAction::AdjustMasterVol(0.1),
                        Val::Px(175.0),
                        Val::Px(36.0),
                        STYLE_BLUEPRINT,
                        13.5,
                    );
                    spawn_styled_button(
                        row,
                        crate::locale::text("Muzyka Kosmosu -"),
                        UiAction::AdjustMusicVol(-0.1),
                        Val::Px(175.0),
                        Val::Px(36.0),
                        STYLE_SLATE,
                        13.5,
                    );
                    spawn_styled_button(
                        row,
                        crate::locale::text("Muzyka Kosmosu +"),
                        UiAction::AdjustMusicVol(0.1),
                        Val::Px(175.0),
                        Val::Px(36.0),
                        STYLE_BLUEPRINT,
                        13.5,
                    );
                    spawn_styled_button(
                        row,
                        crate::locale::text("Efekty Dźwiękowe -"),
                        UiAction::AdjustSfxVol(-0.1),
                        Val::Px(175.0),
                        Val::Px(36.0),
                        STYLE_SLATE,
                        13.5,
                    );
                    spawn_styled_button(
                        row,
                        crate::locale::text("Efekty Dźwiękowe +"),
                        UiAction::AdjustSfxVol(0.1),
                        Val::Px(175.0),
                        Val::Px(36.0),
                        STYLE_BLUEPRINT,
                        13.5,
                    );
                    spawn_styled_button(
                        row,
                        crate::locale::text("Przybliż Kamerę (Zoom -)"),
                        UiAction::AdjustZoom(-0.08),
                        Val::Px(175.0),
                        Val::Px(36.0),
                        STYLE_AMBER,
                        13.0,
                    );
                    spawn_styled_button(
                        row,
                        crate::locale::text("Oddal Kamerę (Zoom +)"),
                        UiAction::AdjustZoom(0.08),
                        Val::Px(175.0),
                        Val::Px(36.0),
                        STYLE_AMBER,
                        13.0,
                    );
                });

                card.spawn(NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::Row,
                        flex_wrap: FlexWrap::Wrap,
                        justify_content: JustifyContent::Center,
                        margin: UiRect::vertical(Val::Px(8.0)),
                        ..default()
                    },
                    ..default()
                })
                .with_children(|row| {
                    spawn_styled_button(
                        row,
                        crate::locale::text("[ZAŁOGA] Tryb SOLO vs 4 Astronautów (Przełącz [Z])"),
                        UiAction::ToggleAiCrew,
                        Val::Px(350.0),
                        Val::Px(40.0),
                        STYLE_HUSARIA,
                        13.5,
                    );
                    spawn_styled_button(
                        row,
                        crate::locale::text("Przełącz Radar Orbitalny (O)"),
                        UiAction::ToggleCornerRadar,
                        Val::Px(230.0),
                        Val::Px(40.0),
                        STYLE_VITAL,
                        13.5,
                    );
                    spawn_styled_button(
                        row,
                        crate::locale::text("Przełącz Gwiazdy Tła"),
                        UiAction::ToggleHorizonPlanets,
                        Val::Px(210.0),
                        Val::Px(40.0),
                        STYLE_SLATE,
                        13.5,
                    );
                    spawn_styled_button(
                        row,
                        crate::locale::text("Ekranowy Pad Dotykowy (Mobile)"),
                        UiAction::ToggleTouchControls,
                        Val::Px(260.0),
                        Val::Px(40.0),
                        STYLE_SLATE,
                        13.5,
                    );
                });

                card.spawn(NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::Row,
                        justify_content: JustifyContent::Center,
                        margin: UiRect::vertical(Val::Px(8.0)),
                        ..default()
                    },
                    ..default()
                })
                .with_children(|row| {
                    spawn_styled_button(
                        row,
                        crate::locale::text("HOSTUJ GRĘ · TCP 7878"),
                        UiAction::StartLanHost,
                        Val::Px(260.0),
                        Val::Px(42.0),
                        STYLE_EMERALD,
                        14.5,
                    );
                    spawn_styled_button(
                        row,
                        crate::locale::text("DOŁĄCZ DO GRY LAN (KLIENT)"),
                        UiAction::JoinLanClient,
                        Val::Px(260.0),
                        Val::Px(42.0),
                        STYLE_EMERALD,
                        14.5,
                    );
                });

                spawn_styled_button(
                    card,
                    crate::locale::text("Kliknij i wpisz IPv4 z ekranu hosta"),
                    UiAction::EditLanAddress,
                    Val::Px(380.0),
                    Val::Px(36.0),
                    STYLE_SLATE,
                    14.0,
                );
                #[cfg(windows)]
                spawn_lan_firewall_control(card);
                card.spawn(
                    TextBundle::from_section(
                        GAME_CREATORS_LINE,
                        TextStyle {
                            font_size: 13.0,
                            color: Color::rgb(1.0, 0.86, 0.48),
                            ..default()
                        },
                    )
                    .with_style(Style {
                        margin: UiRect::vertical(Val::Px(6.0)),
                        ..default()
                    }),
                );

                spawn_styled_button(
                    card,
                    crate::locale::text("Wróć  [Esc]"),
                    UiAction::Back,
                    Val::Px(320.0),
                    Val::Px(42.0),
                    STYLE_BLUEPRINT,
                    15.5,
                );
            });
        });
}

pub fn refresh_settings_language(
    world: &mut World,
    mut previous: Local<Option<crate::locale::Language>>,
) {
    use bevy::ecs::system::RunSystemOnce;
    let language = world.resource::<GameSettings>().language;
    if previous
        .replace(language)
        .is_some_and(|old| old != language)
        && *world.resource::<State<AppState>>().get() == AppState::Settings
    {
        world.run_system_once(cleanup_ui_root);
        world.run_system_once(setup_settings_ui);
    }
}

pub fn update_settings_labels(
    settings: Res<GameSettings>,
    lan: Res<LanMultiplayer>,
    main_window: Query<&Window, With<PrimaryWindow>>,
    mut q: Query<(&DynamicLabel, &mut Text)>,
    config: Res<MissionConfig>,
    edit: Option<Res<SettingsEdit>>,
) {
    let fullscreen = main_window
        .get_single()
        .is_ok_and(|window| window.mode != WindowMode::Windowed);
    for (lbl, mut text) in &mut q {
        if lbl.0 == LabelId::SettingsName {
            let caret = if edit.as_ref().is_some_and(|edit| edit.name) {
                " ▌"
            } else {
                ""
            };
            text.sections[0].value = moon_base_localize::localized_format!(
                "Imię astronauty · {}{caret}",
                config.callsign
            );
        }
        if lbl.0 == LabelId::SettingsFps {
            text.sections[0].value = settings.fps_limit.map_or_else(
                || crate::locale::text("FPS · bez limitu").into(),
                |fps| format!("Limit FPS · {fps}"),
            );
        }
        if lbl.0 == LabelId::SettingsFullscreen {
            text.sections[0].value = if fullscreen {
                crate::locale::text("Pełny ekran · włączony [F11]")
            } else {
                crate::locale::text("Pełny ekran · wyłączony [F11]")
            }
            .into();
        }
        if lbl.0 == LabelId::SettingsSummary {
            text.sections[0].value = moon_base_localize::localized_format!(
                "Głośność {:.0}% · Muzyka {:.0}% · Efekty {:.0}% · Kamera {:.2}×\n\
                 {} · Radar {} · Tło {} · Pad {}\n\
                 LAN · {} · {}{}",
                settings.master_volume * 100.0,
                settings.music_volume * 100.0,
                settings.sfx_volume * 100.0,
                settings.camera_zoom,
                if settings.co_op_ai_crew {
                    crate::locale::text("Załoga 4 astronautów")
                } else {
                    "Solo"
                },
                if settings.show_corner_radar {
                    "✓"
                } else {
                    "–"
                },
                if settings.show_horizon_planets {
                    "✓"
                } else {
                    "–"
                },
                if settings.show_touch_controls {
                    "✓"
                } else {
                    "–"
                },
                lan.bind_status,
                lan.target_addr,
                if lan.editing_address { " ▌" } else { "" }
            );
        }
    }
}

// ============================================================================
// 3. HOW TO PLAY / GUIDE SCREEN (Zero Copyright Terms, 12+ Friendly Polish)
// ============================================================================

pub fn setup_controls_ui(mut commands: Commands) {
    commands
        .spawn((
            ScreenUiRoot,
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    padding: UiRect::all(Val::Px(20.0)),
                    ..default()
                },
                background_color: BG_DARK.into(),
                ..default()
            },
        ))
        .with_children(|root| {
            root.spawn(NodeBundle {
                style: Style {
                    width: Val::Px(880.0),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    padding: UiRect::all(Val::Px(26.0)),
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                background_color: CARD_BG.into(),
                border_color: CARD_BORDER.into(),
                ..default()
            })
            .with_children(|card| {
                card.spawn(TextBundle::from_section(
                    crate::locale::text("Jak grać"),
                    TextStyle {
                        font_size: 26.0,
                        color: Color::rgb(1.0, 0.84, 0.36),
                        ..default()
                    },
                ));

                card.spawn(
                    TextBundle::from_section(
                        crate::locale::text("W A S D lub strzałki — chodź. Spacja — skocz.\n\n\
                         Kopanie — przytrzymaj lewy przycisk myszy na skale. Podejdź do odłamków, żeby je zebrać.\n\n\
                         E — wejdź do bazy lub łazika. W bazie z generatorem tlenu uzupełnisz zapas.\n\n\
                         R — zjedz posiłek. T — napij się wody. Pod paskiem tlenu sprawdzisz, ile czasu zostało.\n\n\
                         G przy stacji — wejdź do kokpitu. Spacja — wznieś się. WASD — leć. Ctrl — ląduj. G — wysiądź po lądowaniu.\n\n\
                         Buduj — wybierz budynek na dolnym pasku, a następnie kliknij wolne pole.\n\n\
                         I — plecak. M — mapa. Esc — pauza."),
                        TextStyle {
                            font_size: 14.2,
                            color: Color::rgb(0.92, 0.97, 1.0),
                            ..default()
                        },
                    )
                    .with_style(Style {
                        margin: UiRect::vertical(Val::Px(14.0)),
                        ..default()
                    }),
                );

                spawn_styled_button(
                    card,
                    crate::locale::text("Wróć"),
                    UiAction::Back,
                    Val::Px(380.0),
                    Val::Px(44.0),
                    STYLE_EMERALD,
                    16.0,
                );
            });
        });
}

// ============================================================================
// 4. GAME MODE & CHARACTER LAB (4 Game Modes: Survival, Sandbox, Hard, Hardcore)
// ============================================================================

fn spawn_selector_row(
    parent: &mut ChildBuilder,
    title: &str,
    opt: MdlOption,
    label_id: LabelId,
    btn_style: ButtonVisualStyle,
) {
    parent
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Px(27.0),
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                margin: UiRect::vertical(Val::Px(1.5)),
                ..default()
            },
            ..default()
        })
        .with_children(|row| {
            row.spawn(TextBundle::from_section(
                title,
                TextStyle {
                    font_size: 12.5,
                    color: Color::rgb(0.78, 0.90, 1.0),
                    ..default()
                },
            ));
            row.spawn(NodeBundle {
                style: Style {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    ..default()
                },
                ..default()
            })
            .with_children(|ctrl| {
                spawn_styled_button(
                    ctrl,
                    "<",
                    UiAction::MdlCycle(opt, -1),
                    Val::Px(26.0),
                    Val::Px(22.0),
                    btn_style,
                    13.0,
                );
                ctrl.spawn((
                    DynamicLabel(label_id),
                    TextBundle::from_section(
                        "...",
                        TextStyle {
                            font_size: 12.2,
                            color: Color::rgb(0.98, 0.96, 0.82),
                            ..default()
                        },
                    )
                    .with_style(Style {
                        width: Val::Px(205.0),
                        margin: UiRect::horizontal(Val::Px(5.0)),
                        ..default()
                    }),
                ));
                spawn_styled_button(
                    ctrl,
                    ">",
                    UiAction::MdlCycle(opt, 1),
                    Val::Px(26.0),
                    Val::Px(22.0),
                    btn_style,
                    13.0,
                );
            });
        });
}

pub fn setup_mdl_ui(mut commands: Commands) {
    commands
        .spawn((
            ScreenUiRoot,
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::SpaceBetween,
                    padding: UiRect::all(Val::Px(10.0)),
                    ..default()
                },
                background_color: BG_DARK.into(),
                ..default()
            },
        ))
        .with_children(|root| {
            // Top Header + 4 Game Modes Bar
            root.spawn(NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(8.0)),
                    border: UiRect::all(Val::Px(1.5)),
                    ..default()
                },
                background_color: CARD_BG.into(),
                border_color: CARD_BORDER.into(),
                ..default()
            })
            .with_children(|top| {
                top.spawn(NodeBundle {
                    style: Style {
                        width: Val::Percent(100.0),
                        flex_direction: FlexDirection::Row,
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    ..default()
                })
                .with_children(|row1| {
                    row1.spawn(TextBundle::from_section(
                        crate::locale::text("WYBÓR TRYBU GRY (SURVIVAL / SANDBOX / TRUDNY / HARDCORE) & KREATOR POSTACI"),
                        TextStyle {
                            font_size: 16.5,
                            color: Color::rgb(0.38, 0.94, 1.0),
                            ..default()
                        },
                    ));
                    row1.spawn(NodeBundle {
                        style: Style {
                            flex_direction: FlexDirection::Row,
                            ..default()
                        },
                        ..default()
                    })
                    .with_children(|btns| {
                        spawn_styled_button(
                            btns,
                            crate::locale::text("Zapisz Profil"),
                            UiAction::MdlSaveJson,
                            Val::Px(105.0),
                            Val::Px(28.0),
                            STYLE_SLATE,
                            12.0,
                        );
                        spawn_styled_button(
                            btns,
                            crate::locale::text("Wczytaj Profil"),
                            UiAction::MdlLoadJson,
                            Val::Px(105.0),
                            Val::Px(28.0),
                            STYLE_SLATE,
                            12.0,
                        );
                        spawn_styled_button(
                            btns,
                            crate::locale::text("<< Menu Główne"),
                            UiAction::GotoState(AppState::MainMenu),
                            Val::Px(125.0),
                            Val::Px(28.0),
                            STYLE_BLUEPRINT,
                            12.0,
                        );
                    });
                });

                // 4 Game Mode Preset Buttons
                top.spawn(NodeBundle {
                    style: Style {
                        width: Val::Percent(100.0),
                        flex_direction: FlexDirection::Row,
                        justify_content: JustifyContent::SpaceBetween,
                        margin: UiRect::top(Val::Px(4.0)),
                        ..default()
                    },
                    ..default()
                })
                .with_children(|presets| {
                    spawn_styled_button(
                        presets,
                        crate::locale::text("[1] TRYB SURVIVAL (ZBALANSOWANY)"),
                        UiAction::MdlApplyPreset(0),
                        Val::Percent(24.5),
                        Val::Px(34.0),
                        STYLE_EMERALD,
                        12.2,
                    );
                    spawn_styled_button(
                        presets,
                        crate::locale::text("[2] TRYB SANDBOX (KREATYWNY)"),
                        UiAction::MdlApplyPreset(1),
                        Val::Percent(24.5),
                        Val::Px(34.0),
                        STYLE_AMBER,
                        12.2,
                    );
                    spawn_styled_button(
                        presets,
                        crate::locale::text("[3] TRYB TRUDNY (WYZWANIE)"),
                        UiAction::MdlApplyPreset(2),
                        Val::Percent(24.5),
                        Val::Px(34.0),
                        STYLE_BLUEPRINT,
                        12.2,
                    );
                    spawn_styled_button(
                        presets,
                        crate::locale::text("[4] TRYB HARDCORE (JEDNO ŻYCIE!)"),
                        UiAction::MdlApplyPreset(3),
                        Val::Percent(24.5),
                        Val::Px(34.0),
                        STYLE_DANGER,
                        12.2,
                    );
                });
            });

            // 3-Column Interactive Body
            root.spawn(NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    flex_grow: 1.0,
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::SpaceBetween,
                    margin: UiRect::vertical(Val::Px(6.0)),
                    ..default()
                },
                ..default()
            })
            .with_children(|cols| {
                // COLUMN 1: Game Mode, Rocket, Orbit & Subsystems
                cols.spawn(NodeBundle {
                    style: Style {
                        width: Val::Percent(33.5),
                        height: Val::Percent(100.0),
                        flex_direction: FlexDirection::Column,
                        padding: UiRect::all(Val::Px(10.0)),
                        border: UiRect::all(Val::Px(1.5)),
                        ..default()
                    },
                    background_color: CARD_BG.into(),
                    border_color: CARD_BORDER.into(),
                    ..default()
                })
                .with_children(|c1| {
                    c1.spawn(TextBundle::from_section(
                        crate::locale::text("1. TRYB ROZGRYWKI I WYPOSAŻENIE BAZY"),
                        TextStyle {
                            font_size: 15.0,
                            color: Color::rgb(0.40, 0.92, 1.0),
                            ..default()
                        },
                    ));
                    spawn_selector_row(
                        c1,
                        crate::locale::text("Tryb Gry:"),
                        MdlOption::Scenario,
                        LabelId::MdlScenario,
                        STYLE_BLUEPRINT,
                    );
                    spawn_styled_button(c1,crate::locale::text("Sandbox · zadania włączone"),UiAction::ToggleSandboxQuests,Val::Percent(100.0),Val::Px(34.0),STYLE_SLATE,12.0);
                    c1.spawn((
                        DynamicLabel(LabelId::MdlScenarioDesc),
                        TextBundle::from_section(
                            "...",
                            TextStyle {
                                font_size: 11.5,
                                color: Color::rgb(0.74, 0.86, 0.98),
                                ..default()
                            },
                        )
                        .with_style(Style {
                            margin: UiRect::bottom(Val::Px(4.0)),
                            ..default()
                        }),
                    ));
                    spawn_selector_row(
                        c1,
                        crate::locale::text("Poziom Trudności:"),
                        MdlOption::Difficulty,
                        LabelId::MdlDifficulty,
                        STYLE_BLUEPRINT,
                    );
                    spawn_selector_row(
                        c1,
                        crate::locale::text("Rola w Załodze:"),
                        MdlOption::TeamSeat,
                        LabelId::MdlTeamSeat,
                        STYLE_BLUEPRINT,
                    );
                    spawn_selector_row(
                        c1,
                        crate::locale::text("Rakieta Nośna:"),
                        MdlOption::Launch,
                        LabelId::MdlLaunch,
                        STYLE_BLUEPRINT,
                    );
                    spawn_selector_row(
                        c1,
                        crate::locale::text("Orbita Księżyca:"),
                        MdlOption::Orbit,
                        LabelId::MdlOrbit,
                        STYLE_BLUEPRINT,
                    );
                    spawn_selector_row(
                        c1,
                        crate::locale::text("Zasilanie Bazy:"),
                        MdlOption::Power,
                        LabelId::MdlPower,
                        STYLE_BLUEPRINT,
                    );
                    spawn_selector_row(
                        c1,
                        crate::locale::text("Antena Łączności:"),
                        MdlOption::Comms,
                        LabelId::MdlComms,
                        STYLE_BLUEPRINT,
                    );
                    spawn_selector_row(
                        c1,
                        crate::locale::text("Łazik Startowy:"),
                        MdlOption::Rover,
                        LabelId::MdlRover,
                        STYLE_BLUEPRINT,
                    );
                    spawn_selector_row(
                        c1,
                        crate::locale::text("Czas Wyprawy (Sol):"),
                        MdlOption::Lifetime,
                        LabelId::MdlLifetime,
                        STYLE_BLUEPRINT,
                    );

                    spawn_styled_button(
                        c1,
                        crate::locale::text("[+BONUS] NARADA ZAŁOGI (+8% UDŹWIGU I BUDŻETU)"),
                        UiAction::MdlNegotiateDilemma,
                        Val::Percent(98.0),
                        Val::Px(32.0),
                        STYLE_EMERALD,
                        11.8,
                    );
                });

                // COLUMN 2: Character Customizer & Instruments
                cols.spawn(NodeBundle {
                    style: Style {
                        width: Val::Percent(34.0),
                        height: Val::Percent(100.0),
                        flex_direction: FlexDirection::Column,
                        padding: UiRect::all(Val::Px(10.0)),
                        border: UiRect::all(Val::Px(1.5)),
                        ..default()
                    },
                    background_color: CARD_BG.into(),
                    border_color: Color::rgb(1.0, 0.78, 0.35).into(),
                    ..default()
                })
                .with_children(|c2| {
                    c2.spawn(TextBundle::from_section(
                        crate::locale::text("2. KREATOR POSTACI & WYRAŻENIE SIEBIE"),
                        TextStyle {
                            font_size: 15.0,
                            color: Color::rgb(1.0, 0.84, 0.38),
                            ..default()
                        },
                    ));

                    // Visual Color Swatch Preview Row
                    c2.spawn(NodeBundle {
                        style: Style {
                            flex_direction: FlexDirection::Row,
                            align_items: AlignItems::Center,
                            margin: UiRect::vertical(Val::Px(3.0)),
                            ..default()
                        },
                        ..default()
                    })
                    .with_children(|preview| {
                        preview.spawn((
                            SuitPreviewSwatch,
                            NodeBundle {
                                style: Style {
                                    width: Val::Px(30.0),
                                    height: Val::Px(30.0),
                                    margin: UiRect::right(Val::Px(8.0)),
                                    border: UiRect::all(Val::Px(2.0)),
                                    ..default()
                                },
                                background_color: Color::WHITE.into(),
                                border_color: Color::BLACK.into(),
                                ..default()
                            },
                        ));
                        preview.spawn((
                            VisorPreviewSwatch,
                            NodeBundle {
                                style: Style {
                                    width: Val::Px(30.0),
                                    height: Val::Px(20.0),
                                    margin: UiRect::right(Val::Px(10.0)),
                                    border: UiRect::all(Val::Px(2.0)),
                                    ..default()
                                },
                                background_color: Color::GOLD.into(),
                                border_color: Color::WHITE.into(),
                                ..default()
                            },
                        ));
                        preview.spawn(TextBundle::from_section(
                            crate::locale::text("Barwy skafandra i wizjera (zmienisz też w grze pod [K]!)"),
                            TextStyle {
                                font_size: 11.2,
                                color: Color::rgb(0.88, 0.92, 1.0),
                                ..default()
                            },
                        ));
                    });

                    spawn_selector_row(
                        c2,
                        crate::locale::text("Kryptonim:"),
                        MdlOption::Callsign,
                        LabelId::MdlCallsign,
                        STYLE_HUSARIA,
                    );
                    spawn_selector_row(
                        c2,
                        crate::locale::text("Kolor Skafandra:"),
                        MdlOption::Suit,
                        LabelId::MdlSuit,
                        STYLE_HUSARIA,
                    );
                    spawn_selector_row(
                        c2,
                        crate::locale::text("Wizjer Hełmu:"),
                        MdlOption::Visor,
                        LabelId::MdlVisor,
                        STYLE_HUSARIA,
                    );
                    spawn_selector_row(
                        c2,
                        crate::locale::text("Ozdoba Hełmu:"),
                        MdlOption::Accessory,
                        LabelId::MdlAccessory,
                        STYLE_HUSARIA,
                    );
                    spawn_selector_row(
                        c2,
                        crate::locale::text("Smuga Plecaka:"),
                        MdlOption::TrailColor,
                        LabelId::MdlTrail,
                        STYLE_HUSARIA,
                    );
                    spawn_selector_row(
                        c2,
                        crate::locale::text("Naszywka:"),
                        MdlOption::Patch,
                        LabelId::MdlPatch,
                        STYLE_HUSARIA,
                    );
                    spawn_selector_row(
                        c2,
                        crate::locale::text("Moduł Plecaka:"),
                        MdlOption::Backpack,
                        LabelId::MdlBackpack,
                        STYLE_HUSARIA,
                    );
                    spawn_selector_row(
                        c2,
                        crate::locale::text("Specjalizacja:"),
                        MdlOption::Trait,
                        LabelId::MdlTrait,
                        STYLE_HUSARIA,
                    );

                    c2.spawn(
                        TextBundle::from_section(
                            crate::locale::text("3. SPRZĘT BADAWCZY NA LĄDOWNIKU (KLIKNIJ BY WYBRAĆ)"),
                            TextStyle {
                                font_size: 13.0,
                                color: Color::rgb(0.40, 0.92, 1.0),
                                ..default()
                            },
                        )
                        .with_style(Style {
                            margin: UiRect::top(Val::Px(4.0)),
                            ..default()
                        }),
                    );

                    for (idx, inst) in SciInstrument::ALL.iter().enumerate() {
                        c2.spawn((
                            ButtonBundle {
                                style: Style {
                                    width: Val::Percent(98.0),
                                    height: Val::Px(22.0),
                                    justify_content: JustifyContent::FlexStart,
                                    align_items: AlignItems::Center,
                                    padding: UiRect::horizontal(Val::Px(6.0)),
                                    margin: UiRect::vertical(Val::Px(1.0)),
                                    border: UiRect::all(Val::Px(1.0)),
                                    ..default()
                                },
                                background_color: STYLE_SLATE.normal_bg.into(),
                                border_color: STYLE_SLATE.border_col.into(),
                                ..default()
                            },
                            STYLE_SLATE,
                            UiAction::MdlToggleInst(*inst),
                        ))
                        .with_children(|b| {
                            b.spawn((
                                DynamicLabel(LabelId::MdlInst(idx)),
                                TextBundle::from_section(
                                    inst.short_name(),
                                    TextStyle {
                                        font_size: 11.2,
                                        color: Color::WHITE,
                                        ..default()
                                    },
                                ),
                            ));
                        });
                    }
                });

                // COLUMN 3: Visual Resource Progress Bars & Launch Button
                cols.spawn(NodeBundle {
                    style: Style {
                        width: Val::Percent(31.0),
                        height: Val::Percent(100.0),
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::SpaceBetween,
                        padding: UiRect::all(Val::Px(10.0)),
                        border: UiRect::all(Val::Px(1.5)),
                        ..default()
                    },
                    background_color: CARD_BG.into(),
                    border_color: CARD_BORDER.into(),
                    ..default()
                })
                .with_children(|c3| {
                    c3.spawn(TextBundle::from_section(
                        crate::locale::text("4. PODSUMOWANIE RAKIETY I START"),
                        TextStyle {
                            font_size: 15.0,
                            color: Color::rgb(0.40, 0.92, 1.0),
                            ..default()
                        },
                    ));

                    spawn_labeled_progress_bar(
                        c3,
                        crate::locale::text("1. MASA STATKU I ŁADUNKU (kg)"),
                        BarMetric::MdlMass,
                        Color::rgb(0.25, 0.85, 0.95),
                    );
                    spawn_labeled_progress_bar(
                        c3,
                        crate::locale::text("2. ZAPOTRZEBOWANIE NA MOC (W)"),
                        BarMetric::MdlPower,
                        Color::rgb(0.98, 0.80, 0.22),
                    );
                    spawn_labeled_progress_bar(
                        c3,
                        crate::locale::text("3. BUDŻET WYPRAWY ($M)"),
                        BarMetric::MdlBudget,
                        Color::rgb(0.35, 0.92, 0.55),
                    );
                    spawn_labeled_progress_bar(
                        c3,
                        crate::locale::text("4. PRZESYŁ DANYCH vs ANTENA (Mbps)"),
                        BarMetric::MdlComms,
                        Color::rgb(0.75, 0.52, 1.0),
                    );

                    c3.spawn((
                        DynamicLabel(LabelId::MdlMetricsSummary),
                        TextBundle::from_section(
                            "...",
                            TextStyle {
                                font_size: 12.5,
                                color: Color::rgb(0.92, 0.96, 1.0),
                                ..default()
                            },
                        ),
                    ));

                    c3.spawn((
                        DynamicLabel(LabelId::MdlStatusBanner),
                        TextBundle::from_section(
                            "...",
                            TextStyle {
                                font_size: 13.0,
                                color: Color::rgb(0.45, 1.0, 0.65),
                                ..default()
                            },
                        ),
                    ));

                    spawn_styled_button(
                        c3,
                        crate::locale::text(">> START RAKIETY — LECIMY NA KSIĘŻYC! <<"),
                        UiAction::MdlLaunchIfReady,
                        Val::Percent(98.0),
                        Val::Px(48.0),
                        STYLE_EMERALD,
                        16.0,
                    );
                });
            });
        });
}

#[allow(clippy::type_complexity)]
pub fn update_mdl_ui(
    keys: Res<ButtonInput<KeyCode>>,
    config: Res<MissionConfig>,
    navigation: Res<MenuNavigation>,
    mut next_state: ResMut<NextState<AppState>>,
    mut label_q: Query<(&DynamicLabel, &mut Text)>,
    mut bar_q: Query<(&BarFillKind, &mut Style, &mut BackgroundColor)>,
    mut suit_q: Query<&mut BackgroundColor, (With<SuitPreviewSwatch>, Without<BarFillKind>)>,
    mut visor_q: Query<
        &mut BackgroundColor,
        (
            With<VisorPreviewSwatch>,
            Without<SuitPreviewSwatch>,
            Without<BarFillKind>,
        ),
    >,
) {
    if keys.just_pressed(KeyCode::Enter) && config.validate_launch_readiness().is_ok() {
        next_state.set(AppState::LaunchSequence);
        return;
    }

    if let Ok(mut sc) = suit_q.get_single_mut() {
        *sc = config.suit_color.to_color().into();
    }
    if let Ok(mut vc) = visor_q.get_single_mut() {
        *vc = config.visor_tint.to_color().into();
    }
    let _patch_col = config.mission_patch.color();

    for (bar, mut style, mut bg) in &mut bar_q {
        let (ratio, ok) = match bar.0 {
            BarMetric::MdlMass => {
                let r = config.total_mass_kg() / config.effective_max_mass_kg();
                (r, r <= 1.0)
            }
            BarMetric::MdlPower => {
                let r = config.total_power_demand_w() / config.total_power_supply_w();
                (r, r <= 1.0)
            }
            BarMetric::MdlBudget => {
                let r = config.total_cost_musd() / config.max_budget_musd();
                (r, r <= 1.0)
            }
            BarMetric::MdlComms => {
                let r =
                    config.total_data_rate_mbps() / config.effective_comms_capacity_mbps().max(1.0);
                (r, r <= 1.15)
            }
            _ => continue,
        };
        style.width = Val::Percent((ratio * 100.0).clamp(4.0, 100.0));
        *bg = if ok {
            Color::rgb(0.22, 0.88, 0.58).into()
        } else {
            Color::rgb(0.98, 0.30, 0.30).into()
        };
    }

    for (lbl, mut text) in &mut label_q {
        match lbl.0 {
            LabelId::SandboxQuests => {
                text.sections[0].value = moon_base_localize::localized_format!(
                    "Sandbox · zadania {}",
                    if config.sandbox_quests {
                        crate::locale::text("włączone")
                    } else {
                        crate::locale::text("wyłączone")
                    }
                )
            }
            LabelId::MdlScenario => {
                text.sections[0].value = config.scenario.short_name().to_string()
            }
            LabelId::MdlScenarioDesc => {
                text.sections[0].value = format!(
                    "{}\nOrbita: {} | Rola: {}",
                    config.scenario.objective_summary(),
                    config.orbit.description(),
                    config.active_role.responsibility(),
                );
            }
            LabelId::MdlDifficulty => text.sections[0].value = config.difficulty.name().to_string(),
            LabelId::MdlTeamSeat => text.sections[0].value = config.active_role.title().to_string(),
            LabelId::MdlLaunch => text.sections[0].value = config.launch_vehicle.name().to_string(),
            LabelId::MdlOrbit => text.sections[0].value = config.orbit.name().to_string(),
            LabelId::MdlPower => text.sections[0].value = config.power_system.name().to_string(),
            LabelId::MdlComms => text.sections[0].value = config.comms_system.name().to_string(),
            LabelId::MdlRover => text.sections[0].value = config.prelanded_rover.name().to_string(),
            LabelId::MdlLifetime => {
                text.sections[0].value = moon_base_localize::localized_format!(
                    "{} Dni Księżycowych (Sol)",
                    config.planned_lifetime_sols
                )
            }
            LabelId::MdlCallsign => text.sections[0].value = config.callsign.clone(),
            LabelId::MdlSuit => text.sections[0].value = config.suit_color.name().to_string(),
            LabelId::MdlVisor => text.sections[0].value = config.visor_tint.name().to_string(),
            LabelId::MdlAccessory => text.sections[0].value = config.accessory.name().to_string(),
            LabelId::MdlTrail => text.sections[0].value = config.trail_color.name().to_string(),
            LabelId::MdlPatch => text.sections[0].value = config.mission_patch.name().to_string(),
            LabelId::MdlBackpack => {
                text.sections[0].value = config.backpack_module.name().to_string()
            }
            LabelId::MdlTrait => text.sections[0].value = config.astronaut_trait.name().to_string(),
            LabelId::MdlInst(idx) => {
                let inst = SciInstrument::ALL[idx];
                let selected = config.selected_instruments.contains(&inst);
                let mark = if selected { "[X]" } else { "[ ]" };
                text.sections[0].value = format!(
                    "{} {} ({:.0}kg, {:.0}W, ${:.0}M)",
                    mark,
                    inst.short_name(),
                    inst.mass_kg(),
                    inst.power_w(),
                    inst.cost_musd()
                );
                text.sections[0].style.color = if selected {
                    Color::rgb(0.45, 1.0, 0.72)
                } else {
                    Color::rgb(0.68, 0.76, 0.86)
                };
            }
            LabelId::MdlMetricsSummary => {
                text.sections[0].value = moon_base_localize::localized_format!(
                    "Gotowość Rakiety: Poziom {} / 4\n\
                     • Masa: {:.0} / {:.0} kg\n\
                     • Moc:  {:.0} W poboru / {:.0} W generacji\n\
                     • Koszt: ${:.1}M / ${:.1}M\n\
                     • Dane: {:.1} / {:.1} Mbps (Wartość: {:.0} pkt)",
                    config.cml_level(),
                    config.total_mass_kg(),
                    config.effective_max_mass_kg(),
                    config.total_power_demand_w(),
                    config.total_power_supply_w(),
                    config.total_cost_musd(),
                    config.max_budget_musd(),
                    config.total_data_rate_mbps(),
                    config.effective_comms_capacity_mbps(),
                    config.total_payload_science_value(),
                );
            }
            LabelId::MdlStatusBanner if !navigation.file_status.is_empty() => {
                text.sections[0].value = navigation.file_status.clone();
            }
            LabelId::MdlStatusBanner => match config.validate_launch_readiness() {
                Ok(()) => {
                    text.sections[0].value =
                        crate::locale::text("[OK] WSZYSTKIE SYSTEMY SPRAWNE! Możesz wystartować.")
                            .to_string();
                    text.sections[0].style.color = Color::rgb(0.40, 1.0, 0.65);
                }
                Err(e) => {
                    text.sections[0].value = moon_base_localize::localized_format!(
                        "[!] {} (Lub kliknij jeden z 4 Trybów Gry u góry!)",
                        e
                    );
                    text.sections[0].style.color = Color::rgb(1.0, 0.45, 0.40);
                }
            },
            _ => {}
        }
    }
}

// ============================================================================
// 5. ANIMATED LAUNCH SEQUENCE
// ============================================================================

pub fn setup_launch_sequence_ui(mut commands: Commands, mut timer: ResMut<LaunchTimer>) {
    timer.elapsed = 0.0;
    commands
        .spawn((
            ScreenUiRoot,
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::FlexEnd,
                    align_items: AlignItems::Center,
                    padding: UiRect::all(Val::Px(28.0)),
                    ..default()
                },
                background_color: Color::rgba(0.02, 0.04, 0.09, 0.45).into(),
                ..default()
            },
        ))
        .with_children(|parent| {
            parent.spawn((
                LaunchSeqText,
                TextBundle::from_section(
                    crate::locale::text("Inicjalizacja sekwencji startowej..."),
                    TextStyle {
                        font_size: 18.5,
                        color: Color::rgb(0.85, 0.96, 1.0),
                        ..default()
                    },
                )
                .with_style(Style {
                    margin: UiRect::bottom(Val::Px(12.0)),
                    ..default()
                }),
            ));
            spawn_styled_button(
                parent,
                crate::locale::text(">> POMIŃ ANIMACJĘ -> LĄDUJ NA KSIĘŻYCU <<"),
                UiAction::GotoState(AppState::LunarSurface),
                Val::Px(390.0),
                Val::Px(42.0),
                STYLE_EMERALD,
                15.0,
            );
        });
}

pub fn update_launch_sequence(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    config: Res<MissionConfig>,
    mut timer: ResMut<LaunchTimer>,
    mut next_state: ResMut<NextState<AppState>>,
    mut text_q: Query<&mut Text, With<LaunchSeqText>>,
) {
    timer.elapsed += time.delta_seconds();
    let stage = if timer.elapsed < 1.1 {
        crate::locale::text(
            "ETAP 1: Zapłon silników i wejście na trajektorię Trans-Lunar Injection!",
        )
    } else if timer.elapsed < 2.2 {
        crate::locale::text("ETAP 2: Przelot Ziemia-Księżyc i wejście na orbitę docelową!")
    } else if timer.elapsed < 3.3 {
        crate::locale::text("ETAP 3: Zestawienie łącza satelitarnego i zejście z orbity!")
    } else {
        crate::locale::text("ETAP 4: Lądowanie potwierdzone! Otwieranie śluz powietrznych...")
    };

    if let Ok(mut txt) = text_q.get_single_mut() {
        txt.sections[0].value = moon_base_localize::localized_format!(
            "=== LOT NA KSIĘŻYC ({:.1}s / 4.0s) ===\n\
             Astronauta: {} ({}) | Tryb: {} | Rakieta: {}\n\
             {}",
            timer.elapsed.min(4.0),
            config.callsign,
            config.accessory.name(),
            config.scenario.short_name(),
            config.launch_vehicle.name(),
            stage
        );
    }

    if timer.elapsed >= 4.0 || keys.just_pressed(KeyCode::Space) {
        next_state.set(AppState::LunarSurface);
    }
}

// ============================================================================
// 6. SURFACE HUD, MAP & WAYPOINTS [M], PROGRESSION & ACHIEVEMENTS [J], BIG HOTBAR INFO
// ============================================================================

pub fn setup_surface_hud(mut commands: Commands, assets: Res<ProceduralAssets>) {
    commands
        .spawn((
            ScreenUiRoot,
            ChatPanel,
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    bottom: Val::Px(108.0),
                    left: Val::Px(14.0),
                    width: Val::Px(480.0),
                    max_width: Val::Percent(70.0),
                    padding: UiRect::all(Val::Px(12.0)),
                    display: Display::None,
                    ..default()
                },
                background_color: Color::rgba(0.025, 0.045, 0.075, 0.92).into(),
                z_index: ZIndex::Global(1100),
                focus_policy: bevy::ui::FocusPolicy::Pass,
                ..default()
            },
        ))
        .with_children(|panel| {
            panel.spawn((
                ChatText,
                TextBundle::from_section(
                    "",
                    TextStyle {
                        font_size: 15.0,
                        color: Color::rgb(0.85, 0.94, 1.0),
                        ..default()
                    },
                ),
            ));
        });
    commands
        .spawn((
            ScreenUiRoot,
            PlayerListPanel,
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    top: Val::Px(110.0),
                    left: Val::Percent(28.0),
                    width: Val::Percent(44.0),
                    padding: UiRect::all(Val::Px(22.0)),
                    display: Display::None,
                    ..default()
                },
                background_color: Color::rgba(0.025, 0.045, 0.075, 0.98).into(),
                z_index: ZIndex::Global(1200),
                focus_policy: bevy::ui::FocusPolicy::Pass,
                ..default()
            },
        ))
        .with_children(|panel| {
            panel.spawn((
                PlayerListText,
                TextBundle::from_section(
                    "",
                    TextStyle {
                        font_size: 18.0,
                        color: Color::rgb(0.87, 0.96, 1.0),
                        ..default()
                    },
                ),
            ));
        });
    commands
        .spawn((
            ScreenUiRoot,
            WorldInputBlocker,
            QuestPanel,
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    top: Val::Px(64.0),
                    left: Val::Px(278.0),
                    max_width: Val::Px(660.0),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(16.0)),
                    border: UiRect::left(Val::Px(4.0)),
                    row_gap: Val::Px(7.0),
                    ..default()
                },
                background_color: Color::rgba(0.025, 0.045, 0.075, 0.96).into(),
                border_color: Color::rgb(1.0, 0.78, 0.30).into(),
                ..default()
            },
        ))
        .with_children(|panel| {
            panel
                .spawn((
                    UiAction::ToggleQuestPanel,
                    STYLE_SLATE,
                    ButtonBundle {
                        style: Style {
                            align_self: AlignSelf::FlexStart,
                            padding: UiRect::axes(Val::Px(12.0), Val::Px(5.0)),
                            border: UiRect::all(Val::Px(1.0)),
                            ..default()
                        },
                        background_color: STYLE_SLATE.normal_bg.into(),
                        border_color: STYLE_SLATE.border_col.into(),
                        ..default()
                    },
                ))
                .with_children(|button| {
                    button.spawn((
                        DynamicLabel(LabelId::HudQuestToggle),
                        TextBundle::from_section(
                            crate::locale::text("Zwiń zadania"),
                            TextStyle {
                                font_size: 14.0,
                                color: Color::WHITE,
                                ..default()
                            },
                        ),
                    ));
                });
            panel
                .spawn((
                    QuestDetails,
                    NodeBundle {
                        style: Style {
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(7.0),
                            ..default()
                        },
                        ..default()
                    },
                ))
                .with_children(|details| {
                    menu_text(
                        details,
                        crate::locale::text(campaign::GOAL),
                        16.0,
                        Color::rgb(0.72, 0.83, 0.92),
                    );
                    details.spawn((
                        DynamicLabel(LabelId::HudTopMission),
                        TextBundle::from_section(
                            crate::locale::text("Twoje zadanie"),
                            TextStyle {
                                font_size: 25.0,
                                color: Color::rgb(1.0, 0.84, 0.43),
                                ..default()
                            },
                        ),
                    ));
                    details.spawn((
                        DynamicLabel(LabelId::HudQuestInstruction),
                        TextBundle::from_section(
                            "",
                            TextStyle {
                                font_size: 18.0,
                                color: Color::WHITE,
                                ..default()
                            },
                        ),
                    ));
                });
        });
    commands
        .spawn((
            ScreenUiRoot,
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::SpaceBetween,
                    padding: UiRect::all(Val::Px(6.0)),
                    ..default()
                },
                ..default()
            },
        ))
        .with_children(|root| {
            // 1. COMPACT TOP BAR: Graphical Resource Pills + Distinct Action Buttons
            root.spawn((WorldInputBlocker, NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Px(50.0),
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    padding: UiRect::horizontal(Val::Px(8.0)),
                    border: UiRect::all(Val::Px(1.5)),
                    ..default()
                },
                background_color: CARD_BG.into(),
                border_color: CARD_BORDER.into(),
                ..default()
            }))
            .with_children(|top| {
                // Left: Graphical Resource Pills with Icons
                let pill_icons: [(Handle<Image>, Color); 7] = [
                    (assets.icon_regolith.clone(), Color::rgb(0.75, 0.80, 0.88)),
                    (assets.icon_ice.clone(), Color::rgb(0.40, 0.92, 1.0)),
                    (assets.icon_titanium.clone(), Color::rgb(0.88, 0.92, 1.0)),
                    (assets.icon_silicon.clone(), Color::rgb(0.45, 0.95, 0.72)),
                    (assets.icon_he3.clone(), Color::rgb(1.0, 0.84, 0.32)),
                    (assets.icon_ration.clone(), Color::rgb(0.55, 0.96, 0.45)),
                    (assets.icon_medkit.clone(), Color::rgb(1.0, 0.55, 0.62)),
                ];

                top.spawn(NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    ..default()
                })
                .with_children(|pills| {
                    for (idx, (icon, col)) in pill_icons.into_iter().enumerate() {
                        pills
                            .spawn(NodeBundle {
                                style: Style {
                                    flex_direction: FlexDirection::Row,
                                    align_items: AlignItems::Center,
                                    padding: UiRect::axes(Val::Px(6.0), Val::Px(3.0)),
                                    margin: UiRect::right(Val::Px(4.0)),
                                    border: UiRect::all(Val::Px(1.0)),
                                    ..default()
                                },
                                background_color: Color::rgb(0.04, 0.08, 0.15).into(),
                                border_color: col.into(),
                                ..default()
                            })
                            .with_children(|p| {
                                p.spawn(ImageBundle {
                                    style: Style {
                                        width: Val::Px(18.0),
                                        height: Val::Px(18.0),
                                        margin: UiRect::right(Val::Px(4.0)),
                                        ..default()
                                    },
                                    image: UiImage::new(icon),
                                    ..default()
                                });
                                p.spawn((
                                    DynamicLabel(LabelId::HudResourcePill(idx)),
                                    TextBundle::from_section(
                                        "0",
                                        TextStyle {
                                            font_size: 12.5,
                                            color: col,
                                            ..default()
                                        },
                                    ),
                                ));
                            });
                    }
                });

                // Center: Short Sol / Power / Event telemetry
                top.spawn(NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    ..default()
                })
                .with_children(|mid_info| {
                    mid_info.spawn((
                        DynamicLabel(LabelId::HudTopMission),
                        TextBundle::from_section(
                            "SOL 0.0",
                            TextStyle {
                                font_size: 12.0,
                                color: Color::rgb(0.90, 0.96, 1.0),
                                ..default()
                            },
                        ),
                    ));
                    mid_info.spawn((
                        DynamicLabel(LabelId::HudTopEvent),
                        TextBundle::from_section(
                            crate::locale::text("WARUNKI NOMINALNE"),
                            TextStyle {
                                font_size: 11.5,
                                color: Color::rgb(0.45, 0.95, 0.65),
                                ..default()
                            },
                        ),
                    ));
                });

                // Right: Color-Coded Action Buttons (Backpack, Map, Progression, Character, Crew, Flag, Zoom, Pause)
                top.spawn(NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    ..default()
                })
                .with_children(|btns| {
                    spawn_styled_button(
                        btns,
                        crate::locale::text("Plecak [I]"),
                        UiAction::ToggleInventoryModal,
                        Val::Px(84.0),
                        Val::Px(32.0),
                        STYLE_AMBER,
                        11.5,
                    );
                    spawn_styled_button(
                        btns,
                        crate::locale::text("Mapa [M]"),
                        UiAction::ToggleMapModal,
                        Val::Px(78.0),
                        Val::Px(32.0),
                        STYLE_EMERALD,
                        11.5,
                    );
                    spawn_styled_button(
                        btns, crate::locale::text("Buduj [B]"), UiAction::SurfaceCmd(SurfaceUiCmd::SetBuildMode(true)),
                        Val::Px(92.0), Val::Px(38.0), STYLE_BLUEPRINT, 15.0,
                    );
                    spawn_styled_button(btns, crate::locale::text("Rozbiórka"), UiAction::SurfaceCmd(SurfaceUiCmd::SetDemolitionMode), Val::Px(100.0), Val::Px(38.0), STYLE_DANGER, 14.0);
                    spawn_styled_button(btns, crate::locale::text("Czat [Enter]"), UiAction::ToggleChat, Val::Px(105.0), Val::Px(38.0), STYLE_SLATE, 13.0);
                    spawn_styled_button(btns, crate::locale::text("Dziennik [N]"), UiAction::GotoState(AppState::PlanetAtlas), Val::Px(118.0), Val::Px(38.0), STYLE_EMERALD, 15.0);
                    spawn_styled_button(
                        btns,
                        crate::locale::text("Pauza"),
                        UiAction::GotoState(AppState::Paused),
                        Val::Px(64.0),
                        Val::Px(32.0),
                        STYLE_SLATE,
                        11.5,
                    );
                });
            });

            // 2. MIDDLE ROW: LEFT VITALS CARD + CENTER MODALS (Inventory / Customizer / Map / Progression)
            root.spawn(NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    flex_grow: 1.0,
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::FlexStart,
                    margin: UiRect::vertical(Val::Px(4.0)),
                    ..default()
                },
                ..default()
            })
            .with_children(|mid| {
                // Left Compact Survival Vitals Card
                mid.spawn((WorldInputBlocker, NodeBundle {
                    style: Style {
                        width: Val::Px(258.0),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(4.0),
                        padding: UiRect::all(Val::Px(10.0)),
                        border: UiRect::all(Val::Px(1.5)),
                        ..default()
                    },
                    background_color: Color::rgba(0.039, 0.066, 0.106, 0.95).into(),
                    border_color: CARD_BORDER.into(),
                    ..default()
                }))
                .with_children(|vitals| {
                    vitals.spawn((
                        DynamicLabel(LabelId::HudVitalsHeader),
                        TextBundle::from_section(
                            crate::locale::text("Skafander"),
                            TextStyle {
                                font_size: 19.0,
                                color: Color::rgb(0.87, 0.96, 1.0),
                                ..default()
                            },
                        ),
                    ));
                    for (metric, title, color) in VITALS {
                        spawn_vital_row(vitals, metric, crate::locale::text(title), color);
                        if metric == BarMetric::HudO2 {
                            vitals.spawn((DynamicLabel(LabelId::HudOxygenHint),TextBundle::from_section(
                                "Sprawdzanie butli…", TextStyle {font_size:12.0,color:Color::rgb(0.66,0.81,0.9),..default()}
                            ).with_style(Style {margin:UiRect::axes(Val::Px(8.0),Val::Px(1.0)),..default()})));
                        }
                    }
                    vitals.spawn(NodeBundle {style:Style {column_gap:Val::Px(6.0),margin:UiRect::vertical(Val::Px(3.0)),..default()},..default()})
                        .with_children(|row| {
                            spawn_consumable_button(row, crate::locale::text("Zjedz [R]"), SurfaceUiCmd::UseRation, LabelId::HudFoodAction, STYLE_EMERALD);
                            spawn_consumable_button(row, crate::locale::text("Wypij [T]"), SurfaceUiCmd::DrinkWater, LabelId::HudWaterAction, STYLE_BLUEPRINT);
                        });
                    vitals.spawn((SupplyControls, NodeBundle {
                        style: Style { display: Display::None, flex_direction: FlexDirection::Column, row_gap: Val::Px(4.0), ..default() },
                        ..default()
                    })).with_children(|station| {
                        spawn_styled_button(station,crate::locale::text("Odłóż ładunek w magazynie"),UiAction::SurfaceCmd(SurfaceUiCmd::StoreCargo),Val::Percent(100.0),Val::Px(28.0),STYLE_SLATE,12.0);
                        spawn_styled_button(station,crate::locale::text("Weź ładunek z magazynu"),UiAction::SurfaceCmd(SurfaceUiCmd::TakeCargo),Val::Percent(100.0),Val::Px(28.0),STYLE_SLATE,12.0);
                        spawn_consumable_button(station, crate::locale::text("Zrób wodę"), SurfaceUiCmd::MakeWater, LabelId::HudMakeWater, STYLE_BLUEPRINT);
                        spawn_consumable_button(station, crate::locale::text("Zrób posiłek"), SurfaceUiCmd::MakeMeal, LabelId::HudMakeMeal, STYLE_EMERALD);
                    });

                    vitals.spawn((
                        DynamicLabel(LabelId::HudToolAndRover),
                        TextBundle::from_section(
                            "...",
                            TextStyle {
                                font_size: 11.5,
                                color: Color::rgb(0.90, 0.95, 1.0),
                                ..default()
                            },
                        )
                        .with_style(Style {
                            margin: UiRect::vertical(Val::Px(3.0)),
                            ..default()
                        }),
                    ));

                    // Quick Survival & Airlock/Rover Buttons with distinct themes
                    vitals
                        .spawn(NodeBundle {
                            style: Style {
                                flex_direction: FlexDirection::Row,
                                flex_wrap: FlexWrap::Wrap,
                                justify_content: JustifyContent::SpaceBetween,
                                ..default()
                            },
                            ..default()
                        })
                        .with_children(|qbtns| {
                            spawn_styled_button(qbtns, crate::locale::text("Zbadaj [P]"), UiAction::CampaignAction, Val::Px(230.0), Val::Px(35.0), STYLE_AMBER, 15.0);
                            spawn_styled_button(qbtns, crate::locale::text("Napraw skafander [F]"), UiAction::SurfaceCmd(SurfaceUiCmd::PatchSuit), Val::Px(230.0), Val::Px(28.0), STYLE_SLATE, 12.0);
                            spawn_styled_button(
                                qbtns,
                                crate::locale::text("Wejdź / wyjdź [E]"),
                                UiAction::SurfaceCmd(SurfaceUiCmd::EnterOrExitBuilding),
                                Val::Px(236.0),
                                Val::Px(28.0),
                                STYLE_VITAL,
                                11.2,
                            );
                            spawn_styled_button(
                                qbtns, crate::locale::text("Pilotuj stację [G]"), UiAction::SurfaceCmd(SurfaceUiCmd::PilotStation),
                                Val::Px(236.0), Val::Px(36.0), STYLE_EMERALD, 15.0,
                            );
                        });
                });

                // Center Modal Container (Holds Inventory, Customizer, World Map & Progression Map)
                mid.spawn(NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    ..default()
                })
                .with_children(|center_modals| {
                    // ================================================================
                    // MODAL A: Graphical Backpack & Workshop (I)
                    // ================================================================
                    center_modals
                        .spawn((
                            InventoryModalNode,
                            WorldInputBlocker,
                            NodeBundle {
                                style: Style {
                                    width: Val::Px(640.0),
                                    flex_direction: FlexDirection::Column,
                                    align_items: AlignItems::Center,
                                    padding: UiRect::all(Val::Px(16.0)),
                                    border: UiRect::all(Val::Px(2.0)),
                                    ..default()
                                },
                                background_color: Color::rgba(0.05, 0.10, 0.20, 0.96).into(),
                                border_color: Color::rgb(1.0, 0.82, 0.32).into(),
                                visibility: Visibility::Hidden,
                                z_index: ZIndex::Global(10),
                                ..default()
                            },
                        ))
                        .with_children(|inv| {
                            inv.spawn(TextBundle::from_section(
                                crate::locale::text("PLECAK · WARSZTAT [I]"),
                                TextStyle {
                                    font_size: 17.5,
                                    color: Color::rgb(1.0, 0.84, 0.36),
                                    ..default()
                                },
                            ));

                            menu_text(inv, crate::locale::text("Przy przedmiocie wybierz, ile chcesz wyrzucić."), 13.0, Color::rgb(0.72, 0.83, 0.92));

                            let slot_meta: [(usize, &str, Handle<Image>, Color); 11] = [
                                (
                                    0,
                                    crate::locale::text("Regolit Księżycowy"),
                                    assets.icon_regolith.clone(),
                                    Color::rgb(0.65, 0.70, 0.78),
                                ),
                                (
                                    2,
                                    crate::locale::text("Lód Wodny (H2O)"),
                                    assets.icon_ice.clone(),
                                    Color::rgb(0.35, 0.90, 1.0),
                                ),
                                (
                                    3,
                                    crate::locale::text("Ruda Tytanu (Ti)"),
                                    assets.icon_titanium.clone(),
                                    Color::rgb(0.82, 0.88, 0.96),
                                ),
                                (
                                    1,
                                    crate::locale::text("Wafle Krzemowe (Si)"),
                                    assets.icon_silicon.clone(),
                                    Color::rgb(0.40, 0.92, 0.68),
                                ),
                                (
                                    4,
                                    crate::locale::text("Izotop Hel-3 (3He)"),
                                    assets.icon_he3.clone(),
                                    Color::rgb(1.0, 0.80, 0.28),
                                ),
                                (
                                    5,
                                    crate::locale::text("Meteoryt (Rzadki)"),
                                    assets.icon_rare.clone(),
                                    Color::rgb(1.0, 0.52, 0.28),
                                ),
                                (
                                    7,
                                    crate::locale::text("Racje [R]"),
                                    assets.icon_ration.clone(),
                                    Color::rgb(0.52, 0.95, 0.42),
                                ),
                                (
                                    10,
                                    crate::locale::text("Apteczki [X]"),
                                    assets.icon_medkit.clone(),
                                    Color::rgb(1.0, 0.48, 0.55),
                                ),
                                (
                                    6,
                                    crate::locale::text("Próbki Naukowe"),
                                    assets.icon_sample.clone(),
                                    Color::rgb(0.76, 0.55, 1.0),
                                ),
                                (8, crate::locale::text("Posiłki ze szklarni [R]"), assets.icon_ration.clone(), Color::rgb(0.68,0.91,0.50)),
                                (9, crate::locale::text("Woda do picia [T]"), assets.icon_drink.clone(), Color::rgb(0.42,0.77,1.0)),
                            ];

                            inv.spawn(NodeBundle {
                                style: Style {
                                    width: Val::Percent(100.0),
                                    flex_direction: FlexDirection::Row,
                                    flex_wrap: FlexWrap::Wrap,
                                    justify_content: JustifyContent::Center,
                                    margin: UiRect::vertical(Val::Px(10.0)),
                                    ..default()
                                },
                                ..default()
                            })
                            .with_children(|grid| {
                                for (kind, title, icon, border_col) in slot_meta {
                                    grid.spawn(NodeBundle {
                                        style: Style {
                                            width: Val::Px(142.0),
                                            height: Val::Px(126.0),
                                            flex_direction: FlexDirection::Column,
                                            justify_content: JustifyContent::SpaceBetween,
                                            align_items: AlignItems::Center,
                                            padding: UiRect::all(Val::Px(6.0)),
                                            margin: UiRect::all(Val::Px(4.0)),
                                            border: UiRect::all(Val::Px(2.0)),
                                            ..default()
                                        },
                                        background_color: Color::rgb(0.08, 0.15, 0.26).into(),
                                        border_color: border_col.into(),
                                        ..default()
                                    })
                                    .with_children(|slot| {
                                        slot.spawn(ImageBundle {
                                            style: Style {
                                                width: Val::Px(25.0),
                                                height: Val::Px(25.0),
                                                ..default()
                                            },
                                            image: UiImage::new(icon),
                                            ..default()
                                        });
                                        slot.spawn(TextBundle::from_section(
                                            title,
                                            TextStyle {
                                                font_size: 11.2,
                                                color: Color::rgb(0.86, 0.92, 1.0),
                                                ..default()
                                            },
                                        ));
                                        slot.spawn((
                                            DynamicLabel(LabelId::InvSlotCount(kind)),
                                            TextBundle::from_section(
                                                "x0",
                                                TextStyle {
                                                    font_size: 15.0,
                                                    color: Color::WHITE,
                                                    ..default()
                                                },
                                            ),
                                        ));
                                        slot.spawn(NodeBundle::default()).with_children(|row| {
                                            for (label, all, width) in [("−1", false, 40.0), (crate::locale::text("Całość"), true, 63.0)] {
                                                spawn_styled_button(
                                                    row, label,
                                                    UiAction::SurfaceCmd(SurfaceUiCmd::DropCargo(kind, all)),
                                                    Val::Px(width), Val::Px(24.0), STYLE_SLATE, 11.0,
                                                );
                                            }
                                        });
                                        spawn_styled_button(slot,"+100 · Sandbox",UiAction::SurfaceCmd(SurfaceUiCmd::GrantCargo(kind,100)),Val::Px(118.0),Val::Px(23.0),STYLE_EMERALD,10.5);
                                    });
                                }
                            });

                            inv.spawn(NodeBundle {
                                style: Style {
                                    flex_direction: FlexDirection::Row,
                                    flex_wrap: FlexWrap::Wrap,
                                    justify_content: JustifyContent::Center,
                                    ..default()
                                },
                                ..default()
                            })
                            .with_children(|wb| {
                                spawn_styled_button(
                                    wb,
                                    crate::locale::text("Ulepsz kilof [C]"),
                                    UiAction::SurfaceCmd(SurfaceUiCmd::CraftNextTool),
                                    Val::Px(245.0),
                                    Val::Px(36.0),
                                    STYLE_AMBER,
                                    12.2,
                                );
                                spawn_styled_button(wb,crate::locale::text("Plecak +15 kg"),UiAction::SurfaceCmd(SurfaceUiCmd::UpgradeBackpack),Val::Px(245.0),Val::Px(36.0),STYLE_EMERALD,12.2);
                                spawn_styled_button(
                                    wb,
                                    crate::locale::text("Zbuduj łazik [V]"),
                                    UiAction::SurfaceCmd(SurfaceUiCmd::FabricateRover),
                                    Val::Px(245.0),
                                    Val::Px(36.0),
                                    STYLE_EMERALD,
                                    12.2,
                                );
                                spawn_styled_button(
                                    wb,
                                    crate::locale::text("Apteczka [X]"),
                                    UiAction::SurfaceCmd(SurfaceUiCmd::UseMedkit),
                                    Val::Px(245.0),
                                    Val::Px(36.0),
                                    STYLE_HUSARIA,
                                    12.2,
                                );
                                spawn_styled_button(
                                    wb,
                                    crate::locale::text("Wystrzel flarę"),
                                    UiAction::SurfaceCmd(SurfaceUiCmd::ThrowSurveyFlare),
                                    Val::Px(245.0),
                                    Val::Px(36.0),
                                    STYLE_BLUEPRINT,
                                    12.2,
                                );
                                spawn_styled_button(
                                    wb,
                                    crate::locale::text("ZAMKNIJ PLECAK [I]"),
                                    UiAction::ToggleInventoryModal,
                                    Val::Px(210.0),
                                    Val::Px(34.0),
                                    STYLE_SLATE,
                                    12.2,
                                );
                            });
                        });

                    // ================================================================
                    // MODAL B: Live Character Customizer & Self-Expression (K)
                    // ================================================================
                    center_modals
                        .spawn((
                            CustomizerModalNode,
                            NodeBundle {
                                style: Style {
                                    width: Val::Px(560.0),
                                    flex_direction: FlexDirection::Column,
                                    align_items: AlignItems::Center,
                                    padding: UiRect::all(Val::Px(16.0)),
                                    border: UiRect::all(Val::Px(2.5)),
                                    ..default()
                                },
                                background_color: Color::rgba(0.08, 0.06, 0.16, 0.96).into(),
                                border_color: Color::rgb(1.0, 0.82, 0.36).into(),
                                visibility: Visibility::Hidden,
                                ..default()
                            },
                        ))
                        .with_children(|cust| {
                            cust.spawn(TextBundle::from_section(
                                crate::locale::text("KREATOR POSTACI NA ŻYWO & WYRAŻENIE SIEBIE [K]"),
                                TextStyle {
                                    font_size: 17.5,
                                    color: Color::rgb(1.0, 0.84, 0.38),
                                    ..default()
                                },
                            ));
                            cust.spawn(TextBundle::from_section(
                                crate::locale::text("Zmieniaj wygląd skafandra, skrzydła husarskie, smugę odrzutową i członka załogi w dowolnej chwili!"),
                                TextStyle {
                                    font_size: 11.8,
                                    color: Color::rgb(0.86, 0.92, 1.0),
                                    ..default()
                                },
                            ));

                            spawn_selector_row(
                                cust,
                                crate::locale::text("Kryptonim:"),
                                MdlOption::Callsign,
                                LabelId::MdlCallsign,
                                STYLE_HUSARIA,
                            );
                            spawn_selector_row(
                                cust,
                                crate::locale::text("Kolor Skafandra:"),
                                MdlOption::Suit,
                                LabelId::MdlSuit,
                                STYLE_HUSARIA,
                            );
                            spawn_selector_row(
                                cust,
                                crate::locale::text("Wizjer Hełmu:"),
                                MdlOption::Visor,
                                LabelId::MdlVisor,
                                STYLE_HUSARIA,
                            );
                            spawn_selector_row(
                                cust,
                                crate::locale::text("Ozdoba Hełmu:"),
                                MdlOption::Accessory,
                                LabelId::MdlAccessory,
                                STYLE_HUSARIA,
                            );
                            spawn_selector_row(
                                cust,
                                crate::locale::text("Smuga Plecaka RCS:"),
                                MdlOption::TrailColor,
                                LabelId::MdlTrail,
                                STYLE_HUSARIA,
                            );
                            spawn_selector_row(
                                cust,
                                crate::locale::text("Specjalizacja:"),
                                MdlOption::Trait,
                                LabelId::MdlTrait,
                                STYLE_HUSARIA,
                            );

                            cust.spawn(NodeBundle {
                                style: Style {
                                    flex_direction: FlexDirection::Row,
                                    flex_wrap: FlexWrap::Wrap,
                                    justify_content: JustifyContent::Center,
                                    margin: UiRect::top(Val::Px(8.0)),
                                    ..default()
                                },
                                ..default()
                            })
                            .with_children(|cbtns| {
                                spawn_styled_button(
                                    cbtns,
                                    crate::locale::text("POSTAW FLAGĘ PL [H]"),
                                    UiAction::SurfaceCmd(SurfaceUiCmd::PlantPolishFlag),
                                    Val::Px(210.0),
                                    Val::Px(34.0),
                                    STYLE_HUSARIA,
                                    12.0,
                                );
                                spawn_styled_button(
                                    cbtns,
                                    crate::locale::text("ZMIEŃ ASTRONAUTĘ [Z]"),
                                    UiAction::SurfaceCmd(SurfaceUiCmd::SwitchCrewMember),
                                    Val::Px(210.0),
                                    Val::Px(34.0),
                                    STYLE_VITAL,
                                    12.0,
                                );
                                spawn_styled_button(
                                    cbtns,
                                    crate::locale::text("ZAŁOGA CO-OP AI: WŁ/WYŁ"),
                                    UiAction::ToggleAiCrew,
                                    Val::Px(210.0),
                                    Val::Px(34.0),
                                    STYLE_BLUEPRINT,
                                    12.0,
                                );
                                spawn_styled_button(
                                    cbtns,
                                    "ZAMKNIJ KREATOR [K]",
                                    UiAction::ToggleCustomizerModal,
                                    Val::Px(210.0),
                                    Val::Px(34.0),
                                    STYLE_SLATE,
                                    12.0,
                                );
                            });
                        });

                    // ================================================================
                    // MODAL C: Interactive World Map, Waypoints & Crew/LAN Tracker [M]
                    // ================================================================
                    center_modals
                        .spawn((
                            MapModalNode,
                            NodeBundle {
                                style: Style {
                                    width: Val::Px(690.0),
                                    flex_direction: FlexDirection::Column,
                                    align_items: AlignItems::Center,
                                    padding: UiRect::all(Val::Px(16.0)),
                                    border: UiRect::all(Val::Px(2.5)),
                                    ..default()
                                },
                                background_color: Color::rgba(0.03, 0.10, 0.18, 0.96).into(),
                                border_color: Color::rgb(0.35, 0.95, 0.72).into(),
                                visibility: Visibility::Hidden,
                                ..default()
                            },
                        ))
                        .with_children(|map_ui| {
                            map_ui.spawn(TextBundle::from_section(
                                crate::locale::text("Mapa Księżyca"),
                                TextStyle {
                                    font_size: 17.0,
                                    color: Color::rgb(0.38, 0.98, 0.76),
                                    ..default()
                                },
                            ));

                            spawn_map_grid(map_ui);
                            menu_text(map_ui, crate::locale::text("Biały: ty   •   Turkusowy: stacja   •   Fioletowy: punkt badań   •   Złoty: znacznik"), 14.0, Color::WHITE);
                            menu_text(map_ui, crate::locale::text("Kliknij pole na mapie, aby je oznaczyć."), 14.0, Color::WHITE);

                            map_ui
                                .spawn(NodeBundle {
                                    style: Style {
                                        flex_direction: FlexDirection::Row,
                                        flex_wrap: FlexWrap::Wrap,
                                        justify_content: JustifyContent::Center,
                                        margin: UiRect::top(Val::Px(8.0)),
                                        ..default()
                                    },
                                    ..default()
                                })
                                .with_children(|mbtns| {
                                    spawn_styled_button(
                                        mbtns,
                                        crate::locale::text("Oznacz, gdzie jestem"),
                                        UiAction::SurfaceCmd(SurfaceUiCmd::AddWaypointAtPlayer),
                                        Val::Px(260.0),
                                        Val::Px(34.0),
                                        STYLE_EMERALD,
                                        12.0,
                                    );
                                    spawn_styled_button(
                                        mbtns,
                                        crate::locale::text("[- USUŃ WAYPOINT] Usuń ostatni punkt"),
                                        UiAction::SurfaceCmd(SurfaceUiCmd::RemoveLastWaypoint),
                                        Val::Px(260.0),
                                        Val::Px(34.0),
                                        STYLE_DANGER,
                                        12.0,
                                    );
                                    spawn_styled_button(
                                        mbtns,
                                        crate::locale::text("Zamknij [M]"),
                                        UiAction::ToggleMapModal,
                                        Val::Px(260.0),
                                        Val::Px(34.0),
                                        STYLE_SLATE,
                                        12.0,
                                    );
                                });
                        });

                    // ================================================================
                    // MODAL D: Progression Map & Achievements [J] (Green / Gold / Gray!)
                    // ================================================================
                    center_modals
                        .spawn((
                            ProgressionModalNode,
                            NodeBundle {
                                style: Style {
                                    width: Val::Px(740.0),
                                    flex_direction: FlexDirection::Column,
                                    align_items: AlignItems::FlexStart,
                                    padding: UiRect::all(Val::Px(14.0)),
                                    border: UiRect::all(Val::Px(2.5)),
                                    ..default()
                                },
                                background_color: Color::rgba(0.04, 0.08, 0.16, 0.96).into(),
                                border_color: Color::rgb(1.0, 0.84, 0.36).into(),
                                visibility: Visibility::Hidden,
                                ..default()
                            },
                        ))
                        .with_children(|prog| {
                            prog.spawn((
                                DynamicLabel(LabelId::ProgModalHeader),
                                TextBundle::from_section(
                                    crate::locale::text("MAPA POSTĘPU (CO ROBIĆ DALEJ?) ORAZ OSIĄGNIĘCIA [J]"),
                                    TextStyle {
                                        font_size: 16.5,
                                        color: Color::rgb(1.0, 0.86, 0.36),
                                        ..default()
                                    },
                                ),
                            ));

                            prog.spawn(
                                TextBundle::from_section(
                                    crate::locale::text("Kolejne etapy rozwoju bazy (Zielone = ukończone, Złote = aktualny cel, Szare = jeszcze nieodblokowane):"),
                                    TextStyle {
                                        font_size: 11.8,
                                        color: Color::rgb(0.82, 0.90, 1.0),
                                        ..default()
                                    },
                                )
                                .with_style(Style {
                                    margin: UiRect::vertical(Val::Px(3.0)),
                                    ..default()
                                }),
                            );

                            for idx in 0..8 {
                                prog.spawn((
                                    DynamicLabel(LabelId::ProgStepRow(idx)),
                                    TextBundle::from_section(
                                        "...",
                                        TextStyle {
                                            font_size: 12.0,
                                            color: Color::rgb(0.50, 0.54, 0.60),
                                            ..default()
                                        },
                                    )
                                    .with_style(Style {
                                        margin: UiRect::vertical(Val::Px(1.5)),
                                        ..default()
                                    }),
                                ));
                            }

                            prog.spawn(
                                TextBundle::from_section(
                                    crate::locale::text("--- OSIĄGNIĘCIA GRACZA (ODBLOKOWANE vs SZARE) ---"),
                                    TextStyle {
                                        font_size: 13.5,
                                        color: Color::rgb(0.40, 0.92, 1.0),
                                        ..default()
                                    },
                                )
                                .with_style(Style {
                                    margin: UiRect::top(Val::Px(6.0)),
                                    ..default()
                                }),
                            );

                            for idx in 0..8 {
                                prog.spawn((
                                    DynamicLabel(LabelId::AchievementRow(idx)),
                                    TextBundle::from_section(
                                        "...",
                                        TextStyle {
                                            font_size: 11.6,
                                            color: Color::rgb(0.50, 0.54, 0.60),
                                            ..default()
                                        },
                                    )
                                    .with_style(Style {
                                        margin: UiRect::vertical(Val::Px(1.0)),
                                        ..default()
                                    }),
                                ));
                            }

                            prog.spawn(NodeBundle {
                                style: Style {
                                    width: Val::Percent(100.0),
                                    flex_direction: FlexDirection::Row,
                                    justify_content: JustifyContent::SpaceBetween,
                                    align_items: AlignItems::Center,
                                    margin: UiRect::top(Val::Px(6.0)),
                                    ..default()
                                },
                                ..default()
                            })
                            .with_children(|bot_row| {
                                bot_row.spawn(TextBundle::from_section(
                                    GAME_CREATORS_LINE,
                                    TextStyle {
                                        font_size: 11.5,
                                        color: Color::rgb(0.95, 0.88, 0.58),
                                        ..default()
                                    },
                                ));
                                spawn_styled_button(
                                    bot_row,
                                    crate::locale::text("ZAMKNIJ OKNO POSTĘPU [J]"),
                                    UiAction::ToggleProgressionModal,
                                    Val::Px(210.0),
                                    Val::Px(32.0),
                                    STYLE_SLATE,
                                    12.0,
                                );
                            });
                        });
                });

                mid.spawn(NodeBundle {
                    style: Style {
                        width: Val::Px(8.0),
                        ..default()
                    },
                    ..default()
                });
            });

            // 3. OPTIONAL ON-SCREEN MOBILE / TOUCH CONTROLS OVERLAY
            root.spawn((
                TouchControlsNode,
                NodeBundle {
                    style: Style {
                        width: Val::Percent(100.0),
                        flex_direction: FlexDirection::Row,
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::Center,
                        padding: UiRect::horizontal(Val::Px(18.0)),
                        margin: UiRect::bottom(Val::Px(4.0)),
                        ..default()
                    },
                    visibility: Visibility::Hidden,
                    ..default()
                },
            ))
            .with_children(|touch| {
                touch
                    .spawn(NodeBundle {
                        style: Style {
                            flex_direction: FlexDirection::Row,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        ..default()
                    })
                    .with_children(|dpad| {
                        spawn_styled_button(
                            dpad,
                            crate::locale::text("LEWO"),
                            UiAction::TouchMove(Vec2::new(-1.0, 0.0)),
                            Val::Px(64.0),
                            Val::Px(46.0),
                            STYLE_BLUEPRINT,
                            13.5,
                        );
                        spawn_styled_button(
                            dpad,
                            crate::locale::text("GÓRA"),
                            UiAction::TouchMove(Vec2::new(0.0, 1.0)),
                            Val::Px(64.0),
                            Val::Px(46.0),
                            STYLE_BLUEPRINT,
                            13.5,
                        );
                        spawn_styled_button(
                            dpad,
                            crate::locale::text("DÓŁ"),
                            UiAction::TouchMove(Vec2::new(0.0, -1.0)),
                            Val::Px(64.0),
                            Val::Px(46.0),
                            STYLE_BLUEPRINT,
                            13.5,
                        );
                        spawn_styled_button(
                            dpad,
                            crate::locale::text("PRAWO"),
                            UiAction::TouchMove(Vec2::new(1.0, 0.0)),
                            Val::Px(64.0),
                            Val::Px(46.0),
                            STYLE_BLUEPRINT,
                            13.5,
                        );
                    });

                touch
                    .spawn(NodeBundle {
                        style: Style {
                            flex_direction: FlexDirection::Row,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        ..default()
                    })
                    .with_children(|abtns| {
                        spawn_styled_button(
                            abtns,
                            crate::locale::text("Skok"),
                            UiAction::SurfaceCmd(SurfaceUiCmd::TriggerJump),
                            Val::Px(88.0),
                            Val::Px(46.0),
                            STYLE_EMERALD,
                            13.5,
                        );
                        spawn_styled_button(
                            abtns,
                            "JETPACK",
                            UiAction::TouchJetpack,
                            Val::Px(88.0),
                            Val::Px(46.0),
                            STYLE_HUSARIA,
                            13.5,
                        );
                        spawn_styled_button(
                            abtns,
                            crate::locale::text("AKCJA"),
                            UiAction::SurfaceCmd(SurfaceUiCmd::MineOrBuildAtPlayer),
                            Val::Px(95.0),
                            Val::Px(46.0),
                            STYLE_AMBER,
                            13.5,
                        );
                    });
            });

            // 4. BOTTOM HOTBAR WITH HIGH-VISIBILITY 2-LINE INFO BANNER + MINING PROGRESS BAR!
            root.spawn((WorldInputBlocker, NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(8.0)),
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                background_color: CARD_BG.into(),
                border_color: CARD_BORDER.into(),
                ..default()
            }))
            .with_children(|bot| {
                // HIGH-VISIBILITY INFO PANEL ABOVE HOTBAR (Large 17.5px & 14.5px text + Mining Bar)
                bot.spawn(NodeBundle {
                    style: Style {
                        width: Val::Percent(100.0),
                        flex_direction: FlexDirection::Column,
                        padding: UiRect::axes(Val::Px(12.0), Val::Px(6.0)),
                        margin: UiRect::bottom(Val::Px(6.0)),
                        border: UiRect::all(Val::Px(1.5)),
                        ..default()
                    },
                    background_color: Color::rgba(0.03, 0.07, 0.14, 0.95).into(),
                    border_color: Color::rgba(1.0, 0.84, 0.36, 0.75).into(),
                    ..default()
                })
                .with_children(|info_box| {
                    // Line 1: Large Bold Selected Tool / Building Title + Simple Kid-Friendly Explanation
                    info_box.spawn((
                        DynamicLabel(LabelId::HudBigSelectionTitle),
                        TextBundle::from_section(
                            crate::locale::text("Przytrzymaj lewy przycisk myszy na skale."),
                            TextStyle {
                                font_size: 17.2,
                                color: Color::rgb(1.0, 0.90, 0.42),
                                ..default()
                            },
                        ),
                    ));

                    info_box.spawn((DynamicLabel(LabelId::CargoWeight), TextBundle::from_section(crate::locale::text("Plecak 0 / 35 kg"),TextStyle{font_size:14.0,color:Color::rgb(0.70,0.86,0.91),..default()})));
                    // Line 2: Hovered Block Info + Mining Progress % + Latest Action Feedback
                    info_box.spawn((
                        DynamicLabel(LabelId::HudBigTileAndMiningInfo),
                        TextBundle::from_section(
                            crate::locale::text("Witaj na Księżycu!"),
                            TextStyle {
                                font_size: 14.5,
                                color: Color::rgb(0.88, 0.96, 1.0),
                                ..default()
                            },
                        )
                        .with_style(Style {
                            margin: UiRect::vertical(Val::Px(2.0)),
                            ..default()
                        }),
                    ));

                    // Visual Block Mining Progress Bar (0% -> 100%)
                    info_box
                        .spawn(NodeBundle {
                            style: Style {
                                width: Val::Percent(100.0),
                                height: Val::Px(9.0),
                                border: UiRect::all(Val::Px(1.0)),
                                margin: UiRect::top(Val::Px(2.0)),
                                ..default()
                            },
                            background_color: Color::rgb(0.05, 0.09, 0.16).into(),
                            border_color: Color::rgba(0.40, 0.85, 1.0, 0.65).into(),
                            ..default()
                        })
                        .with_children(|mbar_bg| {
                            mbar_bg.spawn((
                                BarFillKind(BarMetric::HudMiningProgress),
                                NodeBundle {
                                    style: Style {
                                        width: Val::Percent(0.0),
                                        height: Val::Percent(100.0),
                                        ..default()
                                    },
                                    background_color: Color::rgb(1.0, 0.78, 0.22).into(),
                                    ..default()
                                },
                            ));
                        });
                });

                // Building choices are shown only while building.
                bot.spawn((BuildingDock, NodeBundle {
                    style: Style {
                        width: Val::Percent(100.0),
                        flex_direction: FlexDirection::Row,
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    ..default()
                }))
                .with_children(|dock| {
                    // SLOT 0: Dedicated Pickaxe / Deconstruct / Collect Tool [Q / B]
                    dock.spawn((
                        ButtonBundle {
                            style: Style {
                                width: Val::Percent(10.5),
                                height: Val::Px(64.0),
                                flex_direction: FlexDirection::Column,
                                justify_content: JustifyContent::SpaceBetween,
                                align_items: AlignItems::Center,
                                padding: UiRect::all(Val::Px(3.0)),
                                border: UiRect::all(Val::Px(2.0)),
                                ..default()
                            },
                            background_color: STYLE_AMBER.normal_bg.into(),
                            border_color: STYLE_AMBER.border_col.into(),
                            ..default()
                        },
                        STYLE_AMBER,
                        HotbarSlotButton(None),
                        UiAction::SurfaceCmd(SurfaceUiCmd::SetBuildMode(false)),
                    ))
                    .with_children(|b| {
                        b.spawn(ImageBundle {
                            style: Style {
                                width: Val::Px(26.0),
                                height: Val::Px(26.0),
                                ..default()
                            },
                            image: UiImage::new(assets.icon_pickaxe.clone()),
                            ..default()
                        });
                        b.spawn((
                            DynamicLabel(LabelId::PickaxeSlotBtn),
                            TextBundle::from_section(
                                crate::locale::text("Kilof [Q]"),
                                TextStyle {
                                    font_size: 11.2,
                                    color: Color::WHITE,
                                    ..default()
                                },
                            ),
                        ));
                    });

                    // SLOTS 1..0: 10 Distinct Building Cards with their 2.5D Isometric Model Icons!
                    for (idx, st) in StructureKind::ALL.iter().enumerate() {
                        let key_num = if idx == 9 { 0 } else { idx + 1 };
                        let (r, ti, si, _) = st.build_cost();
                        dock.spawn((
                            ButtonBundle {
                                style: Style {
                                    width: Val::Percent(8.7),
                                    height: Val::Px(64.0),
                                    flex_direction: FlexDirection::Column,
                                    justify_content: JustifyContent::SpaceBetween,
                                    align_items: AlignItems::Center,
                                    padding: UiRect::all(Val::Px(2.5)),
                                    border: UiRect::all(Val::Px(1.5)),
                                    ..default()
                                },
                                background_color: STYLE_BLUEPRINT.normal_bg.into(),
                                border_color: STYLE_BLUEPRINT.border_col.into(),
                                ..default()
                            },
                            STYLE_BLUEPRINT,
                            HotbarSlotButton(Some(idx)),
                            UiAction::SurfaceCmd(SurfaceUiCmd::SelectStructure(idx)),
                        ))
                        .with_children(|b| {
                            b.spawn(ImageBundle {
                                style: Style {
                                    width: Val::Px(28.0),
                                    height: Val::Px(28.0),
                                    ..default()
                                },
                                image: UiImage::new(assets.structure_handle(*st)),
                                ..default()
                            });
                            b.spawn((
                                DynamicLabel(LabelId::BuildSlotBtn(idx)),
                                TextBundle::from_section(
                                    format!(
                                        "[{}] {}\n{}R {}Ti {}Si",
                                        key_num,
                                        st.short_name(),
                                        r,
                                        ti,
                                        si
                                    ),
                                    TextStyle {
                                        font_size: 10.8,
                                        color: Color::WHITE,
                                        ..default()
                                    },
                                ),
                            ));
                        });
                    }
                });
            });
        });
}

#[derive(Component)]
pub struct PlayerListPanel;
#[derive(Component)]
pub struct PlayerListText;

pub fn update_player_list(
    keys: Res<ButtonInput<KeyCode>>,
    lan: Res<LanMultiplayer>,
    config: Res<MissionConfig>,
    mut panels: Query<&mut Style, With<PlayerListPanel>>,
    mut texts: Query<&mut Text, With<PlayerListText>>,
) {
    let show = lan.is_active && keys.pressed(KeyCode::Tab);
    for mut panel in &mut panels {
        panel.display = if show { Display::Flex } else { Display::None };
    }
    if !show {
        return;
    }
    let ping = |value: Option<u32>| {
        value.map_or_else(
            || crate::locale::text("mierzę…").into(),
            |value| format!("{value} ms"),
        )
    };
    let mut rows = vec![format!(
        "{} · Ty · {}",
        config.callsign,
        if lan.is_host {
            "host".into()
        } else {
            ping(lan.latency_ms)
        }
    )];
    for peer in lan.peers.iter().filter(|peer| !peer.is_ai_bot) {
        rows.push(format!(
            "{} · {} · {}",
            peer.callsign,
            peer.body.name(),
            ping(
                lan.peer_pings
                    .get(&peer.peer_id)
                    .copied()
                    .or(peer.latency_ms)
            )
        ));
    }
    let content = moon_base_localize::localized_format!(
        "GRACZE · {}/4\n\n{}\n\nPuść Tab, aby wrócić do gry",
        rows.len(),
        rows.join("\n")
    );
    for mut text in &mut texts {
        text.sections[0].value = content.clone();
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn update_surface_hud(
    lan: Option<Res<LanMultiplayer>>,
    config: Res<MissionConfig>,
    settings: Res<GameSettings>,
    map: Res<LunarMap>,
    stats: Res<SurvivalStats>,
    events: Res<LunarEventManager>,
    player_q: Query<&AstronautPlayer>,
    mut modal_vis_q: Query<
        (
            &mut Visibility,
            &mut Style,
            Option<&InventoryModalNode>,
            Option<&CustomizerModalNode>,
            Option<&MapModalNode>,
            Option<&ProgressionModalNode>,
            Option<&TouchControlsNode>,
            Option<&QuestDetails>,
            Option<&QuestPanel>,
        ),
        (Without<BarFillKind>, Without<BuildingDock>),
    >,
    mut building_dock: Query<&mut Style, (With<BuildingDock>, Without<BarFillKind>)>,
    mut hotbar_btn_q: Query<(&HotbarSlotButton, &mut ButtonVisualStyle)>,
    mut bar_q: Query<(&BarFillKind, &mut Style, &mut BackgroundColor)>,
    mut label_q: Query<(&DynamicLabel, &mut Text)>,
) {
    for (
        mut vis,
        mut layout,
        is_inv,
        is_cust,
        is_map,
        is_prog,
        is_touch,
        quest_details,
        quest_panel,
    ) in &mut modal_vis_q
    {
        if quest_panel.is_some() {
            layout.display = if map.show_inventory_window || map.quests_disabled {
                Display::None
            } else {
                Display::Flex
            };
        }
        if quest_details.is_some() {
            layout.display = if map.quest_panel_collapsed {
                Display::None
            } else {
                Display::Flex
            };
        }
        if is_inv.is_some() {
            *vis = if map.show_inventory_window {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
        } else if is_cust.is_some() {
            *vis = if map.show_customizer_window {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
        } else if is_map.is_some() {
            *vis = if map.show_map_window {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
        } else if is_prog.is_some() {
            *vis = if map.show_progression_window && !map.quests_disabled {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
        } else if is_touch.is_some() {
            *vis = if settings.show_touch_controls {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
        }
        if is_inv.is_some()
            || is_cust.is_some()
            || is_map.is_some()
            || is_prog.is_some()
            || is_touch.is_some()
        {
            layout.display = if *vis == Visibility::Hidden {
                Display::None
            } else {
                Display::Flex
            };
        }
    }

    for mut style in &mut building_dock {
        style.display = if map.build_mode && !map.piloting_station && !map.any_modal_open() {
            Display::Flex
        } else {
            Display::None
        };
    }

    // Highlight active hotbar slot (Pickaxe vs Building 1..0)
    for (slot_btn, mut style) in &mut hotbar_btn_q {
        match slot_btn.0 {
            None => {
                *style = if !map.build_mode {
                    STYLE_AMBER
                } else {
                    STYLE_SLATE
                };
            }
            Some(idx) => {
                *style = if map.build_mode && map.selected_build == idx {
                    STYLE_EMERALD
                } else {
                    STYLE_BLUEPRINT
                };
            }
        }
    }

    for (bar, mut style, mut color) in &mut bar_q {
        if let Some(percent) = vital_percent(bar.0, &stats) {
            style.width = Val::Percent(percent);
            *color = vital_color(bar.0, percent).into();
        } else if bar.0 == BarMetric::HudMiningProgress {
            style.width = Val::Percent((map.mining_progress * 100.0).clamp(0.0, 100.0));
        }
    }

    let pill_values = [
        stats.regolith,
        stats.water_ice,
        stats.titanium,
        stats.silicon,
        stats.helium3,
        stats.rations + stats.fresh_meals,
        stats.medkits,
    ];

    let inv_counts = stats.cargo_counts();

    let base_modules_built = map
        .cells
        .values()
        .filter(|c| c.structure.is_some() && c.structure_hp > 0.0 && !c.is_world_generated_base)
        .count();
    let active_prog_step = stats.current_active_step(&map);

    let player_opt = player_q.get_single().ok();

    for (lbl, mut text) in &mut label_q {
        match lbl.0 {
            LabelId::HudResourcePill(idx) => {
                if let Some(&v) = pill_values.get(idx) {
                    text.sections[0].value = format!("{}", v);
                }
            }
            LabelId::HudTopMission => {
                if map.quests_disabled {
                    text.sections[0].value =
                        crate::locale::text("Sandbox · swobodna eksploracja").into();
                    continue;
                }
                if let Some(campaign) = &stats.campaign {
                    let step = Quest::ALL
                        .iter()
                        .position(|&q| q == campaign.quest)
                        .map_or(8, |index| index + 1);
                    text.sections[0].value = format!("{}/8 · {}", step, campaign.quest.title());
                    continue;
                }
                let objective = if map.body != WorldBody::Moon && map.surveyed.count_ones() < 4 {
                    format!(
                        "Zbadaj fioletowe punkty {}/4 [P]",
                        map.surveyed.count_ones()
                    )
                } else if !stats.collected_minerals {
                    if stats.blocks_mined == 0 {
                        crate::locale::text("Pierwszy cel · Wykop skałę").into()
                    } else {
                        crate::locale::text("Pierwszy cel · Zbierz odłamki").into()
                    }
                } else if !stats.entered_building_once {
                    crate::locale::text("Następny cel · Wejdź do bazy [E]").into()
                } else if !stats.flew_station {
                    crate::locale::text("Następny cel · Poleć stacją [G]").into()
                } else if stats.mineral_catalog.count_ones() < 4 {
                    moon_base_localize::localized_format!(
                        "Atlas minerałów · {}/4",
                        stats.mineral_catalog.count_ones()
                    )
                } else if !stats.geology_report_delivered {
                    crate::locale::text("Atlas gotowy · Wróć do bazy").into()
                } else {
                    moon_base_localize::localized_format!(
                        "Atlas ukończony · Dane {:.0}/{:.0} GB",
                        stats.transmitted_science_gb,
                        config.scenario.target_science_gb()
                    )
                };
                text.sections[0].value = format!("{} · {}", map.body.name(), objective);
            }
            LabelId::HudQuestInstruction => {
                text.sections[0].value = stats.campaign.as_ref().map_or_else(
                    || {
                        crate::locale::text("Odkrywaj świat i badaj fioletowe znaczniki [P].")
                            .into()
                    },
                    |c| {
                        campaign::guidance(
                            c,
                            &map,
                            player_opt,
                            lan.as_ref().is_some_and(|l| l.is_active),
                        )
                    },
                );
            }
            LabelId::HudQuestToggle => {
                text.sections[0].value = if map.quest_panel_collapsed {
                    crate::locale::text("Rozwiń zadania")
                } else {
                    crate::locale::text("Zwiń zadania")
                }
                .into();
            }
            LabelId::HudTopEvent => {
                text.sections[0].value = match events.active_event {
                    LunarEventKind::Nominal => "",
                    LunarEventKind::DustStorm => {
                        crate::locale::text("Burza pyłowa · Panele dają mniej prądu")
                    }
                    LunarEventKind::MeteoriteShower => {
                        crate::locale::text("Spadają meteoryty! Schowaj się w bazie.")
                    }
                    LunarEventKind::SolarFlareStorm => {
                        crate::locale::text("Burza słoneczna! Schowaj się w bazie.")
                    }
                    LunarEventKind::Moonquake => {
                        crate::locale::text("Trzęsienie gruntu! Wejdź do bazy.")
                    }
                    LunarEventKind::LunarNightDeepFreeze => {
                        crate::locale::text("Zimna noc. Pilnuj baterii skafandra.")
                    }
                    LunarEventKind::OrbitalSupplyDrop => {
                        crate::locale::text("W pobliżu wylądowały zapasy.")
                    }
                }
                .into();
                text.sections[0].style.color = events.active_event.color();
            }
            LabelId::HudVitalsHeader => {
                text.sections[0].value = if player_opt
                    .is_some_and(|p| p.is_passenger && p.mounted_rover.is_some())
                {
                    crate::locale::text("Jesteś pasażerem\nKierowca prowadzi · E — wysiądź").into()
                } else if map.piloting_station {
                    crate::locale::text("Kokpit stacji")
                } else if player_opt.is_some_and(|p| p.inside_building) {
                    crate::locale::text("W bazie")
                } else if player_opt.is_some_and(|p| p.mounted_rover.is_some()) {
                    crate::locale::text("W łaziku")
                } else {
                    crate::locale::text("Skafander")
                }
                .into();
            }
            LabelId::HudVitalValue(metric) => {
                if let Some(percent) = vital_percent(metric, &stats) {
                    text.sections[0].value = format!("{:.0}%", percent);
                    text.sections[0].style.color = vital_color(metric, percent);
                }
            }
            LabelId::HudOxygenHint => {
                let percent = vital_percent(BarMetric::HudO2, &stats).unwrap_or_default();
                text.sections[0].style.color = if percent < 30.0 {
                    vital_color(BarMetric::HudO2, percent)
                } else {
                    Color::rgb(0.66, 0.81, 0.9)
                };
                text.sections[0].value = if stats.oxygen_use_per_sec <= 0.0 {
                    if percent >= 99.5 {
                        crate::locale::text("Butla pełna").into()
                    } else {
                        crate::locale::text("Uzupełnianie tlenu").into()
                    }
                } else {
                    let seconds = (stats.oxygen / stats.oxygen_use_per_sec).ceil() as u32;
                    if seconds < 30 {
                        moon_base_localize::localized_format!("Wróć do bazy · ok. {} s", seconds)
                    } else {
                        moon_base_localize::localized_format!(
                            "Zapas na ok. {}:{:02}",
                            seconds / 60,
                            seconds % 60
                        )
                    }
                };
            }
            LabelId::HudFoodAction => {
                text.sections[0].value =
                    format!("Zjedz [R] · {}", stats.rations + stats.fresh_meals);
            }
            LabelId::HudWaterAction => {
                text.sections[0].value = format!("Wypij [T] · {}", stats.drinking_water);
            }
            LabelId::HudMakeWater | LabelId::HudMakeMeal => {
                use crate::base::SupplyRecipe;
                let (recipe, title, detail) = if lbl.0 == LabelId::HudMakeWater {
                    (
                        SupplyRecipe::Water,
                        crate::locale::text("Zrób wodę"),
                        crate::locale::text("1 lód → 2 porcje · 4 s"),
                    )
                } else {
                    (
                        SupplyRecipe::Meal,
                        crate::locale::text("Zrób posiłek"),
                        crate::locale::text("1 woda + szklarnia · 12 s"),
                    )
                };
                text.sections[0].value = if let Some(job) = recipe.job(&map) {
                    if recipe.can_run(&map, job.station) {
                        format!("{} · {:.0} s", title, job.remaining.ceil())
                    } else {
                        format!("{} · wstrzymano", title)
                    }
                } else if player_opt.is_some_and(|p| {
                    !recipe.can_run(
                        &map,
                        (p.iso_pos.x.round() as i32, p.iso_pos.y.round() as i32),
                    )
                }) {
                    if lbl.0 == LabelId::HudMakeWater {
                        crate::locale::text("Woda · podejdź do elektrolizera").into()
                    } else {
                        crate::locale::text("Posiłek · podejdź do szklarni").into()
                    }
                } else {
                    format!("{}\n{}", title, detail)
                };
            }
            LabelId::HudToolAndRover => {
                text.sections[0].value = if map.piloting_station {
                    let pos = map.station_position;
                    let height = pos.z - map.get_elevation(pos.x, pos.y);
                    moon_base_localize::localized_format!(
                        "Wysokość {:.1} m\n{}",
                        height,
                        if height <= 0.1 {
                            crate::locale::text("Na ziemi")
                        } else if crate::flight::landing_clear(&map, pos.truncate()) {
                            crate::locale::text("Możesz tu lądować")
                        } else {
                            crate::locale::text("Poszukaj płaskiego miejsca")
                        }
                    )
                } else if stats.oxygen < stats.max_oxygen * 0.25 {
                    crate::locale::text("Mało tlenu! Wróć do bazy.").into()
                } else if player_opt.is_some_and(|p| p.inside_building) {
                    moon_base_localize::localized_format!(
                        "Prąd {} · Tlen {}\nBilans {:+.0} W · Zapas {:.0}%",
                        if stats.local_powered {
                            crate::locale::text("działa")
                        } else {
                            crate::locale::text("brak")
                        },
                        if stats.local_oxygen {
                            crate::locale::text("działa")
                        } else {
                            crate::locale::text("z butli")
                        },
                        stats.local_power_w,
                        stats.local_charge
                    )
                } else {
                    moon_base_localize::localized_format!(
                        "Na zewnątrz {:+.0} °C · Ciało {:.1} °C\nGrawitacja {:.2} m/s² · Dawka {:.2} mSv{}",
                        stats.outside_temperature_c,
                        stats.body_temp_c, map.body.gravity(), stats.radiation,
                        if map.body == WorldBody::Venus {crate::locale::text("\nCiśnienie ok. 93 bar · skafander fikcyjny")} else {""},
                    )
                };
            }
            LabelId::HudBigSelectionTitle => {
                text.sections[0].value = if let Some(campaign) = &stats.campaign {
                    campaign::guidance(
                        campaign,
                        &map,
                        player_opt,
                        lan.as_ref().is_some_and(|l| l.is_active),
                    )
                } else if map.piloting_station {
                    crate::locale::text(
                        "Spacja ↑  ·  WASD lot  ·  Ctrl ↓  ·  G wyjście  ·  N podróż",
                    )
                    .into()
                } else if let Some((_, point)) =
                    map.survey_points()
                        .into_iter()
                        .enumerate()
                        .find(|(i, point)| {
                            map.surveyed & (1 << i) == 0
                                && player_opt.is_some_and(|p| {
                                    p.iso_pos.truncate().distance(point.pos.as_vec2()) < 1.7
                                })
                        })
                {
                    format!("P — zbadaj: {}", point.name)
                } else if map.build_mode {
                    let st = StructureKind::ALL[map.selected_build];
                    let (rock, metal, silicon, helium) = st.build_cost();
                    let helium = if helium > 0 {
                        format!(", {} helu", helium)
                    } else {
                        String::new()
                    };
                    moon_base_localize::localized_format!(
                        "{} · Koszt: {} skały, {} tytanu, {} krzemu{}",
                        st.short_name(),
                        rock,
                        metal,
                        silicon,
                        helium
                    )
                } else if map.mining_progress > 0.01 {
                    format!("{} · {:.0}%", map.mining_label, map.mining_progress * 100.0)
                } else if !stats.collected_minerals && stats.blocks_mined > 0 {
                    crate::locale::text("Podejdź do odłamków, by je zebrać.").into()
                } else if player_opt.is_some_and(|p| {
                    !p.inside_building
                        && p.mounted_rover.is_none()
                        && p.iso_pos
                            .truncate()
                            .distance(map.station_position.truncate())
                            <= 3.0
                }) {
                    crate::locale::text(
                        "G — pilotuj stację  ·  Przytrzymaj lewy przycisk myszy, żeby kopać",
                    )
                    .into()
                } else {
                    crate::locale::text(
                        "WASD — chodź  ·  Przytrzymaj lewy przycisk myszy, żeby kopać",
                    )
                    .into()
                };
            }
            LabelId::HudBigTileAndMiningInfo => {
                text.sections[0].value = stats.status_log.clone();
            }
            LabelId::CargoWeight => {
                text.sections[0].value = moon_base_localize::localized_format!(
                    "PLECAK {:.1} / {} kg · Magazyn: {} rzeczy",
                    stats.cargo_weight(),
                    if stats.creative_cargo {
                        "∞".into()
                    } else {
                        format!("{:.0}", stats.cargo_limit())
                    },
                    map.storage.iter().sum::<u32>()
                );
            }
            LabelId::InvSlotCount(idx) => {
                if let Some(&c) = inv_counts.get(idx) {
                    text.sections[0].value = format!("x{}", c);
                }
            }
            LabelId::PickaxeSlotBtn => {
                text.sections[0].style.color = if !map.build_mode {
                    Color::rgb(1.0, 0.92, 0.45)
                } else {
                    Color::rgb(0.80, 0.86, 0.94)
                };
            }
            LabelId::BuildSlotBtn(idx) => {
                text.sections[0].style.color = if map.selected_build == idx && map.build_mode {
                    Color::rgb(0.45, 1.0, 0.72)
                } else {
                    Color::WHITE
                };
            }
            // Progression Map & Achievements Modal [J]
            LabelId::ProgModalHeader => {
                let done_count = (0..8)
                    .filter(|&i| stats.is_progression_step_done(i, &map))
                    .count();
                text.sections[0].value = moon_base_localize::localized_format!(
                    "MAPA POSTĘPU ({}/8 KROKÓW UKOŃCZONYCH) — TRYB: {} [J]",
                    done_count,
                    config.scenario.short_name()
                );
            }
            LabelId::ProgStepRow(idx) => {
                let (title, hint) = PROGRESSION_STEPS_META[idx];
                let (title, hint) = (crate::locale::text(title), crate::locale::text(hint));
                let is_done = stats.is_progression_step_done(idx, &map);
                if is_done {
                    text.sections[0].value = moon_base_localize::localized_format!(
                        "[UKOŃCZONO] {}\n   -> {}",
                        title,
                        hint
                    );
                    text.sections[0].style.color = Color::rgb(0.36, 0.98, 0.62);
                } else if idx == active_prog_step {
                    text.sections[0].value =
                        format!(">>> [TERAZ WYKONAJ TEN KROK] {} <<<\n   -> {}", title, hint);
                    text.sections[0].style.color = Color::rgb(1.0, 0.88, 0.32);
                } else {
                    // Locked future steps rendered in GRAY ("na szaro") as requested!
                    text.sections[0].value = moon_base_localize::localized_format!(
                        "[ZABLOKOWANE — NAJPIERW UKOŃCZ POPRZEDNI KROK] {}\n   -> {}",
                        title,
                        hint
                    );
                    text.sections[0].style.color = Color::rgb(0.48, 0.52, 0.58);
                }
            }
            LabelId::AchievementRow(idx) => {
                let (ach_title, ach_desc) = ACHIEVEMENTS_META[idx];
                let (ach_title, ach_desc) = (
                    crate::locale::text(ach_title),
                    crate::locale::text(ach_desc),
                );
                let unlocked = match idx {
                    0 => stats.blocks_mined >= 10,
                    1 => stats.entered_building_once,
                    2 => stats.highest_tool != crate::types::ToolTier::GeoHammer,
                    3 => base_modules_built >= 4,
                    4 => stats.settled_world_structure || stats.placed_waypoint_once,
                    5 => {
                        stats.planted_flag_once
                            || config.accessory == crate::types::HelmetAccessory::HussarWings
                    }
                    6 => stats.drove_rover_once,
                    7 => stats.helium3 >= 3,
                    _ => false,
                };
                if unlocked {
                    text.sections[0].value = format!("[ODBLOKOWANE!] {} — {}", ach_title, ach_desc);
                    text.sections[0].style.color = Color::rgb(1.0, 0.86, 0.36);
                } else {
                    // Locked achievements rendered in GRAY ("na szaro")
                    text.sections[0].value =
                        format!("[NIEODBLOKOWANE] {} — {}", ach_title, ach_desc);
                    text.sections[0].style.color = Color::rgb(0.48, 0.52, 0.58);
                }
            }
            // Live Customizer Modal labels inside Surface HUD
            LabelId::MdlCallsign => text.sections[0].value = config.callsign.clone(),
            LabelId::MdlSuit => text.sections[0].value = config.suit_color.name().to_string(),
            LabelId::MdlVisor => text.sections[0].value = config.visor_tint.name().to_string(),
            LabelId::MdlAccessory => text.sections[0].value = config.accessory.name().to_string(),
            LabelId::MdlTrail => text.sections[0].value = config.trail_color.name().to_string(),
            LabelId::MdlTrait => text.sections[0].value = config.astronaut_trait.name().to_string(),
            _ => {}
        }
    }
}

// ============================================================================
// 7. PAUSE MENU OVERLAY
// ============================================================================

pub fn setup_pause_menu_ui(mut commands: Commands) {
    commands
        .spawn((
            ScreenUiRoot,
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                background_color: Color::rgba(0.01, 0.03, 0.08, 0.84).into(),
                ..default()
            },
        ))
        .with_children(|root| {
            root.spawn(NodeBundle {
                style: Style {
                    width: Val::Px(500.0),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    padding: UiRect::all(Val::Px(24.0)),
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                background_color: CARD_BG.into(),
                border_color: CARD_BORDER.into(),
                ..default()
            })
            .with_children(|card| {
                card.spawn(TextBundle::from_section(
                    crate::locale::text("PAUZA GRY"),
                    TextStyle {
                        font_size: 28.0,
                        color: Color::rgb(0.38, 0.94, 1.0),
                        ..default()
                    },
                ));
                spawn_styled_button(
                    card,
                    crate::locale::text(">> WZNÓW ROZGRYWKĘ NA KSIĘŻYCU <<"),
                    UiAction::GotoState(AppState::LunarSurface),
                    Val::Px(400.0),
                    Val::Px(44.0),
                    STYLE_EMERALD,
                    16.0,
                );
                spawn_styled_button(
                    card,
                    crate::locale::text("Zadania i postęp"),
                    UiAction::GotoState(AppState::MissionReport),
                    Val::Px(400.0),
                    Val::Px(40.0),
                    STYLE_BLUEPRINT,
                    14.5,
                );
                spawn_styled_button(
                    card,
                    crate::locale::text("ZMIEŃ TRYB GRY / KREATOR POSTACI"),
                    UiAction::GotoState(AppState::MissionDesignLab),
                    Val::Px(400.0),
                    Val::Px(40.0),
                    STYLE_HUSARIA,
                    14.5,
                );
                spawn_styled_button(
                    card,
                    crate::locale::text("Ustawienia"),
                    UiAction::GotoState(AppState::Settings),
                    Val::Px(400.0),
                    Val::Px(40.0),
                    STYLE_AMBER,
                    14.5,
                );
                spawn_styled_button(
                    card,
                    crate::locale::text("Menu główne"),
                    UiAction::GotoState(AppState::MainMenu),
                    Val::Px(400.0),
                    Val::Px(40.0),
                    STYLE_SLATE,
                    14.5,
                );
            });
        });
}

// ============================================================================
// 8. PROGRESSION & ACHIEVEMENTS SUMMARY SCREEN (Replaced old Mission Alert)
// ============================================================================

pub fn setup_mission_report_ui(
    mut commands: Commands,
    config: Res<MissionConfig>,
    stats: Res<SurvivalStats>,
    map: Res<LunarMap>,
) {
    if let Some(mission) = stats
        .campaign
        .as_ref()
        .filter(|mission| mission.quest == Quest::Complete)
    {
        let seconds = mission.seconds as u32;
        commands
            .spawn((ScreenUiRoot, atlas_root()))
            .with_children(|root| {
                menu_text(
                    root,
                    crate::locale::text("MISJA UKOŃCZONA"),
                    19.0,
                    Color::rgb(0.39, 0.93, 0.78),
                );
                menu_text(
                    root,
                    crate::locale::text("Ziemia odebrała raport!"),
                    52.0,
                    Color::WHITE,
                );
                menu_text(
                    root,
                    crate::locale::text("Zbadano Księżyc, Marsa i Wenus."),
                    24.0,
                    Color::rgb(0.82, 0.91, 0.96),
                );
                menu_text(
                    root,
                    &moon_base_localize::localized_format!(
                        "12 punktów badań · 8 zadań · {}:{:02} gry",
                        seconds / 60,
                        seconds % 60
                    ),
                    20.0,
                    Color::rgb(0.67, 0.83, 0.95),
                );
                spawn_styled_button(
                    root,
                    crate::locale::text("Zagraj od nowa"),
                    UiAction::GotoState(AppState::CampaignBriefing),
                    Val::Px(300.0),
                    Val::Px(46.0),
                    STYLE_EMERALD,
                    18.0,
                );
                spawn_styled_button(
                    root,
                    crate::locale::text("Menu główne"),
                    UiAction::GotoState(AppState::MainMenu),
                    Val::Px(300.0),
                    Val::Px(40.0),
                    STYLE_SLATE,
                    16.0,
                );
            });
        return;
    }
    if let Some(mission) = &stats.campaign {
        commands
            .spawn((ScreenUiRoot, atlas_root()))
            .with_children(|root| {
                menu_text(
                    root,
                    crate::locale::text("Raport dla Ziemi"),
                    38.0,
                    Color::WHITE,
                );
                menu_text(
                    root,
                    crate::locale::text(campaign::GOAL),
                    22.0,
                    Color::WHITE,
                );
                let current = Quest::ALL
                    .iter()
                    .position(|&quest| quest == mission.quest)
                    .unwrap_or_default();
                for (index, quest) in Quest::ALL.into_iter().enumerate() {
                    let (mark, color) = if index < current {
                        (crate::locale::text("Gotowe"), Color::rgb(0.52, 0.92, 0.73))
                    } else if index == current {
                        (crate::locale::text("Teraz"), Color::rgb(1.0, 0.81, 0.42))
                    } else {
                        (crate::locale::text("Później"), Color::rgb(0.62, 0.72, 0.82))
                    };
                    menu_text(
                        root,
                        &format!("{}. {} · {}", index + 1, quest.title(), mark),
                        17.0,
                        color,
                    );
                }
                spawn_styled_button(
                    root,
                    crate::locale::text("Wróć do misji"),
                    UiAction::GotoState(AppState::LunarSurface),
                    Val::Px(280.0),
                    Val::Px(43.0),
                    STYLE_EMERALD,
                    17.0,
                );
            });
        return;
    }
    let base_modules_built = map
        .cells
        .values()
        .filter(|c| c.structure.is_some() && c.structure_hp > 0.0 && !c.is_world_generated_base)
        .count();

    let steps_done = (0..8)
        .filter(|&i| stats.is_progression_step_done(i, &map))
        .count();
    let active_step = stats.current_active_step(&map);

    commands
        .spawn((
            ScreenUiRoot,
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    padding: UiRect::all(Val::Px(20.0)),
                    ..default()
                },
                background_color: BG_DARK.into(),
                ..default()
            },
        ))
        .with_children(|root| {
            root.spawn(NodeBundle {
                style: Style {
                    width: Val::Px(820.0),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::FlexStart,
                    padding: UiRect::all(Val::Px(22.0)),
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                background_color: CARD_BG.into(),
                border_color: CARD_BORDER.into(),
                ..default()
            })
            .with_children(|card| {
                card.spawn(TextBundle::from_section(
                    moon_base_localize::localized_format!(
                        "MAPA POSTĘPU GRACZA ({}/8) & OSIĄGNIĘCIA — TRYB: {}",
                        steps_done,
                        config.scenario.short_name()
                    ),
                    TextStyle {
                        font_size: 21.0,
                        color: Color::rgb(0.38, 0.94, 1.0),
                        ..default()
                    },
                ));

                card.spawn(
                    TextBundle::from_section(
                        moon_base_localize::localized_format!(
                            "Astronauta: {} ({}) | Wykopane bloki: {} | Zbudowane moduły: {} | Dane: {:.1} GB",
                            config.callsign,
                            config.accessory.name(),
                            stats.blocks_mined,
                            base_modules_built,
                            stats.transmitted_science_gb
                        ),
                        TextStyle {
                            font_size: 13.5,
                            color: Color::rgb(1.0, 0.86, 0.42),
                            ..default()
                        },
                    )
                    .with_style(Style {
                        margin: UiRect::vertical(Val::Px(6.0)),
                        ..default()
                    }),
                );

                for (idx, (title, hint)) in PROGRESSION_STEPS_META.iter().enumerate() {
                    let (title, hint) = (crate::locale::text(title), crate::locale::text(hint));
                    let is_done = stats.is_progression_step_done(idx, &map);
                    let (line, col) = if is_done {
                        (
                            moon_base_localize::localized_format!("[UKOŃCZONO] {} — {}", title, hint),
                            Color::rgb(0.36, 0.98, 0.62),
                        )
                    } else if idx == active_step {
                        (
                            moon_base_localize::localized_format!(">>> [AKTUALNY CEL] {} — {} <<<", title, hint),
                            Color::rgb(1.0, 0.88, 0.32),
                        )
                    } else {
                        (
                            moon_base_localize::localized_format!("[ZABLOKOWANE — NA SZARO] {} — {}", title, hint),
                            Color::rgb(0.48, 0.52, 0.58),
                        )
                    };
                    card.spawn(
                        TextBundle::from_section(
                            line,
                            TextStyle {
                                font_size: 12.2,
                                color: col,
                                ..default()
                            },
                        )
                        .with_style(Style {
                            margin: UiRect::vertical(Val::Px(2.0)),
                            ..default()
                        }),
                    );
                }

                card.spawn(
                    TextBundle::from_section(
                        GAME_CREATORS_LINE,
                        TextStyle {
                            font_size: 13.0,
                            color: Color::rgb(0.95, 0.88, 0.58),
                            ..default()
                        },
                    )
                    .with_style(Style {
                        margin: UiRect::vertical(Val::Px(10.0)),
                        ..default()
                    }),
                );

                card.spawn(NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::Row,
                        ..default()
                    },
                    ..default()
                })
                .with_children(|btns| {
                    spawn_styled_button(
                        btns,
                        crate::locale::text(">> WRÓĆ NA POWIERZCHNIĘ KSIĘŻYCA <<"),
                        UiAction::GotoState(AppState::LunarSurface),
                        Val::Px(320.0),
                        Val::Px(42.0),
                        STYLE_EMERALD,
                        14.5,
                    );
                    spawn_styled_button(
                        btns,
                        crate::locale::text("ZMIEŃ TRYB GRY / KREATOR POSTACI"),
                        UiAction::GotoState(AppState::MissionDesignLab),
                        Val::Px(320.0),
                        Val::Px(42.0),
                        STYLE_BLUEPRINT,
                        14.5,
                    );
                });
            });
        });
}

pub fn update_mission_report_input(
    keys: Res<ButtonInput<KeyCode>>,
    stats: Res<SurvivalStats>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if stats
        .campaign
        .as_ref()
        .is_some_and(|c| c.quest == Quest::Complete)
    {
        if keys.just_pressed(KeyCode::Escape) {
            next_state.set(AppState::MainMenu);
        }
    } else if keys.just_pressed(KeyCode::Escape) || keys.just_pressed(KeyCode::KeyJ) {
        next_state.set(AppState::LunarSurface);
    } else if keys.just_pressed(KeyCode::Backspace) {
        next_state.set(AppState::MissionDesignLab);
    }
}

// ============================================================================
// 9. GAME OVER / SURVIVAL FAILURE SCREEN (Respects Hardcore Single-Life Mode!)
// ============================================================================

pub fn setup_game_over_ui(
    mut commands: Commands,
    config: Res<MissionConfig>,
    stats: Res<SurvivalStats>,
    map: Res<LunarMap>,
) {
    let base_modules_built = map
        .cells
        .values()
        .filter(|c| c.structure.is_some() && c.structure_hp > 0.0 && !c.is_world_generated_base)
        .count();

    let reason = if stats.death_reason.is_empty() {
        crate::locale::text("Krytyczna utrata parametrów życiowych na powierzchni Księżyca.")
    } else {
        stats.death_reason.as_str()
    };

    let is_hardcore = config.is_hardcore();

    commands
        .spawn((
            ScreenUiRoot,
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    padding: UiRect::all(Val::Px(24.0)),
                    ..default()
                },
                background_color: Color::rgba(0.12, 0.02, 0.04, 0.90).into(),
                ..default()
            },
        ))
        .with_children(|root| {
            root.spawn(NodeBundle {
                style: Style {
                    width: Val::Px(740.0),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    padding: UiRect::all(Val::Px(28.0)),
                    border: UiRect::all(Val::Px(2.5)),
                    ..default()
                },
                background_color: Color::rgba(0.14, 0.05, 0.08, 0.96).into(),
                border_color: Color::rgb(1.0, 0.35, 0.35).into(),
                ..default()
            })
            .with_children(|card| {
                card.spawn(TextBundle::from_section(
                    if is_hardcore {
                        crate::locale::text("[!] KONIEC GRY W TRYBIE HARDCORE (JEDNO ŻYCIE)")
                    } else {
                        crate::locale::text("Astronauta potrzebuje pomocy")
                    },
                    TextStyle {
                        font_size: 24.0,
                        color: Color::rgb(1.0, 0.42, 0.42),
                        ..default()
                    },
                ));

                card.spawn(
                    TextBundle::from_section(
                        moon_base_localize::localized_format!(
                            "Astronauta: {} ({})  |  Tryb: {}\n\
                             PRZYCZYNA: {}\n\n\
                             --- PODSUMOWANIE DOTYCHCZASOWEJ ROZGRYWKI ---\n\
                             • Czas przetrwania: {:.2} Dni Księżycowych (Sol)\n\
                             • Wykopane bloki: {}  |  Zbudowane moduły bazy: {}\n\
                             • Przesłane dane badawcze: {:.1} GB\n\n\
                             WSKAZÓWKA SURVIVALOWA:\n\
                             • Pilnuj czasu pod paskiem tlenu. Klawiszem E wejdziesz do bazy, żeby uzupełnić butlę.",
                            config.callsign,
                            config.astronaut_trait.name(),
                            config.scenario.short_name(),
                            reason,
                            stats.elapsed_sols,
                            stats.blocks_mined,
                            base_modules_built,
                            stats.transmitted_science_gb,
                        ),
                        TextStyle {
                            font_size: 14.5,
                            color: Color::rgb(0.96, 0.92, 0.94),
                            ..default()
                        },
                    )
                    .with_style(Style {
                        margin: UiRect::vertical(Val::Px(16.0)),
                        ..default()
                    }),
                );

                card.spawn(NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    ..default()
                })
                .with_children(|btns| {
                    if !is_hardcore {
                        spawn_styled_button(
                            btns,
                            crate::locale::text(">> KLONOWANIE AWARYJNE W LĄDOWNIKU (Zachowaj całą bazę i świat!) <<"),
                            UiAction::RespawnInLander,
                            Val::Px(560.0),
                            Val::Px(46.0),
                            STYLE_EMERALD,
                            15.0,
                        );
                    } else {
                        btns.spawn(
                            TextBundle::from_section(
                                crate::locale::text("W trybie HARDCORE posiadasz tylko jedno życie — klonowanie awaryjne jest wyłączone."),
                                TextStyle {
                                    font_size: 14.0,
                                    color: Color::rgb(1.0, 0.65, 0.45),
                                    ..default()
                                },
                            )
                            .with_style(Style {
                                margin: UiRect::bottom(Val::Px(8.0)),
                                ..default()
                            }),
                        );
                    }
                    spawn_styled_button(
                        btns,
                        crate::locale::text("WRÓĆ DO Menu główneGO / NOWA GRA"),
                        UiAction::GotoState(AppState::MainMenu),
                        Val::Px(560.0),
                        Val::Px(42.0),
                        STYLE_DANGER,
                        15.0,
                    );
                });
            });
        });
}
