use crate::isometric::{iso_depth, iso_to_world};
use crate::net_lan::LanMultiplayer;
use crate::world::{AstronautPlayer, LunarMap, RoverVehicle};
use bevy::prelude::*;

#[derive(Component)]
pub struct CrewVisor;
#[derive(Component)]
pub struct CrewAccessory;

pub fn remote_appearance(
    mut commands: Commands,
    time: Res<Time>,
    lan: Res<LanMultiplayer>,
    assets: Res<crate::isometric::ProceduralAssets>,
    crew: Query<(&crate::world::CoOpCrewMember, &Children, &Visibility)>,
    mut visors: Query<(&mut Handle<Image>, &mut Sprite), (With<CrewVisor>, Without<CrewAccessory>)>,
    mut accessories: Query<
        (&mut Handle<Image>, &mut Sprite),
        (With<CrewAccessory>, Without<CrewVisor>),
    >,
    mut elapsed: Local<f32>,
) {
    *elapsed += time.delta_seconds().min(0.05);
    let emit = *elapsed >= 0.04;
    if emit {
        *elapsed = 0.0;
    }
    for (member, children, visibility) in &crew {
        let Some(peer) = lan.peers.get(member.peer_index) else {
            continue;
        };
        let angle = if peer.vel_xy.length() > 0.2 {
            peer.vel_xy.y.atan2(peer.vel_xy.x)
        } else if peer.facing_right {
            0.0
        } else {
            std::f32::consts::PI
        };
        let direction = if peer.vel_xy.length() > 0.2 {
            crate::isometric::ProceduralAssets::angle_to_dir_index(angle)
        } else if peer.facing_right {
            1
        } else {
            7
        };
        for &child in children {
            if let Ok((mut texture, mut sprite)) = visors.get_mut(child) {
                *texture = assets.visor_dir_tex[direction].clone();
                sprite.color = peer.visor.to_color();
            }
            if let Ok((mut texture, mut sprite)) = accessories.get_mut(child) {
                *texture = assets.accessory_handle(peer.accessory);
                sprite.flip_x = angle.cos() < -0.25;
            }
        }
        if emit
            && *visibility != Visibility::Hidden
            && !peer.is_ai_bot
            && peer.jet_active
            && time.elapsed_seconds() - peer.last_seen_sec < 0.3
        {
            commands.spawn((
                crate::world::SurfaceEntity,
                crate::world::DustParticle {
                    iso_pos: peer.iso_pos,
                    iso_vel: Vec3::new(0.0, 0.0, -1.7),
                    life: 0.0,
                    max_life: 0.45,
                    size: 4.6,
                    color: peer.trail.to_color(),
                },
            ));
        }
    }
}

pub fn smooth_remote_vehicles(
    mut commands: Commands,
    time: Res<Time>,
    assets: Res<crate::isometric::ProceduralAssets>,
    lan: Res<LanMultiplayer>,
    map: Res<LunarMap>,
    player: Query<&AstronautPlayer>,
    mut rovers: Query<
        (
            Entity,
            &RoverVehicle,
            &mut Transform,
            &mut Sprite,
            &mut Handle<Image>,
        ),
        Without<crate::flight::FlyingStation>,
    >,
    mut station: Query<&mut Transform, (With<crate::flight::FlyingStation>, Without<RoverVehicle>)>,
    mut dust_elapsed: Local<f32>,
) {
    if !lan.is_active {
        return;
    }
    let factor = 1.0 - (-time.delta_seconds().min(0.05) * 18.0).exp();
    *dust_elapsed += time.delta_seconds().min(0.05);
    let dust = *dust_elapsed >= 0.06;
    if dust {
        *dust_elapsed = 0.0;
    }
    let mounted = player.get_single().ok().and_then(|p| {
        if p.is_passenger {
            None
        } else {
            p.mounted_rover
        }
    });
    for (id, rover, mut transform, mut sprite, mut texture) in &mut rovers {
        if mounted == Some(id) {
            continue;
        }
        let screen = iso_to_world(rover.iso_pos.x, rover.iso_pos.y, rover.iso_pos.z);
        let target = Vec3::new(
            screen.x,
            screen.y + 18.0,
            iso_depth(rover.iso_pos.x, rover.iso_pos.y, rover.iso_pos.z, 2.1),
        );
        transform.translation = transform.translation.lerp(target, factor);
        sprite.flip_x = rover.heading_rad.cos() < 0.0;
        let moving = rover.iso_vel.length() > 0.2;
        let frame = if moving {
            (time.elapsed_seconds() * 12.0) as usize % 3
        } else {
            0
        };
        *texture = match rover.kind {
            crate::types::RoverKind::LightBuggy => assets.rover_light_frames[frame].clone(),
            crate::types::RoverKind::HeavyCruiser => assets.rover_heavy_frames[frame].clone(),
        };
        if dust
            && moving
            && player
                .get_single()
                .is_ok_and(|player| player.iso_pos.distance(rover.iso_pos) < 28.0)
        {
            commands.spawn((
                crate::world::SurfaceEntity,
                crate::world::DustParticle {
                    iso_pos: rover.iso_pos - (rover.iso_vel.normalize_or_zero() * 0.8).extend(0.0),
                    iso_vel: Vec3::new(0.0, 0.0, 0.4),
                    life: 0.0,
                    max_life: 0.5,
                    size: 3.5,
                    color: Color::rgba(0.62, 0.64, 0.69, 0.5),
                },
            ));
        }
    }
    if !lan.is_host {
        if let Ok(mut transform) = station.get_single_mut() {
            let p = map.station_position;
            let screen = iso_to_world(p.x, p.y, p.z);
            let height = (p.z - map.get_elevation(p.x, p.y)).max(0.0);
            let target = Vec3::new(
                screen.x,
                screen.y + 36.0 + (time.elapsed_seconds() * 4.1).sin() * height.min(1.0) * 1.8,
                iso_depth(p.x, p.y, p.z, 2.2),
            );
            transform.translation = transform.translation.lerp(target, factor);
        }
    }
}
