//! Independent authoritative planet worlds. Only the host's current planet is rendered.
use crate::isometric::ProceduralAssets;
use crate::net_lan::LanMultiplayer;
use crate::net_world::{Authority, Replica};
use crate::physics_gameplay::LunarEventManager;
use crate::planets::{SavedPlanet, WorldBody};
use crate::types::*;
use crate::world::*;
use bevy::asset::{AssetApp, AssetPlugin};
use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use std::collections::HashMap;

#[derive(Resource, Default)]
pub struct ServerDimensions {
    pub(crate) worlds: HashMap<WorldBody, World>,
    pub(crate) pending: HashMap<WorldBody, SavedPlanet>,
}
#[derive(Resource, Clone, Default)]
pub struct SurveyLedger {
    pub surveys: [u8; 3],
    pub moon_link: bool,
}

impl ServerDimensions {
    pub(crate) fn take_saved(&mut self, body: WorldBody) -> Option<SavedPlanet> {
        if let Some(saved) = self.pending.remove(&body) {
            return Some(saved);
        }
        self.worlds
            .remove(&body)
            .map(|mut world| capture(&mut world))
    }
}

fn capture(world: &mut World) -> SavedPlanet {
    let mut map = world.remove_resource::<LunarMap>().unwrap();
    for cell in map.cells.values_mut() {
        cell.tile_entity = None;
        cell.feature_entity = None;
    }
    let rovers: Vec<_> = {
        let mut q = world.query::<(Entity, &RoverVehicle, Option<&Replica>)>();
        q.iter(world)
            .map(|(e, r, n)| (n.map_or(e.to_bits(), |n| n.0), r.clone()))
            .collect()
    };
    let drops: Vec<_> = {
        let mut q = world.query::<(Entity, &crate::mineral::ResourcePickup, Option<&Replica>)>();
        q.iter(world)
            .map(|(e, r, n)| (n.map_or(e.to_bits(), |n| n.0), r.clone()))
            .collect()
    };
    let meteors = {
        let mut q = world.query::<&ThrownObject>();
        q.iter(world).cloned().collect()
    };
    SavedPlanet {
        map,
        rover_ids: rovers.iter().map(|r| r.0).collect(),
        rovers: rovers.into_iter().map(|r| r.1).collect(),
        drop_ids: drops.iter().map(|r| r.0).collect(),
        drops: drops.into_iter().map(|r| r.1).collect(),
        meteors,
        events: world.resource::<LunarEventManager>().clone(),
        base_charge: world.resource::<SurvivalStats>().base_battery_wh,
    }
}

fn build_world(
    body: WorldBody,
    saved: Option<SavedPlanet>,
    assets: ProceduralAssets,
    config: MissionConfig,
    settings: GameSettings,
    audio: crate::audio::GameAudio,
    campaign: bool,
) -> World {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Image>()
        .init_asset::<bevy::render::render_resource::Shader>()
        .add_plugins(bevy::gizmos::GizmoPlugin)
        .insert_resource(assets)
        .insert_resource(config.clone())
        .insert_resource(settings)
        .insert_resource(audio)
        .init_resource::<LanMultiplayer>()
        .init_resource::<Authority>()
        .init_resource::<SurveyLedger>()
        .init_resource::<crate::planets::Expedition>()
        .init_resource::<crate::celestial::SolarSystemState>()
        .init_resource::<TouchAndUiInput>()
        .init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<ButtonInput<MouseButton>>()
        .init_state::<AppState>()
        .insert_resource(SurvivalStats::from_config(&config));
    let fresh = saved.is_none();
    let saved = saved.unwrap_or_else(|| SavedPlanet {
        map: LunarMap {
            body,
            campaign_route: campaign,
            ..default()
        },
        rovers: vec![],
        rover_ids: vec![],
        drops: vec![],
        drop_ids: vec![],
        meteors: vec![],
        events: LunarEventManager {
            duration_sec: 120.0,
            ..default()
        },
        base_charge: 720.0,
    });
    app.insert_resource(saved.map).insert_resource(saved.events);
    app.world.resource_mut::<SurvivalStats>().base_battery_wh = saved.base_charge;
    app.world.run_system_once(
        move |mut commands: Commands,
              assets: Res<ProceduralAssets>,
              config: Res<MissionConfig>,
              lan: Res<LanMultiplayer>,
              mut map: ResMut<LunarMap>| {
            spawn_planet_surface(&mut commands, &assets, &config, &lan, &mut map, fresh);
            for (index, rover) in saved.rovers.iter().cloned().enumerate() {
                let entity = spawn_rover_entity(&mut commands, &assets, rover.kind, rover.iso_pos);
                commands.entity(entity).insert(rover);
                if let Some(id) = saved.rover_ids.get(index) {
                    commands.entity(entity).insert(Replica(*id));
                }
            }
            for (index, drop) in saved.drops.iter().cloned().enumerate() {
                let entity = crate::mineral::spawn_saved_pickup(&mut commands, &assets, drop);
                if let Some(id) = saved.drop_ids.get(index) {
                    commands.entity(entity).insert(Replica(*id));
                }
            }
            for meteor in saved.meteors.iter().cloned() {
                commands.spawn((SurfaceEntity, meteor));
            }
        },
    );
    std::mem::take(&mut app.world)
}

fn copy_shared(source: &SurvivalStats, target: &mut SurvivalStats) {
    target.campaign = source.campaign.clone();
    target.generated_science_gb = source.generated_science_gb;
    target.transmitted_science_gb = source.transmitted_science_gb;
    target.blocks_mined = source.blocks_mined;
    crate::net_world::apply_flags(target, crate::net_world::progress_flags(source));
}

pub fn record_active(
    map: Res<LunarMap>,
    mut ledger: ResMut<SurveyLedger>,
    mut expedition: ResMut<crate::planets::Expedition>,
) {
    ledger.surveys[map.body.portrait_index()] = map.surveyed;
    if map.body == WorldBody::Moon {
        ledger.moon_link = map
            .networks
            .get(&(3, 0))
            .is_some_and(|b| b.powered && b.relay);
    }
    expedition.update_surveys(ledger.surveys);
}

pub fn tick(world: &mut World) {
    if !world.resource::<LanMultiplayer>().is_active
        || !world.resource::<LanMultiplayer>().is_host
        || !world.contains_resource::<SurvivalStats>()
    {
        return;
    }
    if !world.contains_resource::<ServerDimensions>() {
        world.init_resource::<ServerDimensions>();
    }
    if !world.contains_resource::<SurveyLedger>() {
        world.init_resource::<SurveyLedger>();
    }
    let active = world.resource::<LunarMap>().body;
    let assets = world.resource::<ProceduralAssets>().clone();
    let config = world.resource::<MissionConfig>().clone();
    let settings = world.resource::<GameSettings>().clone();
    let audio = world.resource::<crate::audio::GameAudio>().clone();
    let campaign = world.resource::<LunarMap>().campaign_route;
    let needed: Vec<_> = world
        .resource::<LanMultiplayer>()
        .peers
        .iter()
        .filter(|p| !p.is_ai_bot && p.body != active)
        .map(|p| p.body)
        .collect();
    let mut dimensions = world.remove_resource::<ServerDimensions>().unwrap();
    let pending: Vec<_> = dimensions.pending.drain().collect();
    for (body, saved) in pending {
        if body != active {
            dimensions.worlds.insert(
                body,
                build_world(
                    body,
                    Some(saved),
                    assets.clone(),
                    config.clone(),
                    settings.clone(),
                    audio.clone(),
                    campaign,
                ),
            );
        }
    }
    for body in needed {
        dimensions.worlds.entry(body).or_insert_with(|| {
            build_world(
                body,
                None,
                assets.clone(),
                config.clone(),
                settings.clone(),
                audio.clone(),
                campaign,
            )
        });
    }
    for (&body, background) in &mut dimensions.worlds {
        if body == active {
            continue;
        }
        let time = world.resource::<Time>().clone();
        background.insert_resource(time);
        let solar = world.resource::<crate::celestial::SolarSystemState>();
        {
            let mut s = background.resource_mut::<crate::celestial::SolarSystemState>();
            s.lunar_sun_angle_rad = solar.lunar_sun_angle_rad;
            s.relay_orbit_angle_rad = solar.relay_orbit_angle_rad;
        }
        copy_shared(
            world.resource::<SurvivalStats>(),
            &mut background.resource_mut::<SurvivalStats>(),
        );
        {
            let mut stats = background.resource_mut::<SurvivalStats>();
            stats.health = 100.0;
            stats.oxygen = stats.max_oxygen;
        }
        let pos = world
            .resource::<LanMultiplayer>()
            .peers
            .iter()
            .find(|p| !p.is_ai_bot && p.body == body)
            .and_then(|p| p.samples.back().map(|s| s.1))
            .unwrap_or(Vec3::new(-2.0, 0.0, 0.0));
        {
            let mut q = background.query::<&mut AstronautPlayer>();
            let mut proxy = q.single_mut(background);
            proxy.iso_pos = pos;
            proxy.inside_building = false;
            proxy.mounted_rover = None;
        }
        background.insert_resource(world.remove_resource::<LanMultiplayer>().unwrap());
        background.insert_resource(world.remove_resource::<Authority>().unwrap());
        background.insert_resource(world.resource::<SurveyLedger>().clone());
        crate::mobile_player::update(background);
        crate::net_world::synchronize_vehicles(background);
        crate::net_world::serve_actions(background);
        background.run_system_once(crate::mineral::update_background_pickups);
        background.run_system_once(crate::physics_gameplay::update_projectiles_and_particles);
        background.run_system_once(crate::physics_gameplay::update_survival_base_and_events);
        background.run_system_once(record_active);
        background.run_system_once(crate::net_world::publish_world);
        background.run_schedule(bevy::app::Last);
        copy_shared(
            background.resource::<SurvivalStats>(),
            &mut world.resource_mut::<SurvivalStats>(),
        );
        world.insert_resource(background.remove_resource::<LanMultiplayer>().unwrap());
        world.insert_resource(background.remove_resource::<Authority>().unwrap());
        world.insert_resource(background.resource::<SurveyLedger>().clone());
    }
    world.insert_resource(dimensions);
    world.run_system_once(record_active);
}
