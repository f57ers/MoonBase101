use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

use crate::mineral::ResourcePickup;
use crate::net_lan::{LanMultiplayer, LanPacket, Message};
use crate::net_world::{Action, Authority, Replica};
use crate::physics_gameplay::LunarEventManager;
use crate::planets::WorldBody;
use crate::types::{LunarEventKind, StructureKind, TileKind};
use crate::world::{AstronautPlayer, LunarMap, RoverVehicle, SurfaceUiCmd, SurvivalStats};

const VIEW_RADIUS: i32 = 13;
const ACTION_RANGE: f32 = 7.5;

#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Control {
    key: String,
    name: String,
    dx: f32,
    dy: f32,
    mining: Option<[i32; 2]>,
    demolishing: bool,
    latency_ms: u32,
    action: Option<PhoneAction>,
}

impl Control {
    pub fn valid(&self) -> bool {
        self.key.len() == 32
            && self.key.bytes().all(|b| b.is_ascii_hexdigit())
            && self.name.len() <= 100
            && self.dx.abs() <= 1.0
            && self.dy.abs() <= 1.0
            && self.latency_ms <= 10_000
    }
}

#[derive(Clone, Deserialize)]
#[serde(tag = "kind", content = "target", rename_all = "snake_case")]
enum PhoneAction {
    Backpack,
    Flag,
    Grant([usize; 2]),
    Chat(String),
    Enter,
    Survey,
    Build([i32; 3]),
    Drop([usize; 2]),
    Travel(WorldBody),
    Water,
    Food,
    Drink,
    Eat,
    Heal,
    Repair,
    Tool,
    Flare,
    Wave,
    HighFive,
    Dance,
    Respawn,
    Leave,
}

pub struct MobilePlayer {
    pub(crate) id: u32,
    name: String,
    body: WorldBody,
    position: Option<Vec3>,
    seen: Instant,
    input: Control,
    oxygen: f32,
    health: f32,
    hunger: f32,
    hydration: f32,
    suit: f32,
    inside: bool,
    riding: Option<u64>,
    note: String,
    mining: Option<([i32; 2], f32)>,
    scan: Option<(usize, f32)>,
    pickup_wait: f32,
    snapshot: MobileState,
    last_snapshot: f32,
    last_broadcast: f32,
}

impl MobilePlayer {
    fn new(id: u32, name: &str, body: WorldBody) -> Self {
        let clean: String = name
            .trim()
            .chars()
            .filter(|c| !c.is_control())
            .take(20)
            .collect();
        Self {
            id,
            name: if clean.is_empty() {
                format!("Astronauta {}", id % 1000)
            } else {
                clean
            },
            body,
            position: None,
            seen: Instant::now(),
            input: Control::default(),
            oxygen: 100.0,
            health: 100.0,
            hunger: 100.0,
            hydration: 100.0,
            suit: 100.0,
            inside: false,
            riding: None,
            note: crate::locale::text("Wylądowałeś. Dotknij pola na mapie, aby je wybrać.").into(),
            mining: None,
            scan: None,
            pickup_wait: 0.0,
            snapshot: MobileState::default(),
            last_snapshot: -1.0,
            last_broadcast: -1.0,
        }
    }

    pub fn receive(&mut self, message: &Message) {
        match message {
            Message::Seat(seat) if seat.body == self.body => {
                self.riding = seat.vehicle;
                if let Some(exit) = seat.exit {
                    self.position = Some(Vec3::from_array(exit));
                }
                self.note = if seat.vehicle.is_none() {
                    crate::locale::text("Wysiadłeś z łazika.")
                } else if seat.passenger {
                    crate::locale::text("Jedziesz jako pasażer.")
                } else {
                    crate::locale::text("Prowadzisz łazik. Wysiądź przyciskiem Wejdź / wysiądź.")
                }
                .into();
            }
            Message::Notice(note) => self.note = note.clone(),
            Message::Inventory(inventory) => {
                if let Some([health, hunger, hydration, suit]) = inventory.1 {
                    self.health = health;
                    self.hunger = hunger;
                    self.hydration = hydration;
                    self.suit = suit;
                }
                if !inventory.0.status_log.is_empty() {
                    self.note = inventory.0.status_log.clone();
                }
            }
            _ => {}
        }
    }
}

#[derive(Clone, Default, Serialize)]
pub struct MobileState {
    view: [i32; 2],
    cargo_limit: Option<f32>,
    creative: bool,
    quests_disabled: bool,
    flare_lights: Vec<(i32, i32, f32)>,
    chat: Vec<crate::net_lan::ChatLine>,
    ready: bool,
    pub(crate) id: u32,
    body: &'static str,
    planet: &'static str,
    pos: [f32; 3],
    station: [f32; 3],
    ground_tint: [f32; 4],
    inside: bool,
    vitals: [f32; 5],
    tiles: Vec<(
        i32,
        i32,
        f32,
        TileKind,
        Option<StructureKind>,
        Option<&'static str>,
        u32,
        Option<crate::types::WorldFeatureKind>,
    )>,
    crew: Vec<(u32, String, [f32; 3], f32)>,
    looks: Vec<(u32, [f32; 4], crate::net_lan::Appearance, bool)>,
    rovers: Vec<([f32; 3], bool, crate::types::RoverKind)>,
    drops: Vec<[f32; 3]>,
    surveys: Vec<(i32, i32, bool, &'static str)>,
    quest: &'static str,
    event: &'static str,
    note: String,
    cargo: [u32; 11],
    cargo_kg: f32,
    mining: f32,
    scanning: f32,
}

pub fn exchange(world: &mut World, control: Control) -> Result<MobileState, &'static str> {
    let body = world.resource::<LunarMap>().body;
    let mut lan = world.resource_mut::<LanMultiplayer>();
    if matches!(control.action, Some(PhoneAction::Leave)) {
        if let Some(player) = lan.mobile_players.remove(&control.key) {
            lan.remove_mobile_peer(player.id);
            let id = player.id;
            drop(lan);
            world.resource_mut::<Authority>().forget_mobile_player(id);
        }
        return Ok(MobileState::default());
    }
    if !lan.mobile_players.contains_key(&control.key) {
        if lan.session_full() {
            return Err(crate::locale::text(
                "Sesja jest pełna. Host może przyjąć trzech graczy.",
            ));
        }
        let mut id = rand::random::<u32>().max(100);
        while lan.local_peer_id == id
            || lan.endpoints.contains_key(&id)
            || lan.mobile_players.values().any(|p| p.id == id)
        {
            id = rand::random::<u32>().max(100);
        }
        lan.mobile_players.insert(
            control.key.clone(),
            MobilePlayer::new(id, &control.name, body),
        );
    }
    let player = lan.mobile_players.get_mut(&control.key).unwrap();
    player.seen = Instant::now();
    player.input.dx = control.dx;
    player.input.dy = control.dy;
    player.input.mining = control.mining;
    player.input.demolishing = control.demolishing;
    player.input.latency_ms = control.latency_ms;
    if control.action.is_some() {
        player.input.action = control.action;
    }
    Ok(player.snapshot.clone())
}

pub fn reap(world: &mut World) {
    let expired: Vec<_> = world
        .resource::<LanMultiplayer>()
        .mobile_players
        .iter()
        .filter(|(_, player)| player.seen.elapsed() > Duration::from_secs(20))
        .map(|(key, player)| (key.clone(), player.id))
        .collect();
    for (key, id) in expired {
        let mut lan = world.resource_mut::<LanMultiplayer>();
        lan.mobile_players.remove(&key);
        lan.remove_mobile_peer(id);
        let mut authority = world.resource_mut::<Authority>();
        authority.forget_mobile_player(id);
    }
}

pub fn update(world: &mut World) {
    let body = world.resource::<LunarMap>().body;
    let keys: Vec<_> = world
        .resource::<LanMultiplayer>()
        .mobile_players
        .iter()
        .filter(|(_, player)| player.body == body)
        .map(|(key, _)| key.clone())
        .collect();
    if keys.is_empty() {
        return;
    }
    let now = world.resource::<Time>().elapsed_seconds();
    let dt = world.resource::<Time>().delta_seconds().min(0.05);
    let mut snapshots = Vec::new();
    for key in keys {
        let mut player = world
            .resource_mut::<LanMultiplayer>()
            .mobile_players
            .remove(&key)
            .unwrap();
        let old_body = player.body;
        let mut actions = Vec::new();
        let active = player.seen.elapsed() < Duration::from_millis(550);
        let input = if active {
            Vec2::new(player.input.dx, player.input.dy)
        } else {
            Vec2::ZERO
        };
        let direction = Vec2::new(input.x + input.y, input.y - input.x).normalize_or_zero();
        let (origin, position) = step(world, &mut player, direction, dt);
        if let Some(position) = player.position {
            let packet = LanPacket {
                peer_id: player.id,
                body: old_body,
                callsign: player.name.clone(),
                role_idx: 0,
                suit_idx: 0,
                appearance: default(),
                jet_active: false,
                latency_ms: Some(player.input.latency_ms),
                iso_pos: position.to_array(),
                vel_xy: ((position - origin).truncate() / dt.max(0.001))
                    .clamp_length_max(12.0)
                    .to_array(),
                facing_right: input.x >= 0.0,
                vitals: [player.health, player.hunger, player.hydration, player.suit],
                vehicle: None,
                built_structure: None,
                shared_science_gb: 0.0,
            };
            let mut lan = world.resource_mut::<LanMultiplayer>();
            let broadcast = now - player.last_broadcast >= 0.05;
            lan.receive_mobile_pose(packet, now, broadcast);
            if broadcast {
                player.last_broadcast = now;
            }
        }
        if player.health > 0.0 {
            if let Some(position) = position {
                advance_actions(world, &mut player, position, active, dt, &mut actions);
            }
        } else if matches!(player.input.action.take(), Some(PhoneAction::Respawn)) {
            player.position = None;
            player.oxygen = 100.0;
            player.health = 100.0;
            player.hunger = 70.0;
            player.hydration = 70.0;
            player.suit = 100.0;
            player.inside = false;
            player.note = crate::locale::text("Wróciłeś do stacji.").into();
        }
        if player.body == old_body && now - player.last_snapshot >= 0.08 {
            player.last_snapshot = now;
            snapshots.push(key.clone());
        }
        let mut lan = world.resource_mut::<LanMultiplayer>();
        // Keep temporary commands within the same queue limit as desktop input.
        for action in actions {
            if lan.actions.len() >= 128 {
                break;
            }
            lan.actions.push_back((player.id, old_body, action));
        }
        lan.mobile_players.insert(key, player);
    }
    // Snapshot after every pose is committed so all phones see the same set of players.
    for key in snapshots {
        let Some(mut player) = world
            .resource_mut::<LanMultiplayer>()
            .mobile_players
            .remove(&key)
        else {
            continue;
        };
        player.snapshot = snapshot(world, &player);
        world
            .resource_mut::<LanMultiplayer>()
            .mobile_players
            .insert(key, player);
    }
}

fn step(
    world: &mut World,
    player: &mut MobilePlayer,
    direction: Vec2,
    dt: f32,
) -> (Vec3, Option<Vec3>) {
    if let Some(id) = player.riding {
        if let Some(entity) = crate::net_world::find_rover(world, id) {
            let rover = world.get::<RoverVehicle>(entity).unwrap();
            let origin = rover.iso_pos;
            let driver = world
                .resource::<Authority>()
                .seats
                .get(&(player.body, id))
                .is_some_and(|s| s.driver == Some(player.id));
            if driver && rover.battery > 0.0 && player.health > 0.0 {
                let target = origin.truncate() + direction * rover.kind.max_speed() * dt;
                let map = world.resource::<LunarMap>();
                let safe = move_on_map(map, origin.truncate(), target, false);
                let height = map.get_elevation(safe.x, safe.y);
                let mut rover = world.get_mut::<RoverVehicle>(entity).unwrap();
                rover.iso_pos = safe.extend(height);
                rover.iso_vel = (safe - origin.truncate()) / dt.max(0.001);
                if direction.length_squared() > 0.0 {
                    rover.heading_rad = direction.y.atan2(direction.x);
                    rover.battery = (rover.battery - dt * 0.15).max(0.0);
                }
            }
            player.position = Some(world.get::<RoverVehicle>(entity).unwrap().iso_pos);
            return (origin, player.position);
        }
        player.riding = None;
    }
    let occupied: Vec<Vec2> = world
        .resource::<LanMultiplayer>()
        .mobile_players
        .values()
        .filter(|other| other.body == player.body)
        .filter_map(|other| other.position.map(|position| position.truncate()))
        .collect();
    let map = world.resource::<LunarMap>();
    if player.position.is_none() {
        let center = map.station_position.truncate();
        let destination = (0..8)
            .filter_map(|index| {
                let angle = index as f32 * std::f32::consts::FRAC_PI_4;
                map.safe_exit(center + Vec2::from_angle(angle) * 3.0, 2.5)
            })
            .find(|position| occupied.iter().all(|other| position.distance(*other) > 1.3));
        let Some(p) = destination else {
            return (Vec3::ZERO, None);
        };
        player.position = Some(p.extend(map.get_elevation(p.x, p.y)));
    }
    let origin = player.position.unwrap();
    if player.health <= 0.0 {
        return (origin, player.position);
    }
    if player.inside && !map.is_inside_pressurized(origin.x, origin.y) {
        player.inside = false;
    }
    let target = origin.truncate() + direction * 4.8 * dt;
    let safe = move_on_map(map, origin.truncate(), target, player.inside);
    let height = map.get_elevation(safe.x, safe.y);
    player.position = Some(safe.extend(height));
    let event = world.resource::<LunarEventManager>().active_event;
    let tile = safe.round().as_ivec2();
    let inside_station = safe.distance(map.station_position.truncate()) < 1.25;
    let supplied = player.inside
        && (inside_station
            || map
                .networks
                .get(&(tile.x, tile.y))
                .is_some_and(|n| n.oxygen && n.powered));
    player.oxygen = if supplied {
        (player.oxygen + 18.0 * dt).min(100.0)
    } else {
        (player.oxygen - (0.30 + direction.length() * 0.35) * dt).max(0.0)
    };
    player.hunger = (player.hunger - 0.035 * dt).max(0.0);
    player.hydration = (player.hydration - 0.055 * dt).max(0.0);
    if event == LunarEventKind::SolarFlareStorm && !player.inside {
        player.suit = (player.suit - 0.6 * dt).max(0.0);
    }
    if player.oxygen <= 0.0 || player.hunger <= 0.0 || player.hydration <= 0.0 {
        player.health = (player.health - 8.0 * dt).max(0.0);
    }
    (origin, player.position)
}

fn move_on_map(map: &LunarMap, origin: Vec2, target: Vec2, inside: bool) -> Vec2 {
    let permitted = |position: Vec2| {
        let tile = position.round().as_ivec2();
        map.cells.get(&(tile.x, tile.y)).is_some_and(|cell| {
            if inside {
                map.is_inside_pressurized(position.x, position.y)
            } else {
                cell.structure.is_none() || cell.structure_hp <= 0.0
            }
        }) && (map.get_elevation(position.x, position.y) - map.get_elevation(origin.x, origin.y))
            .abs()
            < 3.5
    };
    let x = Vec2::new(target.x, origin.y);
    let mut position = if permitted(x) { x } else { origin };
    let y = Vec2::new(position.x, target.y);
    if permitted(y) {
        position = y;
    }
    position
}

fn advance_actions(
    world: &mut World,
    player: &mut MobilePlayer,
    position: Vec3,
    active: bool,
    dt: f32,
    actions: &mut Vec<Action>,
) {
    let request = player.input.action.take();
    let map = world.resource::<LunarMap>();
    if let Some(request) = request {
        let command = match request {
            PhoneAction::Backpack => Some(SurfaceUiCmd::UpgradeBackpack),
            PhoneAction::Flag => Some(SurfaceUiCmd::PlantPolishFlag),
            PhoneAction::Grant([kind, amount]) if kind < 11 => Some(SurfaceUiCmd::GrantCargo(
                kind,
                amount.min(i32::MAX as usize) as u32,
            )),
            PhoneAction::Water => Some(SurfaceUiCmd::MakeWater),
            PhoneAction::Food => Some(SurfaceUiCmd::MakeMeal),
            PhoneAction::Drink => Some(SurfaceUiCmd::DrinkWater),
            PhoneAction::Eat => Some(SurfaceUiCmd::UseRation),
            PhoneAction::Heal => Some(SurfaceUiCmd::UseMedkit),
            PhoneAction::Repair => Some(SurfaceUiCmd::PatchSuit),
            PhoneAction::Tool => Some(SurfaceUiCmd::CraftNextTool),
            PhoneAction::Flare => Some(SurfaceUiCmd::ThrowSurveyFlare),
            PhoneAction::Drop([kind, all]) if kind < 11 && all <= 1 => {
                if player.riding.is_some() || player.inside {
                    player.note =
                        crate::locale::text("Wyjdź na zewnątrz, żeby wyrzucić przedmioty.").into();
                    None
                } else {
                    Some(SurfaceUiCmd::DropCargo(kind, all == 1))
                }
            }
            PhoneAction::Build([x, y, kind])
                if (0..10).contains(&kind) && !player.inside && player.riding.is_none() =>
            {
                Some(SurfaceUiCmd::ServerBuild(x, y, kind as usize))
            }
            _ => None,
        };
        if let Some(command) = command {
            actions.push(Action::Command(command));
        }
        match request {
            PhoneAction::Chat(text) => {
                let now = world.resource::<Time>().elapsed_seconds();
                world.resource_mut::<LanMultiplayer>().publish_chat(
                    player.id,
                    player.name.clone(),
                    text,
                    now,
                );
            }
            PhoneAction::Travel(body) if body != player.body => {
                if position
                    .truncate()
                    .distance(map.station_position.truncate())
                    <= 4.0
                    && player.riding.is_none()
                {
                    player.body = body;
                    player.position = None;
                    player.inside = false;
                    player.mining = None;
                    player.scan = None;
                    player.oxygen = 100.0;
                    player.snapshot = MobileState::default();
                    player.note =
                        moon_base_localize::localized_format!("Wylądowałeś: {}", body.name());
                    // Seed the peer's new dimension before its background world is created.
                    let packet = LanPacket {
                        peer_id: player.id,
                        body,
                        callsign: player.name.clone(),
                        role_idx: 0,
                        suit_idx: 0,
                        appearance: default(),
                        jet_active: false,
                        latency_ms: Some(player.input.latency_ms),
                        iso_pos: [-2.0, 0.0, 0.0],
                        vel_xy: [0.0; 2],
                        facing_right: true,
                        vitals: [player.health, player.hunger, player.hydration, player.suit],
                        vehicle: None,
                        built_structure: None,
                        shared_science_gb: 0.0,
                    };
                    let now = world.resource::<Time>().elapsed_seconds();
                    world
                        .resource_mut::<LanMultiplayer>()
                        .receive_mobile_pose(packet, now, true);
                    return;
                }
                player.note =
                    crate::locale::text("Podejdź do stacji, żeby zmienić planetę.").into();
            }
            PhoneAction::Enter => {
                if player.riding.is_some() {
                    actions.push(Action::Board(None));
                } else if player.inside {
                    if let Some(exit) = map.safe_exit(position.truncate(), 4.5) {
                        player.position = Some(exit.extend(map.get_elevation(exit.x, exit.y)));
                        player.inside = false;
                    }
                } else {
                    // A nearby rover takes precedence over entering a building.
                    let room = map
                        .find_nearby_enterable_building(position.x, position.y, 2.6)
                        .map(|(x, y, _)| {
                            (
                                Vec2::new(x as f32, y as f32),
                                map.get_elevation(x as f32, y as f32),
                            )
                        });
                    if let Some(id) = nearby_rover(world, position) {
                        actions.push(Action::Board(Some(id)));
                    } else if let Some((point, height)) = room {
                        player.position = Some(point.extend(height));
                        player.inside = true;
                        player.note = crate::locale::text(
                            "Jesteś w środku. Na mapie możesz wybrać sąsiednie pomieszczenie.",
                        )
                        .into();
                    } else {
                        player.note = crate::locale::text("Podejdź do śluzy albo łazika.").into();
                    }
                    return;
                }
            }
            PhoneAction::Survey if player.riding.is_none() => {
                let nearby = map
                    .survey_points()
                    .iter()
                    .enumerate()
                    .find(|(index, point)| {
                        map.surveyed & (1 << index) == 0
                            && point.pos.as_vec2().distance(position.truncate()) < 1.9
                    })
                    .map(|(index, _)| index);
                if let Some(index) = nearby {
                    player.scan = Some((index, 0.0));
                    actions.push(Action::Scan { index, start: true });
                    player.note = crate::locale::text("Trwa pomiar. Zostań przy znaczniku.").into();
                } else {
                    actions.push(Action::Mission);
                    player.note =
                        crate::locale::text("Podejdź do znacznika badań lub konsoli bazy.").into();
                }
            }
            PhoneAction::HighFive | PhoneAction::Dance => {
                let lan = world.resource::<LanMultiplayer>();
                let kind = if matches!(request, PhoneAction::HighFive) {
                    1
                } else {
                    2
                };
                let from_invite = lan
                    .invite
                    .filter(|(_, to, k, _)| *to == player.id && *k == kind)
                    .map(|(from, _, _, _)| from);
                if let Some(target) = from_invite {
                    actions.push(Action::Coop { target, kind });
                } else {
                    let target = lan
                        .peers
                        .iter()
                        .filter(|p| !p.is_ai_bot && p.peer_id != player.id && p.body == player.body)
                        .filter(|p| p.iso_pos.distance(position) < 2.7)
                        .min_by(|a, b| {
                            a.iso_pos
                                .distance_squared(position)
                                .total_cmp(&b.iso_pos.distance_squared(position))
                        })
                        .map(|peer| peer.peer_id);
                    if let Some(target) = target {
                        actions.push(Action::Coop { target, kind });
                    } else if lan.current_body == player.body {
                        let host = lan.local_peer_id;
                        let mut query = world.query::<&AstronautPlayer>();
                        if query
                            .get_single(world)
                            .is_ok_and(|p| p.iso_pos.distance(position) < 2.7)
                        {
                            actions.push(Action::Coop { target: host, kind });
                        }
                    }
                }
                return;
            }
            PhoneAction::Wave => {
                let now = world.resource::<Time>().elapsed_seconds();
                world
                    .resource_mut::<LanMultiplayer>()
                    .mobile_emote(player.id, now);
                return;
            }
            _ => {}
        }
    }
    let map = world.resource::<LunarMap>();
    if let Some((index, elapsed)) = player.scan {
        let points = map.survey_points();
        if map.surveyed & (1 << index) != 0
            || points[index].pos.as_vec2().distance(position.truncate()) > 2.1
        {
            player.scan = None;
        } else {
            let duration = if map.campaign_route {
                crate::campaign::SCAN_SECONDS
            } else {
                12.0
            };
            if elapsed + dt >= duration {
                actions.push(Action::Scan {
                    index,
                    start: false,
                });
                player.scan = None;
                player.note = points[index].fact.into();
            } else {
                player.scan = Some((index, elapsed + dt));
            }
        }
    }
    let target = player.input.mining.filter(|&[x, y]| {
        active
            && player.riding.is_none()
            && !player.inside
            && Vec2::new(x as f32, y as f32).distance(position.truncate()) <= ACTION_RANGE
            && map.cells.get(&(x, y)).is_some_and(|cell| {
                if player.input.demolishing {
                    cell.structure.is_some()
                } else {
                    cell.structure.is_none() && (cell.resource_yield > 0 || cell.feature.is_some())
                }
            })
    });
    player.mining = target.map(|target| {
        let old = player
            .mining
            .filter(|(previous, _)| *previous == target)
            .map_or(0.0, |(_, progress)| progress);
        let cell = &map.cells[&(target[0], target[1])];
        let duration = if cell.structure.is_some() {
            if cell.is_world_generated_base {
                1.6
            } else {
                0.95
            }
        } else if cell.feature.is_some() {
            0.85
        } else {
            cell.kind.base_mine_time_sec()
        };
        let speed = world
            .resource::<Authority>()
            .mobile_inventory(player.id)
            .map_or(1.0, |inv| inv.highest_tool.mining_speed_multiplier());
        let progress = old + dt * speed / duration;
        if progress >= 1.0 {
            actions.push(Action::Command(SurfaceUiCmd::ServerMine(
                target[0], target[1],
            )));
            (target, 0.0)
        } else {
            (target, progress)
        }
    });
    player.pickup_wait = (player.pickup_wait - dt).max(0.0);
    if actions.is_empty() && player.pickup_wait == 0.0 && player.riding.is_none() && !player.inside
    {
        let mut query = world.query::<(Entity, &ResourcePickup, Option<&Replica>)>();
        if let Some((entity, _, replica)) = query
            .iter(world)
            .find(|(_, drop, _)| drop.age >= 0.8 && drop.position.distance(position) < 1.55)
        {
            actions.push(Action::Pickup(replica.map_or(entity.to_bits(), |r| r.0)));
            player.pickup_wait = 0.20;
        }
    }
}

fn nearby_rover(world: &mut World, position: Vec3) -> Option<u64> {
    let mut query = world.query::<(Entity, &RoverVehicle, Option<&Replica>)>();
    query
        .iter(world)
        .filter(|(_, rover, _)| rover.iso_pos.distance(position) < 2.7)
        .min_by(|(_, a, _), (_, b, _)| {
            a.iso_pos
                .distance_squared(position)
                .total_cmp(&b.iso_pos.distance_squared(position))
        })
        .map(|(entity, _, replica)| replica.map_or(entity.to_bits(), |r| r.0))
}

fn snapshot(world: &mut World, player: &MobilePlayer) -> MobileState {
    let Some(position) = player.position else {
        return MobileState::default();
    };
    let body = player.body;
    let mut rover_query = world.query::<&RoverVehicle>();
    let rovers = rover_query
        .iter(world)
        .filter(|rover| rover.iso_pos.distance(position) < 45.0)
        .map(|rover| (rover.iso_pos.to_array(), rover.battery > 0.0, rover.kind))
        .collect();
    let mut drop_query = world.query::<&ResourcePickup>();
    let drops = drop_query
        .iter(world)
        .filter(|drop| drop.age > 0.3 && drop.position.distance(position) < 34.0)
        .take(128)
        .map(|drop| drop.position.to_array())
        .collect();
    let lan = world.resource::<LanMultiplayer>();
    let now = world.resource::<Time>().elapsed_seconds();
    let mut crew: Vec<_> = lan
        .peers
        .iter()
        .filter(|peer| !peer.is_ai_bot && peer.peer_id != player.id && peer.body == body)
        .map(|peer| {
            let position = peer.samples.back().map_or(peer.iso_pos, |(_, p, _)| *p);
            let emote = if peer.emote_until > now {
                peer.emote as f32
            } else {
                0.0
            };
            (
                peer.peer_id,
                peer.callsign.clone(),
                position.to_array(),
                emote,
            )
        })
        .collect();
    let mut looks: Vec<_> = lan
        .peers
        .iter()
        .filter(|peer| !peer.is_ai_bot && peer.body == body)
        .map(|peer| {
            (
                peer.peer_id,
                peer.suit.to_color().as_rgba_f32(),
                crate::net_lan::Appearance {
                    visor: peer.visor,
                    accessory: peer.accessory,
                    trail: peer.trail,
                },
                peer.jet_active,
            )
        })
        .collect();
    let host_body = lan.current_body;
    let host_id = lan.local_peer_id;
    let invite = lan
        .invite
        .filter(|(_, to, _, until)| *to == player.id && *until > now);
    let coop = lan
        .coop
        .as_ref()
        .filter(|c| (c.a == player.id || c.b == player.id) && now > c.start && now - c.start < 2.7)
        .map(|c| c.kind);
    let chat = lan.chat.iter().cloned().collect();
    if host_body == body {
        let config = world.resource::<crate::types::MissionConfig>();
        looks.push((
            host_id,
            config.suit_color.to_color().as_rgba_f32(),
            crate::net_lan::Appearance {
                visor: config.visor_tint,
                accessory: config.accessory,
                trail: config.trail_color,
            },
            false,
        ));
        let mut query = world.query::<&AstronautPlayer>();
        if let Ok(host) = query.get_single(world) {
            let keys = world.resource::<ButtonInput<KeyCode>>();
            let boosting = !host.grounded
                && !host.inside_building
                && host.mounted_rover.is_none()
                && (keys.pressed(KeyCode::ShiftLeft)
                    || keys.pressed(KeyCode::ShiftRight)
                    || world
                        .resource::<crate::world::TouchAndUiInput>()
                        .jetpack_held)
                && world.resource::<SurvivalStats>().suit_power > 2.0;
            if let Some(look) = looks.last_mut() {
                look.3 = boosting;
            }
            crew.push((host_id, "Host".into(), host.iso_pos.to_array(), 0.0));
        }
    }
    let map = world.resource::<LunarMap>();
    let center = position.truncate().round().as_ivec2();
    let mut tiles = Vec::new();
    for x in center.x - VIEW_RADIUS..=center.x + VIEW_RADIUS {
        for y in center.y - VIEW_RADIUS..=center.y + VIEW_RADIUS {
            if x.abs_diff(center.x) + y.abs_diff(center.y) > VIEW_RADIUS as u32 + 4 {
                continue;
            }
            if let Some(cell) = map.cells.get(&(x, y)) {
                tiles.push((
                    x,
                    y,
                    cell.elevation,
                    cell.kind,
                    cell.structure.filter(|_| cell.structure_hp > 0.0),
                    cell.feature.map(|feature| feature.name()),
                    cell.resource_yield,
                    cell.feature,
                ));
            }
        }
    }
    let inventory = world.resource::<Authority>().mobile_inventory(player.id);
    let cargo = inventory.map_or([0; 11], |inv| inv.cargo_counts());
    let stats = world.resource::<SurvivalStats>();
    let sun = world
        .resource::<crate::celestial::SolarSystemState>()
        .sun_illumination();
    let ambient = (0.38 + sun * 0.62).clamp(0.36, 1.0);
    let blue = (1.0 - sun) * 0.16;
    let [r, g, b] = map.body.ground_tint();
    MobileState {
        view: [center.x, center.y],
        creative: world.resource::<crate::types::MissionConfig>().scenario
            == crate::types::MissionScenario::OpenSandbox,
        cargo_limit: inventory.map_or(Some(35.0), |inv| {
            inv.cargo_limit().is_finite().then_some(inv.cargo_limit())
        }),
        quests_disabled: map.quests_disabled,
        flare_lights: map
            .flare_lights
            .iter()
            .filter_map(|light| {
                let remaining = light.until - now;
                (remaining > 0.0).then_some((light.x, light.y, remaining))
            })
            .collect(),
        chat,
        ready: true,
        id: player.id,
        body: match body {
            WorldBody::Moon => "Moon",
            WorldBody::Mars => crate::locale::text("Mars"),
            WorldBody::Venus => "Venus",
        },
        planet: body.name(),
        pos: position.to_array(),
        station: map.station_position.to_array(),
        ground_tint: [
            ambient * r,
            (ambient + blue * 0.3).min(1.0) * g,
            (ambient + blue).min(1.0) * b,
            1.0,
        ],
        inside: player.inside,
        vitals: [
            player.health,
            player.oxygen,
            player.hunger,
            player.hydration,
            player.suit,
        ],
        tiles,
        crew,
        looks,
        rovers,
        drops,
        surveys: map
            .survey_points()
            .iter()
            .enumerate()
            .map(|(i, point)| {
                (
                    point.pos.x,
                    point.pos.y,
                    map.surveyed & (1 << i) != 0,
                    point.name,
                )
            })
            .collect(),
        quest: stats
            .campaign
            .as_ref()
            .map_or(crate::locale::text("Odkrywaj powierzchnię"), |campaign| {
                campaign.quest.title()
            }),
        event: world.resource::<LunarEventManager>().active_event.title(),
        note: if let Some(kind) = coop {
            if kind == 1 {
                crate::locale::text("✋ Piątka!").into()
            } else {
                crate::locale::text("♪ Wspólny taniec!").into()
            }
        } else if let Some((_, _, kind, _)) = invite {
            if kind == 1 {
                crate::locale::text("Zaproszenie do piątki. Dotknij Piątka.").into()
            } else {
                crate::locale::text("Zaproszenie do tańca. Dotknij Taniec.").into()
            }
        } else if player.health <= 0.0 {
            crate::locale::text("Koniec zapasów. Wróć do stacji przyciskiem Odrodź.").into()
        } else {
            player.note.clone()
        },
        cargo,
        cargo_kg: inventory.map_or(0.0, |inv| inv.cargo_weight()),
        mining: player.mining.map_or(0.0, |(_, percentage)| percentage),
        scanning: player.scan.map_or(0.0, |(_, elapsed)| {
            elapsed
                / if map.campaign_route {
                    crate::campaign::SCAN_SECONDS
                } else {
                    12.0
                }
        }),
    }
}
