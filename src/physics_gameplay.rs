use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use rand::Rng;
use std::f32::consts::{PI, TAU};

use crate::audio::{GameAudio, SfxKind};
use crate::celestial::SolarSystemState;
use crate::isometric::{iso_depth, iso_to_world, ProceduralAssets};
use crate::net_lan::LanMultiplayer;
use crate::types::{
    AppState, AstronautTrait, BackpackModule, GameSettings, LunarEventKind, MissionConfig,
    MissionScenario, RoverKind, StructureKind, TileKind, ToolTier, WorldFeatureKind,
};

const SUIT_OXYGEN_USE: f32 = 0.72;
use crate::world::{
    refresh_structure_cluster_around, spawn_rover_entity, stream_infinite_chunks_around,
    AstronautAccessoryMarker, AstronautPlayer, AstronautVisorMarker, BuildGhostMarker,
    CoOpCrewMember, CursorMarker, DustParticle, IsoTileMarker, LunarMap, MiningCrackMarker,
    RoverVehicle, StructureConnectorMarker, StructureSpriteMarker, SurfaceEntity, SurfaceUiCmd,
    SurvivalStats, ThrownObject, TouchAndUiInput, Waypoint,
};

use crate::planets::WorldBody;

#[derive(Resource, Clone)]
pub struct LunarEventManager {
    pub active_event: LunarEventKind,
    pub timer_sec: f32,
    pub duration_sec: f32,
    pub meteor_spawn_timer: f32,
    pub camera_shake_intensity: f32,
    pub last_laser_beam: Option<(Vec2, Vec2, f32)>,
}

impl Default for LunarEventManager {
    fn default() -> Self {
        Self {
            active_event: LunarEventKind::Nominal,
            timer_sec: 0.0,
            duration_sec: 36.0,
            meteor_spawn_timer: 0.0,
            camera_shake_intensity: 0.0,
            last_laser_beam: None,
        }
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn update_cursor_and_build_hotkeys(
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_q: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut mouse_wheel: EventReader<MouseWheel>,
    (audio, assets, config): (Res<GameAudio>, Res<ProceduralAssets>, Res<MissionConfig>),
    mut settings: ResMut<GameSettings>,
    mut map: ResMut<LunarMap>,
    mut touch_ui: ResMut<TouchAndUiInput>,
    mut cursor_q: Query<
        (&mut Transform, &mut Sprite, &mut Visibility),
        (
            With<CursorMarker>,
            Without<BuildGhostMarker>,
            Without<MiningCrackMarker>,
        ),
    >,
    mut ghost_q: Query<
        (
            &mut Transform,
            &mut Sprite,
            &mut Handle<Image>,
            &mut Visibility,
        ),
        (
            With<BuildGhostMarker>,
            Without<CursorMarker>,
            Without<MiningCrackMarker>,
        ),
    >,
    mut crack_q: Query<
        (&mut Transform, &mut Handle<Image>, &mut Visibility),
        (
            With<MiningCrackMarker>,
            Without<CursorMarker>,
            Without<BuildGhostMarker>,
        ),
    >,
    player_q: Query<&AstronautPlayer>,
    mut stats: ResMut<SurvivalStats>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    let sfx_vol = settings.sfx_volume * settings.master_volume;

    if keys.just_pressed(KeyCode::KeyN) {
        map.close_all_modals();
        next_state.set(AppState::PlanetAtlas);
        return;
    }

    if keys.just_pressed(KeyCode::Escape) {
        if map.any_modal_open() {
            map.close_all_modals();
            audio.play(SfxKind::UiClick, sfx_vol);
            return;
        }
        audio.play(SfxKind::UiClick, sfx_vol);
        next_state.set(AppState::Paused);
        return;
    }

    // Smooth Camera Zoom via Mouse Scroll Wheel or [-]/[+] keys (0.68x .. 1.55x)
    for ev in mouse_wheel.read() {
        let step = if ev.y > 0.0 {
            0.08
        } else if ev.y < 0.0 {
            -0.08
        } else {
            0.0
        };
        if step != 0.0 {
            settings.camera_zoom = (settings.camera_zoom + step).clamp(0.68, 1.55);
        }
    }
    if keys.just_pressed(KeyCode::Equal) || keys.just_pressed(KeyCode::BracketRight) {
        settings.camera_zoom = (settings.camera_zoom + 0.10).clamp(0.68, 1.55);
        audio.play(SfxKind::UiClick, sfx_vol);
    }
    if keys.just_pressed(KeyCode::Minus) || keys.just_pressed(KeyCode::BracketLeft) {
        settings.camera_zoom = (settings.camera_zoom - 0.10).clamp(0.68, 1.55);
        audio.play(SfxKind::UiClick, sfx_vol);
    }

    // Toggle Backpack/Inventory [I]
    if keys.just_pressed(KeyCode::KeyI) {
        audio.play(SfxKind::UiClick, sfx_vol);
        let next = !map.show_inventory_window;
        map.close_all_modals();
        map.show_inventory_window = next;
    }

    // Toggle Character Customizer [K]
    if keys.just_pressed(KeyCode::KeyK) {
        audio.play(SfxKind::UiClick, sfx_vol);
        let next = !map.show_customizer_window;
        map.close_all_modals();
        map.show_customizer_window = next;
    }

    // Toggle World Map & Waypoints [M]
    if keys.just_pressed(KeyCode::KeyM) {
        audio.play(SfxKind::UiClick, sfx_vol);
        let next = !map.show_map_window;
        map.close_all_modals();
        map.show_map_window = next;
    }

    // Toggle Progression Map & Achievements [J] (Never bound to Tab so Alt+Tab never pops up anything!)
    if keys.just_pressed(KeyCode::KeyJ) {
        audio.play(SfxKind::UiClick, sfx_vol);
        let next = !map.show_progression_window;
        map.close_all_modals();
        map.show_progression_window = next;
    }

    if keys.just_pressed(KeyCode::KeyZ) {
        touch_ui.cmds.push(SurfaceUiCmd::SwitchCrewMember);
    }

    if keys.just_pressed(KeyCode::KeyH) {
        touch_ui.cmds.push(SurfaceUiCmd::PlantPolishFlag);
    }

    if keys.just_pressed(KeyCode::KeyX) {
        touch_ui.cmds.push(SurfaceUiCmd::UseMedkit);
    }

    // Toggle between Mining/Deconstruct mode and Build mode ([B] or [Q])
    if keys.just_pressed(KeyCode::KeyB) || keys.just_pressed(KeyCode::KeyQ) {
        map.demolition_mode = false;
        audio.play(SfxKind::UiClick, sfx_vol);
        map.build_mode = !map.build_mode;
        map.mining_progress = 0.0;
        map.mining_target = None;
        stats.status_log = if map.build_mode {
            let st = StructureKind::ALL[map.selected_build];
            moon_base_localize::localized_format!(
                "TRYB BUDOWY ({}): Kliknij LPM lub PPM aby postawić budynek. Sąsiednie budynki łączą się w jedność!",
                st.name()
            )
        } else {
            crate::locale::text("TRYB KILOFU: Przytrzymaj LPM aby wykopać blok lub rozebrać budynek (100% zwrotu surowców)!")
                .to_string()
        };
    }

    let number_keys = [
        (KeyCode::Digit1, 0),
        (KeyCode::Digit2, 1),
        (KeyCode::Digit3, 2),
        (KeyCode::Digit4, 3),
        (KeyCode::Digit5, 4),
        (KeyCode::Digit6, 5),
        (KeyCode::Digit7, 6),
        (KeyCode::Digit8, 7),
        (KeyCode::Digit9, 8),
        (KeyCode::Digit0, 9),
    ];
    for (k, idx) in number_keys {
        if keys.just_pressed(k) {
            audio.play(SfxKind::UiClick, sfx_vol);
            map.mining_progress = 0.0;
            map.mining_target = None;
            if map.build_mode && map.selected_build == idx {
                map.build_mode = false;
                stats.status_log =
                    crate::locale::text("Kilof gotowy. Przytrzymaj lewy przycisk myszy na skale.")
                        .to_string();
            } else {
                map.selected_build = idx;
                map.build_mode = true;
                let st = StructureKind::ALL[idx];
                let (r, ti, si, he) = st.build_cost();
                stats.status_log = moon_base_localize::localized_format!(
                    "Wybrano: {} | Koszt: {} Regolit, {} Tytan, {} Krzem, {} Hel-3 — {}",
                    st.short_name(),
                    r,
                    ti,
                    si,
                    he,
                    st.simple_kid_hint()
                );
            }
        }
    }

    // Update progressive mining crack overlay sprite
    if let Ok((mut c_tf, mut c_tex, mut c_vis)) = crack_q.get_single_mut() {
        if let Some((mx, my)) = map.mining_target {
            if map.mining_progress > 0.02
                && map.cells.get(&(mx, my)).is_some_and(|cell| {
                    crate::mineral::deposit_index(cell.kind) == 0
                        && cell.feature != Some(WorldFeatureKind::GlowingCrystalCluster)
                        || cell.structure.is_some()
                })
            {
                *c_vis = if settings.hide_hud {
                    Visibility::Hidden
                } else {
                    Visibility::Inherited
                };
                let stage = ((map.mining_progress * 4.0) as usize).clamp(0, 3);
                *c_tex = assets.crack_stage_tex[stage].clone();
                let elev = map.get_elevation(mx as f32, my as f32);
                let w_pos = iso_to_world(mx as f32, my as f32, elev);
                c_tf.translation = Vec3::new(w_pos.x, w_pos.y + 4.0, 858.0);
            } else {
                *c_vis = Visibility::Hidden;
            }
        } else {
            *c_vis = Visibility::Hidden;
        }
    }

    let Ok(window) = windows.get_single() else {
        return;
    };
    let Ok((camera, cam_transform)) = camera_q.get_single() else {
        return;
    };

    if let Ok((_, _, mut visibility)) = cursor_q.get_single_mut() {
        *visibility = Visibility::Hidden;
    }
    if let Ok((_, _, _, mut visibility)) = ghost_q.get_single_mut() {
        *visibility = Visibility::Hidden;
    }
    if map.piloting_station {
        return;
    }
    if let Some(screen_pos) = window.cursor_position() {
        if !touch_ui.pointer_over_ui && !map.any_modal_open() {
            if let Some(world_pos) = camera.viewport_to_world_2d(cam_transform, screen_pos) {
                let (gx, gy) = map.pick_tile_at_world(world_pos);
                map.hovered_tile = (gx, gy);

                let elev = map.get_elevation(gx as f32, gy as f32);
                let snapped = iso_to_world(gx as f32, gy as f32, elev);

                let player_pos = player_q
                    .get_single()
                    .map(|p| p.iso_pos.truncate())
                    .unwrap_or(Vec2::ZERO);
                let in_range = player_pos.distance(Vec2::new(gx as f32, gy as f32)) <= 7.5;
                let cell_free = map
                    .cells
                    .get(&(gx, gy))
                    .map(|c| {
                        (c.structure.is_none() || c.structure_hp <= 0.0) && c.feature.is_none()
                    })
                    .unwrap_or(false)
                    && (gx.abs() > 1 || gy.abs() > 1);

                if let Ok((mut tf, mut sp, mut visibility)) = cursor_q.get_single_mut() {
                    *visibility = if settings.hide_hud {
                        Visibility::Hidden
                    } else {
                        Visibility::Inherited
                    };
                    tf.translation.x = snapped.x;
                    tf.translation.y = snapped.y;
                    tf.translation.z = 860.0;
                    sp.color = if map.build_mode {
                        if in_range && cell_free {
                            Color::rgba(0.25, 1.0, 0.55, 0.95)
                        } else {
                            Color::rgba(1.0, 0.35, 0.35, 0.95)
                        }
                    } else {
                        Color::rgba(0.35, 0.92, 1.0, 0.88)
                    };
                }

                if let Ok((mut g_tf, mut g_sp, mut g_tex, mut g_vis)) = ghost_q.get_single_mut() {
                    if map.build_mode {
                        *g_vis = if settings.hide_hud {
                            Visibility::Hidden
                        } else {
                            Visibility::Inherited
                        };
                        let st = StructureKind::ALL[map.selected_build];
                        *g_tex = assets.structure_handle(st);
                        g_tf.translation.x = snapped.x;
                        g_tf.translation.y = snapped.y + 20.0;
                        g_tf.translation.z = 855.0;
                        let (mut rc, mut tc, mut sc, hc) = st.build_cost();
                        if config.astronaut_trait == AstronautTrait::SystemsArchitect {
                            rc = (rc * 3) / 4;
                            tc = (tc * 3) / 4;
                            sc = (sc * 3) / 4;
                        }
                        let can_afford = config.scenario == MissionScenario::OpenSandbox
                            || ((stats.highest_tool as u8) >= (st.required_tool() as u8)
                                && stats.regolith >= rc
                                && stats.titanium >= tc
                                && stats.silicon >= sc
                                && stats.helium3 >= hc);
                        g_sp.color = if in_range && cell_free && can_afford {
                            Color::rgba(0.35, 1.0, 0.65, 0.58)
                        } else {
                            Color::rgba(1.0, 0.32, 0.32, 0.52)
                        };
                    } else {
                        *g_vis = Visibility::Hidden;
                    }
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn update_astronaut_and_rover_physics(
    mut commands: Commands,
    mut lan: Option<ResMut<LanMultiplayer>>,
    (time, keys): (Res<Time>, Res<ButtonInput<KeyCode>>),
    (config, settings, audio): (Res<MissionConfig>, Res<GameSettings>, Res<GameAudio>),
    mut map: ResMut<LunarMap>,
    assets: Res<ProceduralAssets>,
    events: Res<LunarEventManager>,
    mut touch_ui: ResMut<TouchAndUiInput>,
    mut stats: ResMut<SurvivalStats>,
    mut gizmos: Gizmos,
    net_ids: Query<&crate::net_world::Replica>,
    mut player_q: Query<
        (
            &mut AstronautPlayer,
            &mut Transform,
            &mut Sprite,
            &mut Handle<Image>,
        ),
        Without<RoverVehicle>,
    >,
    mut visor_q: Query<
        (&mut Handle<Image>, &mut Sprite, &mut Visibility),
        (
            With<AstronautVisorMarker>,
            Without<AstronautPlayer>,
            Without<RoverVehicle>,
        ),
    >,
    mut acc_q: Query<
        (&mut Handle<Image>, &mut Sprite, &mut Visibility),
        (
            With<AstronautAccessoryMarker>,
            Without<AstronautVisorMarker>,
            Without<AstronautPlayer>,
            Without<RoverVehicle>,
        ),
    >,
    mut rover_q: Query<
        (
            Entity,
            &mut RoverVehicle,
            &mut Transform,
            &mut Sprite,
            &mut Handle<Image>,
        ),
        Without<AstronautPlayer>,
    >,
    mut cam_q: Query<
        (&mut Transform, &mut OrthographicProjection),
        (
            With<Camera2d>,
            Without<AstronautPlayer>,
            Without<RoverVehicle>,
        ),
    >,
) {
    if map.piloting_station {
        touch_ui.cmds.retain(|cmd| {
            matches!(
                cmd,
                SurfaceUiCmd::AddWaypointAtPlayer | SurfaceUiCmd::RemoveLastWaypoint
            )
        });
        return;
    }
    let dt = time.delta_seconds().min(0.05);
    let sfx_vol = settings.sfx_volume * settings.master_volume;

    let Ok((mut player, mut player_tf, mut player_sprite, mut player_tex)) =
        player_q.get_single_mut()
    else {
        return;
    };
    if player.inside_building
        && player.airlock_anim_timer <= 0.0
        && !map.is_inside_pressurized(player.iso_pos.x, player.iso_pos.y)
    {
        player.inside_building = false;
    }

    let mut ui_mount_rover = false;
    let mut ui_enter_building = false;
    let mut ui_build_rover = false;
    let mut ui_jump = false;
    let mut remaining_cmds = Vec::new();
    for cmd in touch_ui.cmds.drain(..) {
        match cmd {
            SurfaceUiCmd::MountOrExitRover => ui_mount_rover = true,
            SurfaceUiCmd::EnterOrExitBuilding => ui_enter_building = true,
            SurfaceUiCmd::FabricateRover => ui_build_rover = true,
            SurfaceUiCmd::TriggerJump => ui_jump = true,
            other => remaining_cmds.push(other),
        }
    }
    touch_ui.cmds = remaining_cmds;

    // Advance Airlock Decompression / Compression Door Animation if active!
    if player.airlock_anim_timer > 0.0 {
        player.airlock_anim_timer = (player.airlock_anim_timer - dt).max(0.0);
        let target_z = map.get_elevation(player.airlock_target_iso.x, player.airlock_target_iso.y);
        let lerp_t = (dt * 6.5).min(1.0);
        player.iso_pos.x += (player.airlock_target_iso.x - player.iso_pos.x) * lerp_t;
        player.iso_pos.y += (player.airlock_target_iso.y - player.iso_pos.y) * lerp_t;
        player.iso_pos.z = target_z;
        player.iso_vel = Vec3::ZERO;
        if player.airlock_anim_timer <= 0.0 {
            player.iso_pos.x = player.airlock_target_iso.x;
            player.iso_pos.y = player.airlock_target_iso.y;
            player.grounded = true;
            player.inside_building = player.airlock_entering;
        }
    }

    // [E] Key: Smart Context Action — Enter/Exit Building via Airlock OR Mount/Exit Rover!
    if player.airlock_anim_timer <= 0.0
        && ((!map.any_modal_open() && keys.just_pressed(KeyCode::KeyE))
            || ui_mount_rover
            || ui_enter_building)
    {
        if player.mounted_rover.is_some() && lan.as_ref().is_some_and(|l| l.is_active) {
            if let Some(lan) = lan.as_deref_mut() {
                lan.request(crate::net_world::Action::Board(None));
            }
        } else if player.mounted_rover.is_some() {
            player.is_passenger = false;
            player.mounted_rover = None;
            player.iso_pos.z += 0.35;
            player.iso_vel = Vec3::new(0.0, 0.0, 1.8);
            player.grounded = false;
            audio.play(SfxKind::RoverToggle, sfx_vol);
            stats.status_log = crate::locale::text("Wysiadłeś z łazika.").to_string();
        } else if player.inside_building {
            let from = player.iso_pos.truncate();
            let anchor = map
                .find_nearby_enterable_building(from.x, from.y, 4.5)
                .map_or(from, |(x, y, _)| Vec2::new(x as f32, y as f32));
            let Some(exit_spot) = map
                .safe_exit(anchor, 3.0)
                .or_else(|| map.safe_exit(from, 5.0))
            else {
                stats.status_log =
                    crate::locale::text("Wyjście jest zablokowane. Zwolnij miejsce przy śluzie.")
                        .into();
                return;
            };

            player.airlock_anim_timer = 0.75;
            player.airlock_entering = false;
            player.airlock_target_iso = exit_spot;
            audio.play(SfxKind::RoverToggle, sfx_vol);
            stats.status_log = crate::locale::text("Wychodzisz z bazy.").to_string();
            for i in 0..8 {
                let a = (i as f32 / 8.0) * TAU;
                commands.spawn((
                    SurfaceEntity,
                    DustParticle {
                        iso_pos: player.iso_pos + Vec3::new(0.0, 0.0, 0.3),
                        iso_vel: Vec3::new(a.cos() * 1.4, a.sin() * 1.4, 1.2),
                        life: 0.0,
                        max_life: 0.65,
                        size: 5.0,
                        color: Color::rgba(0.65, 0.95, 1.0, 0.80),
                    },
                ));
            }
        } else {
            // Check if a Rover is closer or an Airlock/Building is closer
            let mut closest_rover: Option<(Entity, f32, &'static str)> = None;
            for (r_ent, rover, _, _, _) in &rover_q {
                let d = player.iso_pos.truncate().distance(rover.iso_pos.truncate());
                if d <= 2.6 && closest_rover.is_none_or(|(_, distance, _)| d < distance) {
                    closest_rover = Some((r_ent, d, rover.kind.name()));
                }
            }

            let nearby_bldg = if ui_mount_rover {
                None
            } else {
                map.find_nearby_enterable_building(player.iso_pos.x, player.iso_pos.y, 2.8)
            };

            let prefer_building = nearby_bldg.map_or(false, |(x, y, _)| {
                closest_rover.map_or(true, |(_, distance, _)| {
                    player
                        .iso_pos
                        .truncate()
                        .distance(Vec2::new(x as f32, y as f32))
                        < distance
                })
            });
            if let Some((r_ent, _, _)) = closest_rover.filter(|_| !prefer_building) {
                if let Some(lan) = lan.as_deref_mut().filter(|l| l.is_active) {
                    let id = net_ids.get(r_ent).map_or(r_ent.to_bits(), |n| n.0);
                    lan.request(crate::net_world::Action::Board(Some(id)));
                } else {
                    player.mounted_rover = Some(r_ent);
                    player.is_passenger = false;
                    stats.drove_rover_once = true;
                    stats.status_log =
                        crate::locale::text("Jesteś w łaziku. WASD — jedź. E — wysiądź.").into();
                }
                audio.play(SfxKind::RoverToggle, sfx_vol);
            } else if let Some((bx, by, b_kind)) = nearby_bldg {
                // Find an connected indoor Habitat/Lab/Hydroponics tile next to the Airlock if entering via Airlock!
                let mut target_indoor = Vec2::new(bx as f32, by as f32);
                if b_kind == StructureKind::Airlock {
                    for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                        if let Some(nc) = map.cells.get(&(bx + dx, by + dy)) {
                            if let Some(nst) = nc.structure {
                                if nc.structure_hp > 10.0
                                    && nst.is_pressurized_room()
                                    && nst != StructureKind::Airlock
                                {
                                    target_indoor = Vec2::new((bx + dx) as f32, (by + dy) as f32);
                                    break;
                                }
                            }
                        }
                    }
                }
                let is_discovered_outpost = map
                    .cells
                    .get(&(bx, by))
                    .map(|c| c.is_world_generated_base)
                    .unwrap_or(false);

                player.airlock_anim_timer = 0.80;
                player.airlock_entering = true;
                player.airlock_target_iso = target_indoor;
                stats.entered_building_once = true;
                if is_discovered_outpost {
                    stats.settled_world_structure = true;
                }
                audio.play(SfxKind::BuildModule, sfx_vol);
                stats.status_log =
                    crate::locale::text("Wchodzisz przez śluzę. Naciśnij E, aby wyjść.").into();
                for i in 0..10 {
                    let a = (i as f32 / 10.0) * TAU;
                    commands.spawn((
                        SurfaceEntity,
                        DustParticle {
                            iso_pos: Vec3::new(bx as f32, by as f32, 0.3),
                            iso_vel: Vec3::new(a.cos() * 1.5, a.sin() * 1.5, 1.4),
                            life: 0.0,
                            max_life: 0.70,
                            size: 5.2,
                            color: Color::rgba(0.45, 0.98, 1.0, 0.85),
                        },
                    ));
                }
            } else {
                stats.status_log =
                    crate::locale::text("Podejdź do drzwi bazy albo do łazika i naciśnij E.")
                        .to_string();
            }
        }
    }

    // Fabricate Rover ([V] or UI button)
    if lan.as_ref().is_some_and(|l| l.is_active && !l.is_host)
        && (keys.just_pressed(KeyCode::KeyV) || ui_build_rover)
    {
        if let Some(lan) = lan.as_deref_mut() {
            lan.send(crate::net_lan::Message::Action(
                crate::net_world::Action::Command(SurfaceUiCmd::FabricateRover),
            ));
        }
        ui_build_rover = false;
    }
    if (keys.just_pressed(KeyCode::KeyV)
        && !lan.as_ref().is_some_and(|l| l.is_active && !l.is_host))
        || ui_build_rover
    {
        if time.elapsed_seconds() < stats.placement_ready_at {
            stats.status_log =
                crate::locale::text("Odczekaj chwilę przed kolejnym postawieniem.").into();
        } else if stats.titanium >= 16 && stats.silicon >= 10 && stats.regolith >= 18
            || config.scenario == MissionScenario::OpenSandbox
        {
            if config.scenario != MissionScenario::OpenSandbox {
                stats.titanium -= 16;
                stats.silicon -= 10;
                stats.regolith -= 18;
            }
            let new_kind = if stats.helium3 >= 3 {
                if config.scenario != MissionScenario::OpenSandbox {
                    stats.helium3 -= 3;
                }
                RoverKind::HeavyCruiser
            } else {
                RoverKind::LightBuggy
            };
            let spawn_pos = Vec3::new(
                player.iso_pos.x + 1.5,
                player.iso_pos.y,
                map.get_elevation(player.iso_pos.x + 1.5, player.iso_pos.y),
            );
            spawn_rover_entity(&mut commands, &assets, new_kind, spawn_pos);
            stats.placement_ready_at = time.elapsed_seconds() + 0.5;
            audio.play(SfxKind::BuildModule, sfx_vol);
            stats.status_log = moon_base_localize::localized_format!(
                "Zbudowano nowy łazik: {}! Wsiądź klawiszem [E].",
                new_kind.short_name()
            );
        } else {
            stats.status_log =
                crate::locale::text("Budowa łazika wymaga: 18 Regolitu, 16 Tytanu i 10 Krzemu.")
                    .to_string();
        }
    }

    // Keyboard + Virtual Touch D-Pad movement input
    let mut screen_input = touch_ui.move_dir;
    if keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp) {
        screen_input.y += 1.0;
    }
    if keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown) {
        screen_input.y -= 1.0;
    }
    if keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight) {
        screen_input.x += 1.0;
    }
    if keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft) {
        screen_input.x -= 1.0;
    }
    if map.any_modal_open() || player.airlock_anim_timer > 0.0 {
        screen_input = Vec2::ZERO;
        player.iso_vel = Vec3::ZERO;
    }
    if screen_input.x > 0.1 {
        player.facing_right = true;
    } else if screen_input.x < -0.1 {
        player.facing_right = false;
    }

    // Smooth angular target when pressing W / A / S / D!
    if screen_input.length_squared() > 0.01 {
        screen_input = screen_input.normalize();
        player.target_facing_rad = screen_input.y.atan2(screen_input.x);
    }

    // Shortest-arc smooth angular interpolation so the astronaut rotates fluidly between W, A, S, D!
    let mut angle_diff =
        (player.target_facing_rad - player.facing_angle_rad + PI).rem_euclid(TAU) - PI;
    if angle_diff.abs() < 0.005 {
        player.facing_angle_rad = player.target_facing_rad;
        angle_diff = 0.0;
    } else {
        player.facing_angle_rad =
            (player.facing_angle_rad + angle_diff * (dt * 11.5).min(1.0)).rem_euclid(TAU);
    }

    let iso_input = Vec2::new(
        screen_input.x - screen_input.y,
        -screen_input.x - screen_input.y,
    )
    .normalize_or_zero();

    if let Some(rover_ent) = player.mounted_rover {
        if let Ok((_, mut rover, mut r_tf, mut r_sprite, mut r_tex)) = rover_q.get_mut(rover_ent) {
            if !player.is_passenger {
                let pilot_mult = if config.astronaut_trait == AstronautTrait::RoverTestPilot {
                    1.35
                } else {
                    1.0
                };
                let accel = 26.0 * pilot_mult;
                if rover.battery > 0.5 && iso_input.length_squared() > 0.01 {
                    rover.iso_vel += iso_input * accel * dt;
                    let max_spd = if map.campaign_route {
                        crate::campaign::ROVER_SPEED
                    } else {
                        rover.kind.max_speed() * pilot_mult
                    };
                    if rover.iso_vel.length() > max_spd {
                        rover.iso_vel = rover.iso_vel.normalize() * max_spd;
                    }
                    rover.heading_rad = rover.iso_vel.y.atan2(rover.iso_vel.x);
                    let drain = if map.campaign_route { 0.30 } else { 1.15 };
                    rover.battery = (rover.battery - dt * drain / pilot_mult).max(0.0);

                    rover.wheel_anim = (rover.wheel_anim + dt * rover.iso_vel.length() * 3.2) % 3.0;
                    let frame_idx = (rover.wheel_anim as usize) % 3;
                    *r_tex = match rover.kind {
                        RoverKind::LightBuggy => assets.rover_light_frames[frame_idx].clone(),
                        RoverKind::HeavyCruiser => assets.rover_heavy_frames[frame_idx].clone(),
                    };

                    if rand::thread_rng().gen_bool(0.55) {
                        commands.spawn((
                            SurfaceEntity,
                            DustParticle {
                                iso_pos: rover.iso_pos,
                                iso_vel: Vec3::new(
                                    -rover.iso_vel.x * 0.28,
                                    -rover.iso_vel.y * 0.28,
                                    1.25,
                                ),
                                life: 0.0,
                                max_life: 0.85,
                                size: 6.0,
                                color: Color::rgba(0.72, 0.74, 0.80, 0.70),
                            },
                        ));
                    }
                }

                let drag = (-(if iso_input.length_squared() > 0.01 {
                    0.65
                } else {
                    3.5
                }) * dt)
                    .exp();
                rover.iso_vel *= drag;

                let next = rover.iso_pos.truncate() + rover.iso_vel * dt;
                if !map.blocks_movement(next.x, rover.iso_pos.y) {
                    rover.iso_pos.x = next.x;
                } else {
                    rover.iso_vel.x = 0.0;
                }
                if !map.blocks_movement(rover.iso_pos.x, next.y) {
                    rover.iso_pos.y = next.y;
                } else {
                    rover.iso_vel.y = 0.0;
                }
                rover.iso_pos.z = map.get_elevation(rover.iso_pos.x, rover.iso_pos.y);

                if rover.iso_pos.truncate().distance(Vec2::new(3.0, 0.0)) < 5.0 {
                    rover.battery = (rover.battery + dt * 16.0).min(rover.kind.max_battery());
                }
            }
            let rw = iso_to_world(rover.iso_pos.x, rover.iso_pos.y, rover.iso_pos.z);
            let rz = iso_depth(rover.iso_pos.x, rover.iso_pos.y, rover.iso_pos.z, 2.1);
            let tilt = (rover.wheel_anim * 2.1).sin() * 0.04 * (rover.iso_vel.length() / 5.0);
            if !player.is_passenger {
                r_tf.translation = Vec3::new(rw.x, rw.y + 18.0, rz);
            }
            r_tf.rotation = Quat::from_rotation_z(tilt);
            r_sprite.flip_x = !player.facing_right;

            let beam_dir = if player.facing_right {
                Vec2::new(65.0, -14.0)
            } else {
                Vec2::new(-65.0, -14.0)
            };
            let headlight_origin = Vec2::new(rw.x, rw.y + 12.0);
            gizmos.line_2d(
                headlight_origin,
                headlight_origin + beam_dir + Vec2::new(0.0, 16.0),
                Color::rgba(1.0, 0.96, 0.65, 0.42),
            );
            gizmos.line_2d(
                headlight_origin,
                headlight_origin + beam_dir + Vec2::new(0.0, -16.0),
                Color::rgba(1.0, 0.96, 0.65, 0.42),
            );

            player.iso_pos = rover.iso_pos;
            player_tf.translation = Vec3::new(rw.x, rw.y + 26.0, rz - 0.2);
            player_sprite.color = config.suit_color.to_color().with_a(0.0);
            if let Ok((_, _, mut v_vis)) = visor_q.get_single_mut() {
                *v_vis = Visibility::Hidden;
            }
            if let Ok((_, _, mut a_vis)) = acc_q.get_single_mut() {
                *a_vis = Visibility::Hidden;
            }
        }
    } else {
        player_sprite.color = config.suit_color.to_color();
        player_sprite.flip_x = false;

        if let Ok((mut v_tex, mut v_sp, mut v_vis)) = visor_q.get_single_mut() {
            *v_vis = Visibility::Inherited;
            *v_tex = assets.visor_for_angle(player.facing_angle_rad);
            let mut v_col = config.visor_tint.to_color();
            if player.inside_building {
                // Inside a pressurized habitat, the astronaut opens their visor!
                v_col.set_a(0.32);
            }
            v_sp.color = v_col;
            v_sp.flip_x = false;
        }
        if let Ok((mut a_tex, mut a_sp, mut a_vis)) = acc_q.get_single_mut() {
            *a_vis = Visibility::Inherited;
            *a_tex = assets.accessory_handle(config.accessory);
            a_sp.flip_x = player.facing_angle_rad.cos() < -0.25;
        }

        if player.airlock_anim_timer <= 0.0 {
            let accel = if player.grounded { 13.0 } else { 6.5 };
            player.iso_vel.x += iso_input.x * accel * dt;
            player.iso_vel.y += iso_input.y * accel * dt;

            let max_walk = 5.0;
            let horiz_spd = Vec2::new(player.iso_vel.x, player.iso_vel.y).length();
            if horiz_spd > max_walk {
                let clamped = Vec2::new(player.iso_vel.x, player.iso_vel.y).normalize() * max_walk;
                player.iso_vel.x = clamped.x;
                player.iso_vel.y = clamped.y;
            }

            let friction = if player.grounded { 3.4 } else { 0.85 };
            let damp = (-friction * dt).exp();
            player.iso_vel.x *= damp;
            player.iso_vel.y *= damp;

            // 1/6g Jump ([Space] or Touch Jump button)
            let jet_mult = if config.backpack_module == BackpackModule::HighImpulseRcsJet {
                1.45
            } else {
                1.0
            };
            if (keys.just_pressed(KeyCode::Space) || ui_jump)
                && player.grounded
                && !map.any_modal_open()
                && !player.inside_building
            {
                player.iso_vel.z = 3.35 * jet_mult;
                player.grounded = false;
                audio.play(SfxKind::JumpLowG, sfx_vol);
                for i in 0..6 {
                    let a = i as f32 * 1.05;
                    commands.spawn((
                        SurfaceEntity,
                        DustParticle {
                            iso_pos: player.iso_pos,
                            iso_vel: Vec3::new(a.cos() * 0.95, a.sin() * 0.95, 0.75),
                            life: 0.0,
                            max_life: 0.75,
                            size: 4.5,
                            color: Color::rgba(0.76, 0.80, 0.86, 0.75),
                        },
                    ));
                }
            }

            // Cold-Gas RCS Jetpack Hover ([Shift] or Touch JET button)
            let jet_active = keys.pressed(KeyCode::ShiftLeft)
                || keys.pressed(KeyCode::ShiftRight)
                || touch_ui.jetpack_held;
            if jet_active
                && !map.any_modal_open()
                && !player.grounded
                && !player.inside_building
                && stats.suit_power > 2.0
                && player.iso_pos.z - map.get_elevation(player.iso_pos.x, player.iso_pos.y) < 4.0
            {
                player.iso_vel.z = (player.iso_vel.z + 3.6 * jet_mult * dt).min(4.8);
                stats.suit_power = (stats.suit_power - dt * 4.2).max(0.0);
                if rand::thread_rng().gen_bool(0.18) {
                    audio.play(SfxKind::JetpackBurst, sfx_vol * 0.45);
                }
                if rand::thread_rng().gen_bool(0.65) {
                    commands.spawn((
                        SurfaceEntity,
                        DustParticle {
                            iso_pos: player.iso_pos,
                            iso_vel: Vec3::new(0.0, 0.0, -1.7),
                            life: 0.0,
                            max_life: 0.45,
                            size: 4.6,
                            color: config.trail_color.to_color(),
                        },
                    ));
                }
            }

            // Apply 1/6g Lunar Gravity
            player.iso_vel.z -= map.body.gravity() * dt;

            let next_x = player.iso_pos.x + player.iso_vel.x * dt;
            let next_y = player.iso_pos.y + player.iso_vel.y * dt;

            // If inside a building, keep the player inside connected indoor rooms until they exit via [E]!
            if player.inside_building {
                if map.is_inside_pressurized(next_x, next_y) {
                    player.iso_pos.x = next_x;
                    player.iso_pos.y = next_y;
                } else {
                    player.iso_vel.x = 0.0;
                    player.iso_vel.y = 0.0;
                    stats.status_log =
                        crate::locale::text("Ściana modułu! Aby wyjść na powierzchnię Księżyca, naciśnij [E] przy Śluzie.")
                            .to_string();
                }
            } else {
                if !map.blocks_movement(next_x, player.iso_pos.y) {
                    player.iso_pos.x = next_x;
                } else {
                    player.iso_vel.x = 0.0;
                }
                if !map.blocks_movement(player.iso_pos.x, next_y) {
                    player.iso_pos.y = next_y;
                } else {
                    player.iso_vel.y = 0.0;
                }
            }
            player.iso_pos.z += player.iso_vel.z * dt;
        }

        let horiz_spd = Vec2::new(player.iso_vel.x, player.iso_vel.y).length();
        let ground_z = map.get_elevation(player.iso_pos.x, player.iso_pos.y);
        if player.iso_pos.z <= ground_z {
            let impact_speed = -player.iso_vel.z;
            if !player.grounded && impact_speed > 4.6 {
                let dmg = (impact_speed - 4.6) * 11.0;
                stats.suit_integrity = (stats.suit_integrity - dmg).max(0.0);
                stats.health = (stats.health - dmg * 0.65).max(0.0);
                audio.play(SfxKind::AlertSiren, sfx_vol * 0.7);
                stats.status_log = moon_base_localize::localized_format!(
                    "Twarde lądowanie ({:.1} m/s). Skafander: -{:.0}% szczelności.",
                    impact_speed,
                    dmg
                );
            }
            player.iso_pos.z = ground_z;
            player.iso_vel.z = 0.0;
            player.grounded = true;

            if horiz_spd > 1.3 && !player.inside_building && rand::thread_rng().gen_bool(0.28) {
                commands.spawn((
                    SurfaceEntity,
                    DustParticle {
                        iso_pos: player.iso_pos,
                        iso_vel: Vec3::new(
                            -player.iso_vel.x * 0.22,
                            -player.iso_vel.y * 0.22,
                            0.70,
                        ),
                        life: 0.0,
                        max_life: 0.65,
                        size: 3.8,
                        color: Color::rgba(0.68, 0.72, 0.78, 0.55),
                    },
                ));
            }
        } else {
            player.grounded = false;
        }

        // Update 8-directional & 4-frame Astronaut Walking / Turning / Jumping Animation!
        player.action_swing = (player.action_swing - dt * 4.0).max(0.0);
        let mut stride_bob = 0.0f32;
        let mut body_tilt = (-angle_diff.clamp(-0.7, 0.7)) * 0.14;
        let anim_frame = if !player.grounded {
            body_tilt += if player.facing_right { -0.06 } else { 0.06 };
            3
        } else if horiz_spd > 0.35 || player.airlock_anim_timer > 0.0 {
            player.walk_cycle += dt * (horiz_spd.max(2.2)) * 2.8;
            let step_phase = (player.walk_cycle.sin() > 0.0) as usize;
            stride_bob = (player.walk_cycle * 2.0).sin().abs() * 4.5;
            body_tilt += player.walk_cycle.sin() * 0.05;
            1 + step_phase
        } else {
            player.walk_cycle += dt * 2.0;
            stride_bob = player.walk_cycle.sin() * 1.0;
            0
        };

        *player_tex = assets.astronaut_frame_for_angle(player.facing_angle_rad, anim_frame);

        if player.action_swing > 0.01 {
            body_tilt += if player.facing_right { -0.22 } else { 0.22 } * player.action_swing;
        }

        // Subtle 3D-like turning squash during rotation between directions
        let turn_squash = 1.0 - (angle_diff.abs() * 0.10).min(0.14);
        let pw = iso_to_world(player.iso_pos.x, player.iso_pos.y, player.iso_pos.z);
        let pz = iso_depth(player.iso_pos.x, player.iso_pos.y, player.iso_pos.z, 2.0);
        player_tf.translation = Vec3::new(pw.x, pw.y + 22.0 + stride_bob, pz);
        player_tf.rotation = Quat::from_rotation_z(body_tilt);
        player_tf.scale = Vec3::new(turn_squash, 1.0, 1.0);
    }

    let px = player.iso_pos.x.round() as i32;
    let py = player.iso_pos.y.round() as i32;
    stream_infinite_chunks_around(&mut commands, &assets, &mut map, px, py, config.scenario);

    // Smooth camera tracking + Balanced Camera Zoom (0.68x .. 1.55x)
    if let Ok((mut cam_tf, mut proj)) = cam_q.get_single_mut() {
        let target_scale = (1.0 / settings.camera_zoom.clamp(0.68, 1.55)).clamp(0.64, 1.48);
        proj.scale += (target_scale - proj.scale) * (dt * 10.0).min(1.0);

        let target_world = iso_to_world(player.iso_pos.x, player.iso_pos.y, player.iso_pos.z * 0.4);
        let lerp_speed = (dt * 6.5).min(1.0);
        cam_tf.translation.x += (target_world.x - cam_tf.translation.x) * lerp_speed;
        cam_tf.translation.y += (target_world.y - cam_tf.translation.y) * lerp_speed;

        if events.camera_shake_intensity > 0.01 {
            let mut rng = rand::thread_rng();
            cam_tf.translation.x += rng.gen_range(-1.0..1.0) * events.camera_shake_intensity * 9.0;
            cam_tf.translation.y += rng.gen_range(-1.0..1.0) * events.camera_shake_intensity * 9.0;
        }
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn update_coop_and_lan_peers(
    time: Res<Time>,
    mut config: ResMut<MissionConfig>,
    mut settings: ResMut<GameSettings>,
    audio: Res<GameAudio>,
    assets: Res<ProceduralAssets>,
    map: Res<LunarMap>,
    mut stats: ResMut<SurvivalStats>,
    mut lan: ResMut<LanMultiplayer>,
    mut touch_ui: ResMut<TouchAndUiInput>,
    mut player_q: Query<&mut AstronautPlayer>,
    mut crew_q: Query<(
        &mut CoOpCrewMember,
        &mut Transform,
        &mut Sprite,
        &mut Handle<Image>,
        &mut Visibility,
    )>,
) {
    let dt = time.delta_seconds().min(0.05);
    let now = time.elapsed_seconds();
    let sfx_vol = settings.sfx_volume * settings.master_volume;

    let mut switch_requested = false;
    let mut remaining = Vec::new();
    for cmd in touch_ui.cmds.drain(..) {
        match cmd {
            SurfaceUiCmd::SwitchCrewMember => switch_requested = true,
            other => remaining.push(other),
        }
    }
    touch_ui.cmds = remaining;

    lan.crew_blend = (lan.crew_blend - dt).max(0.0);
    lan.crew_cooldown = (lan.crew_cooldown - dt).max(0.0);
    if switch_requested && lan.is_active {
        stats.status_log = crate::locale::text(
            "W sieci sterujesz swoim astronautą. Zmienisz jego wygląd w panelu K.",
        )
        .into();
    }
    if switch_requested && !lan.is_active && !map.piloting_station && !lan.peers.is_empty() {
        settings.co_op_ai_crew = true;
        if let Ok(mut player) = player_q.get_single_mut() {
            if player.mounted_rover.is_some()
                || player.inside_building
                || !player.grounded
                || player.airlock_anim_timer > 0.0
                || lan.crew_cooldown > 0.0
            {
                stats.status_log = crate::locale::text(
                    "Aby zmienić astronautę, wyjdź z kabiny i stań bezpiecznie na ziemi.",
                )
                .into();
                return;
            }
            let bot_indices: Vec<usize> = lan
                .peers
                .iter()
                .enumerate()
                .filter_map(|(idx, peer)| peer.is_ai_bot.then_some(idx))
                .collect();
            let Some(&idx) = bot_indices.get(lan.active_crew_index % bot_indices.len().max(1))
            else {
                return;
            };
            let target = lan.peers[idx].iso_pos;
            let destination = if map.blocks_movement(target.x, target.y)
                || map.is_inside_pressurized(target.x, target.y)
            {
                map.safe_exit(target.truncate(), 4.0)
            } else {
                Some(target.truncate())
            };
            let Some(destination) = destination else {
                stats.status_log =
                    crate::locale::text("Ten astronauta nie ma bezpiecznego miejsca do przejęcia.")
                        .into();
                return;
            };
            let peer = &mut lan.peers[idx];
            peer.iso_pos = destination.extend(map.get_elevation(destination.x, destination.y));
            std::mem::swap(&mut player.iso_pos, &mut peer.iso_pos);
            std::mem::swap(&mut player.facing_right, &mut peer.facing_right);
            std::mem::swap(&mut config.callsign, &mut peer.callsign);
            std::mem::swap(&mut config.active_role, &mut peer.role);
            std::mem::swap(&mut config.suit_color, &mut peer.suit);
            std::mem::swap(&mut config.visor_tint, &mut peer.visor);
            std::mem::swap(&mut config.accessory, &mut peer.accessory);
            std::mem::swap(&mut config.astronaut_trait, &mut peer.spec_trait);

            player.iso_vel = Vec3::ZERO;
            player.airlock_anim_timer = 0.0;
            player.action_swing = 0.0;
            player.grounded = true;
            lan.crew_blend = 0.35;
            lan.crew_cooldown = 0.65;
            lan.active_crew_index = (lan.active_crew_index + 1) % bot_indices.len();
            audio.play(SfxKind::UiConfirm, sfx_vol);
            stats.status_log = moon_base_localize::localized_format!(
                "Przełączono główną postać na: {} ({} | {})!",
                config.callsign,
                config.active_role.title(),
                config.astronaut_trait.name()
            );
        }
    }

    let lan_active = lan.is_active;
    for (mut crew, mut tf, mut sprite, mut tex, mut vis) in &mut crew_q {
        let Some(peer) = lan.peers.get_mut(crew.peer_index) else {
            *vis = Visibility::Hidden;
            continue;
        };

        if (peer.is_ai_bot && !settings.co_op_ai_crew)
            || (!peer.is_ai_bot
                && (!lan_active
                    || peer.body != map.body
                    || now - peer.last_seen_sec > 10.0
                    || peer.vehicle.is_some()))
        {
            *vis = Visibility::Hidden;
            continue;
        }
        *vis = Visibility::Inherited;

        if peer.is_ai_bot {
            peer.vel_xy = Vec2::ZERO;
            peer.iso_pos.z = map.get_elevation(peer.iso_pos.x, peer.iso_pos.y);

            if rand::thread_rng().gen_bool((dt * 0.10) as f64) {
                // Stationed crew do not generate research without samples.
            }
        }

        if !peer.is_ai_bot {
            peer.iso_pos = crate::net_lan::interpolated_position(peer, now);
        }
        crew.walk_cycle += dt * (2.0 + peer.vel_xy.length());
        let frame = if peer.vel_xy.length() > 0.25 {
            1 + ((crew.walk_cycle.sin() > 0.0) as usize)
        } else {
            0
        };
        let dir_idx = if peer.vel_xy.length() > 0.2 {
            ProceduralAssets::angle_to_dir_index(peer.vel_xy.y.atan2(peer.vel_xy.x))
        } else if peer.facing_right {
            1
        } else {
            7
        };
        *tex = assets.astronaut_dir_frames[dir_idx][frame].clone();
        sprite.flip_x = false;
        sprite.color = peer.suit.to_color();

        let idle_bob = crew.walk_cycle.sin() * 1.0;
        let w_pos = iso_to_world(peer.iso_pos.x, peer.iso_pos.y, peer.iso_pos.z);
        let z = iso_depth(peer.iso_pos.x, peer.iso_pos.y, peer.iso_pos.z, 1.95);
        let dance = !peer.is_ai_bot && peer.emote_until > now;
        let sway = if dance { (now * 9.0).sin() * 0.20 } else { 0.0 };
        tf.rotation = Quat::from_rotation_z(sway);
        tf.translation = Vec3::new(
            w_pos.x + if dance { sway * 14.0 } else { 0.0 },
            w_pos.y
                + 22.0
                + idle_bob
                + if dance {
                    (now * 8.0).sin().abs() * 5.0
                } else {
                    0.0
                },
            z,
        );
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn handle_mining_building_crafting_and_throwing(
    mut commands: Commands,
    (time, mouse, keys): (
        Res<Time>,
        Res<ButtonInput<MouseButton>>,
        Res<ButtonInput<KeyCode>>,
    ),
    windows: Query<&Window, With<PrimaryWindow>>,
    (config, settings, audio, assets): (
        Res<MissionConfig>,
        Res<GameSettings>,
        Res<GameAudio>,
        Res<ProceduralAssets>,
    ),
    mut lan: ResMut<LanMultiplayer>,
    mut touch_ui: ResMut<TouchAndUiInput>,
    mut events: ResMut<LunarEventManager>,
    mut map: ResMut<LunarMap>,
    mut stats: ResMut<SurvivalStats>,
    mut player_q: Query<&mut AstronautPlayer>,
    struct_q: Query<(Entity, &StructureSpriteMarker)>,
    conn_q: Query<(Entity, &StructureConnectorMarker)>,
) {
    if lan.is_active && !lan.is_host && map.body != lan.current_body {
        return;
    }
    let dt = time.delta_seconds().min(0.05);
    let Ok(mut player) = player_q.get_single_mut() else {
        return;
    };
    let sfx_vol = settings.sfx_volume * settings.master_volume;

    if lan.is_active && !lan.is_host && keys.just_pressed(KeyCode::KeyR) && !map.any_modal_open() {
        lan.send(crate::net_lan::Message::Action(
            crate::net_world::Action::Command(SurfaceUiCmd::UseRation),
        ));
    }
    if lan.is_active && !lan.is_host && keys.just_pressed(KeyCode::KeyT) && !map.any_modal_open() {
        lan.send(crate::net_lan::Message::Action(
            crate::net_world::Action::Command(SurfaceUiCmd::DrinkWater),
        ));
    }
    if lan.is_active && !lan.is_host && keys.just_pressed(KeyCode::KeyC) && !map.any_modal_open() {
        lan.send(crate::net_lan::Message::Action(
            crate::net_world::Action::Command(SurfaceUiCmd::CraftNextTool),
        ));
    }
    if lan.is_active && !lan.is_host && keys.just_pressed(KeyCode::KeyF) && !map.any_modal_open() {
        lan.send(crate::net_lan::Message::Action(
            crate::net_world::Action::Command(SurfaceUiCmd::PatchSuit),
        ));
    }
    let mut server_target = None;
    let mut ui_craft = false;
    let mut ui_ration = false;
    let mut ui_drink = false;
    let mut make_water = false;
    let mut make_meal = false;
    let mut ui_medkit = false;
    let mut ui_patch = false;
    let mut ui_flag = false;
    let mut ui_flare = false;
    let mut ui_mine_or_build = false;
    let mut ui_add_wp_player = false;
    let mut ui_rem_wp = false;

    if touch_ui.action_held && !map.build_mode {
        touch_ui.cmds.push(SurfaceUiCmd::MineOrBuildAtPlayer);
    }
    for cmd in touch_ui.cmds.drain(..) {
        if lan.is_active && !lan.is_host && crate::net_world::shared_command(cmd) {
            lan.send(crate::net_lan::Message::Action(
                crate::net_world::Action::Command(cmd),
            ));
            continue;
        }
        match cmd {
            SurfaceUiCmd::ServerMine(x, y) => {
                server_target = Some((x, y));
                map.build_mode = false;
                map.mining_target = Some((x, y));
                map.mining_progress = 1.0;
                ui_mine_or_build = true;
            }
            SurfaceUiCmd::ServerBuild(x, y, index) => {
                server_target = Some((x, y));
                map.build_mode = true;
                map.selected_build = index;
                ui_mine_or_build = true;
            }
            SurfaceUiCmd::SetBuildMode(val) => {
                map.demolition_mode = false;
                map.build_mode = val;
                map.mining_progress = 0.0;
                map.mining_target = None;
                audio.play(SfxKind::UiClick, sfx_vol);
            }
            SurfaceUiCmd::SelectStructure(idx) => {
                map.demolition_mode = false;
                let clamped = idx.min(StructureKind::ALL.len() - 1);
                map.mining_progress = 0.0;
                map.mining_target = None;
                if map.build_mode && map.selected_build == clamped {
                    map.build_mode = false;
                } else {
                    map.selected_build = clamped;
                    map.build_mode = true;
                }
                audio.play(SfxKind::UiClick, sfx_vol);
            }
            SurfaceUiCmd::DropCargo(kind, all) => {
                crate::mineral::drop_cargo(
                    &mut commands,
                    &assets,
                    &map,
                    &mut stats,
                    &player,
                    kind,
                    all,
                );
            }
            SurfaceUiCmd::SetDemolitionMode => {
                map.demolition_mode = !map.demolition_mode;
                map.build_mode = false;
                map.mining_progress = 0.0;
                map.mining_target = None;
                stats.status_log = if map.demolition_mode {
                    crate::locale::text(
                        "Rozbiórka: przytrzymaj LPM na budynku. Materiały spadną na ziemię.",
                    )
                } else {
                    crate::locale::text("Kilof gotowy do wydobycia.")
                }
                .into();
            }
            SurfaceUiCmd::StoreCargo | SurfaceUiCmd::TakeCargo => {
                let near = map.cells.iter().any(|(&(x, y), c)| {
                    c.structure == Some(StructureKind::StorageBay)
                        && c.structure_hp > 10.0
                        && Vec2::new(x as f32, y as f32).distance(player.iso_pos.truncate()) < 2.0
                });
                if !near {
                    stats.status_log = crate::locale::text("Podejdź do magazynu bazy.").into();
                    continue;
                }
                for kind in 0..11 {
                    let count = if matches!(cmd, SurfaceUiCmd::StoreCargo) {
                        stats.cargo_counts()[kind]
                    } else {
                        let room = ((stats.cargo_limit() - stats.cargo_weight()).max(0.0)
                            / SurvivalStats::item_weight(kind))
                        .floor() as u32;
                        room.min(map.storage[kind])
                    };
                    if matches!(cmd, SurfaceUiCmd::StoreCargo) {
                        stats.change_cargo(kind, -(count as i32));
                        map.storage[kind] += count;
                    } else {
                        stats.change_cargo(kind, count as i32);
                        map.storage[kind] -= count;
                    }
                }
                stats.status_log = moon_base_localize::localized_format!(
                    "Magazyn: {} rzeczy · Plecak {:.1}/35 kg",
                    map.storage.iter().sum::<u32>(),
                    stats.cargo_weight()
                );
            }
            SurfaceUiCmd::CraftNextTool => ui_craft = true,
            SurfaceUiCmd::UpgradeBackpack => {
                let level = u32::from(stats.backpack_level) + 1;
                let (rock, titanium, silicon) = (10 * level, 4 * level, 3 * level);
                if stats.backpack_level >= 5 {
                    stats.status_log =
                        crate::locale::text("Plecak ma już maksymalną pojemność 110 kg.").into();
                } else if stats.creative_cargo
                    || (stats.regolith >= rock
                        && stats.titanium >= titanium
                        && stats.silicon >= silicon)
                {
                    if !stats.creative_cargo {
                        stats.regolith -= rock;
                        stats.titanium -= titanium;
                        stats.silicon -= silicon;
                    }
                    stats.backpack_level += 1;
                    stats.status_log = moon_base_localize::localized_format!(
                        "Ulepszono plecak · {:.0} kg",
                        stats.cargo_limit()
                    );
                } else {
                    stats.status_log=moon_base_localize::localized_format!("Plecak +15 kg: potrzeba {rock} regolitu, {titanium} tytanu, {silicon} krzemu.");
                }
            }
            SurfaceUiCmd::GrantCargo(kind, amount) => {
                if config.scenario == MissionScenario::OpenSandbox && kind < 11 {
                    stats.change_cargo(kind, amount.min(i32::MAX as u32) as i32);
                    stats.status_log = format!(
                        "Sandbox · dodano {} × {}",
                        amount,
                        crate::mineral::ITEM_NAMES[kind]
                    );
                }
            }
            SurfaceUiCmd::UseRation => ui_ration = true,
            SurfaceUiCmd::DrinkWater => ui_drink = true,
            SurfaceUiCmd::MakeWater => make_water = true,
            SurfaceUiCmd::MakeMeal => make_meal = true,
            SurfaceUiCmd::UseMedkit => ui_medkit = true,
            SurfaceUiCmd::PatchSuit => ui_patch = true,
            SurfaceUiCmd::PlantPolishFlag => ui_flag = true,
            SurfaceUiCmd::ThrowSurveyFlare => ui_flare = true,
            SurfaceUiCmd::AddWaypointAtPlayer => ui_add_wp_player = true,
            SurfaceUiCmd::MarkAt(x, y) => {
                if Vec2::new(x as f32, y as f32).distance(player.iso_pos.truncate()) <= 45.0
                    && map.waypoints.len() < 128
                    && !map.waypoints.iter().any(|w| w.gx == x && w.gy == y)
                {
                    map.waypoints.push(Waypoint {
                        name: crate::locale::text("Znacznik załogi").into(),
                        gx: x,
                        gy: y,
                        color: Color::rgb(1.0, 0.79, 0.36),
                    });
                    stats.placed_waypoint_once = true;
                }
            }
            SurfaceUiCmd::RemoveLastWaypoint => ui_rem_wp = true,
            SurfaceUiCmd::MineOrBuildAtPlayer => {
                let step = if player.facing_right {
                    IVec2::new(1, 0)
                } else {
                    IVec2::new(-1, 0)
                };
                map.hovered_tile = (
                    player.iso_pos.x.round() as i32 + step.x,
                    player.iso_pos.y.round() as i32 + step.y,
                );
                ui_mine_or_build = true;
            }
            _ => {}
        }
    }

    if let Some(pos) = server_target {
        map.hovered_tile = pos;
    }
    // Waypoint management commands (from Map modal [M] or hotkeys)
    if ui_add_wp_player {
        let wx = player.iso_pos.x.round() as i32;
        let wy = player.iso_pos.y.round() as i32;
        let idx = map.waypoints.len() + 1;
        let col = match idx % 4 {
            0 => Color::rgb(0.35, 0.96, 1.0),
            1 => Color::rgb(1.0, 0.84, 0.32),
            2 => Color::rgb(0.42, 1.0, 0.65),
            _ => Color::rgb(1.0, 0.45, 0.65),
        };
        map.waypoints.push(Waypoint {
            name: format!("Punkt Nawigacyjny #{} ({}, {})", idx, wx, wy),
            gx: wx,
            gy: wy,
            color: col,
        });
        stats.placed_waypoint_once = true;
        audio.play(SfxKind::UiConfirm, sfx_vol);
        stats.status_log = moon_base_localize::localized_format!(
            "Dodano nowy Waypoint na Twojej pozycji ({}, {})!",
            wx,
            wy
        );
    }
    if ui_rem_wp && map.waypoints.len() > 1 {
        let removed = map.waypoints.pop().unwrap();
        audio.play(SfxKind::UiClick, sfx_vol);
        stats.status_log =
            moon_base_localize::localized_format!("Usunięto Waypoint: {}", removed.name);
    }

    if (keys.just_pressed(KeyCode::KeyR)
        && !(lan.is_active && !lan.is_host)
        && !map.any_modal_open())
        || ui_ration
    {
        if stats.hunger >= 99.5 {
            stats.status_log = crate::locale::text("Masz pełny zapas energii z jedzenia.").into();
        } else if stats.fresh_meals > 0 || stats.rations > 0 {
            if stats.fresh_meals > 0 {
                stats.fresh_meals -= 1;
                stats.hunger = (stats.hunger + 60.0).min(100.0);
                stats.status_log =
                    crate::locale::text("Zjedzono świeży posiłek ze szklarni.").into();
            } else {
                stats.rations -= 1;
                stats.hunger = (stats.hunger + 45.0).min(100.0);
                stats.status_log = crate::locale::text("Zjedzono rację z plecaka.").into();
            }
            audio.play(SfxKind::UiConfirm, sfx_vol);
        } else {
            stats.status_log = crate::locale::text(
                "Brak jedzenia. Otwórz kapsułę z zapasami lub wyhoduj posiłek w szklarni.",
            )
            .into();
        }
    }

    if (keys.just_pressed(KeyCode::KeyT)
        && !(lan.is_active && !lan.is_host)
        && !map.any_modal_open())
        || ui_drink
    {
        if stats.hydration >= 99.5 {
            stats.status_log = crate::locale::text("Masz pełny zapas wody w organizmie.").into();
        } else if stats.drinking_water > 0 {
            stats.drinking_water -= 1;
            stats.status_log = crate::locale::text("Wypito porcję wody.").into();
            stats.hydration = (stats.hydration + 50.0).min(100.0);
            audio.play(SfxKind::UiConfirm, sfx_vol);
        } else {
            stats.status_log =
                crate::locale::text("Brak wody. W bazie wybierz „Zrób wodę”, żeby przetopić lód.")
                    .into();
        }
    }

    if make_water || make_meal {
        use crate::base::SupplyRecipe;
        let pos = player.iso_pos.truncate().round().as_ivec2();
        for (requested, recipe) in [
            (make_water, SupplyRecipe::Water),
            (make_meal, SupplyRecipe::Meal),
        ] {
            if !requested {
                continue;
            }
            let result = if player.inside_building
                && player.airlock_anim_timer <= 0.0
                && !map.piloting_station
            {
                recipe.start(&mut map, &mut stats, (pos.x, pos.y))
            } else {
                Err(crate::locale::text(
                    "Wejdź do bazy, aby przygotować zapasy.",
                ))
            };
            stats.status_log = match result {
                Ok(message) => {
                    audio.play(SfxKind::CraftUpgrade, sfx_vol);
                    message
                }
                Err(message) => message,
            }
            .into();
        }
    }

    if map.piloting_station {
        return;
    }

    // 1. Progressive Tool Crafting ([C] or UI Button)
    if (keys.just_pressed(KeyCode::KeyC) && !(lan.is_active && !lan.is_host)) || ui_craft {
        if let Some(next_tool) = stats.highest_tool.next() {
            let (req_reg, req_ice, req_ti, req_si) = next_tool.craft_cost();
            if config.scenario == MissionScenario::OpenSandbox
                || (stats.regolith >= req_reg
                    && stats.water_ice >= req_ice
                    && stats.titanium >= req_ti
                    && stats.silicon >= req_si)
            {
                if config.scenario != MissionScenario::OpenSandbox {
                    stats.regolith -= req_reg;
                    stats.water_ice -= req_ice;
                    stats.titanium -= req_ti;
                    stats.silicon -= req_si;
                }
                stats.highest_tool = next_tool;
                audio.play(SfxKind::CraftUpgrade, sfx_vol);
                stats.status_log = moon_base_localize::localized_format!(
                    "ULEPSZONO NARZĘDZIE: {}! {}",
                    next_tool.name(),
                    next_tool.description()
                );
            } else {
                audio.play(SfxKind::UiClick, sfx_vol);
                stats.status_log = moon_base_localize::localized_format!(
                    "Brak surowców na {}: potrzeba {} Regolitu, {} Lodu, {} Tytanu, {} Krzemu.",
                    next_tool.short_name(),
                    req_reg,
                    req_ice,
                    req_ti,
                    req_si
                );
            }
        } else {
            stats.status_log =
                crate::locale::text("Posiadasz już maksymalne ulepszenie narzędzi (Wiertło Kwantowe Mk.VI — 6x szybkość)!")
                    .to_string();
        }
    }

    // Flare launched from the backpack.
    if ui_flare && time.elapsed_seconds() < stats.flare_ready_at {
        stats.status_log = moon_base_localize::localized_format!(
            "Następna flara za {:.1} s",
            stats.flare_ready_at - time.elapsed_seconds()
        );
    }
    if ui_flare && time.elapsed_seconds() >= stats.flare_ready_at {
        stats.flare_ready_at = time.elapsed_seconds() + 6.0;
        let (hx, hy) = map.hovered_tile;
        let target = Vec2::new(hx as f32, hy as f32);
        let dir = (target - player.iso_pos.truncate()).normalize_or_zero();
        let dist = player.iso_pos.truncate().distance(target).clamp(2.5, 9.0);
        let flight_vel_xy = dir * (dist * 0.85);
        commands.spawn((
            SurfaceEntity,
            ThrownObject {
                iso_pos: player.iso_pos + Vec3::new(0.0, 0.0, 0.5),
                iso_vel: Vec3::new(flight_vel_xy.x, flight_vel_xy.y, 2.85),
                ttl: 5.5,
                is_flare: true,
                is_meteor: false,
            },
        ));
        audio.play(SfxKind::FlareThrow, sfx_vol);
        stats.status_log = crate::locale::text("Flara oświetla i skanuje teren.").to_string();
    }

    if ui_medkit {
        if stats.medkits > 0 {
            stats.medkits -= 1;
            stats.health = (stats.health + 50.0).min(100.0);

            audio.play(SfxKind::UiConfirm, sfx_vol);
            stats.status_log = crate::locale::text(
                "Użyto apteczki: +50 zdrowia. Dawka promieniowania pozostaje w historii.",
            )
            .to_string();
        } else if stats.water_ice >= 6 && stats.regolith >= 6 {
            stats.water_ice -= 6;
            stats.regolith -= 6;
            stats.medkits += 1;

            audio.play(SfxKind::CraftUpgrade, sfx_vol);
            stats.status_log =
                crate::locale::text("Apteczka gotowa. Naciśnij X, aby jej użyć.").to_string();
        } else {
            stats.status_log = crate::locale::text(
                "Brak Apteczek! Szukaj Kapsuł Zaopatrzeniowych lub wykop 6 Lodu i 6 Regolitu.",
            )
            .to_string();
        }
    }

    if (keys.just_pressed(KeyCode::KeyF) && !(lan.is_active && !lan.is_host)) || ui_patch {
        let repair_in_base = map.campaign_route && player.inside_building && stats.local_powered;
        if stats.suit_integrity < 99.0 && repair_in_base {
            stats.suit_integrity = 100.0;
            stats.status_log = crate::locale::text("Warsztat bazy uszczelnił skafander.").into();
            audio.play(SfxKind::CraftUpgrade, sfx_vol);
        } else if stats.suit_integrity < 99.0 && stats.regolith >= 5 && stats.titanium >= 2 {
            stats.regolith -= 5;
            stats.titanium -= 2;
            stats.suit_integrity = (stats.suit_integrity + 40.0).min(100.0);
            audio.play(SfxKind::CraftUpgrade, sfx_vol);
            stats.status_log =
                crate::locale::text("Naprawiono poszycie skafandra (+40% Szczelności).")
                    .to_string();
        } else if stats.suit_integrity >= 99.0 {
            stats.status_log =
                crate::locale::text("Twój skafander jest w 100% szczelny!").to_string();
        } else {
            stats.status_log =
                crate::locale::text("Naprawa skafandra wymaga 5 Regolitu i 2 Tytanu.").to_string();
        }
    }

    if ui_flag {
        if time.elapsed_seconds() < stats.placement_ready_at {
            stats.status_log = format!(
                "Stawianie za {:.1} s",
                stats.placement_ready_at - time.elapsed_seconds()
            );
            return;
        }
        let fx = player.iso_pos.x.round() as i32 + if player.facing_right { 1 } else { -1 };
        let fy = player.iso_pos.y.round() as i32;
        if let Some(cell) = map.cells.get_mut(&(fx, fy)) {
            if cell.structure.is_none() && cell.feature.is_none() {
                cell.feature = Some(WorldFeatureKind::LunarFlagPlaque);
                let w_pos = iso_to_world(fx as f32, fy as f32, cell.elevation);
                let fz = iso_depth(fx as f32, fy as f32, cell.elevation, 1.5);
                let f_ent = commands
                    .spawn((
                        SurfaceEntity,
                        crate::world::WorldFeatureSpriteMarker { gx: fx, gy: fy },
                        SpriteBundle {
                            texture: assets.feature_handle(WorldFeatureKind::LunarFlagPlaque),
                            transform: Transform::from_xyz(w_pos.x, w_pos.y + 22.0, fz),
                            ..default()
                        },
                    ))
                    .id();
                cell.feature_entity = Some(f_ent);
                stats.planted_flag_once = true;
                stats.placement_ready_at = time.elapsed_seconds() + 0.5;
                map.waypoints.push(Waypoint {
                    name: crate::locale::text("Flaga ekspedycji").into(),
                    gx: fx,
                    gy: fy,
                    color: Color::WHITE,
                });
                audio.play(SfxKind::BuildModule, sfx_vol);
                stats.generated_science_gb += 5.0;
                stats.status_log = moon_base_localize::localized_format!(
                    "Wbito Biało-Czerwoną Flagę Kosmiczną PL na ({}, {})! Polska w kosmosie!",
                    fx,
                    fy
                );
                return;
            }
        }
        stats.status_log =
            crate::locale::text("Brak wolnego pola przed astronautą. Podejdź w inne miejsce.")
                .into();
        return;
    }

    // Check if mouse is on the 3D playfield (not on top/bottom HUD bars or open modals)
    let cursor_on_playfield = windows
        .get_single()
        .ok()
        .and_then(|w| w.cursor_position())
        .map(|_| !touch_ui.pointer_over_ui && !map.any_modal_open())
        .unwrap_or(false);

    let left_just_clicked =
        (mouse.just_pressed(MouseButton::Left) && cursor_on_playfield) || ui_mine_or_build;
    let left_held = (mouse.pressed(MouseButton::Left) && cursor_on_playfield) || ui_mine_or_build;
    let right_just_clicked = mouse.just_pressed(MouseButton::Right) && cursor_on_playfield;

    let (gx, gy) = map.hovered_tile;
    let dist_to_tile = player
        .iso_pos
        .truncate()
        .distance(Vec2::new(gx as f32, gy as f32));

    let has_building = map
        .cells
        .get(&(gx, gy))
        .map(|c| c.structure.is_some() && c.structure_hp > 0.0)
        .unwrap_or(false);
    let has_feature = map
        .cells
        .get(&(gx, gy))
        .map(|c| c.feature.is_some())
        .unwrap_or(false);
    if map.campaign_route
        && ((gx, gy) == crate::campaign::RELAY && map.body == WorldBody::Moon
            || map
                .cells
                .get(&(gx, gy))
                .is_some_and(|c| c.is_world_generated_base && c.structure.is_some()))
    {
        if left_just_clicked || right_just_clicked {
            stats.status_log =
                crate::locale::text("To wyposażenie bazy misji. Buduj na sąsiednim wolnym polu.")
                    .into();
        }
        map.mining_progress = 0.0;
        map.mining_target = None;
        return;
    }

    // A) BUILDING PLACEMENT (Instant on LPM in Build Mode, or PPM when tile is empty!)
    let want_build = map.cells.contains_key(&(gx, gy))
        && !has_building
        && !has_feature
        && ((left_just_clicked && map.build_mode) || right_just_clicked);

    if want_build && lan.is_active && !lan.is_host {
        if dist_to_tile <= 7.5 {
            let index = map.selected_build;
            lan.send(crate::net_lan::Message::Action(
                crate::net_world::Action::Command(SurfaceUiCmd::ServerBuild(gx, gy, index)),
            ));
        }
        return;
    }
    if want_build {
        if time.elapsed_seconds() < stats.placement_ready_at {
            stats.status_log = format!(
                "Stawianie za {:.1} s",
                stats.placement_ready_at - time.elapsed_seconds()
            );
            return;
        }
        if dist_to_tile > 7.5 {
            stats.status_log = moon_base_localize::localized_format!(
                "Kafelek ({}, {}) jest za daleko ({:.1}m). Podejdź bliżej (do 7.5m)!",
                gx,
                gy,
                dist_to_tile
            );
            return;
        }
        // Turn astronaut toward the tile they are building on!
        let build_dir = Vec2::new(gx as f32 - player.iso_pos.x, gy as f32 - player.iso_pos.y);
        if build_dir.length_squared() > 0.01 {
            let screen_vec = Vec2::new(build_dir.x - build_dir.y, -(build_dir.x + build_dir.y));
            player.target_facing_rad = screen_vec.y.atan2(screen_vec.x);
        }
        player.action_swing = 1.0;

        let kind = StructureKind::ALL[map.selected_build];
        let req_tool = kind.required_tool();
        if (stats.highest_tool as u8) < (req_tool as u8)
            && config.scenario != MissionScenario::OpenSandbox
        {
            stats.status_log = moon_base_localize::localized_format!(
                "Budowa {} wymaga narzędzia: {}! Kliknij [Ulepsz (C)].",
                kind.short_name(),
                req_tool.short_name()
            );
            return;
        }

        if gx.abs() <= 1 && gy.abs() <= 1 {
            stats.status_log =
                crate::locale::text("Wybierz wolne miejsce obok lądowiska.").to_string();
            return;
        }

        let (mut r_cost, mut ti_cost, mut si_cost, he_cost) = kind.build_cost();
        if config.astronaut_trait == AstronautTrait::SystemsArchitect {
            r_cost = (r_cost * 3) / 4;
            ti_cost = (ti_cost * 3) / 4;
            si_cost = (si_cost * 3) / 4;
        }

        if config.scenario == MissionScenario::OpenSandbox
            || (stats.regolith >= r_cost
                && stats.titanium >= ti_cost
                && stats.silicon >= si_cost
                && stats.helium3 >= he_cost)
        {
            if config.scenario != MissionScenario::OpenSandbox {
                stats.regolith -= r_cost;
                stats.titanium -= ti_cost;
                stats.silicon -= si_cost;
                stats.helium3 -= he_cost;
            }
            if let Some(cell) = map.cells.get_mut(&(gx, gy)) {
                cell.structure = Some(kind);
                cell.structure_hp = 100.0;
                cell.charge_wh = 0.0;
                cell.paid_cost = Some(if config.scenario == MissionScenario::OpenSandbox {
                    (0, 0, 0, 0)
                } else {
                    (r_cost, ti_cost, si_cost, he_cost)
                });
                cell.is_world_generated_base = false;
            }
            refresh_structure_cluster_around(
                &mut commands,
                &assets,
                &map,
                gx,
                gy,
                &struct_q,
                &conn_q,
            );

            if matches!(kind, StructureKind::HabitatModule | StructureKind::Airlock) {
                stats.built_habitat_once = true;
            }
            let has_power = map.cells.values().any(|c| {
                matches!(
                    c.structure,
                    Some(StructureKind::SolarArray | StructureKind::RtgGenerator)
                ) && c.structure_hp > 0.0
            });
            let has_o2 = map.cells.values().any(|c| {
                c.structure == Some(StructureKind::OxygenElectrolyzer) && c.structure_hp > 0.0
            });
            if has_power && has_o2 {
                stats.built_power_and_o2 = true;
            }

            let neighbors = map.connected_neighbors_count(gx, gy, kind);
            audio.play(SfxKind::BuildModule, sfx_vol);
            stats.generated_science_gb += 12.0 + neighbors as f32 * 4.0;
            stats.placement_ready_at = time.elapsed_seconds() + 0.5;
            stats.status_log = if neighbors > 0 {
                moon_base_localize::localized_format!(
                    "Zbudowano {} na ({}, {}) — POŁĄCZONO W JEDNOŚĆ z {} sąsiednimi modułami (+{}% wydajności)! Naciśnij [E] przy Śluzie by wejść!",
                    kind.short_name(),
                    gx,
                    gy,
                    neighbors,
                    neighbors * 15
                )
            } else {
                moon_base_localize::localized_format!(
                    "Zbudowano {} na ({}, {})! Postaw obok Śluzę [1] lub kolejny moduł, aby połączyć je w dom!",
                    kind.short_name(),
                    gx,
                    gy
                )
            };
        } else {
            stats.status_log = moon_base_localize::localized_format!(
                "Brak surowców na {}: potrzeba {} Regolit, {} Tytan, {} Krzem, {} Hel-3.",
                kind.short_name(),
                r_cost,
                ti_cost,
                si_cost,
                he_cost
            );
        }
        return;
    }

    // If right-clicked in Build Mode on an empty/existing spot without building, switch back to Pickaxe Mode!
    if right_just_clicked && map.build_mode {
        map.build_mode = false;
        map.mining_progress = 0.0;
        map.mining_target = None;
        audio.play(SfxKind::UiClick, sfx_vol);
        stats.status_log =
            crate::locale::text("Kilof gotowy. Przytrzymaj lewy przycisk myszy na skale.")
                .to_string();
        return;
    }

    // B) PROGRESSIVE BLOCK MINING, LOOTING & BUILDING DECONSTRUCTION (Hold LPM!)
    let want_progressive_mine = left_held && !map.build_mode;

    if !want_progressive_mine {
        map.mining_progress = (map.mining_progress - dt * 3.5).max(0.0);
        if map.mining_progress <= 0.01 {
            map.mining_target = None;
            map.mining_label.clear();
        }
        return;
    }

    if dist_to_tile > 7.5 {
        map.mining_progress = 0.0;
        map.mining_target = None;
        if left_just_clicked {
            stats.status_log = crate::locale::text("Podejdź bliżej i spróbuj jeszcze raz.").into();
        }
        return;
    }

    // Smoothly turn the astronaut toward the block being mined!
    let mine_dir = Vec2::new(gx as f32 - player.iso_pos.x, gy as f32 - player.iso_pos.y);
    if mine_dir.length_squared() > 0.01 {
        let screen_vec = Vec2::new(mine_dir.x - mine_dir.y, -(mine_dir.x + mine_dir.y));
        player.target_facing_rad = screen_vec.y.atan2(screen_vec.x);
    }
    player.action_swing = (player.action_swing + dt * 8.0).min(1.0);

    // If target tile changed, reset mining progress to 0% for the new tile
    if map.mining_target != Some((gx, gy)) {
        map.mining_target = Some((gx, gy));
        map.mining_progress = 0.0;
    }

    let Some(cell_ref) = map.cells.get(&(gx, gy)) else {
        return;
    };
    if map.demolition_mode && server_target.is_none() && cell_ref.structure.is_none() {
        map.mining_progress = 0.0;
        map.mining_target = None;
        stats.status_log = crate::locale::text(
            "Wybierz budynek do rozbiórki. Kliknij Rozbiórka ponownie, by wrócić do kopania.",
        )
        .into();
        return;
    }
    if cell_ref.resource_yield == 0 && cell_ref.structure.is_none() && cell_ref.feature.is_none() {
        map.mining_progress = 0.0;
        map.mining_target = None;
        stats.status_log = crate::locale::text("Złoże wyczerpane. Poszukaj kolejnego.").into();
        return;
    }
    let cell_elev = cell_ref.elevation;
    let has_mining_dust = crate::mineral::deposit_index(cell_ref.kind) == 0
        && cell_ref.feature != Some(WorldFeatureKind::GlowingCrystalCluster);

    // Compute how fast this block/structure mines based on the player's ToolTier!
    let tool_speed = stats.highest_tool.mining_speed_multiplier()
        * if map.campaign_route {
            0.63
        } else if config.scenario == MissionScenario::OpenSandbox {
            3.5
        } else if config.difficulty == crate::types::DifficultyLevel::Cadet {
            1.35
        } else {
            1.0
        };

    let (base_mine_sec, target_name) = if let Some(st) = cell_ref.structure {
        if cell_ref.is_world_generated_base {
            (
                1.6,
                moon_base_localize::localized_format!(
                    "Rozbiórka odkrytej bazy: {} (Wskazówka: naciśnij [E] aby w niej zamieszkać!)",
                    st.short_name()
                ),
            )
        } else {
            (
                0.95,
                moon_base_localize::localized_format!(
                    "Rozbiórka budynku: {} (100% zwrotu)",
                    st.short_name()
                ),
            )
        }
    } else if let Some(feat) = cell_ref.feature {
        (0.85, format!("Wydobywanie / Otwieranie: {}", feat.name()))
    } else {
        (
            cell_ref.kind.base_mine_time_sec(),
            moon_base_localize::localized_format!("Kopanie: {}", cell_ref.kind.name()),
        )
    };

    map.mining_label = target_name;
    map.mining_progress += (dt * tool_speed) / base_mine_sec;

    // Continuous laser beam + mining dust sparks while holding LPM!
    let p_world = iso_to_world(player.iso_pos.x, player.iso_pos.y, player.iso_pos.z);
    let t_world = iso_to_world(gx as f32, gy as f32, cell_elev);
    events.last_laser_beam = Some((p_world + Vec2::new(0.0, 24.0), t_world, 0.08));

    if has_mining_dust && rand::thread_rng().gen_bool(0.32) {
        let elev = cell_elev;
        let a = rand::thread_rng().gen_range(0.0..TAU);
        commands.spawn((
            SurfaceEntity,
            DustParticle {
                iso_pos: Vec3::new(gx as f32, gy as f32, elev + 0.15),
                iso_vel: Vec3::new(a.cos() * 1.1, a.sin() * 1.1, 1.3),
                life: 0.0,
                max_life: 0.35,
                size: 3.6,
                color: Color::rgba(0.35, 0.96, 1.0, 0.85),
            },
        ));
    }

    // Wait until progressive mining reaches 100%!
    if map.mining_progress < 1.0 {
        return;
    }

    if lan.is_active && !lan.is_host {
        lan.send(crate::net_lan::Message::Action(
            crate::net_world::Action::Command(SurfaceUiCmd::ServerMine(gx, gy)),
        ));
        map.mining_progress = 0.0;
        map.mining_target = None;
        return;
    }
    // Mining reached 100%! Reset progress bar and grant rewards!
    map.mining_progress = 0.0;
    map.mining_target = None;
    stats.blocks_mined += 1;

    // Case 1: Deconstructing a Building on `(gx, gy)` with 100% resource refund!
    if has_building {
        let mut removed_kind = None;
        let mut refund = None;
        if let Some(cell) = map.cells.get_mut(&(gx, gy)) {
            refund = cell.paid_cost.take();
            if let Some(st) = cell.structure.take() {
                cell.structure_hp = 0.0;
                cell.is_world_generated_base = false;
                removed_kind = Some(st);
            }
        }
        if let Some(st) = removed_kind {
            let (r, ti, si, he) = refund.unwrap_or_else(|| st.build_cost());
            crate::mineral::spawn_loot(
                &mut commands,
                &assets,
                Vec3::new(gx as f32, gy as f32, cell_elev),
                crate::mineral::MiningLoot {
                    regolith: r,
                    titanium: ti,
                    silicon: si,
                    helium3: he,
                    ..default()
                },
            );
            refresh_structure_cluster_around(
                &mut commands,
                &assets,
                &map,
                gx,
                gy,
                &struct_q,
                &conn_q,
            );
            audio.play(SfxKind::LaserMine, sfx_vol);
            stats.status_log = moon_base_localize::localized_format!(
                "Rozebrano {} na ({}, {}). Zbierz materiały z ziemi (+{} regolit, +{} tytan, +{} krzem).",
                st.short_name(),
                gx,
                gy,
                r,
                ti,
                si
            );
        }
        return;
    }

    // Case 2: Opening / Mining a World Feature (Supply Pod, LRV Wreck, Crystal Cluster, etc.)
    let mut repair_rover_pos = None;
    let mut restore_kopernik_hub = false;

    if map.cells[&(gx, gy)].feature == Some(WorldFeatureKind::ApolloLrvWreck)
        && (stats.silicon < crate::types::LUR_REPAIR_SILICON
            || stats.regolith < crate::types::LUR_REPAIR_REGOLITH)
    {
        stats.status_log = crate::locale::text(crate::types::LUR_REPAIR_HINT).into();
        return;
    }

    let is_moon = map.body == WorldBody::Moon;
    if let Some(cell) = map.cells.get_mut(&(gx, gy)) {
        if let Some(feat) = cell.feature.take() {
            let crystal_remains =
                feat == WorldFeatureKind::GlowingCrystalCluster && cell.resource_yield > 10;
            if crystal_remains {
                cell.feature = Some(feat);
            } else if let Some(f_ent) = cell.feature_entity.take() {
                commands.entity(f_ent).despawn_recursive();
            }
            audio.play(SfxKind::CraftUpgrade, sfx_vol);
            match feat {
                WorldFeatureKind::SupplyDropPod => {
                    if (gx, gy) == (crate::campaign::SUPPLIES.x, crate::campaign::SUPPLIES.y)
                        && is_moon
                    {
                        if let Some(campaign) = &mut stats.campaign {
                            campaign.opened_supplies = true;
                        }
                    }
                    stats.rations += 3;
                    stats.drinking_water += 3;
                    stats.medkits += 1;
                    stats.titanium += 8;
                    stats.water_ice += 8;
                    stats.oxygen = (stats.oxygen + 35.0).min(stats.max_oxygen);
                    stats.suit_power = (stats.suit_power + 35.0).min(stats.max_suit_power);
                    stats.status_log = crate::locale::text(
                        "Kapsuła otwarta. Zebrano 3 posiłki, 3 porcje wody, apteczkę i surowce.",
                    )
                    .into();
                }
                WorldFeatureKind::ApolloLrvWreck => {
                    stats.silicon -= crate::types::LUR_REPAIR_SILICON;
                    stats.regolith -= crate::types::LUR_REPAIR_REGOLITH;
                    stats.generated_science_gb += 30.0;
                    repair_rover_pos = Some(Vec3::new(gx as f32, gy as f32, cell.elevation));
                    stats.status_log = crate::locale::text(
                        "Naprawiono LUR-1. Wsiądź klawiszem [E] albo wybierz Wejdź na telefonie.",
                    )
                    .to_string();
                }
                WorldFeatureKind::TwardowskiRooster => {
                    stats.rations += 4;
                    stats.helium3 += 5;
                    stats.medkits += 2;
                    stats.generated_science_gb += 45.0;
                    stats.status_log =
                        crate::locale::text("SEKRET: Odkryto Koguta i Lusterko Pana Twardowskiego! +45 GB Odkryć, +5 Helu-3, +4 Pierogi!")
                            .to_string();
                }
                WorldFeatureKind::CrashedLunaProbe => {
                    stats.silicon += 12;
                    stats.rare_earth += 3;
                    stats.generated_science_gb += 32.0;
                    stats.status_log =
                        crate::locale::text("Odzyskano czarną skrzynkę rozbitej sondy! +12 Krzemu, +3 Rzadkie Minerały!")
                            .to_string();
                }
                WorldFeatureKind::GlowingCrystalCluster => {
                    cell.resource_yield = cell.resource_yield.saturating_sub(10);
                    crate::mineral::spawn_loot(
                        &mut commands,
                        &assets,
                        Vec3::new(gx as f32, gy as f32, cell.elevation + 0.25),
                        crate::mineral::MiningLoot {
                            silicon: 3,
                            helium3: 1,
                            rare_earth: 1,
                            generated_science_gb: 4.0,
                            ..default()
                        },
                    );
                    stats.status_log =
                        crate::locale::text("Odłupano fragmenty kryształu — podejdź, aby zebrać.")
                            .to_string();
                }
                WorldFeatureKind::KopernikRuinedHub => {
                    stats.regolith += 20;
                    stats.titanium += 14;
                    stats.silicon += 10;
                    stats.generated_science_gb += 40.0;
                    restore_kopernik_hub = true;
                    stats.settled_world_structure = true;
                    stats.status_log =
                        crate::locale::text("Odnowiono Polską Stację 'Kopernik-0' w gotowy do zamieszkania Habitat! Wejdź klawiszem [E]!")
                            .to_string();
                }
                WorldFeatureKind::LemMonolith => {
                    stats.helium3 += 6;
                    stats.rare_earth += 5;
                    stats.generated_science_gb += 65.0;
                    stats.suit_power = stats.max_suit_power;
                    stats.status_log =
                        crate::locale::text("Zbadano Czarny Obelisk 'Niezwyciężony' Stanisława Lema! +65 GB Odkryć, +6 Helu-3 i pełna bateria!")
                            .to_string();
                }
                WorldFeatureKind::HeavyCargoWreck => {
                    stats.regolith += 28;
                    stats.titanium += 20;
                    stats.silicon += 16;
                    stats.helium3 += 6;
                    stats.medkits += 2;
                    stats.rations += 3;
                    stats.drinking_water += 2;
                    stats.status_log =
                        crate::locale::text("Otwarto skarbiec Ciężkiego Frachtowca! +28 Regolitu, +20 Tytanu, +16 Krzemu, +6 Helu-3!")
                            .to_string();
                }
                WorldFeatureKind::AutomatedIceRig => {
                    stats.water_ice += 32;
                    stats.titanium += 12;
                    stats.generated_science_gb += 26.0;
                    stats.status_log =
                        crate::locale::text("Zebrano zapasy z Automatycznej Kopalni Lodu! +32 Lodu Wodnego H2O, +12 Tytanu!")
                            .to_string();
                }
                WorldFeatureKind::LunarFlagPlaque => {
                    stats.generated_science_gb += 10.0;
                    stats.status_log =
                        crate::locale::text("Zebrano Biało-Czerwoną Flagę PL. Możesz wbić nową w dowolnym miejscu klawiszem [H]!")
                            .to_string();
                }
            }
            if repair_rover_pos.is_none() && !restore_kopernik_hub {
                return;
            }
        }
    }

    if let Some(r_pos) = repair_rover_pos {
        spawn_rover_entity(&mut commands, &assets, RoverKind::LightBuggy, r_pos);
        return;
    }
    if restore_kopernik_hub {
        if let Some(cell) = map.cells.get_mut(&(gx, gy)) {
            cell.structure = Some(StructureKind::HabitatModule);
            cell.structure_hp = 100.0;
        }
        refresh_structure_cluster_around(&mut commands, &assets, &map, gx, gy, &struct_q, &conn_q);
        return;
    }

    // Case 3: Standard Terrain Block Mining (Completed 100%!)
    let Some(cell) = map.cells.get_mut(&(gx, gy)) else {
        return;
    };

    audio.play(SfxKind::LaserMine, sfx_vol);

    let bonus = if config.astronaut_trait == AstronautTrait::AstroGeologist {
        2
    } else {
        1
    };
    let scanner_mult = if stats.highest_tool == ToolTier::QuantumScanner {
        1.5
    } else {
        1.0
    };

    let mut loot = crate::mineral::MiningLoot::default();
    match cell.kind {
        TileKind::MariaBasalt
        | TileKind::HighlandRegolith
        | TileKind::CraterRim
        | TileKind::CraterFloor => {
            loot.regolith += 6 * bonus;
            loot.silicon += 2 * bonus;
            if stats.highest_tool as u8 >= ToolTier::ThermalIceExtractor as u8 {
                loot.water_ice += bonus;
            }
            loot.generated_science_gb += 3.5 * scanner_mult;
            stats.status_log = moon_base_localize::localized_format!(
                "Wykopano blok: +{} Regolitu i +{} Krzemu!",
                6 * bonus,
                2 * bonus
            );
        }
        TileKind::SubsurfaceIceVein => {
            let ice_gain = if stats.highest_tool as u8 >= ToolTier::ThermalIceExtractor as u8 {
                8 * bonus
            } else {
                4 * bonus
            };
            loot.water_ice += ice_gain;
            loot.regolith += 2 * bonus;
            loot.raw_samples += 1;
            loot.generated_science_gb += 10.0 * scanner_mult;
            stats.status_log = moon_base_localize::localized_format!(
                "Wykopano Żyłę Lodu: +{} Lodu Wodnego H2O i próbka badawcza!",
                ice_gain
            );
        }
        TileKind::TitaniumOutcrop => {
            let ti_gain = if stats.highest_tool as u8 >= ToolTier::PlasmaCutter as u8 {
                7 * bonus
            } else {
                3 * bonus
            };
            loot.titanium += ti_gain;
            loot.silicon += 2 * bonus;
            loot.generated_science_gb += 8.5 * scanner_mult;
            stats.status_log = moon_base_localize::localized_format!(
                "Wykopano Rudę Tytanu: +{} Tytanu i +{} Krzemu!",
                ti_gain,
                2 * bonus
            );
        }
        TileKind::Helium3Deposit => {
            let he_gain = if stats.highest_tool as u8 >= ToolTier::LaserCoreDrill as u8 {
                5 * bonus
            } else {
                2 * bonus
            };
            loot.helium3 += he_gain;
            loot.silicon += 2 * bonus;
            loot.raw_samples += 1;
            loot.generated_science_gb += 14.0 * scanner_mult;
            stats.status_log = moon_base_localize::localized_format!(
                "Wykopano Złoże Helu-3: +{} Helu-3 i +{} Krzemu!",
                he_gain,
                2 * bonus
            );
        }
        TileKind::RareMeteoriteDeposit => {
            loot.rare_earth += 3 * bonus;
            loot.titanium += 7 * bonus;
            loot.helium3 += 2 * bonus;
            loot.raw_samples += 2;
            loot.generated_science_gb += 22.0 * scanner_mult;
            stats.status_log = moon_base_localize::localized_format!(
                "Wykopano Rzadki Meteoryt: +{} Tytanu, +{} Helu-3!",
                7 * bonus,
                2 * bonus
            );
        }
    }

    crate::mineral::spawn_loot(
        &mut commands,
        &assets,
        Vec3::new(gx as f32, gy as f32, cell.elevation + 0.25),
        loot,
    );
    stats.status_log =
        crate::locale::text("Odłamki spadły na ziemię. Podejdź i je zbierz.").to_string();

    cell.resource_yield = cell.resource_yield.saturating_sub(5);
}

#[allow(clippy::too_many_arguments)]
pub fn update_projectiles_and_particles(
    mut commands: Commands,
    time: Res<Time>,
    lan: Option<Res<LanMultiplayer>>,
    assets: Res<ProceduralAssets>,
    mut gizmos: Gizmos,
    mut events: ResMut<LunarEventManager>,
    mut map: ResMut<LunarMap>,
    mut stats: ResMut<SurvivalStats>,
    player_q: Query<&AstronautPlayer>,
    mut proj_q: Query<(Entity, &mut ThrownObject)>,
    mut dust_q: Query<(Entity, &mut DustParticle)>,
    mut tile_q: Query<(&IsoTileMarker, &mut Handle<Image>)>,
    struct_q: Query<(Entity, &StructureSpriteMarker)>,
    conn_q: Query<(Entity, &StructureConnectorMarker)>,
) {
    let dt = time.delta_seconds().min(0.05);

    map.flare_lights.retain(|light| {
        light.until
            > time.elapsed_seconds()
                + lan.as_ref().map_or(0.0, |lan| {
                    if lan.is_active && !lan.is_host {
                        lan.server_offset
                    } else {
                        0.0
                    }
                })
    });
    if let Some((start, end, ref mut ttl)) = events.last_laser_beam {
        *ttl -= dt;
        gizmos.line_2d(start, end, Color::rgb(0.25, 0.98, 1.0));
        gizmos.circle_2d(end, 8.0, Color::rgba(0.4, 1.0, 0.9, 0.75));
        if *ttl <= 0.0 {
            events.last_laser_beam = None;
        }
    }

    let (player_pos, inside_bldg) = player_q
        .get_single()
        .map(|p| (p.iso_pos.truncate(), p.inside_building))
        .unwrap_or((Vec2::ZERO, false));

    for (ent, mut obj) in &mut proj_q {
        obj.ttl -= dt;
        let grav = if obj.is_meteor {
            map.body.gravity() * 1.8
        } else {
            map.body.gravity()
        };
        obj.iso_vel.z -= grav * dt;
        let delta_pos = obj.iso_vel * dt;
        obj.iso_pos += delta_pos;

        let ground_z = map.get_elevation(obj.iso_pos.x, obj.iso_pos.y);
        let w_pos = iso_to_world(obj.iso_pos.x, obj.iso_pos.y, obj.iso_pos.z);
        let shadow_pos = iso_to_world(obj.iso_pos.x, obj.iso_pos.y, ground_z);

        gizmos.circle_2d(shadow_pos, 5.5, Color::rgba(0.0, 0.0, 0.0, 0.50));
        let col = if obj.is_meteor {
            Color::rgb(1.0, 0.48, 0.15)
        } else {
            Color::rgb(0.25, 0.98, 0.72)
        };
        gizmos.circle_2d(w_pos, if obj.is_meteor { 8.5 } else { 5.5 }, col);
        if obj.is_flare {
            gizmos.circle_2d(
                shadow_pos,
                14.0 + (obj.ttl * 6.0).sin().abs() * 8.0,
                Color::rgba(0.20, 0.95, 0.80, 0.35),
            );
        }
        gizmos.line_2d(
            w_pos,
            w_pos - Vec2::new(obj.iso_vel.x - obj.iso_vel.y, obj.iso_vel.z) * 5.0,
            col.with_a(0.6),
        );

        if obj.iso_pos.z <= ground_z {
            let gx = obj.iso_pos.x.round() as i32;
            let gy = obj.iso_pos.y.round() as i32;
            if obj.is_flare {
                if !lan
                    .as_ref()
                    .is_some_and(|lan| lan.is_active && !lan.is_host)
                {
                    map.flare_lights.push(crate::world::FlareLight {
                        x: gx,
                        y: gy,
                        until: time.elapsed_seconds() + 11.0,
                    });
                }
                commands.entity(ent).despawn_recursive();
                continue;
            }

            if obj.is_meteor {
                if !lan
                    .as_ref()
                    .is_some_and(|lan| lan.is_active && !lan.is_host)
                {
                    let shielded = map.has_adjacent_shield_wall(gx, gy);
                    let mut struct_destroyed = false;
                    if let Some(cell) = map.cells.get_mut(&(gx, gy)) {
                        if cell.structure.is_some() {
                            if !shielded && !cell.is_world_generated_base {
                                cell.structure_hp = (cell.structure_hp - 35.0).max(0.0);
                                if cell.structure_hp <= 0.0 {
                                    cell.structure = None;
                                    struct_destroyed = true;
                                }
                            }
                        } else if gx.abs() > 2 || gy.abs() > 2 {
                            cell.kind = TileKind::RareMeteoriteDeposit;
                            cell.resource_yield = 55;
                            for (marker, mut tex) in &mut tile_q {
                                if marker.gx == gx && marker.gy == gy {
                                    *tex = assets.terrain_handle(cell.kind, cell.elevation);
                                }
                            }
                        }
                    }
                    if struct_destroyed {
                        refresh_structure_cluster_around(
                            &mut commands,
                            &assets,
                            &map,
                            gx,
                            gy,
                            &struct_q,
                            &conn_q,
                        );
                    }
                }
                if player_pos.distance(Vec2::new(gx as f32, gy as f32)) < 1.6
                    && !inside_bldg
                    && !map.is_inside_pressurized(player_pos.x, player_pos.y)
                {
                    stats.health = (stats.health - 24.0).max(0.0);
                    stats.suit_integrity = (stats.suit_integrity - 20.0).max(0.0);
                    stats.status_log = crate::locale::text(
                        "UWAGA! Odłamek meteorytu uderzył blisko Ciebie! (-24% Zdrowia)",
                    )
                    .to_string();
                }
                commands.entity(ent).despawn_recursive();
            } else {
                obj.iso_pos.z = ground_z;
                if obj.iso_vel.z.abs() > 0.8 {
                    obj.iso_vel.z = -obj.iso_vel.z * 0.48;
                    obj.iso_vel.x *= 0.65;
                    obj.iso_vel.y *= 0.65;
                    for dy in -4..=4 {
                        for dx in -4..=4 {
                            if let Some(c) = map.cells.get_mut(&(gx + dx, gy + dy)) {
                                c.scanned = true;
                            }
                        }
                    }
                    stats.generated_science_gb += 5.0;
                } else {
                    obj.iso_vel = Vec3::ZERO;
                }
            }
        }

        if obj.ttl <= 0.0 {
            commands.entity(ent).despawn_recursive();
        }
    }

    for (ent, mut p) in &mut dust_q {
        p.life += dt;
        p.iso_vel.z -= map.body.gravity() * 0.65 * dt;
        let delta_p = p.iso_vel * dt;
        p.iso_pos += delta_p;
        let w = iso_to_world(p.iso_pos.x, p.iso_pos.y, p.iso_pos.z);
        let alpha = (1.0 - p.life / p.max_life).clamp(0.0, 1.0) * p.color.a();
        gizmos.circle_2d(w, p.size * (1.0 + p.life * 0.5), p.color.with_a(alpha));
        if p.life >= p.max_life {
            commands.entity(ent).despawn_recursive();
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn update_survival_base_and_events(
    mut commands: Commands,
    time: Res<Time>,
    lan: Option<Res<LanMultiplayer>>,
    keys: Res<ButtonInput<KeyCode>>,
    config: Res<MissionConfig>,
    settings: Res<GameSettings>,
    audio: Res<GameAudio>,
    solar: Res<SolarSystemState>,
    assets: Res<ProceduralAssets>,
    mut map: ResMut<LunarMap>,
    mut stats: ResMut<SurvivalStats>,
    mut events: ResMut<LunarEventManager>,
    player_q: Query<&AstronautPlayer>,
    rover_q: Query<&RoverVehicle>,
    mut tile_q: Query<(&IsoTileMarker, &mut Handle<Image>)>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    let dt = time.delta_seconds().min(0.05);
    let sfx_vol = settings.sfx_volume * settings.master_volume;
    stats.elapsed_sols += dt * 0.025;

    // NOTE: KeyCode::Tab is intentionally NOT bound to any popup/screen transition
    // so Alt+Tabbing out of the game NEVER triggers any unwanted alert or window!

    let Ok(player) = player_q.get_single() else {
        return;
    };

    // F1 / F2 / F3 Polish astronaut radio emotes
    if keys.just_pressed(KeyCode::F1) {
        audio.play(SfxKind::UiConfirm, sfx_vol);
        stats.status_log = moon_base_localize::localized_format!(
            "[RADIO {}]: 'Tu polska baza księżycowa Kopernik! Pozdrowienia dla Ziemi!'",
            config.callsign
        );
    }
    if keys.just_pressed(KeyCode::F2) {
        audio.play(SfxKind::UiConfirm, sfx_vol);
        stats.status_log = moon_base_localize::localized_format!(
            "[RADIO {}]: 'Śladami Pana Twardowskiego, Mirosława Hermaszewskiego i Sławosza Uznańskiego!'",
            config.callsign
        );
    }
    if keys.just_pressed(KeyCode::F3) {
        audio.play(SfxKind::UiConfirm, sfx_vol);
        stats.status_log = moon_base_localize::localized_format!(
            "[RADIO {}]: 'Koła łazika projektu inż. Mieczysława Bekkera trzymają się regolitu znakomicie!'",
            config.callsign
        );
    }

    // Natural Dynamic World Events
    if !lan
        .as_ref()
        .is_some_and(|lan| lan.is_active && !lan.is_host)
    {
        events.timer_sec += dt;
        if events.timer_sec >= events.duration_sec {
            events.timer_sec = 0.0;
            let sun = solar.sun_illumination();
            events.active_event = if events.active_event != LunarEventKind::Nominal {
                LunarEventKind::Nominal
            } else if map.body == WorldBody::Mars {
                match rand::thread_rng().gen_range(0..4) {
                    0..=1 => LunarEventKind::DustStorm,
                    2 => LunarEventKind::SolarFlareStorm,
                    _ => LunarEventKind::Moonquake,
                }
            } else if map.body == WorldBody::Venus {
                if rand::thread_rng().gen_bool(0.6) {
                    LunarEventKind::SolarFlareStorm
                } else {
                    LunarEventKind::Moonquake
                }
            } else if sun < 0.18 {
                LunarEventKind::LunarNightDeepFreeze
            } else {
                match rand::thread_rng().gen_range(0..6) {
                    0 => LunarEventKind::MeteoriteShower,
                    1 => LunarEventKind::SolarFlareStorm,
                    2 => LunarEventKind::Moonquake,
                    3 => LunarEventKind::OrbitalSupplyDrop,
                    _ => LunarEventKind::Nominal,
                }
            };
            if events.active_event != LunarEventKind::Nominal {
                audio.play(SfxKind::AlertSiren, sfx_vol * 0.65);
            }
            if events.active_event == LunarEventKind::OrbitalSupplyDrop {
                let mut rng = rand::thread_rng();
                let dx = rng.gen_range(-6..=6);
                let dy = rng.gen_range(-6..=6);
                let sx = player.iso_pos.x.round() as i32 + if dx == 0 { 4 } else { dx };
                let sy = player.iso_pos.y.round() as i32 + if dy == 0 { -4 } else { dy };
                if let Some(cell) = map.cells.get_mut(&(sx, sy)) {
                    if cell.structure.is_none() && cell.feature.is_none() {
                        cell.feature = Some(WorldFeatureKind::SupplyDropPod);
                        let w_pos = iso_to_world(sx as f32, sy as f32, cell.elevation);
                        let fz = iso_depth(sx as f32, sy as f32, cell.elevation, 1.5);
                        let f_ent = commands
                            .spawn((
                                SurfaceEntity,
                                crate::world::WorldFeatureSpriteMarker { gx: sx, gy: sy },
                                SpriteBundle {
                                    texture: assets.feature_handle(WorldFeatureKind::SupplyDropPod),
                                    transform: Transform::from_xyz(w_pos.x, w_pos.y + 22.0, fz),
                                    ..default()
                                },
                            ))
                            .id();
                        cell.feature_entity = Some(f_ent);
                    }
                }
            }
            events.duration_sec = match events.active_event {
                LunarEventKind::Nominal
                    if config.difficulty == crate::types::DifficultyLevel::Cadet =>
                {
                    rand::thread_rng().gen_range(120.0..180.0)
                }
                LunarEventKind::Nominal => rand::thread_rng().gen_range(90.0..150.0),
                _ => rand::thread_rng().gen_range(18.0..24.0),
            };
        }

        events.camera_shake_intensity = 0.0;
        match events.active_event {
            LunarEventKind::MeteoriteShower => {
                events.meteor_spawn_timer += dt;
                if events.meteor_spawn_timer >= 1.45 {
                    events.meteor_spawn_timer = 0.0;
                    let mut rng = rand::thread_rng();
                    let tx = player.iso_pos.x + rng.gen_range(-14.0..14.0);
                    let ty = player.iso_pos.y + rng.gen_range(-14.0..14.0);
                    commands.spawn((
                        SurfaceEntity,
                        ThrownObject {
                            iso_pos: Vec3::new(tx - 3.5, ty - 3.5, 9.5),
                            iso_vel: Vec3::new(1.8, 1.8, -3.2),
                            ttl: 6.0,
                            is_flare: false,
                            is_meteor: true,
                        },
                    ));
                }
            }
            LunarEventKind::Moonquake => {
                events.camera_shake_intensity = 0.55;
                if rand::thread_rng().gen_bool((dt * 0.45) as f64) {
                    let mut rng = rand::thread_rng();
                    let fx = player.iso_pos.x.round() as i32 + rng.gen_range(-10..=10);
                    let fy = player.iso_pos.y.round() as i32 + rng.gen_range(-10..=10);
                    if fx.abs() > 2 || fy.abs() > 2 {
                        if let Some(cell) = map.cells.get_mut(&(fx, fy)) {
                            if cell.structure.is_none() {
                                cell.kind = TileKind::TitaniumOutcrop;
                                cell.resource_yield = 45;
                                for (m, mut t) in &mut tile_q {
                                    if m.gx == fx && m.gy == fy {
                                        *t = assets.terrain_handle(cell.kind, cell.elevation);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
    let solar_light = solar.sun_illumination();
    let sun_factor = map.body.sunlight(
        solar_light,
        events.active_event == LunarEventKind::DustStorm,
    );
    crate::base::update_networks(
        &mut map,
        sun_factor,
        if lan
            .as_ref()
            .is_some_and(|lan| lan.is_active && !lan.is_host)
        {
            0.0
        } else {
            dt
        },
    );
    stats.outside_temperature_c = map.body.temperature(
        solar_light,
        solar.surface_temperature_c(config.scenario == MissionScenario::ShackletonIceProspecting),
        map.get_elevation(player.iso_pos.x, player.iso_pos.y),
    );
    if events.active_event == LunarEventKind::SolarFlareStorm {
        stats.outside_temperature_c += 65.0;
    }
    let mut power_gen = config.total_power_supply_w() * (0.35 + 0.65 * sun_factor);
    let mut power_load = config.total_power_demand_w() * 0.55;
    let mut comms_dish_bonus = 0.0;

    for (&(gx, gy), cell) in &map.cells {
        if let Some(st) = cell.structure {
            if cell.structure_hp <= 0.0 {
                continue;
            }
            let neighbors = map.connected_neighbors_count(gx, gy, st);
            let synergy_mult = 1.0 + neighbors as f32 * 0.15;
            let p = st.power_delta_w(sun_factor);
            if p >= 0.0 {
                power_gen += p * synergy_mult;
            } else {
                power_load += -p;
            }
            if st == StructureKind::CommsDish
                && map
                    .networks
                    .get(&(gx, gy))
                    .is_some_and(|base| base.relay && base.powered)
            {
                comms_dish_bonus += 5.8 * synergy_mult;
            }
        }
    }

    stats.base_power_gen_w = power_gen;
    stats.base_power_load_w = power_load;
    let net_power = power_gen - power_load;
    stats.base_battery_wh =
        (stats.base_battery_wh + net_power * dt * 0.08).clamp(0.0, stats.max_base_battery_wh);
    let base_powered = stats.base_battery_wh > 5.0 || net_power >= 0.0;

    let relay_ok = solar.relay_link_active(config.orbit);
    let flare_penalty = if events.active_event == LunarEventKind::SolarFlareStorm {
        0.35
    } else {
        1.0
    };
    let eff_downlink = if relay_ok && base_powered {
        (config.effective_comms_capacity_mbps() + comms_dish_bonus) * flare_penalty
    } else {
        0.0
    };
    stats.comms_downlink_mbps = eff_downlink;
    if stats.transmitted_science_gb < stats.generated_science_gb {
        let tx = (eff_downlink * 0.24 * dt)
            .min(stats.generated_science_gb - stats.transmitted_science_gb);
        stats.transmitted_science_gb += tx;
    }

    // Logical Life-Support & Rover Protection!
    // - Inside a Building (`player.inside_building` or standing in pressurized room) OR inside Heavy Cruiser Rover:
    //   Full O2 regeneration, temperature 37.0°C, radiation drops!
    // - Inside Light Buggy Rover (`mounted_rover.is_some()`):
    //   O2 drops 4x slower (`0.25x` drain) and Radiation does NOT increase at all!
    let mounted_rover_kind = player
        .mounted_rover
        .and_then(|ent| rover_q.get(ent).ok())
        .map(|r| r.kind);
    let in_any_rover = mounted_rover_kind.is_some();
    let in_pressurized_rover = mounted_rover_kind
        .map(|k| k.is_pressurized())
        .unwrap_or(false);
    let position = player.iso_pos.truncate().round().as_ivec2();
    let local = map
        .networks
        .get(&(position.x, position.y))
        .copied()
        .unwrap_or_default();
    let in_cabin = map.piloting_station
        || (player.inside_building
            && player
                .iso_pos
                .truncate()
                .distance(map.station_position.truncate())
                <= 1.2);
    let sheltered = in_cabin || player.inside_building || in_pressurized_rover;
    stats.local_power_w = local.net_watts;
    stats.local_powered = local.powered;
    stats.local_oxygen = local.powered && local.oxygen;
    stats.local_charge = local.charge_percent;
    if player.inside_building
        && local.powered
        && !lan
            .as_ref()
            .is_some_and(|lan| lan.is_active && !lan.is_host)
    {
        if local.lab && stats.raw_samples > 0 {
            stats.lab_progress += dt;
            if stats.lab_progress >= 6.0 {
                stats.raw_samples -= 1;
                stats.generated_science_gb += 24.0;
                stats.lab_progress = 0.0;
                stats.status_log =
                    crate::locale::text("Laboratorium zbadało próbkę. +24 GB danych.").into();
            }
        }
    }
    for recipe in [
        crate::base::SupplyRecipe::Water,
        crate::base::SupplyRecipe::Meal,
    ] {
        if !lan
            .as_ref()
            .is_some_and(|lan| lan.is_active && !lan.is_host)
        {
            recipe.advance(&mut map, &mut stats, dt);
        }
    }

    let drain_mult = config.difficulty.survival_drain_multiplier()
        * if config.astronaut_trait == AstronautTrait::ClosedLoopBotanist {
            0.70
        } else {
            1.0
        };
    let moving = player.iso_vel.truncate().length_squared() > 0.36;
    let exertion = if sheltered || in_any_rover {
        1.0
    } else if map.mining_progress > 0.01 || !player.grounded {
        1.45
    } else if moving {
        1.2
    } else {
        1.0
    };
    let leak = if sheltered {
        1.0
    } else {
        1.0 + (100.0 - stats.suit_integrity) * 0.009
    };
    let oxygen_use = SUIT_OXYGEN_USE
        * drain_mult
        * exertion
        * leak
        * if map.campaign_route { 0.82 } else { 1.0 };
    let refilling =
        in_cabin || in_pressurized_rover || (sheltered && local.powered && local.oxygen);
    stats.oxygen_use_per_sec = if refilling {
        0.0
    } else if in_any_rover || sheltered {
        oxygen_use * 0.55
    } else {
        oxygen_use
    };
    if refilling {
        stats.oxygen = (stats.oxygen + 5.0 * dt).min(stats.max_oxygen);
    } else {
        stats.oxygen = (stats.oxygen - stats.oxygen_use_per_sec * dt).max(0.0);
    }

    if in_cabin || in_pressurized_rover || (sheltered && local.powered) {
        stats.suit_power = (stats.suit_power + 18.0 * dt).min(stats.max_suit_power);
        stats.body_temp_c += (37.0 - stats.body_temp_c) * (dt * 3.0).min(1.0);
        stats.radiation += 0.000003 * dt;
        if stats.hunger > 25.0 && stats.hydration > 25.0 && stats.radiation < 50.0 {
            stats.health = (stats.health + 4.5 * dt).min(100.0);
        }
    } else if in_any_rover {
        stats.suit_power = (stats.suit_power + 4.0 * dt).min(stats.max_suit_power);
        stats.body_temp_c += (37.0 - stats.body_temp_c) * (dt * 2.0).min(1.0);
        stats.radiation += 0.00001 * dt;
    } else {
        let rtg_trickle = if config.backpack_module == BackpackModule::MicroRtgCell {
            0.55
        } else {
            0.0
        };
        let env_temp = stats.outside_temperature_c;
        let thermal_stress = ((env_temp - 20.0).abs() / 160.0).clamp(0.2, 2.8);
        stats.suit_power = (stats.suit_power
            - (0.70 * thermal_stress - rtg_trickle) * drain_mult * dt)
            .clamp(0.0, stats.max_suit_power);

        if stats.suit_power > 5.0 {
            let target_t = if map.body == WorldBody::Venus {
                37.6
            } else {
                37.0 + (env_temp - 20.0) * 0.008
            };
            stats.body_temp_c += (target_t - stats.body_temp_c) * dt * 0.8;
        } else {
            stats.body_temp_c += ((env_temp * 0.08 + 30.0) - stats.body_temp_c) * dt * 0.4;
        }

        let rad_shield_mult = if config.backpack_module == BackpackModule::RadiationLeadShield {
            0.35
        } else {
            1.0
        };
        let base_rad_rate = if events.active_event == LunarEventKind::SolarFlareStorm {
            0.02
        } else {
            0.00001585
        };
        let suit_leak_mult = if stats.suit_integrity < 50.0 {
            1.8
        } else {
            1.0
        };
        stats.radiation = (stats.radiation
            + base_rad_rate
                * (if sheltered { 0.25 } else { 1.0 })
                * map.body.radiation_multiplier()
                * rad_shield_mult
                * suit_leak_mult
                * drain_mult
                * dt)
            .min(100.0);

        if events.active_event == LunarEventKind::SolarFlareStorm {
            stats.suit_integrity = (stats.suit_integrity - 1.2 * rad_shield_mult * dt).max(0.0);
        }
    }

    let heat = if sheltered {
        1.0
    } else {
        1.0 + ((stats.outside_temperature_c - 35.0).max(0.0) / 300.0).min(0.45)
    };
    let supply_rate = if map.campaign_route { 0.65 } else { 1.0 };
    stats.hunger = (stats.hunger - 0.19 * drain_mult * exertion * supply_rate * dt).max(0.0);
    stats.hydration =
        (stats.hydration - 0.25 * drain_mult * exertion * heat * supply_rate * dt).max(0.0);

    if events.active_event == LunarEventKind::Moonquake
        && player.grounded
        && !sheltered
        && !in_any_rover
    {
        stats.health = (stats.health - dt * 0.8).max(0.0);
    }
    if !sheltered && !in_any_rover && !(34.0..=39.5).contains(&stats.body_temp_c) {
        stats.health = (stats.health - dt * 0.8).max(0.0);
    }
    // Health drain & Death check (disabled in Sandbox mode)
    if config.scenario != MissionScenario::OpenSandbox {
        if stats.oxygen <= 0.1 {
            stats.health = (stats.health - 9.5 * dt).max(0.0);
            if stats.death_reason.is_empty() {
                stats.death_reason = crate::locale::text(
                    "BRAK TLENU: Wyczerpał się zapas tlenu (O2 = 0%) poza bazą!",
                )
                .to_string();
            }
        }
        if stats.hunger <= 0.1 {
            stats.health = (stats.health - 4.8 * dt).max(0.0);
            if stats.death_reason.is_empty() {
                stats.death_reason =
                    crate::locale::text("GŁÓD: Wyczerpanie organizmu z braku racji żywnościowych!")
                        .to_string();
            }
        }
        if stats.hydration <= 0.1 {
            stats.health = (stats.health - 4.0 * dt).max(0.0);
            if stats.death_reason.is_empty() {
                stats.death_reason =
                    crate::locale::text("Odwodnienie. Zabrakło wody do picia.").into();
            }
        }
        if stats.radiation >= 80.0 {
            stats.health = (stats.health - 6.5 * dt).max(0.0);
            stats.death_reason = crate::locale::text(
                "PROMIENIOWANIE: Przekroczono bezpieczną dawkę promieniowania kosmicznego!",
            )
            .to_string();
        }
        if stats.suit_integrity <= 0.1 {
            stats.health = (stats.health - 8.5 * dt).max(0.0);
            if stats.death_reason.is_empty() {
                stats.death_reason = crate::locale::text(
                    "USZKODZENIE SKAFANDRA: Rozszczelnienie skafandra w próżni!",
                )
                .to_string();
            }
        }

        if stats.health <= 0.0 || stats.radiation >= 100.0 {
            if stats.death_reason.is_empty() {
                stats.death_reason =
                    crate::locale::text("UTRATA PRZYTOMNOŚCI NA POWIERZCHNI KSIĘŻYCA.").to_string();
            }
            audio.play(SfxKind::AlertSiren, sfx_vol);
            next_state.set(AppState::GameOver);
        }
    }
}
