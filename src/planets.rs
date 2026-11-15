use bevy::prelude::*;
use std::collections::HashMap;

use crate::isometric::ProceduralAssets;
use crate::net_lan::LanMultiplayer;
use crate::physics_gameplay::LunarEventManager;
use crate::types::{AppState, MissionConfig};
use crate::world::{
    spawn_planet_surface, LunarMap, RoverVehicle, SurfaceEntity, SurvivalStats, TouchAndUiInput,
};

pub fn survey_surface(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut map: ResMut<LunarMap>,
    player: Query<&crate::world::AstronautPlayer>,
    mut stats: ResMut<SurvivalStats>,
    mut gizmos: Gizmos,
    settings: Res<crate::types::GameSettings>,
    mut next: ResMut<NextState<AppState>>,
    mut lan: Option<ResMut<LanMultiplayer>>,
) {
    let Ok(player) = player.get_single() else {
        return;
    };
    let is_campaign = stats.campaign.is_some();
    let ready_to_scan = !map.any_modal_open()
        && !map.piloting_station
        && !player.inside_building
        && player.mounted_rover.is_none();
    let requested = keys.just_pressed(KeyCode::KeyP)
        || stats.campaign.as_ref().is_some_and(|c| c.action_requested);
    let campaign_ready = stats.campaign.as_ref().is_none_or(|c| {
        matches!(
            c.quest,
            crate::campaign::Quest::Moon
                | crate::campaign::Quest::Mars
                | crate::campaign::Quest::Venus
        ) && c.quest.target_body() == Some(map.body)
    });
    let points = map.survey_points();
    if let Some(index) = map.scan_target {
        if points
            .get(index)
            .is_none_or(|point| point.pos.as_vec2().distance(player.iso_pos.truncate()) >= 2.1)
        {
            map.scan_target = None;
            map.scan_progress = 0.0;
            stats.status_log =
                crate::locale::text("Pomiar przerwany. Wróć do znacznika i naciśnij P.").into();
        }
    }
    for (index, point) in points.into_iter().enumerate() {
        if map.surveyed & (1 << index) != 0 {
            continue;
        }
        let xy = point.pos.as_vec2();
        let distance = xy.distance(player.iso_pos.truncate());
        if distance > 18.0 {
            continue;
        }
        let elevation = map.get_elevation(xy.x, xy.y);
        let world = crate::isometric::iso_to_world(xy.x, xy.y, elevation);
        let pulse = 1.0 + (time.elapsed_seconds() * 3.0).sin() * 0.16;
        let color = Color::rgb(0.73, 0.55, 1.0);
        let beacon = world + Vec2::Y * 30.0;
        if !settings.hide_hud && !map.quests_disabled {
            gizmos.line_2d(world, beacon, color.with_a(0.62));
            gizmos.rect_2d(
                beacon,
                std::f32::consts::FRAC_PI_4,
                Vec2::splat(10.0 * pulse),
                color,
            );
        }
        if distance < 1.9
            && ready_to_scan
            && requested
            && campaign_ready
            && map.scan_target.is_none()
        {
            if let Some(lan) = lan.as_deref_mut().filter(|l| l.is_active && !l.is_host) {
                lan.send(crate::net_lan::Message::Action(
                    crate::net_world::Action::Scan { index, start: true },
                ));
            }
            map.scan_target = Some(index);
            map.scan_progress = 0.0;
        }
        if map.scan_target == Some(index) && ready_to_scan {
            map.scan_progress = if is_campaign {
                map.scan_progress + time.delta_seconds().min(0.05) / crate::campaign::SCAN_SECONDS
            } else {
                map.scan_progress + time.delta_seconds().min(0.05) / 12.0
            };
            if map.scan_progress < 1.0 {
                continue;
            }
            map.scan_target = None;
            map.scan_progress = 0.0;
            if let Some(lan) = lan.as_deref_mut().filter(|l| l.is_active && !l.is_host) {
                lan.send(crate::net_lan::Message::Action(
                    crate::net_world::Action::Scan {
                        index,
                        start: false,
                    },
                ));
                stats.status_log = format!("{} · {}", point.name, point.fact);
                map.last_discovery = Some(index);
                next.set(AppState::Discovery);
                continue;
            }
            map.surveyed |= 1 << index;
            stats.generated_science_gb += 18.0;
            map.last_discovery = Some(index);
            stats.status_log = format!("{} · {}", point.name, point.fact);
            next.set(AppState::Discovery);
        }
    }
}

pub fn draw_surface_weather(
    time: Res<Time>,
    map: Res<LunarMap>,
    events: Res<LunarEventManager>,
    camera: Query<(&Transform, &OrthographicProjection), With<Camera2d>>,
    mut gizmos: Gizmos,
) {
    use crate::types::LunarEventKind::*;
    let kind = events.active_event;
    if !matches!(kind, DustStorm | SolarFlareStorm) {
        return;
    }
    let Ok((camera, zoom)) = camera.get_single() else {
        return;
    };
    let t = time.elapsed_seconds();
    let (color, speed, trail, spread) = if kind == DustStorm && map.body == WorldBody::Mars {
        (Color::rgba(0.91, 0.51, 0.29, 0.20), 180.0, 94.0, 14.0)
    } else {
        (Color::rgba(1.0, 0.65, 0.2, 0.33), 260.0, 72.0, 24.0)
    };
    for i in 0..if kind == SolarFlareStorm { 116 } else { 78 } {
        let n = i as f32;
        let x = (n * 101.3 + t * speed).rem_euclid(1720.0) - 860.0;
        let y = (n * 63.83 + t * 17.0).rem_euclid(1060.0) - 530.0;
        let point = camera.translation.truncate() + Vec2::new(x, y) * zoom.scale;
        let end = point + Vec2::new(-trail, spread + (n + t * 3.0).sin() * 8.0) * zoom.scale;
        let tint = if kind == SolarFlareStorm {
            match i % 3 {
                0 => Color::rgba(1.0, 0.23, 0.12, 0.38),
                1 => Color::rgba(1.0, 0.55, 0.12, 0.38),
                _ => Color::rgba(1.0, 0.86, 0.25, 0.38),
            }
        } else {
            color
        };
        gizmos.line_2d(point, end, tint);
    }
}

#[derive(Resource, Default)]
pub struct Expedition {
    saved: HashMap<WorldBody, SavedPlanet>,
    known_surveys: [u8; 3],
    pending: Option<WorldBody>,
    elapsed: f32,
    duration: f32,
}

pub(crate) struct SavedPlanet {
    pub(crate) map: LunarMap,
    pub(crate) rovers: Vec<RoverVehicle>,
    pub(crate) rover_ids: Vec<u64>,
    pub(crate) drops: Vec<crate::mineral::ResourcePickup>,
    pub(crate) drop_ids: Vec<u64>,
    pub(crate) meteors: Vec<crate::world::ThrownObject>,
    pub(crate) events: LunarEventManager,
    pub(crate) base_charge: f32,
}

impl Expedition {
    pub fn request(&mut self, destination: WorldBody) {
        self.pending = Some(destination);
        self.elapsed = 0.0;
        self.duration = 3.2;
    }

    pub fn request_campaign(&mut self, destination: WorldBody) {
        self.request(destination);
        self.duration = 9.0;
    }

    pub fn flight(&self) -> Option<(WorldBody, f32)> {
        self.pending
            .map(|body| (body, (self.elapsed / self.duration).clamp(0.0, 1.0)))
    }

    pub fn update_surveys(&mut self, surveys: [u8; 3]) {
        self.known_surveys = surveys;
    }

    pub fn surveyed(&self, body: WorldBody) -> u8 {
        self.saved
            .get(&body)
            .map_or(self.known_surveys[body.portrait_index()], |planet| {
                planet.map.surveyed | self.known_surveys[body.portrait_index()]
            })
    }
}

#[allow(clippy::too_many_arguments)]
pub fn travel_between_worlds(
    mut commands: Commands,
    time: Res<Time>,
    assets: Res<ProceduralAssets>,
    config: Res<MissionConfig>,
    mut lan: ResMut<LanMultiplayer>,
    mut dimensions: Option<ResMut<crate::dimensions::ServerDimensions>>,
    mut expedition: ResMut<Expedition>,
    mut map: ResMut<LunarMap>,
    mut stats: ResMut<SurvivalStats>,
    mut events: ResMut<LunarEventManager>,
    mut touch: ResMut<TouchAndUiInput>,
    mut state: ResMut<NextState<AppState>>,
    surface: Query<Entity, With<SurfaceEntity>>,
    rovers: Query<(Entity, &RoverVehicle, Option<&crate::net_world::Replica>)>,
    drops: Query<(
        Entity,
        &crate::mineral::ResourcePickup,
        Option<&crate::net_world::Replica>,
    )>,
    meteors: Query<&crate::world::ThrownObject>,
) {
    let Some(destination) = expedition.pending else {
        state.set(AppState::LunarSurface);
        return;
    };
    expedition.elapsed += time.delta_seconds();
    if expedition.elapsed < expedition.duration {
        return;
    }
    expedition.pending = None;
    let saved = if lan.is_active && lan.is_host {
        dimensions
            .as_deref_mut()
            .and_then(|d| d.take_saved(destination))
    } else {
        expedition.saved.remove(&destination)
    };
    let first_visit = saved.is_none();
    let incoming = saved.unwrap_or_else(|| SavedPlanet {
        map: LunarMap {
            body: destination,
            campaign_route: map.campaign_route,
            ..default()
        },
        rovers: Vec::new(),
        rover_ids: Vec::new(),
        drops: Vec::new(),
        drop_ids: Vec::new(),
        meteors: Vec::new(),
        events: LunarEventManager {
            duration_sec: 120.0,
            ..default()
        },
        base_charge: 720.0,
    });
    let mut outgoing = std::mem::replace(&mut *map, incoming.map);
    outgoing.piloting_station = false;
    outgoing.station_position = outgoing.station_parked;
    if let Some(marker) = outgoing.waypoints.first_mut() {
        marker.gx = outgoing.station_position.x.round() as i32;
        marker.gy = outgoing.station_position.y.round() as i32;
    }
    for cell in outgoing.cells.values_mut() {
        cell.tile_entity = None;
        cell.feature_entity = None;
    }
    let outgoing_body = outgoing.body;
    let outgoing = SavedPlanet {
        map: outgoing,
        rovers: rovers.iter().map(|(_, r, _)| r.clone()).collect(),
        rover_ids: rovers
            .iter()
            .map(|(id, _, net)| net.map_or(id.to_bits(), |n| n.0))
            .collect(),
        drops: drops.iter().map(|(_, r, _)| r.clone()).collect(),
        drop_ids: drops
            .iter()
            .map(|(id, _, net)| net.map_or(id.to_bits(), |n| n.0))
            .collect(),
        meteors: meteors.iter().cloned().collect(),
        events: events.clone(),
        base_charge: stats.base_battery_wh,
    };
    if lan.is_active && lan.is_host {
        if let Some(dimensions) = dimensions.as_deref_mut() {
            dimensions.pending.insert(outgoing_body, outgoing);
        }
    } else {
        expedition.saved.insert(outgoing_body, outgoing);
    }
    lan.current_body = destination;
    if lan.is_active && !lan.is_host {
        lan.send(crate::net_lan::Message::Action(
            crate::net_world::Action::Resync,
        ));
    }
    for entity in &surface {
        commands.entity(entity).despawn_recursive();
    }
    spawn_planet_surface(&mut commands, &assets, &config, &lan, &mut map, first_visit);
    for (index, rover) in incoming.rovers.into_iter().enumerate() {
        let entity =
            crate::world::spawn_rover_entity(&mut commands, &assets, rover.kind, rover.iso_pos);
        commands.entity(entity).insert(rover);
        if let Some(id) = incoming.rover_ids.get(index) {
            commands
                .entity(entity)
                .insert(crate::net_world::Replica(*id));
        }
    }
    for (index, drop) in incoming.drops.into_iter().enumerate() {
        let entity = crate::mineral::spawn_saved_pickup(&mut commands, &assets, drop);
        if let Some(id) = incoming.drop_ids.get(index) {
            commands
                .entity(entity)
                .insert(crate::net_world::Replica(*id));
        }
    }
    for meteor in incoming.meteors {
        commands.spawn((SurfaceEntity, meteor));
    }
    *events = incoming.events;
    *touch = TouchAndUiInput::default();
    stats.base_battery_wh = incoming.base_charge;
    stats.oxygen = stats.max_oxygen;
    stats.oxygen_use_per_sec = 0.0;
    stats.suit_power = stats.max_suit_power;
    stats.body_temp_c = 37.0;
    if let Some(campaign) = &mut stats.campaign {
        campaign.seconds += expedition.elapsed;
    }
    stats.status_log = moon_base_localize::localized_format!(
        "Cel podróży: {}. Otwórz dziennik [N], by poznać ten świat.",
        destination.name()
    );
    state.set(AppState::LunarSurface);
}

#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize,
)]
pub enum WorldBody {
    #[default]
    Moon,
    Mars,
    Venus,
}

impl WorldBody {
    pub const ALL: [Self; 3] = [Self::Moon, Self::Mars, Self::Venus];

    pub fn portrait_index(self) -> usize {
        match self {
            Self::Moon => 0,
            Self::Mars => 1,
            Self::Venus => 2,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Moon => crate::locale::text("Księżyc"),
            Self::Mars => crate::locale::text("Mars"),
            Self::Venus => crate::locale::text("Wenus"),
        }
    }

    pub fn gravity(self) -> f32 {
        match self {
            Self::Moon => 1.62,
            Self::Mars => 3.73,
            Self::Venus => 8.87,
        }
    }

    pub fn atmosphere(self) -> &'static str {
        match self {
            Self::Moon => crate::locale::text("Prawie próżnia"),
            Self::Mars => crate::locale::text("Rzadkie powietrze, głównie CO₂"),
            Self::Venus => crate::locale::text("Gęsta atmosfera CO₂ · ok. 93 bar"),
        }
    }

    pub fn fact(self) -> &'static str {
        match self {
            Self::Moon => crate::locale::text("Na Księżycu ważysz sześć razy mniej niż na Ziemi. Twoja masa pozostaje taka sama."),
            Self::Mars => crate::locale::text("Rdzawy kolor Marsa pochodzi od tlenków żelaza. Na Marsie jest Olympus Mons, największy wulkan Układu Słonecznego."),
            Self::Venus => crate::locale::text("Wenus ma około 467 °C przy powierzchni. Gęsta atmosfera zatrzymuje ciepło. Panuje tam ciśnienie około 93 razy wyższe niż na Ziemi."),
        }
    }

    pub fn seed(self) -> u64 {
        match self {
            Self::Moon => 0x2026_A87E_5015_C0DE,
            Self::Mars => 0xB0EA_5127,
            Self::Venus => 0x9E7C_0123,
        }
    }

    pub fn ground_tint(self) -> [f32; 3] {
        match self {
            Self::Moon => [1.0, 1.0, 1.0],
            Self::Mars => [1.0, 0.62, 0.43],
            Self::Venus => [0.98, 0.88, 0.75],
        }
    }

    pub fn sunlight(self, sun: f32, dust: bool) -> f32 {
        match self {
            Self::Moon => sun,
            Self::Mars => sun * if dust { 0.20 } else { 0.62 },
            Self::Venus => sun * 0.22,
        }
    }

    pub fn temperature(self, sun: f32, lunar_temperature: f32, _elevation: f32) -> f32 {
        match self {
            Self::Moon => lunar_temperature,
            Self::Mars => -104.0 + 123.0 * sun,

            Self::Venus => 467.0,
        }
    }

    pub fn radiation_multiplier(self) -> f32 {
        match self {
            Self::Moon => 1.0,
            Self::Mars => 0.65,
            Self::Venus => 0.30,
        }
    }

    pub fn survey(self) -> [SurveyPoint; 2] {
        match self {
            Self::Moon => [
                SurveyPoint { pos: IVec2::new(-5, -4), name: crate::locale::text("Ślad uderzenia"), fact: crate::locale::text("Krater powstaje, gdy skała z kosmosu uderza w powierzchnię. Na Księżycu nie ma deszczu ani wiatru, które szybko zatarłyby jego ślad.") },
                SurveyPoint { pos: IVec2::new(6, 4), name: crate::locale::text("Regolit"), fact: crate::locale::text("Regolit to warstwa pyłu i pokruszonych skał. Na Księżycu tworzyły go uderzenia meteoroidów przez miliardy lat.") },
            ],
            Self::Mars => [
                SurveyPoint { pos: IVec2::new(24, -16), name: crate::locale::text("Rdzawy pył"), fact: crate::locale::text("W pyle Marsa jest żelazo. Gdy reaguje z tlenem, powstają związki podobne do rdzy, które barwią powierzchnię na czerwono.") },
                SurveyPoint { pos: IVec2::new(-29, 21), name: crate::locale::text("Zamarznięta woda"), fact: crate::locale::text("Na Marsie jest lód wodny. Cienka atmosfera sprawia, że ciekła woda nie utrzymuje się długo na powierzchni.") },
            ],
            Self::Venus => [
                SurveyPoint { pos: IVec2::new(-22, -27), name: crate::locale::text("Skały wulkaniczne"), fact: crate::locale::text("Powierzchnię Wenus kształtowały wulkany. Sonda Magellan badała ją radarem, który pozwala zajrzeć pod gęste chmury.") },
                SurveyPoint { pos: IVec2::new(31, 18), name: crate::locale::text("Gęsta atmosfera"), fact: crate::locale::text("Atmosfera Wenus to głównie dwutlenek węgla. Silny efekt cieplarniany ogrzewa powierzchnię do około 467 °C, także nocą.") },
            ],
        }
    }
}

#[derive(Clone, Copy)]
pub struct SurveyPoint {
    pub pos: IVec2,
    pub name: &'static str,
    pub fact: &'static str,
}
