mod audio;
mod base;
mod campaign;
mod celestial;
mod controls;
mod dimensions;
mod emotes;
#[cfg(windows)]
mod firewall;
mod flight;
mod hover;
mod isometric;
mod locale;
mod mineral;
mod mobile_host;
mod mobile_player;
mod nasa_apod;
mod net_lan;
mod net_tcp;
mod net_visuals;
mod net_world;
mod physics_gameplay;
mod planets;
mod types;
mod ui_screens;
mod vehicles;
mod world;

use bevy::prelude::*;
use bevy::window::WindowResolution;
use bevy::winit::WinitWindows;

use audio::setup_audio_engine;
use celestial::{render_celestial_backdrop_and_gmat, update_celestial_mechanics, SolarSystemState};
use isometric::{generate_procedural_assets, generate_window_icon_rgba_64};
use net_lan::LanMultiplayer;
use physics_gameplay::{
    handle_mining_building_crafting_and_throwing, update_astronaut_and_rover_physics,
    update_coop_and_lan_peers, update_cursor_and_build_hotkeys, update_projectiles_and_particles,
    update_survival_base_and_events, LunarEventManager,
};
use types::{AppState, GameSettings, MissionConfig};
use ui_screens::{
    cleanup_ui_root, handle_ui_button_clicks, handle_ui_buttons, setup_controls_ui,
    setup_game_over_ui, setup_launch_sequence_ui, setup_main_menu_ui, setup_mdl_ui,
    setup_mission_report_ui, setup_pause_menu_ui, setup_settings_ui, setup_surface_hud,
    update_launch_sequence, update_mdl_ui, update_mission_report_input, update_settings_labels,
    update_surface_hud, LaunchTimer,
};
use world::{
    cleanup_surface_entities, setup_lunar_surface, update_lighting_shadows_and_cutaways, LunarMap,
    TouchAndUiInput,
};

#[cfg(all(target_os = "linux", feature = "linux-x11", feature = "linux-wayland"))]
compile_error!("Build with only one Linux display feature: linux-x11 or linux-wayland.");

fn main() {
    let settings = GameSettings::load();
    locale::set_language(settings.language);
    let mut initial_config = MissionConfig::default();
    if !settings.astronaut_name.trim().is_empty() {
        initial_config.callsign = settings.astronaut_name.clone();
    }
    let connection = match net_lan::launch_connection(std::env::args().skip(1)) {
        Ok(connection) => connection,
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(2);
        }
    };
    let mut app = App::new();
    #[cfg(windows)]
    app.init_resource::<firewall::FirewallAccess>()
        .add_systems(Update, firewall::update.after(handle_ui_button_clicks));

    let plugins = DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            name: Some("moon-base-101".to_string()),
            title: "Moon Base 101".to_string(),
            resolution: WindowResolution::new(1440.0, 900.0),
            resizable: true,
            present_mode: bevy::window::PresentMode::AutoNoVsync,
            ..default()
        }),
        ..default()
    });
    #[cfg(target_os = "linux")]
    let plugins = {
        use bevy::render::{
            settings::{Backends, WgpuSettings},
            RenderPlugin,
        };
        let backend = if cfg!(feature = "linux-wayland") {
            Backends::GL
        } else {
            Backends::VULKAN
        };
        plugins.set(RenderPlugin {
            render_creation: WgpuSettings {
                backends: Some(backend),
                ..default()
            }
            .into(),
            ..default()
        })
    };

    app.insert_resource(connection)
        .insert_resource(ClearColor(Color::rgb(0.015, 0.022, 0.048)))
        .insert_resource(initial_config)
        .insert_resource(settings)
        .init_resource::<controls::ControlsMenu>()
        .insert_resource(SolarSystemState::default())
        .insert_resource(LunarMap::default())
        .insert_resource(TouchAndUiInput::default())
        .insert_resource(LanMultiplayer::default())
        .init_resource::<nasa_apod::NasaApod>()
        .init_resource::<mobile_host::MobileServer>()
        .init_resource::<mobile_host::InvitePanel>()
        .init_resource::<net_world::Authority>()
        .init_resource::<dimensions::ServerDimensions>()
        .init_resource::<dimensions::SurveyLedger>()
        .insert_resource(LunarEventManager::default())
        .insert_resource(LaunchTimer::default())
        .init_resource::<ui_screens::MenuNavigation>()
        .init_resource::<ui_screens::SettingsEdit>()
        .init_resource::<campaign::CampaignLaunch>()
        .init_resource::<planets::Expedition>()
        .add_plugins(plugins)
        .init_state::<AppState>()
        .add_systems(
            Startup,
            (
                setup_camera,
                setup_polish_font_and_window_icon,
                generate_procedural_assets,
                setup_audio_engine,
            ),
        )
        // Global interactive button systems + celestial mechanics
        .add_systems(
            Update,
            (
                net_lan::join_from_command_line,
                controls::input,
                net_lan::edit_host_address,
                ui_screens::chat_keyboard,
                mobile_host::run,
                net_world::network_tick,
                dimensions::tick,
                mobile_host::invite_ui,
                nasa_apod::poll,
                nasa_apod::handle,
                handle_ui_buttons,
                ui_screens::fullscreen_shortcut,
                ui_screens::update_player_list,
                handle_ui_button_clicks,
                ui_screens::update_chat_panel,
                ui_screens::update_menu_navigation,
                ui_screens::update_responsive_ui,
                update_celestial_mechanics,
                render_celestial_backdrop_and_gmat,
            )
                .chain()
                .before(update_cursor_and_build_hotkeys),
        )
        .add_systems(
            Update,
            (
                controls::menu,
                ui_screens::settings_text_input,
                ui_screens::hud_visibility,
                ui_screens::refresh_settings_language,
            )
                .after(handle_ui_button_clicks),
        )
        .add_systems(Last, controls::limit_frames)
        // State 1: Main Menu
        .add_systems(
            OnEnter(AppState::MainMenu),
            (
                nasa_apod::request,
                net_lan::disconnect_on_menu,
                cleanup_surface_entities,
                setup_main_menu_ui,
                reset_camera_pos,
            ),
        )
        .add_systems(OnExit(AppState::MainMenu), cleanup_ui_root)
        .add_systems(OnEnter(AppState::NasaApod), nasa_apod::details)
        .add_systems(
            Update,
            nasa_apod::scroll.run_if(in_state(AppState::NasaApod)),
        )
        .add_systems(OnExit(AppState::NasaApod), cleanup_ui_root)
        .add_systems(
            OnEnter(AppState::CampaignBriefing),
            (
                cleanup_surface_entities,
                ui_screens::setup_campaign_briefing,
            ),
        )
        .add_systems(OnExit(AppState::CampaignBriefing), cleanup_ui_root)
        .add_systems(OnEnter(AppState::Discovery), ui_screens::setup_discovery)
        .add_systems(OnExit(AppState::Discovery), cleanup_ui_root)
        // State 2: Settings & LAN Multiplayer Menu
        .add_systems(OnEnter(AppState::Settings), setup_settings_ui)
        .add_systems(
            Update,
            update_settings_labels.run_if(in_state(AppState::Settings)),
        )
        .add_systems(OnExit(AppState::Settings), cleanup_ui_root)
        // State 3: How To Play Guide
        .add_systems(OnEnter(AppState::Controls), setup_controls_ui)
        .add_systems(OnExit(AppState::Controls), cleanup_ui_root)
        // State 4: Game Mode & Character Lab
        .add_systems(
            OnEnter(AppState::MissionDesignLab),
            (cleanup_surface_entities, setup_mdl_ui, reset_camera_pos),
        )
        .add_systems(
            Update,
            update_mdl_ui.run_if(in_state(AppState::MissionDesignLab)),
        )
        .add_systems(OnExit(AppState::MissionDesignLab), cleanup_ui_root)
        // State 5: Animated Launch & Cislunar Trajectory Sequence
        .add_systems(
            OnEnter(AppState::LaunchSequence),
            (setup_launch_sequence_ui, reset_camera_pos),
        )
        .add_systems(
            Update,
            update_launch_sequence.run_if(in_state(AppState::LaunchSequence)),
        )
        .add_systems(OnExit(AppState::LaunchSequence), cleanup_ui_root)
        // State 6: 2.5D Isometric Lunar Surface Operations
        .add_systems(
            OnEnter(AppState::LunarSurface),
            (
                setup_surface_loader_if_needed,
                setup_surface_hud,
                hover::setup,
            )
                .chain(),
        )
        .add_systems(
            Update,
            (
                net_world::serve_actions,
                update_cursor_and_build_hotkeys,
                flight::update_station_flight,
                flight::animate_thrusters,
                update_coop_and_lan_peers,
                net_visuals::remote_appearance,
                update_astronaut_and_rover_physics,
                world::unload_distant_surface_sprites,
                net_visuals::smooth_remote_vehicles,
                emotes::animate,
                handle_mining_building_crafting_and_throwing,
                mineral::update_deposits,
                mineral::update_crystal_deposits,
                mineral::enforce_capacity,
                mineral::update_pickups,
                mineral::animate_notices,
                update_projectiles_and_particles,
                update_survival_base_and_events,
                update_lighting_shadows_and_cutaways,
                (
                    planets::survey_surface,
                    dimensions::record_active,
                    campaign::update_campaign,
                    campaign::draw_target,
                    planets::draw_surface_weather,
                    update_surface_hud,
                    net_world::publish_world,
                    ui_screens::update_map_grid,
                    ui_screens::update_action_buttons,
                    hover::update,
                )
                    .chain(),
            )
                .chain()
                .run_if(in_state(AppState::LunarSurface)),
        )
        .add_systems(OnExit(AppState::LunarSurface), cleanup_ui_root)
        .add_systems(
            OnEnter(AppState::PlanetAtlas),
            ui_screens::setup_planet_atlas,
        )
        .add_systems(OnExit(AppState::PlanetAtlas), cleanup_ui_root)
        .add_systems(
            OnEnter(AppState::InterplanetaryFlight),
            (ui_screens::setup_interplanetary_flight, reset_camera_pos),
        )
        .add_systems(
            Update,
            (
                ui_screens::update_interplanetary_ui,
                planets::travel_between_worlds,
            )
                .chain()
                .run_if(in_state(AppState::InterplanetaryFlight)),
        )
        .add_systems(OnExit(AppState::InterplanetaryFlight), cleanup_ui_root)
        // State 7: Pause Menu Overlay
        .add_systems(OnEnter(AppState::Paused), setup_pause_menu_ui)
        .add_systems(OnExit(AppState::Paused), cleanup_ui_root)
        // State 8: Progression & Achievements Summary Screen
        .add_systems(OnEnter(AppState::MissionReport), setup_mission_report_ui)
        .add_systems(
            Update,
            update_mission_report_input.run_if(in_state(AppState::MissionReport)),
        )
        .add_systems(OnExit(AppState::MissionReport), cleanup_ui_root)
        // State 9: Game Over / Survival Failure Screen
        .add_systems(OnEnter(AppState::GameOver), setup_game_over_ui)
        .add_systems(OnExit(AppState::GameOver), cleanup_ui_root)
        .run();
}

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

/// Installs the embedded UI font with Polish diacritics and sets the window icon.
fn setup_polish_font_and_window_icon(
    mut fonts: Option<ResMut<Assets<Font>>>,
    winit_windows: Option<NonSend<WinitWindows>>,
) {
    if let Some(ref mut font_assets) = fonts {
        if let Ok(font) =
            Font::try_from_bytes(include_bytes!("../assets/fonts/Technor-Variable.ttf").to_vec())
        {
            font_assets.insert(&Handle::<Font>::default(), font);
        }
    }

    if let Some(winit_windows) = winit_windows {
        let rgba = generate_window_icon_rgba_64();
        if let Ok(icon) = winit::window::Icon::from_rgba(rgba, 64, 64) {
            for window in winit_windows.windows.values() {
                window.set_window_icon(Some(icon.clone()));
            }
        }
    }
}

fn reset_camera_pos(
    mut cam_q: Query<(&mut Transform, &mut OrthographicProjection), With<Camera2d>>,
) {
    if let Ok((mut tf, mut proj)) = cam_q.get_single_mut() {
        tf.translation.x = 0.0;
        tf.translation.y = 0.0;
        proj.scale = 1.0;
    }
}

fn setup_surface_loader_if_needed(
    commands: Commands,
    assets: Res<isometric::ProceduralAssets>,
    config: Res<MissionConfig>,
    mut lan: ResMut<LanMultiplayer>,
    mut map: ResMut<LunarMap>,
    mut expedition: ResMut<planets::Expedition>,
    mut events: ResMut<LunarEventManager>,
    mut touch: ResMut<TouchAndUiInput>,
    mut launch: ResMut<campaign::CampaignLaunch>,
    mut authority: ResMut<net_world::Authority>,
    mut dimensions: ResMut<dimensions::ServerDimensions>,
    mut ledger: ResMut<dimensions::SurveyLedger>,
    existing_q: Query<Entity, With<world::SurfaceEntity>>,
) {
    if existing_q.is_empty() {
        *authority = net_world::Authority::default();
        *dimensions = dimensions::ServerDimensions::default();
        *ledger = default();
        *map = LunarMap::default();
        lan.current_body = map.body;
        map.campaign_route = std::mem::take(&mut launch.0);
        *expedition = planets::Expedition::default();
        *events = LunarEventManager::default();
        if config.difficulty == types::DifficultyLevel::Cadet {
            events.duration_sec = 120.0;
        }
        *touch = TouchAndUiInput::default();
        setup_lunar_surface(commands, assets, config, lan.into(), map);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::isometric::{iso_to_world, world_to_iso};
    use crate::types::{LaunchVehicle, LunarOrbit, MissionScenario, SciInstrument, StructureKind};
    use crate::world::LunarMap;
    use bevy::asset::{AssetApp, AssetPlugin};
    use bevy::ecs::system::RunSystemOnce;

    #[test]
    fn campaign_rover_moves_faster_than_the_walking_speed_limit() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Image>()
            .init_asset::<bevy::render::render_resource::Shader>()
            .add_plugins(bevy::gizmos::GizmoPlugin)
            .init_resource::<Time>()
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<MissionConfig>()
            .init_resource::<GameSettings>()
            .init_resource::<LanMultiplayer>()
            .init_resource::<nasa_apod::NasaApod>()
            .init_resource::<TouchAndUiInput>()
            .init_resource::<LunarEventManager>()
            .insert_resource(audio::GameAudio { tx: None })
            .insert_resource(LunarMap {
                campaign_route: true,
                ..default()
            });
        app.world.run_system_once(generate_procedural_assets);
        app.world.run_system_once(setup_lunar_surface);
        let rover = app
            .world
            .query_filtered::<Entity, With<world::RoverVehicle>>()
            .single(&app.world);
        app.world
            .query::<&mut world::AstronautPlayer>()
            .single_mut(&mut app.world)
            .mounted_rover = Some(rover);
        app.world
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyA);
        for _ in 0..40 {
            app.world
                .resource_mut::<Time>()
                .advance_by(std::time::Duration::from_millis(50));
            app.world
                .run_system_once(update_astronaut_and_rover_physics);
        }
        assert!(
            app.world
                .get::<world::RoverVehicle>(rover)
                .unwrap()
                .iso_vel
                .length()
                > 5.5
        );
    }

    #[test]
    fn campaign_requires_supplies_three_surveys_and_a_powered_final_report() {
        use crate::campaign::{Quest, BASE, RELAY};
        use crate::planets::WorldBody;
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Image>()
            .init_resource::<Time>()
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<LanMultiplayer>()
            .init_resource::<TouchAndUiInput>()
            .init_resource::<LunarEventManager>()
            .init_resource::<planets::Expedition>()
            .init_state::<AppState>()
            .insert_resource(MissionConfig {
                difficulty: types::DifficultyLevel::Cadet,
                ..default()
            })
            .insert_resource(LunarMap {
                campaign_route: true,
                ..default()
            });
        app.world.run_system_once(generate_procedural_assets);
        app.world.run_system_once(setup_lunar_surface);
        let step = |app: &App| {
            app.world
                .resource::<world::SurvivalStats>()
                .campaign
                .as_ref()
                .unwrap()
                .quest
        };
        app.world.run_system_once(campaign::update_campaign);
        assert_eq!(step(&app), Quest::Supplies);
        assert!(!campaign::travel_allowed(
            app.world
                .resource::<world::SurvivalStats>()
                .campaign
                .as_ref(),
            WorldBody::Mars
        ));
        app.world
            .resource_mut::<world::SurvivalStats>()
            .campaign
            .as_mut()
            .unwrap()
            .opened_supplies = true;
        app.world.run_system_once(campaign::update_campaign);
        assert_eq!(step(&app), Quest::Water);
        let mut stats = app.world.remove_resource::<world::SurvivalStats>().unwrap();
        {
            let mut map = app.world.resource_mut::<LunarMap>();
            base::update_networks(&mut map, 0.0, 0.5);
            let pos = (BASE.x, BASE.y);
            assert!(
                map.networks[&pos].powered
                    && map.networks[&pos].oxygen
                    && map.networks[&pos].greenhouse
            );
            stats.water_ice = 1;
            base::SupplyRecipe::Water
                .start(&mut map, &mut stats, pos)
                .unwrap();
            base::SupplyRecipe::Water.advance(&mut map, &mut stats, 4.0);
            base::SupplyRecipe::Meal
                .start(&mut map, &mut stats, pos)
                .unwrap();
            base::SupplyRecipe::Meal.advance(&mut map, &mut stats, 12.0);
        }
        app.world.insert_resource(stats);
        app.world.run_system_once(campaign::update_campaign);
        assert_eq!(step(&app), Quest::Meal);
        app.world.run_system_once(campaign::update_campaign);
        assert_eq!(step(&app), Quest::Relay);
        fn enter_base(app: &mut App) {
            let mut query = app.world.query::<&mut world::AstronautPlayer>();
            let mut player = query.single_mut(&mut app.world);
            player.iso_pos = Vec3::new(3.0, 0.0, 0.0);
            player.inside_building = true;
        }
        fn action(app: &mut App) {
            app.world
                .resource_mut::<world::SurvivalStats>()
                .campaign
                .as_mut()
                .unwrap()
                .action_requested = true;
            app.world.run_system_once(campaign::update_campaign);
        }
        enter_base(&mut app);
        action(&mut app);
        assert_eq!(step(&app), Quest::Moon);
        assert_eq!(
            app.world.resource::<LunarMap>().cells[&RELAY].structure,
            Some(StructureKind::CommsDish)
        );
        for (body, expected) in [
            (WorldBody::Mars, Quest::Mars),
            (WorldBody::Venus, Quest::Venus),
            (WorldBody::Moon, Quest::Report),
        ] {
            app.world.resource_mut::<LunarMap>().surveyed = 15;
            app.world.run_system_once(campaign::update_campaign);
            assert_eq!(step(&app), expected);
            assert!(campaign::travel_allowed(
                app.world
                    .resource::<world::SurvivalStats>()
                    .campaign
                    .as_ref(),
                body
            ));
            app.world
                .resource_mut::<planets::Expedition>()
                .request_campaign(body);
            app.world
                .resource_mut::<Time>()
                .advance_by(std::time::Duration::from_secs(10));
            app.world.run_system_once(planets::travel_between_worlds);
            assert_eq!(app.world.resource::<LunarMap>().body, body);
            assert_eq!(app.world.resource::<LunarMap>().survey_points().len(), 4);
        }
        enter_base(&mut app);
        action(&mut app);
        assert!(app
            .world
            .resource::<world::SurvivalStats>()
            .campaign
            .as_ref()
            .unwrap()
            .uplink
            .is_none());
        base::update_networks(&mut app.world.resource_mut::<LunarMap>(), 0.0, 0.5);
        action(&mut app);
        assert!(app
            .world
            .resource::<world::SurvivalStats>()
            .campaign
            .as_ref()
            .unwrap()
            .uplink
            .is_some());
        for _ in 0..365 {
            app.world.run_system_once(campaign::update_campaign);
        }
        assert_eq!(step(&app), Quest::Complete);
        assert!(matches!(
            *app.world.resource::<NextState<AppState>>(),
            NextState(Some(AppState::MissionReport))
        ));
    }

    #[test]
    fn suit_uses_oxygen_outside_and_base_refills_it_while_food_and_water_are_separate() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Image>()
            .init_resource::<Time>()
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<ButtonInput<MouseButton>>()
            .init_resource::<GameSettings>()
            .init_resource::<LanMultiplayer>()
            .init_resource::<LunarMap>()
            .init_resource::<TouchAndUiInput>()
            .init_resource::<LunarEventManager>()
            .init_resource::<SolarSystemState>()
            .init_state::<AppState>()
            .insert_resource(audio::GameAudio { tx: None })
            .insert_resource(MissionConfig {
                difficulty: types::DifficultyLevel::Cadet,
                ..default()
            });
        app.world.run_system_once(generate_procedural_assets);
        app.world.run_system_once(setup_lunar_surface);
        app.world
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(50));
        app.world.run_system_once(update_survival_base_and_events);
        let stats = app.world.resource::<world::SurvivalStats>();
        assert!(stats.oxygen_use_per_sec > 0.4 && stats.oxygen < stats.max_oxygen);
        let outdoors = stats.oxygen_use_per_sec;

        app.world.resource_mut::<LunarMap>().mining_progress = 0.5;
        app.world.run_system_once(update_survival_base_and_events);
        assert!(
            app.world
                .resource::<world::SurvivalStats>()
                .oxygen_use_per_sec
                > outdoors
        );
        let mut crew = app.world.query::<&mut world::AstronautPlayer>();
        let mut player = crew.single_mut(&mut app.world);
        player.iso_pos = Vec3::new(3.0, 0.0, 0.0);
        player.inside_building = true;
        let before = app.world.resource::<world::SurvivalStats>().oxygen;
        app.world.run_system_once(update_survival_base_and_events);
        let stats = app.world.resource::<world::SurvivalStats>();
        assert_eq!(stats.oxygen_use_per_sec, 0.0);
        assert!(stats.oxygen > before);

        {
            let mut stats = app.world.resource_mut::<world::SurvivalStats>();
            stats.hunger = 30.0;
            stats.hydration = 30.0;
            stats.fresh_meals = 1;
        }
        app.world
            .resource_mut::<TouchAndUiInput>()
            .cmds
            .push(world::SurfaceUiCmd::UseRation);
        app.world
            .run_system_once(handle_mining_building_crafting_and_throwing);
        let stats = app.world.resource::<world::SurvivalStats>();
        assert_eq!(
            (
                stats.hunger,
                stats.hydration,
                stats.fresh_meals,
                stats.rations
            ),
            (90.0, 30.0, 0, 2)
        );
        app.world
            .resource_mut::<TouchAndUiInput>()
            .cmds
            .push(world::SurfaceUiCmd::DrinkWater);
        app.world
            .run_system_once(handle_mining_building_crafting_and_throwing);
        let stats = app.world.resource::<world::SurvivalStats>();
        assert_eq!(
            (stats.hunger, stats.hydration, stats.drinking_water),
            (90.0, 80.0, 1)
        );
    }

    #[test]
    fn station_can_take_off_move_land_and_exit_in_a_new_mission() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<bevy::render::render_resource::Shader>()
            .init_asset::<Image>()
            .add_plugins(bevy::gizmos::GizmoPlugin)
            .init_resource::<Time>()
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<MissionConfig>()
            .init_resource::<LanMultiplayer>()
            .init_resource::<LunarMap>()
            .init_resource::<TouchAndUiInput>();
        app.world.run_system_once(generate_procedural_assets);
        app.world.run_system_once(setup_lunar_surface);

        fn tick(app: &mut App, keys: &[KeyCode]) {
            app.world
                .resource_mut::<Time>()
                .advance_by(std::time::Duration::from_millis(50));
            let mut input = ButtonInput::default();
            for &key in keys {
                input.press(key);
            }
            app.world.insert_resource(input);
            app.world.run_system_once(flight::update_station_flight);
        }

        tick(&mut app, &[KeyCode::KeyG]);
        assert!(app.world.resource::<LunarMap>().piloting_station);
        for _ in 0..22 {
            tick(&mut app, &[KeyCode::Space, KeyCode::KeyA]);
        }
        let position = app.world.resource::<LunarMap>().station_position;
        assert!(position.z > 2.0 && position.truncate().length() > 1.0);
        tick(&mut app, &[KeyCode::KeyG]);
        assert!(app.world.resource::<LunarMap>().piloting_station);

        // Continue across the launch pad to flat ground with no nearby structures.
        for _ in 0..17 {
            tick(&mut app, &[KeyCode::KeyA]);
        }
        let map = app.world.resource::<LunarMap>();
        assert!(flight::landing_clear(map, map.station_position.truncate()));
        for _ in 0..35 {
            tick(&mut app, &[KeyCode::ControlLeft]);
        }
        tick(&mut app, &[KeyCode::KeyG]);
        let map = app.world.resource::<LunarMap>();
        assert!(!map.piloting_station);
        assert!(
            map.station_position.z
                <= map.get_elevation(map.station_position.x, map.station_position.y) + 0.1
        );
        let mut astronaut = app.world.query::<(&world::AstronautPlayer, &Visibility)>();
        let (player, visibility) = astronaut.single(&app.world);
        assert!(player.grounded && !player.inside_building && *visibility == Visibility::Inherited);
    }

    #[test]
    fn planet_round_trip_restores_the_original_surface_and_keeps_cargo() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Image>()
            .init_resource::<Time>()
            .init_resource::<MissionConfig>()
            .init_resource::<LanMultiplayer>()
            .init_resource::<LunarMap>()
            .init_resource::<TouchAndUiInput>()
            .init_resource::<LunarEventManager>()
            .init_resource::<planets::Expedition>()
            .init_state::<AppState>();
        app.world.run_system_once(generate_procedural_assets);
        app.world.run_system_once(setup_lunar_surface);
        {
            let mut map = app.world.resource_mut::<LunarMap>();
            let cell = map.cells.get_mut(&(-4, 0)).unwrap();
            cell.structure = Some(StructureKind::StorageBay);
            cell.structure_hp = 72.0;
            cell.charge_wh = 27.0;
            cell.resource_yield = 3;
            map.surveyed = 1;
        }
        app.world.resource_mut::<world::SurvivalStats>().titanium = 19;
        for body in [
            planets::WorldBody::Mars,
            planets::WorldBody::Venus,
            planets::WorldBody::Moon,
        ] {
            app.world
                .resource_mut::<planets::Expedition>()
                .request(body);
            app.world
                .resource_mut::<Time>()
                .advance_by(std::time::Duration::from_secs(4));
            app.world.run_system_once(planets::travel_between_worlds);
            assert_eq!(app.world.resource::<LunarMap>().body, body);
        }
        let map = app.world.resource::<LunarMap>();
        let cell = &map.cells[&(-4, 0)];
        assert_eq!(cell.structure, Some(StructureKind::StorageBay));
        assert_eq!(
            (cell.structure_hp, cell.charge_wh, cell.resource_yield),
            (72.0, 27.0, 3)
        );
        assert_eq!(map.surveyed, 1);
        assert_eq!(app.world.resource::<world::SurvivalStats>().titanium, 19);
        assert_eq!(
            app.world
                .query::<&world::AstronautPlayer>()
                .iter(&app.world)
                .count(),
            1
        );
    }

    #[test]
    fn test_isometric_projection_roundtrip() {
        let orig = Vec2::new(6.0, -4.0);
        let world = iso_to_world(orig.x, orig.y, 0.0);
        let back = world_to_iso(world);
        assert!((orig.x - back.x).abs() < 1e-4);
        assert!((orig.y - back.y).abs() < 1e-4);
    }

    #[test]
    fn test_mission_config_constraints_and_cml() {
        let mut cfg = MissionConfig::default();
        assert_eq!(cfg.cml_level(), 4);
        assert!(cfg.validate_launch_readiness().is_ok());

        cfg.launch_vehicle = LaunchVehicle::CommercialLight;
        cfg.orbit = LunarOrbit::NearRectilinearHalo;
        cfg.selected_instruments = SciInstrument::ALL.to_vec();
        assert!(cfg.validate_launch_readiness().is_err());

        cfg.scenario = MissionScenario::OpenSandbox;
        assert!(cfg.validate_launch_readiness().is_ok());
    }

    #[test]
    fn imported_mission_normalizes_extreme_and_duplicate_values() {
        let mut cfg = MissionConfig::default();
        cfg.planned_lifetime_sols = u32::MAX;
        cfg.dilemma_index = usize::MAX;
        cfg.dilemma_resolved_bonus = f32::NAN;
        cfg.callsign = "\n".repeat(80);
        cfg.selected_instruments = vec![SciInstrument::CryoIceDrill; 100];
        cfg.normalize();
        cfg.negotiate_dilemma();
        assert_eq!(cfg.planned_lifetime_sols, 45);
        assert!(!cfg.callsign.trim().is_empty());
        assert_eq!(cfg.selected_instruments.len(), 1);
        assert!(cfg.total_cost_musd().is_finite());
    }

    #[test]
    fn unloading_distant_tiles_preserves_exploration_and_buildings() {
        let mut app = App::new();
        let mut map = LunarMap::default();
        map.generate_sector(4, 0, MissionScenario::OpenSandbox);
        let entity = app.world.spawn(world::IsoTileMarker { gx: 48, gy: 0 }).id();
        let cell = map.cells.get_mut(&(48, 0)).unwrap();
        cell.tile_entity = Some(entity);
        cell.structure = Some(StructureKind::HabitatModule);
        cell.structure_hp = 100.0;
        cell.paid_cost = Some((12, 4, 3, 0));
        app.insert_resource(map)
            .add_systems(Update, world::unload_distant_surface_sprites);
        app.update();
        assert!(app.world.get_entity(entity).is_none());
        let cell = &app.world.resource::<LunarMap>().cells[&(48, 0)];
        assert!(cell.tile_entity.is_none());
        assert_eq!(cell.structure, Some(StructureKind::HabitatModule));
        assert_eq!(cell.paid_cost, Some((12, 4, 3, 0)));
    }

    #[test]
    fn test_infinite_world_sector_generation_and_building_synergy() {
        let mut map = LunarMap::default();
        map.generate_sector(0, 0, MissionScenario::PolsaKopernikBase);
        map.generate_sector(0, -1, MissionScenario::PolsaKopernikBase);
        assert!(map.generated_sectors.contains(&(0, 0)));
        // Stream a distant sector (e.g. sector (4, -3))
        map.generate_sector(4, -3, MissionScenario::PolsaKopernikBase);
        assert!(map.generated_sectors.contains(&(4, -3)));
        assert!(map.cells.contains_key(&(48, -36)));

        // Verify world-generated structures spawn as real settleable buildings
        let world_bases_count = map
            .cells
            .values()
            .filter(|c| c.is_world_generated_base && c.structure.is_some())
            .count();
        assert!(world_bases_count >= 3);

        // Verify connected building neighbors count
        if let Some(c1) = map.cells.get_mut(&(5, 5)) {
            c1.structure = Some(StructureKind::SolarArray);
            c1.structure_hp = 100.0;
        }
        if let Some(c2) = map.cells.get_mut(&(6, 5)) {
            c2.structure = Some(StructureKind::SolarArray);
            c2.structure_hp = 100.0;
        }
        assert_eq!(
            map.connected_neighbors_count(5, 5, StructureKind::SolarArray),
            1
        );
    }

    #[test]
    fn test_all_ecs_systems_have_disjoint_queries() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_state::<AppState>()
            .add_systems(
                Update,
                (
                    mineral::update_pickups,
                    mineral::update_deposits,
                    mineral::update_crystal_deposits,
                    flight::update_station_flight,
                ),
            )
            .add_systems(
                Startup,
                (
                    setup_camera,
                    setup_polish_font_and_window_icon,
                    generate_procedural_assets,
                    setup_audio_engine,
                ),
            )
            .add_systems(
                Update,
                (
                    handle_ui_buttons,
                    handle_ui_button_clicks,
                    update_celestial_mechanics,
                    render_celestial_backdrop_and_gmat,
                    update_settings_labels,
                    update_mdl_ui,
                    update_launch_sequence,
                    update_cursor_and_build_hotkeys,
                    update_astronaut_and_rover_physics,
                    world::unload_distant_surface_sprites,
                    update_coop_and_lan_peers,
                    handle_mining_building_crafting_and_throwing,
                    update_projectiles_and_particles,
                    update_survival_base_and_events,
                    update_lighting_shadows_and_cutaways,
                    update_surface_hud,
                    ui_screens::update_map_grid,
                    ui_screens::update_action_buttons,
                    update_mission_report_input,
                ),
            )
            .add_systems(
                OnEnter(AppState::MainMenu),
                (
                    cleanup_surface_entities,
                    setup_main_menu_ui,
                    reset_camera_pos,
                ),
            )
            .add_systems(OnExit(AppState::MainMenu), cleanup_ui_root)
            .add_systems(OnEnter(AppState::Settings), setup_settings_ui)
            .add_systems(OnEnter(AppState::Controls), setup_controls_ui)
            .add_systems(
                OnEnter(AppState::MissionDesignLab),
                (cleanup_surface_entities, setup_mdl_ui, reset_camera_pos),
            )
            .add_systems(
                OnEnter(AppState::LaunchSequence),
                (setup_launch_sequence_ui, reset_camera_pos),
            )
            .add_systems(
                OnEnter(AppState::LunarSurface),
                (setup_surface_loader_if_needed, setup_surface_hud),
            )
            .add_systems(OnEnter(AppState::Paused), setup_pause_menu_ui)
            .add_systems(OnEnter(AppState::MissionReport), setup_mission_report_ui)
            .add_systems(OnEnter(AppState::GameOver), setup_game_over_ui);

        // Initialize all schedules so Bevy validates every Query<...> for B0001/B0002 conflicts!
        app.add_systems(
            Update,
            (
                net_visuals::remote_appearance,
                net_visuals::smooth_remote_vehicles,
                ui_screens::update_player_list,
                ui_screens::update_chat_panel,
            ),
        );
        app.world
            .try_schedule_scope(Startup, |world, schedule| {
                schedule.initialize(world).unwrap();
            })
            .unwrap();
        app.world
            .try_schedule_scope(Update, |world, schedule| {
                schedule.initialize(world).unwrap();
            })
            .unwrap();
        for state in [
            AppState::MainMenu,
            AppState::Settings,
            AppState::Controls,
            AppState::MissionDesignLab,
            AppState::LaunchSequence,
            AppState::LunarSurface,
            AppState::Paused,
            AppState::MissionReport,
            AppState::GameOver,
        ] {
            app.world
                .try_schedule_scope(OnEnter(state), |world, schedule| {
                    schedule.initialize(world).unwrap();
                })
                .unwrap();
        }
    }
}
