use crate::net_lan::{LanMultiplayer, Message};
use crate::net_world::Action;
use crate::world::{AstronautPlayer, SurvivalStats};
use bevy::prelude::*;

#[derive(Clone)]
pub struct CoopAnimation {
    pub a: u32,
    pub b: u32,
    pub kind: u8,
    pub start: f32,
    pub sounded: bool,
}

pub fn input(world: &mut World) {
    let now = world.resource::<Time>().elapsed_seconds();
    let kind = if world
        .resource::<ButtonInput<KeyCode>>()
        .just_pressed(KeyCode::F8)
    {
        1
    } else if world
        .resource::<ButtonInput<KeyCode>>()
        .just_pressed(KeyCode::F9)
    {
        2
    } else {
        0
    };
    if kind == 0 {
        return;
    }
    let lan = world.resource::<LanMultiplayer>();
    let id = lan.local_peer_id;
    let incoming = lan
        .invite
        .filter(|(_, to, k, until)| *to == id && *k == kind && *until > now)
        .map(|v| v.0);
    let pos = {
        let mut q = world.query::<&AstronautPlayer>();
        q.iter(world).next().map(|p| p.iso_pos)
    };
    let target = incoming.or_else(|| {
        pos.and_then(|pos| {
            world
                .resource::<LanMultiplayer>()
                .peers
                .iter()
                .filter(|p| {
                    !p.is_ai_bot
                        && p.body == world.resource::<crate::world::LunarMap>().body
                        && now - p.last_seen_sec < 1.0
                        && p.iso_pos.distance(pos) < 2.2
                })
                .min_by(|a, b| a.iso_pos.distance(pos).total_cmp(&b.iso_pos.distance(pos)))
                .map(|p| p.peer_id)
        })
    });
    let Some(target) = target else {
        if let Some(mut stats) = world.get_resource_mut::<SurvivalStats>() {
            stats.status_log =
                crate::locale::text("Podejdź do gracza: F8 — piątka, F9 — wspólny taniec.").into();
        }
        return;
    };
    if world.resource::<LanMultiplayer>().is_host {
        request(world, id, target, kind);
    } else {
        world
            .resource_mut::<LanMultiplayer>()
            .send(Message::Action(Action::Coop { target, kind }));
    }
}

fn position(world: &mut World, id: u32) -> Option<Vec3> {
    if id == world.resource::<LanMultiplayer>().local_peer_id {
        if world.resource::<LanMultiplayer>().current_body
            != world.resource::<crate::world::LunarMap>().body
        {
            return None;
        }
        let mut q = world.query::<&AstronautPlayer>();
        q.iter(world)
            .next()
            .filter(|p| !p.inside_building && p.mounted_rover.is_none())
            .map(|p| p.iso_pos)
    } else {
        world
            .resource::<LanMultiplayer>()
            .peers
            .iter()
            .find(|p| {
                p.peer_id == id
                    && !p.is_ai_bot
                    && p.body == world.resource::<crate::world::LunarMap>().body
            })
            .and_then(|p| p.samples.back().map(|s| s.1))
    }
}

pub fn request(world: &mut World, from: u32, to: u32, kind: u8) {
    if from == to || !(1..=2).contains(&kind) {
        return;
    }
    let now = world.resource::<Time>().elapsed_seconds();
    let Some(a) = position(world, from) else {
        return;
    };
    let Some(b) = position(world, to) else {
        return;
    };
    if a.distance(b) > 2.2 {
        return;
    }
    let accepted = world
        .resource::<LanMultiplayer>()
        .invite
        .is_some_and(|(a, b, k, t)| a == to && b == from && k == kind && t > now);
    let mut lan = world.resource_mut::<LanMultiplayer>();
    if accepted {
        let start = now + 0.4;
        lan.invite = None;
        lan.coop = Some(CoopAnimation {
            a: from,
            b: to,
            kind,
            start,
            sounded: false,
        });
        lan.send(Message::CoopStart {
            a: from,
            b: to,
            kind,
            start,
        });
    } else {
        if lan.invite.is_some_and(|(_, _, _, t)| t > now) {
            return;
        }
        lan.invite = Some((from, to, kind, now + 8.0));
        lan.send(Message::CoopInvite { from, to, kind });
    }
}

pub fn animate(
    time: Res<Time>,
    mut lan: ResMut<LanMultiplayer>,
    settings: Res<crate::types::GameSettings>,
    audio: Res<crate::audio::GameAudio>,
    mut stats: ResMut<SurvivalStats>,
    mut player: Query<(&AstronautPlayer, &mut Transform)>,
    mut gizmos: Gizmos,
    mut sprites: Query<&mut Sprite, With<AstronautPlayer>>,
) {
    let now = time.elapsed_seconds();
    if lan.crew_blend > 0.0 {
        if let Ok(mut sprite) = sprites.get_single_mut() {
            sprite
                .color
                .set_a((1.0 - lan.crew_blend / 0.35).clamp(0.15, 1.0));
        }
    }
    if lan.crew_blend > 0.0 {
        if let Ok((_, mut tf)) = player.get_single_mut() {
            tf.scale *= 1.0 + (lan.crew_blend * 18.0).sin() * 0.04;
        }
    }
    let local = lan.local_peer_id;
    if let Some((_, to, kind, until)) = lan.invite {
        if until < now {
            lan.invite = None;
        } else if to == local {
            stats.status_log = if kind == 1 {
                crate::locale::text("Dołącz do piątki: naciśnij F8!")
            } else {
                crate::locale::text("Dołącz do wspólnego tańca: naciśnij F9!")
            }
            .into();
        }
    }
    let mut participants = Vec::new();
    let mut clap = false;
    if let Some(coop) = &mut lan.coop {
        let t = now - coop.start;
        if t >= 0.0 && t < 2.8 {
            if t >= 0.7 && !coop.sounded {
                coop.sounded = true;
                clap = coop.kind == 1;
            }
            participants.push((coop.a, coop.kind, t));
            participants.push((coop.b, coop.kind, t));
        }
    }
    if clap {
        audio.play(
            crate::audio::SfxKind::HighFive,
            settings.master_volume * settings.sfx_volume,
        );
    }
    if lan.emote_until > now {
        participants.push((local, lan.emote, now));
    }
    for peer in &lan.peers {
        if !peer.is_ai_bot && peer.body == lan.current_body && peer.emote_until > now {
            participants.push((peer.peer_id, peer.emote, now));
        }
    }
    let local_pos = player.get_single().ok().map(|(p, _)| p.iso_pos);
    for (id, kind, t) in participants {
        let pos = if id == local {
            local_pos
        } else {
            lan.peers
                .iter()
                .find(|p| p.peer_id == id)
                .map(|p| p.iso_pos)
        };
        let Some(pos) = pos else {
            continue;
        };
        let center = crate::isometric::iso_to_world(pos.x, pos.y, pos.z) + Vec2::Y * 28.0;
        let wave = (t * 10.0).sin();
        if id == local {
            if let Ok((p, mut transform)) = player.get_single_mut() {
                if p.mounted_rover.is_none() {
                    transform.rotation = Quat::from_rotation_z(wave * 0.1);
                }
            }
        }
        let mut end = if kind == 1 {
            center + Vec2::new(18.0 + wave * 4.0, 25.0)
        } else {
            center + Vec2::new(wave * 27.0, 15.0)
        };
        if let Some(coop) = lan.coop.as_ref().filter(|c| {
            c.kind == 1 && (c.a == id || c.b == id) && (0.0..2.8).contains(&(now - c.start))
        }) {
            let other = if coop.a == id { coop.b } else { coop.a };
            let other_pos = if other == local {
                local_pos
            } else {
                lan.peers
                    .iter()
                    .find(|p| p.peer_id == other)
                    .map(|p| p.iso_pos)
            };
            if let Some(other_pos) = other_pos {
                let other = crate::isometric::iso_to_world(other_pos.x, other_pos.y, other_pos.z)
                    + Vec2::Y * 28.0;
                let target = (center + other) * 0.5 + Vec2::Y * 21.0;
                end = end.lerp(target, ((now - coop.start) / 0.7).clamp(0.0, 1.0));
            }
        }
        gizmos.line_2d(
            center + Vec2::new(11.0, 0.0),
            end,
            Color::rgb(0.80, 0.94, 1.0),
        );
        gizmos.circle_2d(end, 4.0, Color::rgb(1.0, 0.85, 0.35));
        if kind == 2 {
            gizmos.line_2d(
                center + Vec2::new(-11.0, 0.0),
                center + Vec2::new(-wave * 27.0, 15.0),
                Color::rgb(0.80, 0.94, 1.0),
            );
        }
        if kind == 3 {
            for i in 0..3 {
                let a = i as f32 * 2.1 + t * 2.0;
                gizmos.circle_2d(
                    center + Vec2::new(a.cos() * 18.0, 35.0 + a.sin() * 8.0),
                    2.0,
                    Color::rgb(1.0, 0.83, 0.25),
                );
            }
        }
    }
    if let Some(coop) = &lan.coop {
        let t = now - coop.start;
        if coop.kind == 1 && (0.7..1.0).contains(&t) {
            let a = if coop.a == local {
                local_pos
            } else {
                lan.peers
                    .iter()
                    .find(|p| p.peer_id == coop.a)
                    .map(|p| p.iso_pos)
            };
            let b = if coop.b == local {
                local_pos
            } else {
                lan.peers
                    .iter()
                    .find(|p| p.peer_id == coop.b)
                    .map(|p| p.iso_pos)
            };
            if let (Some(a), Some(b)) = (a, b) {
                let mid = (crate::isometric::iso_to_world(a.x, a.y, a.z)
                    + crate::isometric::iso_to_world(b.x, b.y, b.z))
                    * 0.5
                    + Vec2::Y * 49.0;
                for i in 0..8 {
                    let angle = i as f32 * std::f32::consts::TAU / 8.0;
                    let dir = Vec2::new(angle.cos(), angle.sin());
                    gizmos.line_2d(
                        mid + dir * 9.0,
                        mid + dir * 24.0,
                        Color::rgb(1.0, 0.87, 0.2),
                    );
                }
            }
        }
    }
}
