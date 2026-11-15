use crate::mineral::ResourcePickup;
use crate::net_lan::{LanMultiplayer, Message};
use crate::physics_gameplay::LunarEventManager;
use crate::planets::WorldBody;
use crate::types::*;
use crate::world::*;
use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Serialize, Deserialize)]
pub enum Action {
    Board(Option<u64>),
    Command(SurfaceUiCmd),
    Pickup(u64),
    Scan { index: usize, start: bool },
    Mission,
    Resync,
    Coop { target: u32, kind: u8 },
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Inventory(pub SurvivalStats, pub Option<[f32; 4]>);
#[derive(Component)]
pub struct Replica(pub u64);
#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub struct Tile {
    pub x: i32,
    pub y: i32,
    pub kind: TileKind,
    pub elevation: f32,
    pub amount: u32,
    pub structure: Option<StructureKind>,
    pub hp: f32,
    pub charge: f32,
    pub paid: Option<(u32, u32, u32, u32)>,
    pub feature: Option<WorldFeatureKind>,
    pub generated: bool,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Drop {
    pub id: u64,
    pub kind: usize,
    pub amount: u32,
    pub science: f32,
    pub pos: [f32; 3],
    pub vel: [f32; 3],
    pub age: f32,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Rover {
    pub id: u64,
    pub kind: RoverKind,
    pub pos: [f32; 3],
    pub vel: [f32; 2],
    pub heading: f32,
    pub battery: f32,
    pub cargo: u32,
    pub driver: Option<u32>,
    pub passengers: Vec<u32>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Meteor {
    pub id: u64,
    pub pos: [f32; 3],
    pub vel: [f32; 3],
    pub ttl: f32,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Marker {
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub color: [f32; 4],
}
#[derive(Clone, Serialize, Deserialize)]
pub struct WorldFrame {
    #[serde(default)]
    pub flare_lights: Vec<FlareLight>,
    #[serde(default)]
    pub creative: bool,
    #[serde(default)]
    pub quests_disabled: bool,
    pub tick: u64,
    pub clock: f32,
    pub body: WorldBody,
    pub seed: u64,
    pub campaign_route: bool,
    pub event: LunarEventKind,
    pub timer: f32,
    pub duration: f32,
    pub sun: f32,
    pub relay: f32,
    pub station: [f32; 3],
    pub tiles: Vec<Tile>,
    pub drops: Vec<Drop>,
    pub meteors: Vec<Meteor>,
    pub rovers: Vec<Rover>,
    pub surveyed: u8,
    pub surveys: [u8; 3],
    pub flags: [bool; 9],
    pub blocks: u32,
    pub storage: [u32; 11],
    pub water_job: Option<crate::base::SupplyJob>,
    pub meal_job: Option<crate::base::SupplyJob>,
    pub waypoints: Vec<Marker>,
    pub campaign: Option<crate::campaign::Campaign>,
    pub science: f32,
    pub transmitted: f32,
}
#[derive(Resource, Default)]
pub struct Authority {
    previous: HashMap<(WorldBody, i32, i32), Tile>,
    peers: usize,
    world_timers: [f32; 3],
    inventory: HashMap<u32, SurvivalStats>,
    last_action: HashMap<u32, f32>,
    scans: HashMap<u32, (usize, f32)>,
    drivers: HashMap<(WorldBody, u64), (u32, f32)>,
    pub(crate) seats: HashMap<(WorldBody, u64), crate::vehicles::Seats>,
    jobs: HashMap<(WorldBody, u8), u32>,
}

impl Authority {
    pub(crate) fn mobile_inventory(&self, peer: u32) -> Option<&SurvivalStats> {
        self.inventory.get(&peer)
    }

    pub(crate) fn forget_mobile_player(&mut self, peer: u32) {
        self.inventory.remove(&peer);
        self.last_action.remove(&peer);
        self.scans.remove(&peer);
        self.jobs.retain(|_, owner| *owner != peer);
    }
}

pub fn shared_command(cmd: SurfaceUiCmd) -> bool {
    matches!(
        cmd,
        SurfaceUiCmd::StoreCargo
            | SurfaceUiCmd::TakeCargo
            | SurfaceUiCmd::DropCargo(..)
            | SurfaceUiCmd::CraftNextTool
            | SurfaceUiCmd::UpgradeBackpack
            | SurfaceUiCmd::GrantCargo(..)
            | SurfaceUiCmd::UseRation
            | SurfaceUiCmd::DrinkWater
            | SurfaceUiCmd::FabricateRover
            | SurfaceUiCmd::MakeWater
            | SurfaceUiCmd::MakeMeal
            | SurfaceUiCmd::UseMedkit
            | SurfaceUiCmd::PatchSuit
            | SurfaceUiCmd::AddWaypointAtPlayer
            | SurfaceUiCmd::MarkAt(..)
            | SurfaceUiCmd::PlantPolishFlag
            | SurfaceUiCmd::ThrowSurveyFlare
            | SurfaceUiCmd::ServerMine(..)
            | SurfaceUiCmd::ServerBuild(..)
    )
}

pub fn network_tick(world: &mut World) {
    let now = world.resource::<Time>().elapsed_seconds();
    let was_client = {
        let lan = world.resource::<LanMultiplayer>();
        lan.is_active && !lan.is_host
    };
    world.resource_mut::<LanMultiplayer>().poll_incoming(now);
    let state = *world.resource::<State<AppState>>().get();
    let active = world.resource::<LanMultiplayer>().is_active;
    if !active {
        if was_client {
            world.run_system_once(crate::world::cleanup_surface_entities);
            world
                .resource_mut::<NextState<AppState>>()
                .set(AppState::Settings);
        }
        return;
    }
    if world.contains_resource::<SurvivalStats>() {
        crate::vehicles::apply_replies(world);
    }
    broadcast_pose(world);
    if !world.resource::<LanMultiplayer>().is_host
        && world
            .resource::<ButtonInput<KeyCode>>()
            .just_pressed(KeyCode::KeyP)
    {
        world
            .resource_mut::<LanMultiplayer>()
            .send(Message::Action(Action::Mission));
    }
    crate::emotes::input(world);
    // Polling continues in menus and while paused, so TCP never waits for surface rendering.
    if state == AppState::Settings && world.resource::<LanMultiplayer>().client_enter_pending {
        world.resource_mut::<LanMultiplayer>().client_enter_pending = false;
        world.run_system_once(crate::world::cleanup_surface_entities);
        world
            .resource_mut::<NextState<AppState>>()
            .set(AppState::LunarSurface);
        return;
    }
    let emote = [KeyCode::F5, KeyCode::F6, KeyCode::F7]
        .iter()
        .position(|key| world.resource::<ButtonInput<KeyCode>>().just_pressed(*key));
    if let Some(kind) = emote {
        let mut lan = world.resource_mut::<LanMultiplayer>();
        let peer = lan.local_peer_id;
        lan.emote = kind as u8 + 1;
        lan.emote_until = now + 3.0;
        lan.send(Message::Emote {
            peer,
            kind: kind as u8 + 1,
        });
    }
    if world.contains_resource::<SurvivalStats>() {
        if matches!(
            state,
            AppState::Paused
                | AppState::Settings
                | AppState::Controls
                | AppState::Discovery
                | AppState::PlanetAtlas
                | AppState::InterplanetaryFlight
        ) {
            world.run_system_once(crate::physics_gameplay::update_projectiles_and_particles);
            world.run_system_once(crate::physics_gameplay::update_survival_base_and_events);
            if world.resource::<LanMultiplayer>().is_host {
                serve_actions(world);
                world.run_system_once(publish_world);
            }
        }
        if world.resource::<LanMultiplayer>().is_host {
            synchronize_vehicles(world);
        }
        world.run_system_once(apply_frames);
    }
}

fn apply_inventory(source: &SurvivalStats, target: &mut SurvivalStats) {
    target.backpack_level = source.backpack_level;
    target.placement_ready_at = source.placement_ready_at;
    target.creative_cargo = source.creative_cargo;
    target.regolith = source.regolith;
    target.water_ice = source.water_ice;
    target.titanium = source.titanium;
    target.silicon = source.silicon;
    target.helium3 = source.helium3;
    target.rare_earth = source.rare_earth;
    target.rations = source.rations;
    target.fresh_meals = source.fresh_meals;
    target.drinking_water = source.drinking_water;
    target.medkits = source.medkits;
    target.raw_samples = source.raw_samples;
    target.highest_tool = source.highest_tool;
    target.mineral_catalog = source.mineral_catalog;
    target.collected_minerals = source.collected_minerals;
    target.flare_ready_at = source.flare_ready_at;
    target.status_log = source.status_log.clone();
}

fn apply_frames(
    mut commands: Commands,
    mut lan: ResMut<LanMultiplayer>,
    mut map: ResMut<LunarMap>,
    mut stats: ResMut<SurvivalStats>,
    mut events: ResMut<LunarEventManager>,
    mut solar: ResMut<crate::celestial::SolarSystemState>,
    assets: Res<crate::isometric::ProceduralAssets>,
    structures: Query<(Entity, &StructureSpriteMarker)>,
    connectors: Query<(Entity, &StructureConnectorMarker)>,
    mut replicas: Query<(
        Entity,
        &Replica,
        Option<&mut ResourcePickup>,
        Option<&mut ThrownObject>,
    )>,
    mut tiles: Query<(&IsoTileMarker, &mut Handle<Image>)>,
    mut rovers: Query<(Entity, &mut RoverVehicle, Option<&Replica>)>,
    player: Query<&AstronautPlayer>,
    mut expedition: ResMut<crate::planets::Expedition>,
    mut next: ResMut<NextState<AppState>>,
) {
    if lan.is_host {
        return;
    }
    let inventories: Vec<_> = lan.inventories.drain(..).collect();
    for Inventory(inventory, vitals) in inventories {
        let previous = stats.cargo_counts();
        apply_inventory(&inventory, &mut stats);
        if let Some(v) = vitals {
            stats.health = v[0];
            stats.hunger = v[1];
            stats.hydration = v[2];
            stats.suit_integrity = v[3];
        }
        stats.flare_ready_at -= lan.server_offset;
        if let Ok(player) = player.get_single() {
            for (kind, amount) in stats.cargo_counts().into_iter().enumerate() {
                if amount > previous[kind] {
                    crate::mineral::pickup_notice(
                        &mut commands,
                        kind,
                        amount - previous[kind],
                        player.iso_pos,
                        kind as f32 * 3.0,
                    );
                }
            }
        }
    }
    let mut frames: Vec<_> = lan.world_frames.drain(..).collect();
    if let Some(mut latest) = frames.pop() {
        let mut merged = HashMap::new();
        for frame in frames.drain(..).chain(std::iter::once(latest.clone())) {
            if frame.body == latest.body {
                for tile in frame.tiles {
                    merged.insert((tile.x, tile.y), tile);
                }
            }
        }
        latest.tiles = merged.into_values().collect();
        frames.push(latest);
    }
    for frame in frames {
        if frame.body != map.body {
            continue;
        }
        events.active_event = frame.event;
        events.timer_sec = frame.timer;
        events.duration_sec = frame.duration;
        events.camera_shake_intensity = if frame.event == LunarEventKind::Moonquake {
            0.55
        } else {
            0.0
        };
        solar.lunar_sun_angle_rad = frame.sun;
        solar.relay_orbit_angle_rad = frame.relay;
        map.world_seed = frame.seed;
        map.campaign_route = frame.campaign_route;
        map.flare_lights = frame.flare_lights;
        map.quests_disabled = frame.quests_disabled;
        stats.creative_cargo = frame.creative;
        map.station_position = Vec3::from_array(frame.station);
        map.surveyed = frame.surveyed;
        expedition.update_surveys(frame.surveys);
        apply_flags(&mut stats, frame.flags);
        stats.blocks_mined = frame.blocks;
        map.networks.clear();
        map.network_elapsed = 0.0;
        map.storage = frame.storage;
        map.water_job = frame.water_job;
        map.meal_job = frame.meal_job;
        map.waypoints = frame
            .waypoints
            .into_iter()
            .map(|w| Waypoint {
                name: w.name,
                gx: w.x,
                gy: w.y,
                color: Color::rgba(w.color[0], w.color[1], w.color[2], w.color[3]),
            })
            .collect();
        stats.campaign = frame.campaign;
        if stats
            .campaign
            .as_ref()
            .is_some_and(|c| c.quest == crate::campaign::Quest::Complete)
        {
            next.set(AppState::MissionReport);
        }
        stats.generated_science_gb = frame.science;
        stats.transmitted_science_gb = frame.transmitted;
        for tile in frame.tiles {
            let pos = (tile.x, tile.y);
            let old = map.cells.get(&pos);
            let structure_changed = old.is_none_or(|old| {
                old.structure != tile.structure || (old.structure_hp > 0.0) != (tile.hp > 0.0)
            });
            let feature_changed = old.is_none_or(|old| old.feature != tile.feature);
            let tile_entity = old.and_then(|old| old.tile_entity);
            let mut feature_entity = old.and_then(|old| old.feature_entity);
            if feature_changed {
                if let Some(entity) = feature_entity.take() {
                    commands.entity(entity).despawn_recursive();
                }
            }
            if feature_changed {
                if let Some(feature) = tile.feature {
                    let p = crate::isometric::iso_to_world(
                        tile.x as f32,
                        tile.y as f32,
                        tile.elevation,
                    );
                    feature_entity = Some(
                        commands
                            .spawn((
                                SurfaceEntity,
                                WorldFeatureSpriteMarker {
                                    gx: tile.x,
                                    gy: tile.y,
                                },
                                SpriteBundle {
                                    texture: assets.feature_handle(feature),
                                    transform: Transform::from_xyz(
                                        p.x,
                                        p.y + 22.0,
                                        crate::isometric::iso_depth(
                                            tile.x as f32,
                                            tile.y as f32,
                                            tile.elevation,
                                            1.5,
                                        ),
                                    ),
                                    ..default()
                                },
                            ))
                            .id(),
                    );
                }
            }
            map.cells.insert(
                pos,
                TileCell {
                    kind: tile.kind,
                    elevation: tile.elevation,
                    resource_yield: tile.amount,
                    scanned: true,
                    structure: tile.structure,
                    structure_hp: tile.hp,
                    charge_wh: tile.charge,
                    paid_cost: tile.paid,
                    feature: tile.feature,
                    is_world_generated_base: tile.generated,
                    tile_entity,
                    feature_entity,
                },
            );
            if structure_changed {
                refresh_structure_cluster_around(
                    &mut commands,
                    &assets,
                    &map,
                    tile.x,
                    tile.y,
                    &structures,
                    &connectors,
                );
            }
            for (marker, mut image) in &mut tiles {
                if marker.gx == tile.x && marker.gy == tile.y {
                    *image = assets.terrain_handle(tile.kind, tile.elevation);
                }
            }
        }
        for rover in frame.rovers {
            let found = rovers
                .iter_mut()
                .find(|(_, _, id)| id.is_some_and(|id| id.0 == rover.id))
                .map(|(id, _, _)| id);
            let entity = found.or_else(|| {
                rovers
                    .iter_mut()
                    .find(|(_, r, id)| id.is_none() && r.kind == rover.kind)
                    .map(|(id, _, _)| id)
            });
            let entity = entity.unwrap_or_else(|| {
                spawn_rover_entity(
                    &mut commands,
                    &assets,
                    rover.kind,
                    Vec3::from_array(rover.pos),
                )
            });
            let mounted = player
                .get_single()
                .is_ok_and(|p| p.mounted_rover == Some(entity) && !p.is_passenger);
            if let Ok((_, mut existing, _)) = rovers.get_mut(entity) {
                if !mounted {
                    existing.iso_pos = Vec3::from_array(rover.pos);
                    existing.iso_vel = Vec2::from_array(rover.vel);
                    existing.heading_rad = rover.heading;
                }
                existing.battery = rover.battery;
                existing.cargo_stored = rover.cargo;
            }
            commands.entity(entity).insert((
                Replica(rover.id),
                crate::vehicles::SeatInfo {
                    driver: rover.driver,
                    passengers: rover.passengers,
                },
            ));
        }
        let mut drop_ids = std::collections::HashSet::new();
        for drop in frame.drops {
            drop_ids.insert(drop.id);
            let item = ResourcePickup {
                kind: drop.kind.min(10),
                amount: drop.amount,
                science: drop.science,
                position: Vec3::from_array(drop.pos),
                velocity: Vec3::from_array(drop.vel),
                age: drop.age,
            };
            if let Some((_, _, Some(mut existing), _)) = replicas
                .iter_mut()
                .find(|(_, id, item, _)| id.0 == drop.id && item.is_some())
            {
                *existing = item;
            } else {
                let icons = [
                    &assets.icon_regolith,
                    &assets.icon_silicon,
                    &assets.icon_ice,
                    &assets.icon_titanium,
                    &assets.icon_he3,
                    &assets.icon_rare,
                    &assets.icon_sample,
                ];
                let p = crate::isometric::iso_to_world(
                    item.position.x,
                    item.position.y,
                    item.position.z,
                );
                commands.spawn((
                    SurfaceEntity,
                    Replica(drop.id),
                    item,
                    SpriteBundle {
                        texture: icons[drop.kind.min(6)].clone(),
                        sprite: Sprite {
                            custom_size: Some(Vec2::splat(18.0)),
                            ..default()
                        },
                        transform: Transform::from_xyz(p.x, p.y, 0.0),
                        ..default()
                    },
                ));
            }
        }
        let mut meteor_ids = std::collections::HashSet::new();
        for meteor in frame.meteors {
            meteor_ids.insert(meteor.id);
            let item = ThrownObject {
                iso_pos: Vec3::from_array(meteor.pos),
                iso_vel: Vec3::from_array(meteor.vel),
                ttl: meteor.ttl,
                is_flare: false,
                is_meteor: true,
            };
            if let Some((_, _, _, Some(mut existing))) = replicas
                .iter_mut()
                .find(|(_, id, _, item)| id.0 == meteor.id && item.is_some())
            {
                *existing = item;
            } else {
                commands.spawn((SurfaceEntity, Replica(meteor.id), item));
            }
        }
        for (entity, id, item, meteor) in &mut replicas {
            if item.is_some() && !drop_ids.contains(&id.0)
                || meteor.is_some() && !meteor_ids.contains(&id.0)
            {
                commands.entity(entity).despawn_recursive();
            }
        }
    }
}

pub fn publish_world(
    time: Res<Time>,
    map: Res<LunarMap>,
    mut stats: ResMut<SurvivalStats>,
    events: Res<LunarEventManager>,
    solar: Res<crate::celestial::SolarSystemState>,
    mut lan: ResMut<LanMultiplayer>,
    mut authority: ResMut<Authority>,
    drops: Query<(Entity, &ResourcePickup, Option<&Replica>)>,
    meteors: Query<(Entity, &ThrownObject)>,
    rovers: Query<(Entity, &RoverVehicle, Option<&Replica>)>,
    expedition: Res<crate::planets::Expedition>,
    ledger: Option<Res<crate::dimensions::SurveyLedger>>,
) {
    if !lan.is_active || !lan.is_host {
        return;
    }
    let finished: Vec<_> = authority
        .jobs
        .iter()
        .filter(|((body, kind), _)| {
            if *body != map.body {
                return false;
            }
            if *kind == 0 {
                map.water_job.is_none()
            } else {
                map.meal_job.is_none()
            }
        })
        .map(|(k, p)| (*k, *p))
        .collect();
    for ((body, kind), peer) in finished {
        authority.jobs.remove(&(body, kind));
        if let Some(inventory) = authority.inventory.get_mut(&peer) {
            if kind == 0 && stats.drinking_water >= 2 {
                stats.drinking_water -= 2;
                inventory.drinking_water += 2;
            }
            if kind == 1 && stats.fresh_meals >= 1 {
                stats.fresh_meals -= 1;
                inventory.fresh_meals += 1;
            }
            lan.send_to(peer, Message::Inventory(Inventory(inventory.clone(), None)));
        }
    }
    let dimension = map.body.portrait_index();
    authority.world_timers[dimension] += time.delta_seconds();
    if authority.world_timers[dimension] < 0.1
        && !stats
            .campaign
            .as_ref()
            .is_some_and(|c| c.quest == crate::campaign::Quest::Complete)
    {
        return;
    }
    authority.world_timers[dimension] = 0.0;
    if authority.peers != lan.endpoints.len() {
        authority.previous.clear();
        authority.peers = lan.endpoints.len();
    }
    if lan.endpoints.is_empty() {
        return;
    }
    let mut changed = Vec::new();
    for (&(x, y), cell) in &map.cells {
        let tile = Tile {
            x,
            y,
            kind: cell.kind,
            elevation: cell.elevation,
            amount: cell.resource_yield,
            structure: cell.structure,
            hp: cell.structure_hp,
            charge: (cell.charge_wh * 10.0).round() / 10.0,
            paid: cell.paid_cost,
            feature: cell.feature,
            generated: cell.is_world_generated_base,
        };
        if authority.previous.get(&(map.body, x, y)) != Some(&tile) {
            authority.previous.insert((map.body, x, y), tile.clone());
            changed.push(tile);
            if changed.len() >= 220 {
                break;
            }
        }
    }
    lan.world_tick += 1;
    let frame = WorldFrame {
        flare_lights: map.flare_lights.clone(),
        creative: stats.creative_cargo,
        quests_disabled: map.quests_disabled,
        tick: lan.world_tick,
        clock: time.elapsed_seconds(),
        body: map.body,
        seed: map.world_seed,
        campaign_route: map.campaign_route,
        event: events.active_event,
        timer: events.timer_sec,
        duration: events.duration_sec,
        sun: solar.lunar_sun_angle_rad,
        relay: solar.relay_orbit_angle_rad,
        station: map.station_position.to_array(),
        tiles: changed,
        drops: drops
            .iter()
            .take(512)
            .map(|(id, d, net)| Drop {
                id: net.map_or(id.to_bits(), |net| net.0),
                kind: d.kind,
                amount: d.amount,
                science: d.science,
                pos: d.position.to_array(),
                vel: d.velocity.to_array(),
                age: d.age,
            })
            .collect(),
        meteors: meteors
            .iter()
            .filter(|(_, m)| m.is_meteor)
            .map(|(id, m)| Meteor {
                id: id.to_bits(),
                pos: m.iso_pos.to_array(),
                vel: m.iso_vel.to_array(),
                ttl: m.ttl,
            })
            .collect(),
        rovers: rovers
            .iter()
            .map(|(id, r, net)| Rover {
                id: net.map_or(id.to_bits(), |net| net.0),
                kind: r.kind,
                pos: r.iso_pos.to_array(),
                vel: r.iso_vel.to_array(),
                heading: r.heading_rad,
                battery: r.battery,
                cargo: r.cargo_stored,
                driver: authority
                    .seats
                    .get(&(map.body, net.map_or(id.to_bits(), |n| n.0)))
                    .and_then(|s| s.driver),
                passengers: authority
                    .seats
                    .get(&(map.body, net.map_or(id.to_bits(), |n| n.0)))
                    .map_or_else(Vec::new, |s| s.passengers.clone()),
            })
            .collect(),
        surveyed: map.surveyed,
        surveys: WorldBody::ALL.map(|body| {
            if body == map.body {
                map.surveyed
            } else {
                ledger.as_ref().map_or_else(
                    || expedition.surveyed(body),
                    |l| l.surveys[body.portrait_index()],
                )
            }
        }),
        flags: progress_flags(&stats),
        blocks: stats.blocks_mined,
        storage: map.storage,
        water_job: map.water_job,
        meal_job: map.meal_job,
        waypoints: map
            .waypoints
            .iter()
            .take(128)
            .map(|w| Marker {
                name: w.name.clone(),
                x: w.gx,
                y: w.gy,
                color: w.color.as_rgba_f32(),
            })
            .collect(),
        campaign: stats.campaign.clone(),
        science: stats.generated_science_gb,
        transmitted: stats.transmitted_science_gb,
    };
    let recipients: Vec<_> = lan
        .peers
        .iter()
        .filter(|p| !p.is_ai_bot && p.body == map.body)
        .map(|p| p.peer_id)
        .collect();
    for peer in recipients {
        lan.send_to(peer, Message::World(frame.clone()));
    }
}

pub fn serve_actions(world: &mut World) {
    if !world.resource::<LanMultiplayer>().is_active || !world.resource::<LanMultiplayer>().is_host
    {
        return;
    }
    let body = world.resource::<LunarMap>().body;
    let now = world.resource::<Time>().elapsed_seconds();
    let remote_positions: Vec<_> = world
        .resource::<LanMultiplayer>()
        .peers
        .iter()
        .filter(|p| !p.is_ai_bot && p.body == body)
        .filter_map(|p| p.samples.back().map(|s| s.1))
        .collect();
    let scenario = world.resource::<MissionConfig>().scenario;
    for pos in remote_positions {
        let sx = (pos.x.round() as i32).div_euclid(12);
        let sy = (pos.y.round() as i32).div_euclid(12);
        for x in sx - 1..=sx + 1 {
            for y in sy - 1..=sy + 1 {
                world
                    .resource_mut::<LunarMap>()
                    .generate_sector(x, y, scenario);
            }
        }
    }
    let all: Vec<_> = world
        .resource_mut::<LanMultiplayer>()
        .actions
        .drain(..)
        .collect();
    let mut actions = Vec::new();
    for (peer, dimension, action) in all {
        if dimension == body && actions.len() < 24 {
            actions.push((peer, action));
        } else {
            world
                .resource_mut::<LanMultiplayer>()
                .actions
                .push_back((peer, dimension, action));
        }
    }
    crate::vehicles::cleanup(world);
    for (peer, action) in actions {
        if let Action::Board(target) = action {
            crate::vehicles::board(world, peer, target);
            continue;
        }
        if let Action::Coop { target, kind } = action {
            crate::emotes::request(world, peer, target, kind);
            continue;
        }
        if matches!(action, Action::Resync) {
            world.resource_mut::<Authority>().previous.clear();
            continue;
        }
        let Some(remote) = world
            .resource::<LanMultiplayer>()
            .peers
            .iter()
            .find(|p| p.peer_id == peer && !p.is_ai_bot && p.body == body)
            .cloned()
        else {
            continue;
        };
        let pos = remote.samples.back().map_or(remote.iso_pos, |s| s.1);
        if now - remote.last_seen_sec > 2.0 {
            continue;
        }
        if let Action::Scan { index, start } = action {
            let points = world.resource::<LunarMap>().survey_points();
            if index >= points.len() || points[index].pos.as_vec2().distance(pos.truncate()) > 2.1 {
                continue;
            }
            if start {
                world
                    .resource_mut::<Authority>()
                    .scans
                    .insert(peer, (index, now));
            } else if world
                .resource::<Authority>()
                .scans
                .get(&peer)
                .is_some_and(|(id, t)| {
                    *id == index
                        && now - *t
                            >= if world.resource::<LunarMap>().campaign_route {
                                crate::campaign::SCAN_SECONDS - 1.0
                            } else {
                                11.0
                            }
                })
            {
                world.resource_mut::<Authority>().scans.remove(&peer);
                if world.resource::<LunarMap>().surveyed & (1 << index) == 0 {
                    world.resource_mut::<LunarMap>().surveyed |= 1 << index;
                    world.resource_mut::<SurvivalStats>().generated_science_gb += 18.0;
                }
            }
            continue;
        }
        let last = *world
            .resource::<Authority>()
            .last_action
            .get(&peer)
            .unwrap_or(&-10.0);
        let cooldown = if matches!(action, Action::Command(SurfaceUiCmd::ServerMine(..))) {
            0.45
        } else {
            0.12
        };
        if now - last < cooldown {
            continue;
        }
        world
            .resource_mut::<Authority>()
            .last_action
            .insert(peer, now);
        let player_entity = {
            let mut q = world.query_filtered::<Entity, With<AstronautPlayer>>();
            let Some(e) = q.iter(world).next() else {
                continue;
            };
            e
        };
        let original_player = world.get::<AstronautPlayer>(player_entity).unwrap().clone();
        let host_stats = world.remove_resource::<SurvivalStats>().unwrap();
        let mut personal = world
            .resource_mut::<Authority>()
            .inventory
            .remove(&peer)
            .unwrap_or_else(|| SurvivalStats::from_config(world.resource::<MissionConfig>()));
        personal.campaign = host_stats.campaign.clone();
        personal.blocks_mined = host_stats.blocks_mined;
        let tile = pos.truncate().round().as_ivec2();
        personal.local_powered = world
            .resource::<LunarMap>()
            .networks
            .get(&(tile.x, tile.y))
            .is_some_and(|n| n.powered);
        personal.generated_science_gb = host_stats.generated_science_gb;
        personal.transmitted_science_gb = host_stats.transmitted_science_gb;
        // Personal physiology is predicted locally; inventory and shared rewards are committed here.
        personal.health = remote.vitals[0];
        personal.hunger = remote.vitals[1];
        personal.hydration = remote.vitals[2];
        personal.suit_integrity = remote.vitals[3];
        world.insert_resource(personal);
        let mut actor = original_player.clone();
        actor.iso_pos = pos;
        actor.mounted_rover = None;
        actor.inside_building = world
            .resource::<LunarMap>()
            .is_inside_pressurized(pos.x, pos.y);
        actor.airlock_anim_timer = 0.0;
        *world.get_mut::<AstronautPlayer>(player_entity).unwrap() = actor;
        let old_keys = world.remove_resource::<ButtonInput<KeyCode>>().unwrap();
        world.insert_resource(ButtonInput::<KeyCode>::default());
        let old_mouse = world.remove_resource::<ButtonInput<MouseButton>>().unwrap();
        world.insert_resource(ButtonInput::<MouseButton>::default());
        let old_touch = world.remove_resource::<TouchAndUiInput>().unwrap();
        world.insert_resource(TouchAndUiInput::default());
        let controls = {
            let m = world.resource::<LunarMap>();
            (
                m.hovered_tile,
                m.build_mode,
                m.selected_build,
                m.mining_target,
                m.mining_progress,
                m.piloting_station,
            )
        };
        world.resource_mut::<LunarMap>().piloting_station = false;
        match action {
            Action::Command(cmd) if shared_command(cmd) => {
                let valid = match cmd {
                    SurfaceUiCmd::DropCargo(..) => !world.resource::<Authority>().seats.iter().any(
                        |((dimension, _), seats)| {
                            *dimension == body
                                && (seats.driver == Some(peer) || seats.passengers.contains(&peer))
                        },
                    ),
                    SurfaceUiCmd::ServerBuild(_, _, index) if index >= StructureKind::ALL.len() => {
                        false
                    }
                    SurfaceUiCmd::ServerMine(x, y) | SurfaceUiCmd::ServerBuild(x, y, _) => {
                        Vec2::new(x as f32, y as f32).distance(pos.truncate()) <= 7.5
                    }
                    _ => true,
                };
                if valid {
                    let jobs_before = {
                        let m = world.resource::<LunarMap>();
                        (m.water_job.is_some(), m.meal_job.is_some())
                    };
                    world.resource_mut::<TouchAndUiInput>().cmds.push(cmd);
                    if matches!(cmd, SurfaceUiCmd::FabricateRover) {
                        world.run_system_once(
                            crate::physics_gameplay::update_astronaut_and_rover_physics,
                        );
                    } else {
                        world.run_system_once(
                            crate::physics_gameplay::handle_mining_building_crafting_and_throwing,
                        );
                    }
                    if matches!(cmd, SurfaceUiCmd::MakeWater)
                        && !jobs_before.0
                        && world.resource::<LunarMap>().water_job.is_some()
                    {
                        world
                            .resource_mut::<Authority>()
                            .jobs
                            .entry((body, 0))
                            .or_insert(peer);
                    }
                    if matches!(cmd, SurfaceUiCmd::MakeMeal)
                        && !jobs_before.1
                        && world.resource::<LunarMap>().meal_job.is_some()
                    {
                        world
                            .resource_mut::<Authority>()
                            .jobs
                            .entry((body, 1))
                            .or_insert(peer);
                    }
                    world.run_system_once(crate::mineral::enforce_capacity);
                }
            }
            Action::Pickup(id) => {
                let entity = find_pickup(world, id).unwrap_or(Entity::from_bits(id));
                if let Some(item) = world
                    .get::<ResourcePickup>(entity)
                    .cloned()
                    .filter(|i| i.age >= 0.8 && i.position.distance(pos) < 1.6)
                {
                    let mut stats = world.resource_mut::<SurvivalStats>();
                    if stats.fits_cargo(item.kind, item.amount) {
                        stats.change_cargo(item.kind, item.amount as i32);
                        stats.generated_science_gb += item.science;
                        stats.collected_minerals = true;
                        if (2..=5).contains(&item.kind) {
                            stats.mineral_catalog |= 1 << (item.kind - 2);
                        }
                        world.despawn(entity);
                    }
                }
            }
            Action::Mission => {
                if let Some(c) = &mut world.resource_mut::<SurvivalStats>().campaign {
                    c.action_requested = true;
                }
                world.run_system_once(crate::campaign::update_campaign);
            }
            _ => {}
        }
        let personal = world.remove_resource::<SurvivalStats>().unwrap();
        let mut restored = host_stats;
        restored.campaign = personal.campaign.clone();
        let merged = progress_flags(&restored)
            .into_iter()
            .zip(progress_flags(&personal))
            .map(|(a, b)| a || b)
            .collect::<Vec<_>>();
        apply_flags(&mut restored, merged.try_into().unwrap());
        restored.blocks_mined = restored.blocks_mined.max(personal.blocks_mined);
        restored.generated_science_gb = personal.generated_science_gb;
        restored.transmitted_science_gb = personal.transmitted_science_gb;
        world.resource_mut::<LanMultiplayer>().send_to(
            peer,
            Message::Inventory(Inventory(personal.clone(), {
                let v = [
                    personal.health,
                    personal.hunger,
                    personal.hydration,
                    personal.suit_integrity,
                ];
                (v != remote.vitals).then_some(v)
            })),
        );
        world
            .resource_mut::<Authority>()
            .inventory
            .insert(peer, personal);
        world.insert_resource(restored);
        world.insert_resource(old_keys);
        world.insert_resource(old_mouse);
        world.insert_resource(old_touch);
        *world.get_mut::<AstronautPlayer>(player_entity).unwrap() = original_player;
        let mut m = world.resource_mut::<LunarMap>();
        m.hovered_tile = controls.0;
        m.build_mode = controls.1;
        m.selected_build = controls.2;
        m.mining_target = controls.3;
        m.mining_progress = controls.4;
        m.piloting_station = controls.5;
    }
}

pub(crate) fn synchronize_vehicles(world: &mut World) {
    let now = world.resource::<Time>().elapsed_seconds();
    let dt = world.resource::<Time>().delta_seconds().min(0.05);
    let peers = world.resource::<LanMultiplayer>().peers.clone();
    let host_vehicle = {
        let mut q = world.query::<&AstronautPlayer>();
        q.iter(world).next().and_then(|p| p.mounted_rover)
    };
    let body = world.resource::<LunarMap>().body;
    for peer in peers
        .into_iter()
        .filter(|p| !p.is_ai_bot && p.body == body && now - p.last_seen_sec < 0.5)
    {
        let Some(pose) = peer.vehicle else {
            continue;
        };
        if !pose.driving
            || !world
                .resource::<Authority>()
                .seats
                .get(&(body, pose.id))
                .is_some_and(|s| s.driver == Some(peer.peer_id))
        {
            continue;
        }
        let id = find_rover(world, pose.id).unwrap_or(Entity::from_bits(pose.id));
        if host_vehicle == Some(id) && world.resource::<LanMultiplayer>().current_body == body && {
            let mut q = world.query::<&AstronautPlayer>();
            q.iter(world).next().is_some_and(|p| !p.is_passenger)
        } {
            continue;
        }
        if world
            .resource::<Authority>()
            .drivers
            .get(&(body, pose.id))
            .is_some_and(|(driver, t)| *driver != peer.peer_id && now - *t < 1.0)
        {
            continue;
        }
        if let Some(mut rover) = world.get_mut::<RoverVehicle>(id) {
            let target = Vec3::from_array(pose.pos);
            if rover.iso_pos.distance(target) > 3.0 || !target.is_finite() {
                continue;
            }
            rover.iso_pos = target;
            rover.iso_vel = Vec2::from_array(pose.vel);
            rover.heading_rad = pose.heading;
            if rover.iso_vel.length() > 0.1 {
                rover.battery = (rover.battery - dt * 0.3).max(0.0);
            }
            world
                .resource_mut::<Authority>()
                .drivers
                .insert((body, pose.id), (peer.peer_id, now));
        }
    }
}

fn broadcast_pose(world: &mut World) {
    let dt = world.resource::<Time>().delta_seconds();
    {
        let mut lan = world.resource_mut::<LanMultiplayer>();
        lan.sync_timer += dt;
        if lan.sync_timer < 0.05 {
            return;
        }
        lan.sync_timer = 0.0;
    }
    let player = {
        let mut q = world.query::<&AstronautPlayer>();
        q.iter(world).next().cloned()
    };
    let Some(player) = player else {
        return;
    };
    let Some(stats) = world.get_resource::<SurvivalStats>() else {
        return;
    };
    let config = world.resource::<MissionConfig>();
    let keys = world.resource::<ButtonInput<KeyCode>>();
    let map = world.resource::<LunarMap>();
    let jet_active = (keys.pressed(KeyCode::ShiftLeft)
        || keys.pressed(KeyCode::ShiftRight)
        || world.resource::<TouchAndUiInput>().jetpack_held)
        && !map.any_modal_open()
        && !map.piloting_station
        && !player.grounded
        && !player.inside_building
        && player.mounted_rover.is_none()
        && stats.suit_power > 2.0
        && player.iso_pos.z - map.get_elevation(player.iso_pos.x, player.iso_pos.y) < 4.0;
    let vehicle = player.mounted_rover.and_then(|id| {
        world
            .get::<RoverVehicle>(id)
            .map(|r| crate::net_lan::VehiclePose {
                id: world.get::<Replica>(id).map_or(id.to_bits(), |r| r.0),
                pos: r.iso_pos.to_array(),
                vel: r.iso_vel.to_array(),
                heading: r.heading_rad,
                driving: !player.is_passenger,
            })
    });
    let packet = crate::net_lan::LanPacket {
        peer_id: world.resource::<LanMultiplayer>().local_peer_id,
        body: world.resource::<LunarMap>().body,
        callsign: config.callsign.clone(),
        role_idx: TeamRole::ALL
            .iter()
            .position(|r| *r == config.active_role)
            .unwrap_or(0),
        suit_idx: SuitColor::ALL
            .iter()
            .position(|s| *s == config.suit_color)
            .unwrap_or(0),
        appearance: crate::net_lan::Appearance {
            visor: config.visor_tint,
            accessory: config.accessory,
            trail: config.trail_color,
        },
        jet_active,
        latency_ms: if world.resource::<LanMultiplayer>().is_host {
            Some(0)
        } else {
            world.resource::<LanMultiplayer>().latency_ms
        },
        iso_pos: player.iso_pos.to_array(),
        vel_xy: [player.iso_vel.x, player.iso_vel.y],
        facing_right: player.facing_right,
        vitals: [
            stats.health,
            stats.hunger,
            stats.hydration,
            stats.suit_integrity,
        ],
        vehicle,
        built_structure: None,
        shared_science_gb: 0.0,
    };
    world
        .resource_mut::<LanMultiplayer>()
        .broadcast_state(&packet);
}

pub(crate) fn progress_flags(s: &SurvivalStats) -> [bool; 9] {
    [
        s.collected_minerals,
        s.entered_building_once,
        s.built_habitat_once,
        s.built_power_and_o2,
        s.placed_waypoint_once,
        s.settled_world_structure,
        s.drove_rover_once,
        s.planted_flag_once,
        s.flew_station,
    ]
}
pub(crate) fn apply_flags(s: &mut SurvivalStats, f: [bool; 9]) {
    s.collected_minerals = f[0];
    s.entered_building_once = f[1];
    s.built_habitat_once = f[2];
    s.built_power_and_o2 = f[3];
    s.placed_waypoint_once = f[4];
    s.settled_world_structure = f[5];
    s.drove_rover_once = f[6];
    s.planted_flag_once = f[7];
    s.flew_station = f[8];
}
#[cfg(test)]
mod tests {
    use super::*;
    use bevy::asset::{AssetApp, AssetPlugin};
    fn app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Image>()
            .init_asset::<bevy::render::render_resource::Shader>()
            .add_plugins(bevy::gizmos::GizmoPlugin)
            .init_resource::<Time>()
            .init_resource::<MissionConfig>()
            .init_resource::<GameSettings>()
            .init_resource::<LanMultiplayer>()
            .init_resource::<Authority>()
            .init_resource::<LunarMap>()
            .init_resource::<TouchAndUiInput>()
            .init_resource::<LunarEventManager>()
            .init_resource::<crate::celestial::SolarSystemState>()
            .init_resource::<crate::planets::Expedition>()
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<ButtonInput<MouseButton>>()
            .init_state::<AppState>()
            .insert_resource(crate::audio::GameAudio { tx: None });
        app.world
            .run_system_once(crate::isometric::generate_procedural_assets);
        app.world.run_system_once(setup_lunar_surface);
        app.world
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs(1));
        app
    }
    fn remote(app: &mut App, id: u32, pos: Vec3) {
        let mut peer = crate::net_lan::default_ai_crewmates().remove(0);
        peer.peer_id = id;
        peer.is_ai_bot = false;
        peer.iso_pos = pos;
        peer.samples = std::collections::VecDeque::from([(1.0, pos, Vec2::ZERO)]);
        peer.last_seen_sec = 1.0;
        let mut lan = app.world.resource_mut::<LanMultiplayer>();
        lan.is_active = true;
        lan.is_host = true;
        lan.peers.push(peer);
    }

    #[test]
    fn phones_join_as_independent_players_and_move_in_the_shared_world() {
        use crate::mobile_player::{exchange, update, Control};
        let mut app = app();
        app.world
            .insert_resource(LanMultiplayer::with_test_listener(
                std::net::TcpListener::bind("127.0.0.1:0").unwrap(),
            ));
        let control = |key: u128, dx: f32, action: serde_json::Value| -> Control {
            serde_json::from_value(serde_json::json!({
                "key": format!("{key:032x}"), "name": format!("Telefon {key}"), "dx": dx, "action": action,
            })).unwrap()
        };
        for key in 1..=3 {
            assert!(exchange(&mut app.world, control(key, 0.0, serde_json::Value::Null)).is_ok());
        }
        assert!(exchange(&mut app.world, control(4, 0.0, serde_json::Value::Null)).is_err());
        update(&mut app.world);
        let mut positions = std::collections::HashSet::new();
        for key in 1..=3 {
            let snapshot = serde_json::to_value(
                exchange(&mut app.world, control(key, 0.0, serde_json::Value::Null)).unwrap(),
            )
            .unwrap();
            assert!(
                positions.insert(snapshot["pos"].to_string()),
                "Phones must spawn apart so they remain visible"
            );
            let names: Vec<_> = snapshot["crew"]
                .as_array()
                .unwrap()
                .iter()
                .map(|row| row[1].as_str().unwrap())
                .collect();
            for other in 1..=3 {
                if other != key {
                    assert!(names.contains(&format!("Telefon {other}").as_str()));
                }
            }
        }
        let initial = serde_json::to_value(
            exchange(&mut app.world, control(1, 1.0, serde_json::Value::Null)).unwrap(),
        )
        .unwrap();
        let before = initial["pos"].clone();
        let host_before = app
            .world
            .query::<&AstronautPlayer>()
            .single(&app.world)
            .iso_pos;
        for _ in 0..4 {
            app.world
                .resource_mut::<Time>()
                .advance_by(std::time::Duration::from_millis(50));
            update(&mut app.world);
        }
        let state = serde_json::to_value(
            exchange(&mut app.world, control(1, 0.0, serde_json::Value::Null)).unwrap(),
        )
        .unwrap();
        assert_eq!(state["ready"], true);
        assert_ne!(state["pos"], before);
        assert_eq!(state["crew"].as_array().unwrap().len(), 3);
        exchange(
            &mut app.world,
            control(
                1,
                0.0,
                serde_json::json!({"kind":"chat", "target":"Cześć z telefonu"}),
            ),
        )
        .unwrap();
        app.world
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_millis(100));
        update(&mut app.world);
        for key in 1..=3 {
            let snapshot = serde_json::to_value(
                exchange(&mut app.world, control(key, 0.0, serde_json::Value::Null)).unwrap(),
            )
            .unwrap();
            assert_eq!(
                snapshot["chat"].as_array().unwrap().last().unwrap()["text"],
                "Cześć z telefonu"
            );
        }
        assert_eq!(
            app.world
                .query::<&AstronautPlayer>()
                .single(&app.world)
                .iso_pos,
            host_before
        );
        exchange(
            &mut app.world,
            control(1, 0.0, serde_json::json!({ "kind": "leave" })),
        )
        .unwrap();
        assert!(exchange(&mut app.world, control(4, 0.0, serde_json::Value::Null)).is_ok());
    }
    #[test]
    fn lur_repair_requires_both_materials_and_charges_the_repairer_once() {
        let mut app = app();
        let peer = 99;
        remote(&mut app, peer, Vec3::new(8.0, 8.0, 0.0));
        let tile = (8, 8);
        let wreck = crate::types::WorldFeatureKind::ApolloLrvWreck;
        {
            let mut map = app.world.resource_mut::<LunarMap>();
            let cell = map.cells.get_mut(&tile).unwrap();
            cell.feature = Some(wreck);
            cell.structure = None;
        }
        let mut inventory = app.world.resource::<SurvivalStats>().clone();
        inventory.silicon = 3;
        inventory.regolith = 6;
        app.world
            .resource_mut::<Authority>()
            .inventory
            .insert(peer, inventory);
        let repair = |app: &mut App| {
            app.world
                .resource_mut::<Time>()
                .advance_by(std::time::Duration::from_secs(1));
            app.world
                .resource_mut::<LanMultiplayer>()
                .actions
                .push_back((
                    peer,
                    WorldBody::Moon,
                    Action::Command(SurfaceUiCmd::ServerMine(tile.0, tile.1)),
                ));
            serve_actions(&mut app.world);
        };
        let rovers = app.world.query::<&RoverVehicle>().iter(&app.world).count();
        repair(&mut app);
        assert_eq!(
            app.world.resource::<LunarMap>().cells[&tile].feature,
            Some(wreck)
        );
        let inventory = &app.world.resource::<Authority>().inventory[&peer];
        assert_eq!((inventory.silicon, inventory.regolith), (3, 6));
        app.world
            .resource_mut::<Authority>()
            .inventory
            .get_mut(&peer)
            .unwrap()
            .regolith = 7;
        repair(&mut app);
        assert_eq!(app.world.resource::<LunarMap>().cells[&tile].feature, None);
        let inventory = &app.world.resource::<Authority>().inventory[&peer];
        assert_eq!((inventory.silicon, inventory.regolith), (0, 0));
        assert_eq!(
            app.world.query::<&RoverVehicle>().iter(&app.world).count(),
            rovers + 1
        );
    }

    #[test]
    fn host_mines_once_and_validates_pickups_without_moving_host_player() {
        let mut app = app();
        remote(&mut app, 99, Vec3::new(8.0, 8.0, 0.0));
        let original = {
            let mut q = app.world.query::<&AstronautPlayer>();
            q.single(&app.world).iso_pos
        };
        {
            let mut map = app.world.resource_mut::<LunarMap>();
            let c = map.cells.get_mut(&(8, 8)).unwrap();
            c.kind = TileKind::TitaniumOutcrop;
            c.resource_yield = 45;
            c.feature = None;
            c.structure = None;
        }
        app.world
            .resource_mut::<LanMultiplayer>()
            .actions
            .push_back((
                99,
                WorldBody::Moon,
                Action::Command(SurfaceUiCmd::ServerMine(8, 8)),
            ));
        serve_actions(&mut app.world);
        assert_eq!(
            app.world.resource::<LunarMap>().cells[&(8, 8)].resource_yield,
            40
        );
        assert_eq!(app.world.resource::<SurvivalStats>().titanium, 0);
        assert_eq!(
            app.world
                .query::<&AstronautPlayer>()
                .single(&app.world)
                .iso_pos,
            original
        );
        let entity = app
            .world
            .spawn(ResourcePickup {
                kind: 3,
                amount: 5,
                science: 7.0,
                position: Vec3::new(8.0, 8.0, 0.0),
                velocity: Vec3::ZERO,
                age: 2.0,
            })
            .id();
        app.world.resource_mut::<Authority>().last_action.clear();
        for _ in 0..2 {
            app.world
                .resource_mut::<LanMultiplayer>()
                .actions
                .push_back((99, WorldBody::Moon, Action::Pickup(entity.to_bits())));
        }
        serve_actions(&mut app.world);
        assert!(app.world.get_entity(entity).is_none());
        assert_eq!(app.world.resource::<Authority>().inventory[&99].titanium, 5);
        assert!(app.world.resource::<SurvivalStats>().generated_science_gb >= 7.0);
        assert_eq!(app.world.resource::<SurvivalStats>().titanium, 0);
    }
    #[test]
    fn demolition_removes_the_module_and_returns_its_paid_materials_on_the_ground() {
        let mut app = app();
        remote(&mut app, 99, Vec3::new(8.0, 8.0, 0.0));
        {
            let mut map = app.world.resource_mut::<LunarMap>();
            let cell = map.cells.get_mut(&(8, 8)).unwrap();
            cell.structure = Some(StructureKind::HabitatModule);
            cell.structure_hp = 100.0;
            cell.feature = None;
            cell.paid_cost = Some((12, 7, 5, 1));
        }
        app.world
            .resource_mut::<LanMultiplayer>()
            .actions
            .push_back((
                99,
                WorldBody::Moon,
                Action::Command(SurfaceUiCmd::ServerMine(8, 8)),
            ));
        serve_actions(&mut app.world);
        assert!(app.world.resource::<LunarMap>().cells[&(8, 8)]
            .structure
            .is_none());
        let mut drops = app.world.query::<&ResourcePickup>();
        let mut total = [0; 7];
        for drop in drops.iter(&app.world) {
            total[drop.kind] += drop.amount;
        }
        assert_eq!([total[0], total[3], total[1], total[4]], [12, 7, 5, 1]);
    }

    #[test]
    fn one_player_can_drop_supplies_for_another() {
        let mut app = app();
        let origin = Vec2::splat(8.0);
        let elevation = {
            let mut map = app.world.resource_mut::<LunarMap>();
            for x in 6..=10 {
                for y in 6..=10 {
                    let tile = map.cells.get_mut(&(x, y)).unwrap();
                    tile.structure = None;
                    tile.elevation = 0.0;
                }
            }
            map.get_elevation(origin.x, origin.y)
        };
        remote(&mut app, 99, origin.extend(elevation));
        let mut cargo = SurvivalStats::from_config(app.world.resource::<MissionConfig>());
        cargo.drinking_water = 4;
        app.world
            .resource_mut::<Authority>()
            .inventory
            .insert(99, cargo);
        let run = |app: &mut App, peer, action| {
            app.world.resource_mut::<Authority>().last_action.clear();
            app.world
                .resource_mut::<LanMultiplayer>()
                .actions
                .push_back((peer, WorldBody::Moon, action));
            serve_actions(&mut app.world);
        };
        run(
            &mut app,
            99,
            Action::Command(SurfaceUiCmd::DropCargo(9, false)),
        );
        assert_eq!(
            app.world.resource::<Authority>().inventory[&99].drinking_water,
            3
        );
        run(
            &mut app,
            99,
            Action::Command(SurfaceUiCmd::DropCargo(9, true)),
        );
        assert_eq!(
            app.world.resource::<Authority>().inventory[&99].drinking_water,
            0
        );
        run(
            &mut app,
            99,
            Action::Command(SurfaceUiCmd::DropCargo(99, true)),
        );
        let mut query = app.world.query::<(Entity, &ResourcePickup)>();
        let drops: Vec<_> = query
            .iter(&app.world)
            .filter(|(_, p)| p.kind == 9)
            .collect();
        assert_eq!(drops.len(), 2);
        assert_eq!(drops.iter().map(|(_, p)| p.amount).sum::<u32>(), 4);
        let (&(entity, item), _) = drops.split_first().unwrap();
        let landing = item.position;
        let amount = item.amount;
        assert!(origin.distance(landing.truncate()) > 1.6);
        assert_eq!(item.science, 0.0);
        remote(&mut app, 100, landing);
        let mut receiver = SurvivalStats::from_config(app.world.resource::<MissionConfig>());
        receiver.drinking_water = 0;
        app.world
            .resource_mut::<Authority>()
            .inventory
            .insert(100, receiver);
        for _ in 0..24 {
            app.world
                .run_system_once(crate::mineral::update_background_pickups);
        }
        run(&mut app, 100, Action::Pickup(entity.to_bits()));
        assert_eq!(
            app.world.resource::<Authority>().inventory[&100].drinking_water,
            amount
        );
        assert!(app.world.get_entity(entity).is_none());
    }

    #[test]
    fn full_inventory_does_not_destroy_uncollected_cargo() {
        let mut app = app();
        remote(&mut app, 99, Vec3::new(8.0, 8.0, 0.0));
        let mut inventory = SurvivalStats::from_config(app.world.resource::<MissionConfig>());
        for (kind, amount) in inventory.cargo_counts().into_iter().enumerate() {
            inventory.change_cargo(kind, -(amount as i32));
        }
        inventory.regolith = 140;
        app.world
            .resource_mut::<Authority>()
            .inventory
            .insert(99, inventory);
        let entity = app
            .world
            .spawn(ResourcePickup {
                kind: 3,
                amount: 5,
                science: 7.0,
                position: Vec3::new(8.0, 8.0, 0.0),
                velocity: Vec3::ZERO,
                age: 2.0,
            })
            .id();
        app.world
            .resource_mut::<LanMultiplayer>()
            .actions
            .push_back((99, WorldBody::Moon, Action::Pickup(entity.to_bits())));
        serve_actions(&mut app.world);
        assert!(app.world.get_entity(entity).is_some());
        assert_eq!(
            app.world.resource::<Authority>().inventory[&99].regolith,
            140
        );
    }
    #[test]
    fn client_applies_shared_world_and_never_rolls_its_own_events() {
        let mut app = app();
        {
            let mut lan = app.world.resource_mut::<LanMultiplayer>();
            lan.is_active = true;
            lan.is_host = false;
        }
        let frame = WorldFrame {
            flare_lights: vec![],
            creative: false,
            quests_disabled: false,
            tick: 1,
            clock: 1.0,
            body: WorldBody::Moon,
            seed: WorldBody::Moon.seed(),
            campaign_route: true,
            event: LunarEventKind::SolarFlareStorm,
            timer: 100.0,
            duration: 20.0,
            sun: 0.4,
            relay: 0.7,
            station: [0.0; 3],
            tiles: vec![],
            drops: vec![],
            meteors: vec![],
            rovers: vec![],
            surveyed: 7,
            surveys: [7, 0, 0],
            flags: [true; 9],
            blocks: 12,
            storage: [3; 11],
            water_job: None,
            meal_job: None,
            waypoints: vec![],
            campaign: Some(crate::campaign::Campaign::default()),
            science: 54.0,
            transmitted: 12.0,
        };
        app.world
            .resource_mut::<LanMultiplayer>()
            .world_frames
            .push_back(frame);
        app.world.run_system_once(apply_frames);
        app.world
            .run_system_once(crate::physics_gameplay::update_survival_base_and_events);
        assert_eq!(
            app.world.resource::<LunarEventManager>().active_event,
            LunarEventKind::SolarFlareStorm
        );
        assert_eq!(app.world.resource::<LunarEventManager>().timer_sec, 100.0);
        assert_eq!(app.world.resource::<LunarMap>().surveyed, 7);
        assert_eq!(app.world.resource::<LunarMap>().storage, [3; 11]);
        assert!(app.world.resource::<SurvivalStats>().built_habitat_once);
    }
    #[test]
    fn high_five_requires_acceptance_from_the_other_nearby_player() {
        let mut app = app();
        let position = app
            .world
            .query::<&AstronautPlayer>()
            .single(&app.world)
            .iso_pos;
        remote(&mut app, 99, position + Vec3::X);
        let host = app.world.resource::<LanMultiplayer>().local_peer_id;
        crate::emotes::request(&mut app.world, host, 99, 1);
        assert!(app.world.resource::<LanMultiplayer>().coop.is_none());
        crate::emotes::request(&mut app.world, 99, host, 1);
        let lan = app.world.resource::<LanMultiplayer>();
        assert!(lan.invite.is_none());
        let c = lan.coop.as_ref().unwrap();
        assert_eq!((c.a, c.b, c.kind), (99, host, 1));
        assert!((c.start - 1.4).abs() < 0.01);
    }
    #[test]
    fn storms_damage_exposed_suits_on_every_world_and_always_end_with_a_break() {
        let mut app = app();
        for body in WorldBody::ALL {
            app.world.resource_mut::<LunarMap>().body = body;
            {
                let mut p = app.world.query::<&mut AstronautPlayer>();
                let mut p = p.single_mut(&mut app.world);
                p.iso_pos = Vec3::new(8.0, 8.0, 0.0);
                p.inside_building = false;
                p.mounted_rover = None;
            }
            {
                let mut e = app.world.resource_mut::<LunarEventManager>();
                e.active_event = LunarEventKind::SolarFlareStorm;
                e.timer_sec = 0.0;
                e.duration_sec = 20.0;
            }
            app.world.resource_mut::<SurvivalStats>().suit_integrity = 100.0;
            app.world
                .run_system_once(crate::physics_gameplay::update_survival_base_and_events);
            assert!(
                app.world.resource::<SurvivalStats>().suit_integrity < 100.0,
                "{:?}",
                body
            );
            app.world.resource_mut::<LunarEventManager>().timer_sec = 21.0;
            app.world
                .run_system_once(crate::physics_gameplay::update_survival_base_and_events);
            let events = app.world.resource::<LunarEventManager>();
            assert_eq!(events.active_event, LunarEventKind::Nominal);
            assert!(events.duration_sec >= 90.0);
        }
    }

    #[test]
    fn flare_cooldown_and_crew_switch_preserve_player_state() {
        let mut app = app();
        for _ in 0..2 {
            app.world
                .resource_mut::<TouchAndUiInput>()
                .cmds
                .push(SurfaceUiCmd::ThrowSurveyFlare);
            app.world.run_system_once(
                crate::physics_gameplay::handle_mining_building_crafting_and_throwing,
            );
        }
        assert_eq!(
            app.world
                .query::<&ThrownObject>()
                .iter(&app.world)
                .filter(|o| o.is_flare)
                .count(),
            1
        );
        app.world
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs(7));
        app.world
            .resource_mut::<TouchAndUiInput>()
            .cmds
            .push(SurfaceUiCmd::ThrowSurveyFlare);
        app.world
            .run_system_once(crate::physics_gameplay::handle_mining_building_crafting_and_throwing);
        assert_eq!(
            app.world
                .query::<&ThrownObject>()
                .iter(&app.world)
                .filter(|o| o.is_flare)
                .count(),
            2
        );
        let mut projectiles = app.world.query::<&mut ThrownObject>();
        for mut projectile in projectiles.iter_mut(&mut app.world) {
            projectile.iso_pos = Vec3::new(8.0, 8.0, -100.0);
            projectile.iso_vel = Vec3::ZERO;
        }
        app.world
            .run_system_once(crate::physics_gameplay::update_projectiles_and_particles);
        let now = app.world.resource::<Time>().elapsed_seconds();
        let map = app.world.resource::<LunarMap>();
        let lit_tiles = (4..=12)
            .flat_map(|x| (4..=12).map(move |y| (x, y)))
            .filter(|&(x, y)| map.flare_lit(x, y, now + 10.99))
            .count();
        assert_eq!(lit_tiles, 25);
        assert!(!map.flare_lit(8, 8, now + 11.0));
        let original = app.world.resource::<MissionConfig>().callsign.clone();
        app.world
            .query::<&mut AstronautPlayer>()
            .single_mut(&mut app.world)
            .inside_building = true;
        app.world
            .resource_mut::<TouchAndUiInput>()
            .cmds
            .push(SurfaceUiCmd::SwitchCrewMember);
        app.world
            .run_system_once(crate::physics_gameplay::update_coop_and_lan_peers);
        assert_eq!(app.world.resource::<MissionConfig>().callsign, original);
        assert!(
            app.world
                .query::<&AstronautPlayer>()
                .single(&app.world)
                .inside_building
        );
        {
            let mut q = app.world.query::<&mut AstronautPlayer>();
            let mut p = q.single_mut(&mut app.world);
            p.inside_building = false;
            p.grounded = true;
        }
        let cargo = app.world.resource::<SurvivalStats>().cargo_counts();
        app.world
            .resource_mut::<TouchAndUiInput>()
            .cmds
            .push(SurfaceUiCmd::SwitchCrewMember);
        app.world
            .run_system_once(crate::physics_gameplay::update_coop_and_lan_peers);
        assert_ne!(app.world.resource::<MissionConfig>().callsign, original);
        assert_eq!(app.world.resource::<SurvivalStats>().cargo_counts(), cargo);
        let p = app.world.query::<&AstronautPlayer>().single(&app.world);
        assert!(!app
            .world
            .resource::<LunarMap>()
            .blocks_movement(p.iso_pos.x, p.iso_pos.y));
    }
    #[test]
    fn airlock_is_not_a_workstation_and_overweight_rewards_remain_on_ground() {
        let mut app = app();
        {
            let mut map = app.world.resource_mut::<LunarMap>();
            for (pos, kind) in [
                ((2, 0), StructureKind::Airlock),
                ((3, 0), StructureKind::HabitatModule),
                ((3, 1), StructureKind::OxygenElectrolyzer),
                ((4, 0), StructureKind::HydroponicsDome),
            ] {
                let c = map.cells.get_mut(&pos).unwrap();
                c.structure = Some(kind);
                c.structure_hp = 100.0;
                map.networks.insert(
                    pos,
                    crate::base::BaseStatus {
                        powered: true,
                        greenhouse: true,
                        ..default()
                    },
                );
            }
            assert!(!crate::base::SupplyRecipe::Water.can_run(&map, (2, 0)));
            assert!(!crate::base::SupplyRecipe::Meal.can_run(&map, (2, 0)));
            assert!(crate::base::SupplyRecipe::Water.can_run(&map, (3, 0)));
            assert!(crate::base::SupplyRecipe::Meal.can_run(&map, (3, 0)));
        }
        app.world.resource_mut::<SurvivalStats>().regolith = 200;
        app.world.run_system_once(crate::mineral::enforce_capacity);
        assert!(app.world.resource::<SurvivalStats>().cargo_weight() <= 35.001);
        let dropped: u32 = app
            .world
            .query::<&ResourcePickup>()
            .iter(&app.world)
            .filter(|i| i.kind == 0)
            .map(|i| i.amount)
            .sum();
        assert_eq!(
            dropped + app.world.resource::<SurvivalStats>().regolith,
            200
        );
    }

    #[test]
    fn simultaneous_planets_and_host_travel_preserve_the_other_players_world() {
        let mut app = app();
        app.init_resource::<crate::dimensions::ServerDimensions>()
            .init_resource::<crate::dimensions::SurveyLedger>();
        remote(&mut app, 99, Vec3::new(8.0, 8.0, 0.0));
        remote(&mut app, 100, Vec3::new(8.0, 8.0, 0.0));
        app.world
            .resource_mut::<LanMultiplayer>()
            .peers
            .iter_mut()
            .find(|p| p.peer_id == 100)
            .unwrap()
            .body = WorldBody::Mars;
        let before = app.world.resource::<LunarMap>().cells[&(8, 8)].resource_yield;
        app.world
            .resource_mut::<LanMultiplayer>()
            .actions
            .push_back((
                100,
                WorldBody::Mars,
                Action::Command(SurfaceUiCmd::ServerMine(8, 8)),
            ));
        crate::dimensions::tick(&mut app.world);
        assert_eq!(app.world.resource::<LunarMap>().body, WorldBody::Moon);
        assert_eq!(
            app.world.resource::<LunarMap>().cells[&(8, 8)].resource_yield,
            before
        );
        let remaining = {
            let dimensions = app.world.resource::<crate::dimensions::ServerDimensions>();
            let map = dimensions.worlds[&WorldBody::Mars].resource::<LunarMap>();
            assert!(
                map.cells[&(8, 8)].resource_yield
                    < crate::mineral::initial_yield(map.cells[&(8, 8)].kind)
            );
            map.cells[&(8, 8)].resource_yield
        };
        {
            let mut dimensions = app
                .world
                .resource_mut::<crate::dimensions::ServerDimensions>();
            let mars = dimensions.worlds.get_mut(&WorldBody::Mars).unwrap();
            let mut e = mars.resource_mut::<LunarEventManager>();
            e.active_event = LunarEventKind::SolarFlareStorm;
            e.timer_sec = 0.0;
            e.duration_sec = 30.0;
        }
        app.world
            .resource_mut::<crate::planets::Expedition>()
            .request(WorldBody::Mars);
        app.world
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs(4));
        app.world
            .run_system_once(crate::planets::travel_between_worlds);
        assert_eq!(app.world.resource::<LunarMap>().body, WorldBody::Mars);
        assert_eq!(
            app.world.resource::<LunarMap>().cells[&(8, 8)].resource_yield,
            remaining
        );
        assert_eq!(
            app.world.resource::<LunarEventManager>().active_event,
            LunarEventKind::SolarFlareStorm
        );
        crate::dimensions::tick(&mut app.world);
        let dimensions = app.world.resource::<crate::dimensions::ServerDimensions>();
        assert_eq!(
            dimensions.worlds[&WorldBody::Moon]
                .resource::<LunarMap>()
                .cells[&(8, 8)]
                .resource_yield,
            before
        );
        assert_eq!(
            app.world
                .resource::<LanMultiplayer>()
                .peers
                .iter()
                .find(|p| p.peer_id == 99)
                .unwrap()
                .body,
            WorldBody::Moon
        );
    }
    #[test]
    fn hosting_player_can_board_and_leave_with_the_keyboard() {
        use bevy::input::{
            keyboard::{Key, KeyboardInput},
            ButtonState, InputPlugin,
        };

        let mut app = app();
        app.add_plugins(InputPlugin)
            .add_event::<bevy::window::WindowFocused>()
            .init_resource::<crate::controls::ControlsMenu>()
            .insert_resource(State::new(AppState::LunarSurface))
            .add_systems(
                Update,
                (
                    crate::controls::input,
                    crate::physics_gameplay::update_astronaut_and_rover_physics,
                )
                    .chain(),
            );
        app.world
            .insert_resource(LanMultiplayer::with_test_listener(
                std::net::TcpListener::bind("127.0.0.1:0").unwrap(),
            ));
        let host = app.world.resource::<LanMultiplayer>().local_peer_id;
        let (rover, origin) = {
            let mut query = app.world.query::<(Entity, &RoverVehicle)>();
            let (entity, vehicle) = query.single(&app.world);
            (entity, vehicle.iso_pos)
        };
        app.world
            .query::<&mut AstronautPlayer>()
            .single_mut(&mut app.world)
            .iso_pos = origin;
        let frame = |app: &mut App, input| {
            if let Some(state) = input {
                app.world.send_event(KeyboardInput {
                    key_code: KeyCode::KeyE,
                    logical_key: Key::Character("e".into()),
                    state,
                    window: Entity::PLACEHOLDER,
                });
            }
            app.world.run_schedule(PreUpdate);
            app.world.run_schedule(Update);
            serve_actions(&mut app.world);
            crate::vehicles::apply_replies(&mut app.world);
        };
        frame(&mut app, Some(ButtonState::Pressed));
        let passenger = host ^ 1;
        remote(&mut app, passenger, origin);
        crate::vehicles::board(&mut app.world, passenger, Some(rover.to_bits()));
        frame(&mut app, None);
        let player = app.world.query::<&AstronautPlayer>().single(&app.world);
        assert_eq!(player.mounted_rover, Some(rover));
        assert!(!player.is_passenger);
        let key = (WorldBody::Moon, rover.to_bits());
        let seats = &app.world.resource::<Authority>().seats[&key];
        assert_eq!(seats.driver, Some(host));
        assert_eq!(seats.passengers, vec![passenger]);
        frame(&mut app, Some(ButtonState::Released));
        frame(&mut app, Some(ButtonState::Pressed));
        assert_eq!(
            app.world
                .query::<&AstronautPlayer>()
                .single(&app.world)
                .mounted_rover,
            None
        );
        assert_eq!(
            app.world.resource::<Authority>().seats[&key].driver,
            Some(passenger)
        );
    }

    #[test]
    fn two_players_share_a_rover_and_only_the_driver_can_control_it() {
        let mut app = app();
        let rover_id = app
            .world
            .query_filtered::<Entity, With<RoverVehicle>>()
            .single(&app.world);
        let origin = app.world.get::<RoverVehicle>(rover_id).unwrap().iso_pos;
        remote(&mut app, 99, origin + Vec3::X);
        remote(&mut app, 100, origin + Vec3::Y);
        crate::vehicles::board(&mut app.world, 99, Some(rover_id.to_bits()));
        crate::vehicles::board(&mut app.world, 100, Some(rover_id.to_bits()));
        let seats =
            &app.world.resource::<Authority>().seats[&(WorldBody::Moon, rover_id.to_bits())];
        assert_eq!(seats.driver, Some(99));
        assert_eq!(seats.passengers, vec![100]);
        {
            let mut lan = app.world.resource_mut::<LanMultiplayer>();
            lan.peers
                .iter_mut()
                .find(|p| p.peer_id == 99)
                .unwrap()
                .vehicle = Some(crate::net_lan::VehiclePose {
                id: rover_id.to_bits(),
                pos: (origin + Vec3::X * 0.4).to_array(),
                vel: [1.0, 0.0],
                heading: 0.0,
                driving: true,
            });
            lan.peers
                .iter_mut()
                .find(|p| p.peer_id == 100)
                .unwrap()
                .vehicle = Some(crate::net_lan::VehiclePose {
                id: rover_id.to_bits(),
                pos: (origin - Vec3::X * 2.0).to_array(),
                vel: [-10.0, 0.0],
                heading: 3.0,
                driving: false,
            });
        }
        synchronize_vehicles(&mut app.world);
        assert!(
            (app.world.get::<RoverVehicle>(rover_id).unwrap().iso_pos.x - origin.x - 0.4).abs()
                < 0.01
        );
        crate::vehicles::board(&mut app.world, 99, None);
        assert_eq!(
            app.world.resource::<Authority>().seats[&(WorldBody::Moon, rover_id.to_bits())].driver,
            Some(100)
        );
        let pos = app.world.get::<RoverVehicle>(rover_id).unwrap().iso_pos;
        {
            let mut q = app.world.query::<&mut AstronautPlayer>();
            let mut p = q.single_mut(&mut app.world);
            p.mounted_rover = Some(rover_id);
            p.is_passenger = true;
        }
        app.world
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyW);
        app.world
            .run_system_once(crate::physics_gameplay::update_astronaut_and_rover_physics);
        assert_eq!(
            app.world.get::<RoverVehicle>(rover_id).unwrap().iso_pos,
            pos
        );
        assert_eq!(
            app.world
                .query::<&AstronautPlayer>()
                .single(&app.world)
                .iso_pos,
            pos
        );
    }

    fn pump_apps(host: &mut App, client: &mut App, count: usize) {
        for _ in 0..count {
            for app in [&mut *host, &mut *client] {
                app.world
                    .resource_mut::<Time>()
                    .advance_by(std::time::Duration::from_millis(50));
                network_tick(&mut app.world);
            }
            crate::dimensions::tick(&mut host.world);
            serve_actions(&mut host.world);
            host.world.run_system_once(crate::dimensions::record_active);
            host.world.run_system_once(publish_world);
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
    }
    #[test]
    fn tcp_worlds_and_passenger_roundtrip() {
        let mut host = app();
        let mut client = app();
        for app in [&mut host, &mut client] {
            app.init_resource::<crate::dimensions::ServerDimensions>()
                .init_resource::<crate::dimensions::SurveyLedger>();
            app.world
                .insert_resource(State::new(AppState::LunarSurface));
        }
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap().to_string();
        host.world
            .insert_resource(LanMultiplayer::with_test_listener(listener));
        {
            let mut lan = client.world.resource_mut::<LanMultiplayer>();
            lan.target_addr = address;
            lan.join_lan();
        }
        pump_apps(&mut host, &mut client, 60);
        assert!(client.world.resource::<LanMultiplayer>().is_active);
        client
            .world
            .resource_mut::<crate::planets::Expedition>()
            .request(WorldBody::Mars);
        client
            .world
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs(4));
        client
            .world
            .run_system_once(crate::planets::travel_between_worlds);
        pump_apps(&mut host, &mut client, 60);
        assert_eq!(host.world.resource::<LunarMap>().body, WorldBody::Moon);
        assert_eq!(client.world.resource::<LunarMap>().body, WorldBody::Mars);
        let (vehicle, net, pos) = {
            let mut q = client.world.query::<(Entity, &RoverVehicle, &Replica)>();
            let (e, r, n) = q.iter(&client.world).next().unwrap();
            (e, n.0, r.iso_pos)
        };
        client
            .world
            .query::<&mut AstronautPlayer>()
            .single_mut(&mut client.world)
            .iso_pos = pos + Vec3::X;
        pump_apps(&mut host, &mut client, 4);
        client
            .world
            .resource_mut::<LanMultiplayer>()
            .request(Action::Board(Some(net)));
        pump_apps(&mut host, &mut client, 20);
        let p = client
            .world
            .query::<&AstronautPlayer>()
            .single(&client.world);
        assert_eq!(p.mounted_rover, Some(vehicle));
        assert!(!p.is_passenger);
        host.world
            .resource_mut::<crate::planets::Expedition>()
            .request(WorldBody::Mars);
        host.world
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs(4));
        host.world
            .run_system_once(crate::planets::travel_between_worlds);
        pump_apps(&mut host, &mut client, 20);
        let entity = find_rover(&mut host.world, net).unwrap();
        let origin = host.world.get::<RoverVehicle>(entity).unwrap().iso_pos;
        host.world
            .query::<&mut AstronautPlayer>()
            .single_mut(&mut host.world)
            .iso_pos = origin + Vec3::Y;
        host.world
            .resource_mut::<LanMultiplayer>()
            .request(Action::Board(Some(net)));
        pump_apps(&mut host, &mut client, 15);
        let p = host.world.query::<&AstronautPlayer>().single(&host.world);
        assert_eq!(p.mounted_rover, Some(entity));
        assert!(p.is_passenger);
        assert_eq!(
            client
                .world
                .query::<&AstronautPlayer>()
                .single(&client.world)
                .mounted_rover,
            Some(vehicle)
        );
        let before = client.world.get::<RoverVehicle>(vehicle).unwrap().iso_pos;
        client
            .world
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyW);
        client
            .world
            .run_system_once(crate::physics_gameplay::update_astronaut_and_rover_physics);
        client
            .world
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(KeyCode::KeyW);
        pump_apps(&mut host, &mut client, 12);
        assert!(
            host.world
                .get::<RoverVehicle>(entity)
                .unwrap()
                .iso_pos
                .distance(before)
                > 0.001
        );
        assert!(host
            .world
            .resource::<crate::dimensions::ServerDimensions>()
            .worlds
            .contains_key(&WorldBody::Moon));
        drop(host);
        for _ in 0..12 {
            network_tick(&mut client.world);
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        assert!(!client.world.resource::<LanMultiplayer>().is_active);
        assert_eq!(
            client
                .world
                .query::<&AstronautPlayer>()
                .iter(&client.world)
                .count(),
            0
        );
        assert_eq!(
            client.world.resource::<NextState<AppState>>().0,
            Some(AppState::Settings)
        );
    }
    #[test]
    fn four_unique_surveys_and_sparse_deterministic_deposits_in_each_world() {
        let mut signatures = Vec::new();
        let mut points = std::collections::HashSet::new();
        for body in WorldBody::ALL {
            let mut map = LunarMap {
                body,
                world_seed: body.seed(),
                ..default()
            };
            assert_eq!(map.survey_points().len(), 4);
            for p in map.survey_points() {
                assert!(points.insert((p.pos.x, p.pos.y)));
            }
            for sx in -5..=5 {
                for sy in -5..=5 {
                    map.generate_sector(sx, sy, MissionScenario::PolsaKopernikBase);
                }
            }
            let count = map
                .cells
                .values()
                .filter(|c| crate::mineral::deposit_index(c.kind) > 0)
                .count();
            assert!(
                count * 100 < map.cells.len() * 5,
                "too many deposits on {:?}",
                body
            );
            let structures = map.cells.values().filter(|c| c.structure.is_some()).count();
            assert!(structures * 100 < map.cells.len());
            signatures.push((map.cells[&(19, 19)].kind, map.cells[&(19, 19)].elevation));
        }
        assert_ne!(signatures[0], signatures[1]);
        assert_ne!(signatures[1], signatures[2]);
        for tool in ToolTier::ALL {
            let (a, b, c, d) = tool.craft_cost();
            assert!(a as f32 * 0.25 + b as f32 * 0.5 + c as f32 * 0.6 + d as f32 * 0.35 <= 35.0);
        }
    }
}

pub(crate) fn find_rover(world: &mut World, id: u64) -> Option<Entity> {
    let mut q = world.query::<(Entity, &RoverVehicle, Option<&Replica>)>();
    q.iter(world)
        .find(|(e, _, net)| net.map_or(e.to_bits(), |n| n.0) == id)
        .map(|v| v.0)
}
pub(crate) fn find_pickup(world: &mut World, id: u64) -> Option<Entity> {
    let mut q = world.query::<(Entity, &ResourcePickup, Option<&Replica>)>();
    q.iter(world)
        .find(|(e, _, net)| net.map_or(e.to_bits(), |n| n.0) == id)
        .map(|v| v.0)
}
