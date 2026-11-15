use crate::isometric::{iso_depth, iso_to_world, ProceduralAssets};
use crate::types::MissionConfig;
use crate::world::{
    stream_infinite_chunks_around, AstronautPlayer, LunarMap, SurfaceUiCmd, SurvivalStats,
    TouchAndUiInput,
};
use bevy::prelude::*;

#[derive(Component)]
pub struct FlyingStation;

#[derive(Component)]
pub struct StationThruster(pub f32);

pub fn landing_clear(map: &LunarMap, position: Vec2) -> bool {
    let floor = map.get_elevation(position.x, position.y);
    (-1..=1).all(|dx| {
        (-1..=1).all(|dy| {
            let p = position + Vec2::new(dx as f32, dy as f32);
            !map.blocks_movement(p.x, p.y) && (map.get_elevation(p.x, p.y) - floor).abs() < 0.75
        })
    })
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn update_station_flight(
    mut commands: Commands,
    mut next: Option<ResMut<NextState<crate::types::AppState>>>,
    lan: Option<Res<crate::net_lan::LanMultiplayer>>,
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    config: Res<MissionConfig>,
    assets: Res<ProceduralAssets>,
    mut map: ResMut<LunarMap>,
    mut stats: ResMut<SurvivalStats>,
    mut touch: ResMut<TouchAndUiInput>,
    mut player_q: Query<
        (&mut AstronautPlayer, &mut Transform, &mut Visibility),
        (Without<FlyingStation>, Without<Camera2d>),
    >,
    mut ship_q: Query<
        &mut Transform,
        (
            With<FlyingStation>,
            Without<AstronautPlayer>,
            Without<Camera2d>,
        ),
    >,
    mut camera_q: Query<
        &mut Transform,
        (
            With<Camera2d>,
            Without<AstronautPlayer>,
            Without<FlyingStation>,
        ),
    >,
) {
    let Ok((mut player, mut player_tf, mut visibility)) = player_q.get_single_mut() else {
        return;
    };
    let dt = time.delta_seconds().min(0.05);
    let position = map.station_position;
    let ground = map.get_elevation(position.x, position.y);
    let mut clicked = false;
    touch.cmds.retain(|cmd| {
        if matches!(cmd, SurfaceUiCmd::PilotStation) {
            clicked = true;
            false
        } else {
            true
        }
    });
    let nearby = player.iso_pos.truncate().distance(position.truncate()) <= 3.0;
    if lan.as_ref().is_some_and(|l| l.is_active && !l.is_host)
        && nearby
        && (clicked || keys.just_pressed(KeyCode::KeyG))
        && !map.any_modal_open()
    {
        if let Some(next) = next.as_deref_mut() {
            next.set(crate::types::AppState::PlanetAtlas);
        }
        return;
    }
    if !lan.as_ref().is_some_and(|l| l.is_active && !l.is_host)
        && (clicked || (keys.just_pressed(KeyCode::KeyG) && (nearby || map.piloting_station)))
        && !map.any_modal_open()
    {
        if map.piloting_station {
            if position.z > ground + 0.1 {
                stats.status_log = crate::locale::text(
                    "Najpierw wyląduj: przytrzymaj CTRL nad płaskim, pustym terenem.",
                )
                .into();
            } else if let Some(exit) = [
                Vec2::new(-2.0, 0.0),
                Vec2::new(2.0, 0.0),
                Vec2::new(0.0, -2.0),
                Vec2::new(0.0, 2.0),
            ]
            .into_iter()
            .map(|p| p + position.truncate())
            .find(|p| !map.blocks_movement(p.x, p.y))
            {
                map.piloting_station = false;
                player.inside_building = false;
                player.iso_pos = exit.extend(map.get_elevation(exit.x, exit.y));
                player.iso_vel = Vec3::ZERO;
                player.grounded = true;
                *visibility = Visibility::Inherited;
                stats.status_log = crate::locale::text(
                    "Opuszczono stację. G przy stacji ponownie otwiera kokpit.",
                )
                .into();
            } else {
                stats.status_log =
                    crate::locale::text("Wyjście zablokowane — przestaw stację na otwarty teren.")
                        .into();
            }
        } else if nearby
            && (player.iso_pos.z - position.z).abs() < 1.5
            && player.mounted_rover.is_none()
            && player.airlock_anim_timer <= 0.0
        {
            map.piloting_station = true;
            map.build_mode = false;
            map.mining_progress = 0.0;
            map.mining_target = None;
            player.inside_building = true;
            stats.entered_building_once = true;
            player.iso_vel = Vec3::ZERO;
            *visibility = Visibility::Hidden;
            stats.status_log =
                crate::locale::text("Jesteś w kokpicie. Przytrzymaj spację, aby wystartować.")
                    .into();
        } else {
            stats.status_log =
                crate::locale::text("Podejdź do lądownika/stacji i naciśnij G, aby pilotować.")
                    .into();
        }
    }
    if !map.piloting_station {
        return;
    }
    let mut position = map.station_position;
    if !map.any_modal_open() {
        let mut input = Vec2::ZERO;
        if keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp) {
            input += Vec2::new(-1.0, -1.0);
        }
        if keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown) {
            input += Vec2::new(1.0, 1.0);
        }
        if keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft) {
            input += Vec2::new(-1.0, 1.0);
        }
        if keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight) {
            input += Vec2::new(1.0, -1.0);
        }
        if keys.pressed(KeyCode::Space) {
            position.z = (position.z + dt * 3.0).min(40.0);
        }
        let target = position.truncate() + input.normalize_or_zero() * dt * 5.0;
        if position.z > map.get_elevation(target.x, target.y) + 2.0 {
            position.x = target.x;
            position.y = target.y;
        }
        if keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight) {
            let floor = map.get_elevation(position.x, position.y);
            let safe_floor = if landing_clear(&map, position.truncate()) {
                floor
            } else {
                floor + 2.5
            };
            if position.z > safe_floor {
                position.z = (position.z - dt * 2.5).max(safe_floor);
            }
        }
    }
    map.station_position = position;
    stream_infinite_chunks_around(
        &mut commands,
        &assets,
        &mut map,
        position.x.round() as i32,
        position.y.round() as i32,
        config.scenario,
    );
    player.iso_pos = position;
    player.grounded = position.z <= map.get_elevation(position.x, position.y) + 0.1;
    if player.grounded {
        map.station_parked = position;
    }
    let screen = iso_to_world(position.x, position.y, position.z);
    let height = position.z - map.get_elevation(position.x, position.y);
    player_tf.translation.x = screen.x;
    player_tf.translation.y = screen.y;
    if let Ok(mut ship) = ship_q.get_single_mut() {
        let phase = time.elapsed_seconds();
        let sideways = ((position.x - position.y)
            - (map.station_previous.x - map.station_previous.y))
            / dt.max(0.001);
        let target_angle = (sideways * -0.022).clamp(-0.13, 0.13);
        ship.rotation = ship.rotation.slerp(
            Quat::from_rotation_z(if height > 0.10 {
                target_angle + (phase * 3.0).sin() * 0.018
            } else {
                0.0
            }),
            (dt * 7.0).min(1.0),
        );
        ship.translation = Vec3::new(
            screen.x,
            screen.y + 36.0 + (phase * 4.1).sin() * height.min(1.0) * 1.8,
            iso_depth(position.x, position.y, position.z, 2.2),
        );
    }
    if let Ok(mut camera) = camera_q.get_single_mut() {
        camera.translation.x += (screen.x - camera.translation.x) * (dt * 5.0).min(1.0);
        camera.translation.y += (screen.y - camera.translation.y) * (dt * 5.0).min(1.0);
    }
    if let Some(marker) = map.waypoints.first_mut() {
        marker.gx = position.x.round() as i32;
        marker.gy = position.y.round() as i32;
        marker.name = crate::locale::text("Stacja / lądownik").into();
    }
    map.station_previous = position;
    if height > 0.2 {
        if !stats.flew_station {
            stats.status_log =
                crate::locale::text("Stacja jest w powietrzu. Leć klawiszami WASD.").into();
        }
        stats.flew_station = true;
    }
}

pub fn animate_thrusters(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    map: Res<LunarMap>,
    mut thrusters: Query<(
        &StationThruster,
        &mut Transform,
        &mut Sprite,
        &mut Visibility,
    )>,
    mut gizmos: Gizmos,
) {
    let p = map.station_position;
    let ground = map.get_elevation(p.x, p.y);
    let height = (p.z - ground).max(0.0);
    let t = time.elapsed_seconds();
    for (nozzle, mut tf, mut sprite, mut visibility) in &mut thrusters {
        let lit = height > 0.09;
        *visibility = if lit {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if !lit {
            continue;
        }
        let strength = if map.piloting_station && keys.pressed(KeyCode::Space) {
            1.25
        } else {
            0.78
        };
        let pulse = 0.93 + (t * 27.0 + nozzle.0).sin() * 0.13;
        tf.scale = Vec3::new(pulse, strength * pulse, 1.0);
        tf.translation.y = -27.0 - strength * pulse * 16.0;
        sprite
            .color
            .set_a((0.94 + (t * 22.0).sin() * 0.06).clamp(0.0, 1.0));
    }
    if (0.09..2.4).contains(&height) {
        let center = iso_to_world(p.x, p.y, ground);
        for i in 0..3 {
            let ring = (t * 1.1 + i as f32 * 0.33).fract();
            gizmos.ellipse_2d(
                center,
                0.0,
                Vec2::new(14.0 + ring * 64.0, 5.0 + ring * 26.0),
                Color::rgba(0.86, 0.75, 0.63, (1.0 - ring) * (1.0 - height / 2.4) * 0.28),
            );
        }
    }
}
