use crate::net_lan::{LanMultiplayer, Message};
use crate::net_world::{Authority, Replica};
use crate::planets::WorldBody;
use crate::world::*;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Default)]
pub struct Seats {
    pub driver: Option<u32>,
    pub passengers: Vec<u32>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct SeatReply {
    pub body: WorldBody,
    pub vehicle: Option<u64>,
    pub passenger: bool,
    pub exit: Option<[f32; 3]>,
}
#[derive(Component, Clone, Default)]
pub struct SeatInfo {
    pub driver: Option<u32>,
    pub passengers: Vec<u32>,
}

fn send(world: &mut World, peer: u32, reply: SeatReply) {
    if peer == world.resource::<LanMultiplayer>().local_peer_id {
        world
            .resource_mut::<LanMultiplayer>()
            .seat_replies
            .push_back(reply);
    } else {
        world
            .resource_mut::<LanMultiplayer>()
            .send_to(peer, Message::Seat(reply));
    }
}
fn notice(world: &mut World, peer: u32, text: &str) {
    if peer == world.resource::<LanMultiplayer>().local_peer_id {
        world.resource_mut::<SurvivalStats>().status_log = text.into();
    } else {
        world
            .resource_mut::<LanMultiplayer>()
            .send_to(peer, Message::Notice(text.into()));
    }
}
fn peer_position(world: &mut World, id: u32, body: WorldBody) -> Option<Vec3> {
    if id == world.resource::<LanMultiplayer>().local_peer_id {
        if world.resource::<LanMultiplayer>().current_body != body {
            return None;
        }
        let mut q = world.query::<&AstronautPlayer>();
        q.iter(world).next().map(|p| p.iso_pos)
    } else {
        world
            .resource::<LanMultiplayer>()
            .peers
            .iter()
            .find(|p| p.peer_id == id && p.body == body && !p.is_ai_bot)
            .and_then(|p| p.samples.back().map(|s| s.1))
    }
}
fn mark(world: &mut World, body: WorldBody, id: u64) {
    let seats = world
        .resource::<Authority>()
        .seats
        .get(&(body, id))
        .cloned()
        .unwrap_or_default();
    if let Some(entity) = crate::net_world::find_rover(world, id) {
        world.entity_mut(entity).insert(SeatInfo {
            driver: seats.driver,
            passengers: seats.passengers,
        });
    }
}

fn release(world: &mut World, peer: u32, body: WorldBody, exit: bool) {
    let previous = world
        .resource::<Authority>()
        .seats
        .iter()
        .find(|((b, _), s)| *b == body && (s.driver == Some(peer) || s.passengers.contains(&peer)))
        .map(|(k, _)| *k);
    let Some(key) = previous else {
        return;
    };
    let exit_position = if exit {
        let origin = crate::net_world::find_rover(world, key.1)
            .and_then(|e| world.get::<RoverVehicle>(e).map(|r| r.iso_pos));
        let Some(position) =
            origin.and_then(|p| world.resource::<LunarMap>().safe_exit(p.truncate(), 3.5))
        else {
            notice(
                world,
                peer,
                crate::locale::text(
                    "Wyjście z łazika jest zablokowane. Przestaw go na wolne miejsce.",
                ),
            );
            return;
        };
        Some(
            position
                .extend(
                    world
                        .resource::<LunarMap>()
                        .get_elevation(position.x, position.y),
                )
                .to_array(),
        )
    } else {
        None
    };
    let promoted = {
        let mut authority = world.resource_mut::<Authority>();
        let s = authority.seats.get_mut(&key).unwrap();
        if s.driver == Some(peer) {
            s.driver = if s.passengers.is_empty() {
                None
            } else {
                Some(s.passengers.remove(0))
            };
            s.driver
        } else {
            s.passengers.retain(|id| *id != peer);
            None
        }
    };
    if let Some(driver) = promoted {
        send(
            world,
            driver,
            SeatReply {
                body,
                vehicle: Some(key.1),
                passenger: false,
                exit: None,
            },
        );
    }
    mark(world, body, key.1);
    if exit {
        send(
            world,
            peer,
            SeatReply {
                body,
                vehicle: None,
                passenger: false,
                exit: exit_position,
            },
        );
    }
}

pub fn board(world: &mut World, peer: u32, target: Option<u64>) {
    let body = world.resource::<LunarMap>().body;
    let Some(pos) = peer_position(world, peer, body) else {
        return;
    };
    if let Some(id) = target {
        let Some(entity) = crate::net_world::find_rover(world, id) else {
            return;
        };
        let rover = world.get::<RoverVehicle>(entity).unwrap();
        if rover.iso_pos.distance(pos) > 3.0 {
            notice(world, peer, crate::locale::text("Podejdź bliżej łazika."));
            return;
        }
        let capacity = if rover.kind == crate::types::RoverKind::HeavyCruiser {
            4
        } else {
            2
        };
        let seats = world
            .resource::<Authority>()
            .seats
            .get(&(body, id))
            .cloned()
            .unwrap_or_default();
        if seats.driver == Some(peer) || seats.passengers.contains(&peer) {
            send(
                world,
                peer,
                SeatReply {
                    body,
                    vehicle: Some(id),
                    passenger: seats.driver != Some(peer),
                    exit: None,
                },
            );
            return;
        }
        if seats.passengers.len() + usize::from(seats.driver.is_some()) >= capacity {
            notice(
                world,
                peer,
                crate::locale::text("Łazik jest pełny. Poczekaj na wolne miejsce."),
            );
            return;
        }
        release(world, peer, body, false);
        let passenger = {
            let mut authority = world.resource_mut::<Authority>();
            let seats = authority.seats.entry((body, id)).or_default();
            if seats.driver.is_none() {
                seats.driver = Some(peer);
                false
            } else {
                seats.passengers.push(peer);
                true
            }
        };
        world.entity_mut(entity).insert(Replica(id));
        mark(world, body, id);
        send(
            world,
            peer,
            SeatReply {
                body,
                vehicle: Some(id),
                passenger,
                exit: None,
            },
        );
    } else {
        release(world, peer, body, true);
    }
}

pub fn apply_replies(world: &mut World) {
    let replies: Vec<_> = world
        .resource_mut::<LanMultiplayer>()
        .seat_replies
        .drain(..)
        .collect();
    for reply in replies {
        if reply.body != world.resource::<LunarMap>().body {
            continue;
        }
        let entity = reply
            .vehicle
            .and_then(|id| crate::net_world::find_rover(world, id));
        if reply.vehicle.is_some() && entity.is_none() {
            world
                .resource_mut::<LanMultiplayer>()
                .seat_replies
                .push_back(reply);
            continue;
        }
        let origin = entity.and_then(|id| world.get::<RoverVehicle>(id).map(|r| r.iso_pos));
        let mut q = world.query::<&mut AstronautPlayer>();
        if let Ok(mut player) = q.get_single_mut(world) {
            player.mounted_rover = entity;
            player.is_passenger = reply.passenger;
            player.inside_building = false;
            player.airlock_anim_timer = 0.0;
            player.iso_vel = Vec3::ZERO;
            player.grounded = true;
            if let Some(pos) = origin {
                player.iso_pos = pos;
            } else if let Some(pos) = reply.exit {
                player.iso_pos = Vec3::from_array(pos);
            }
        }
        let mut stats = world.resource_mut::<SurvivalStats>();
        stats.drove_rover_once |= entity.is_some();
        stats.status_log = if entity.is_none() {
            crate::locale::text("Wysiadłeś z łazika.")
        } else if reply.passenger {
            crate::locale::text("Jesteś pasażerem. Kierowca prowadzi, E — wysiądź.")
        } else {
            crate::locale::text("Jesteś kierowcą. WASD — jedź, E — wysiądź.")
        }
        .into();
    }
    let notices: Vec<_> = world
        .resource_mut::<LanMultiplayer>()
        .notices
        .drain(..)
        .collect();
    if let Some(last) = notices.last() {
        world.resource_mut::<SurvivalStats>().status_log = last.clone();
    }
}

pub fn cleanup(world: &mut World) {
    let body = world.resource::<LunarMap>().body;
    let local = world.resource::<LanMultiplayer>().local_peer_id;
    let expired: Vec<_> = world
        .resource::<Authority>()
        .seats
        .iter()
        .filter(|((b, _), _)| *b == body)
        .flat_map(|(_, s)| s.driver.into_iter().chain(s.passengers.iter().copied()))
        .filter(|id| {
            if *id == local {
                world.resource::<LanMultiplayer>().current_body != body
            } else {
                !world
                    .resource::<LanMultiplayer>()
                    .peers
                    .iter()
                    .any(|p| p.peer_id == *id && p.body == body && !p.is_ai_bot)
            }
        })
        .collect();
    for peer in expired {
        release(world, peer, body, false);
    }
}
