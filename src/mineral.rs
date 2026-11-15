//! Finite deposits and physical, recoverable mining cargo.
use crate::isometric::{iso_depth, iso_to_world, ProceduralAssets};
use crate::types::TileKind;
use crate::world::{AstronautPlayer, LunarMap, SurfaceEntity, SurvivalStats};

pub fn update_crystal_deposits(
    map: Res<LunarMap>,
    mut features: Query<(&crate::world::WorldFeatureSpriteMarker, &mut Transform)>,
) {
    for (marker, mut transform) in &mut features {
        let Some(cell) = map.cells.get(&(marker.gx, marker.gy)) else {
            continue;
        };
        if cell.feature != Some(crate::types::WorldFeatureKind::GlowingCrystalCluster) {
            continue;
        }
        let size = deposit_scale(cell, &map, marker.gx, marker.gy, 10.0);
        transform.scale = Vec3::splat(size);
        let ground = iso_to_world(marker.gx as f32, marker.gy as f32, cell.elevation);
        transform.translation.y = ground.y;
    }
}
use bevy::prelude::*;

#[derive(Component)]
pub struct DepositVisual {
    pub gx: i32,
    pub gy: i32,
}

pub fn deposit_index(kind: TileKind) -> usize {
    match kind {
        TileKind::SubsurfaceIceVein => 1,
        TileKind::TitaniumOutcrop => 2,
        TileKind::Helium3Deposit => 3,
        TileKind::RareMeteoriteDeposit => 4,
        _ => 0,
    }
}

pub fn initial_yield(kind: TileKind) -> u32 {
    match kind {
        TileKind::MariaBasalt | TileKind::HighlandRegolith => 30,
        TileKind::CraterRim | TileKind::CraterFloor => 35,
        TileKind::SubsurfaceIceVein => 50,
        TileKind::TitaniumOutcrop => 45,
        TileKind::Helium3Deposit => 40,
        TileKind::RareMeteoriteDeposit => 60,
    }
}

fn deposit_scale(cell: &crate::world::TileCell, map: &LunarMap, x: i32, y: i32, step: f32) -> f32 {
    let preview = if map.mining_target == Some((x, y)) {
        map.mining_progress.clamp(0.0, 1.0) * step.min(cell.resource_yield as f32)
    } else {
        0.0
    };
    ((cell.resource_yield as f32 - preview) / initial_yield(cell.kind) as f32)
        .clamp(0.0, 1.0)
        .sqrt()
}

pub fn update_deposits(
    map: Res<LunarMap>,
    assets: Res<ProceduralAssets>,
    mut deposits: Query<(
        &DepositVisual,
        &mut Transform,
        &mut Visibility,
        &mut Handle<Image>,
    )>,
) {
    for (deposit, mut transform, mut visibility, mut texture) in &mut deposits {
        let Some(cell) = map.cells.get(&(deposit.gx, deposit.gy)) else {
            continue;
        };
        let kind = deposit_index(cell.kind);
        *visibility = if kind == 0
            || cell.resource_yield == 0
            || cell.structure.is_some()
            || cell.feature.is_some()
        {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
        if *visibility == Visibility::Hidden {
            continue;
        }
        if *texture != assets.deposit_tex[kind] {
            *texture = assets.deposit_tex[kind].clone();
        }
        let scale = deposit_scale(cell, &map, deposit.gx, deposit.gy, 5.0);
        transform.scale = Vec3::splat(scale);
        transform.translation.y = 6.0;
    }
}

#[derive(Default)]
pub struct MiningLoot {
    pub regolith: u32,
    pub silicon: u32,
    pub water_ice: u32,
    pub titanium: u32,
    pub helium3: u32,
    pub rare_earth: u32,
    pub raw_samples: u32,
    pub generated_science_gb: f32,
}

#[derive(Component, Clone)]
pub struct ResourcePickup {
    pub(crate) kind: usize,
    pub(crate) amount: u32,
    pub(crate) science: f32,
    pub(crate) position: Vec3,
    pub(crate) velocity: Vec3,
    pub(crate) age: f32,
}

pub const ITEM_NAMES: [&str; 11] = [
    "Regolit",
    "Krzem",
    "Lód wodny",
    "Tytan",
    "Hel-3",
    "Rzadkie minerały",
    "Próbka skały",
    "Racja żywnościowa",
    "Posiłek",
    "Woda",
    "Apteczka",
];

#[derive(Component)]
pub struct PickupNotice {
    pub age: f32,
    pub origin: Vec3,
}

pub fn pickup_notice(
    commands: &mut Commands,
    kind: usize,
    amount: u32,
    position: Vec3,
    offset: f32,
) {
    let screen = iso_to_world(position.x, position.y, position.z);
    let origin = Vec3::new(screen.x, screen.y + 58.0 + offset, 950.0);
    commands.spawn((
        SurfaceEntity,
        PickupNotice { age: 0.0, origin },
        Text2dBundle {
            text: Text::from_section(
                format!("+{} {}", amount, ITEM_NAMES[kind.min(10)]),
                TextStyle {
                    font_size: 19.0,
                    color: Color::rgb(0.67, 1.0, 0.81),
                    ..default()
                },
            )
            .with_justify(JustifyText::Center),
            transform: Transform::from_translation(origin),
            ..default()
        },
    ));
}

pub fn animate_notices(
    mut commands: Commands,
    time: Res<Time>,
    mut notices: Query<(Entity, &mut PickupNotice, &mut Text, &mut Transform)>,
) {
    for (id, mut notice, mut text, mut transform) in &mut notices {
        notice.age += time.delta_seconds();
        if notice.age > 2.0 {
            commands.entity(id).despawn_recursive();
            continue;
        }
        transform.translation = notice.origin + Vec3::Y * notice.age * 22.0;
        text.sections[0]
            .style
            .color
            .set_a(((2.0 - notice.age) / 0.8).clamp(0.0, 1.0));
    }
}

pub fn spawn_saved_pickup(
    commands: &mut Commands,
    assets: &ProceduralAssets,
    item: ResourcePickup,
) -> Entity {
    let icons = [
        &assets.icon_regolith,
        &assets.icon_silicon,
        &assets.icon_ice,
        &assets.icon_titanium,
        &assets.icon_he3,
        &assets.icon_rare,
        &assets.icon_sample,
        &assets.icon_ration,
        &assets.icon_ration,
        &assets.icon_drink,
        &assets.icon_medkit,
    ];
    let screen = iso_to_world(item.position.x, item.position.y, item.position.z);
    let z = iso_depth(item.position.x, item.position.y, item.position.z, 2.0);
    let texture = icons[item.kind.min(icons.len() - 1)].clone();
    commands
        .spawn((
            SurfaceEntity,
            item,
            SpriteBundle {
                texture,
                transform: Transform::from_xyz(screen.x, screen.y, z),
                sprite: Sprite {
                    custom_size: Some(Vec2::splat(18.0)),
                    ..default()
                },
                ..default()
            },
        ))
        .id()
}

pub fn spawn_loot(
    commands: &mut Commands,
    assets: &ProceduralAssets,
    origin: Vec3,
    loot: MiningLoot,
) {
    let amounts = [
        loot.regolith,
        loot.silicon,
        loot.water_ice,
        loot.titanium,
        loot.helium3,
        loot.rare_earth,
        loot.raw_samples,
    ];
    let count = amounts.iter().filter(|&&n| n > 0).count().max(1) as f32;
    for (kind, amount) in amounts.into_iter().enumerate().filter(|(_, n)| *n > 0) {
        let angle = kind as f32 * 2.39996 + origin.x * 0.7 + origin.y * 0.4;
        spawn_saved_pickup(
            commands,
            assets,
            ResourcePickup {
                kind,
                amount,
                science: loot.generated_science_gb / count,
                position: origin,
                velocity: Vec3::new(angle.cos() * 0.45, angle.sin() * 0.45, 1.2),
                age: 0.0,
            },
        );
    }
}

pub fn drop_cargo(
    commands: &mut Commands,
    assets: &ProceduralAssets,
    map: &LunarMap,
    stats: &mut SurvivalStats,
    player: &AstronautPlayer,
    kind: usize,
    all: bool,
) {
    let Some(count) = stats.cargo_counts().get(kind).copied() else {
        return;
    };
    if count == 0 {
        return;
    }
    let origin = player.iso_pos.truncate();
    let floor = map.get_elevation(origin.x, origin.y);
    if player.inside_building
        || player.airlock_anim_timer > 0.0
        || player.mounted_rover.is_some()
        || map.piloting_station
        || (player.iso_pos.z - floor).abs() > 0.9
    {
        stats.status_log =
            crate::locale::text("Wyjdź na zewnątrz i stań na ziemi, żeby wyrzucić przedmioty.")
                .into();
        return;
    }

    // Start outside the automatic pickup radius, including on the phone.
    let landing = (0..8)
        .map(|i| origin + Vec2::from_angle(i as f32 * std::f32::consts::FRAC_PI_4) * 2.0)
        .find(|p| {
            let mid = origin.lerp(*p, 0.5);
            !map.blocks_movement(mid.x, mid.y)
                && !map.blocks_movement(p.x, p.y)
                && !map.is_inside_pressurized(p.x, p.y)
                && (map.get_elevation(p.x, p.y) - floor).abs() <= 0.9
        });
    let Some(landing) = landing else {
        stats.status_log =
            crate::locale::text("Podejdź na otwarty teren, żeby wyrzucić przedmioty.").into();
        return;
    };
    let amount = if all { count } else { 1 };
    stats.change_cargo(kind, -(amount as i32));
    spawn_saved_pickup(
        commands,
        assets,
        ResourcePickup {
            kind,
            amount,
            science: 0.0,
            position: landing.extend(map.get_elevation(landing.x, landing.y) + 0.22),
            velocity: Vec3::new(0.0, 0.0, 0.5),
            age: 0.0,
        },
    );
    stats.status_log =
        moon_base_localize::localized_format!("Na ziemi: {} × {}", amount, ITEM_NAMES[kind]);
}

pub fn update_pickups(
    mut commands: Commands,
    time: Res<Time>,
    map: Res<LunarMap>,
    player: Query<&AstronautPlayer>,
    mut stats: ResMut<SurvivalStats>,
    mut pickups: Query<(
        Entity,
        &mut ResourcePickup,
        &mut Transform,
        &mut Visibility,
        Option<&crate::net_world::Replica>,
    )>,
    mut lan: Option<ResMut<crate::net_lan::LanMultiplayer>>,
) {
    let Ok(player) = player.get_single() else {
        return;
    };
    let dt = time.delta_seconds().min(0.05);
    let mut collected = 0;
    for (entity, mut item, mut transform, mut visibility, replica) in &mut pickups {
        advance_pickup(&mut item, &map, dt);
        let distance = player.iso_pos.truncate().distance(item.position.truncate());
        *visibility = if distance < 24.0 {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        let screen = iso_to_world(item.position.x, item.position.y, item.position.z);
        transform.translation = Vec3::new(
            screen.x,
            screen.y + 6.0 + (item.age * 2.0).sin() * 1.5,
            iso_depth(item.position.x, item.position.y, item.position.z, 2.0),
        );
        if item.age < 1.0
            || distance > 0.95
            || (player.iso_pos.z - item.position.z).abs() > 0.9
            || player.inside_building
            || player.airlock_anim_timer > 0.0
            || player.mounted_rover.is_some()
        {
            continue;
        }
        if let Some(lan) = lan
            .as_deref_mut()
            .filter(|lan| lan.is_active && !lan.is_host)
        {
            if let Some(replica) = replica {
                lan.send(crate::net_lan::Message::Action(
                    crate::net_world::Action::Pickup(replica.0),
                ));
                item.age = 0.0;
            }
            continue;
        }
        if !stats.fits_cargo(item.kind, item.amount) {
            stats.status_log =
                crate::locale::text("Plecak pełny (35 kg). Zostaw ładunek w magazynie bazy.")
                    .into();
            continue;
        }
        stats.change_cargo(item.kind, item.amount as i32);
        pickup_notice(
            &mut commands,
            item.kind,
            item.amount,
            player.iso_pos,
            (collected as f32 * 3.0).min(54.0),
        );
        stats.generated_science_gb += item.science;
        if (2..=5).contains(&item.kind) {
            stats.mineral_catalog |= 1 << (item.kind - 2);
        }
        collected += item.amount;
        commands.entity(entity).despawn_recursive();
    }
    if collected > 0 {
        stats.collected_minerals = true;
        stats.status_log =
            moon_base_localize::localized_format!("Zebrano odłamki: +{} do plecaka.", collected);
    }
    if stats.mineral_catalog == 15
        && !stats.geology_report_delivered
        && player.inside_building
        && player.airlock_anim_timer <= 0.0
    {
        stats.geology_report_delivered = true;
        stats.generated_science_gb += 120.0;
        stats.status_log = crate::locale::text("ATLAS GEOLOGICZNY UKOŃCZONY: dostarczono lód, tytan, He-3 i meteoryt. +120 GB raportu do transmisji.").into();
    }
}

pub fn update_background_pickups(
    time: Res<Time>,
    map: Res<LunarMap>,
    mut pickups: Query<&mut ResourcePickup>,
) {
    let dt = time.delta_seconds().min(0.05);
    for mut item in &mut pickups {
        advance_pickup(&mut item, &map, dt);
    }
}

fn advance_pickup(item: &mut ResourcePickup, map: &LunarMap, dt: f32) {
    item.age += dt;
    let floor = map.get_elevation(item.position.x, item.position.y);
    if item.position.z > floor || item.velocity.z > 0.0 {
        item.velocity.z -= map.body.gravity() * dt;
        item.position += item.velocity * dt;
        if item.position.z <= floor {
            item.position.z = floor;
            item.velocity = Vec3::ZERO;
        }
    }
}

/// Rewards and demolition refunds that exceed capacity remain recoverable in the world.
pub fn enforce_capacity(
    mut commands: Commands,
    assets: Res<ProceduralAssets>,
    mut stats: ResMut<SurvivalStats>,
    player: Query<&AstronautPlayer>,
    lan: Option<Res<crate::net_lan::LanMultiplayer>>,
) {
    if lan.as_ref().is_some_and(|l| l.is_active && !l.is_host) {
        return;
    }
    let Ok(player) = player.get_single() else {
        return;
    };
    for kind in 0..11 {
        let excess = stats.cargo_weight() - stats.cargo_limit();
        if excess <= 0.001 {
            break;
        }
        let amount = (excess / SurvivalStats::item_weight(kind)).ceil() as u32;
        let amount = amount.min(stats.cargo_counts()[kind]);
        if amount == 0 {
            continue;
        }
        stats.change_cargo(kind, -(amount as i32));
        spawn_saved_pickup(
            &mut commands,
            &assets,
            ResourcePickup {
                kind,
                amount,
                science: 0.0,
                position: player.iso_pos + Vec3::new(0.8, 0.0, 0.2),
                velocity: Vec3::ZERO,
                age: 0.0,
            },
        );
        stats.status_log =
            crate::locale::text("Plecak pełny: nadmiar ładunku został obok Ciebie. Limit: 35 kg.")
                .into();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mining_preview_matches_the_size_after_a_completed_cycle() {
        let mut map = LunarMap::default();
        map.generate_sector(0, 0, crate::types::MissionScenario::PolsaKopernikBase);
        let pos = (8, 8);
        let cell = map.cells.get_mut(&pos).unwrap();
        cell.kind = TileKind::TitaniumOutcrop;
        cell.resource_yield = 5;
        map.mining_target = Some(pos);
        let initial = deposit_scale(&map.cells[&pos], &map, pos.0, pos.1, 5.0);
        map.mining_progress = 0.5;
        let half = deposit_scale(&map.cells[&pos], &map, pos.0, pos.1, 5.0);
        assert!(half > 0.0 && half < initial);
        map.mining_progress = 1.0;
        let finished = deposit_scale(&map.cells[&pos], &map, pos.0, pos.1, 5.0);
        map.cells.get_mut(&pos).unwrap().resource_yield = 0;
        map.mining_target = None;
        assert_eq!(
            finished,
            deposit_scale(&map.cells[&pos], &map, pos.0, pos.1, 5.0)
        );
    }
    fn test_app() -> (App, Entity) {
        let mut app = App::new();
        app.insert_resource(Time::<()>::default())
            .insert_resource(LunarMap::default())
            .insert_resource(SurvivalStats::from_config(
                &crate::types::MissionConfig::default(),
            ))
            .add_systems(Update, update_pickups);
        let player = app
            .world
            .spawn(AstronautPlayer {
                iso_pos: Vec3::ZERO,
                iso_vel: Vec3::ZERO,
                grounded: true,
                mounted_rover: None,
                is_passenger: false,
                facing_right: true,
                facing_angle_rad: 0.0,
                target_facing_rad: 0.0,
                walk_cycle: 0.0,
                action_swing: 0.0,
                inside_building: false,
                airlock_anim_timer: 0.0,
                airlock_entering: false,
                airlock_target_iso: Vec2::ZERO,
            })
            .id();
        (app, player)
    }

    #[test]
    fn cargo_stays_in_world_until_approached_and_cannot_duplicate() {
        let (mut app, player) = test_app();
        let drop = app
            .world
            .spawn((
                ResourcePickup {
                    kind: 3,
                    amount: 7,
                    science: 8.5,
                    position: Vec3::new(5.0, 0.0, 0.0),
                    velocity: Vec3::ZERO,
                    age: 2.0,
                },
                Transform::default(),
                Visibility::Inherited,
            ))
            .id();
        app.update();
        assert_eq!(app.world.resource::<SurvivalStats>().titanium, 0);
        assert!(app.world.get_entity(drop).is_some());
        app.world
            .get_mut::<AstronautPlayer>(player)
            .unwrap()
            .iso_pos
            .x = 5.0;
        app.update();
        assert_eq!(app.world.resource::<SurvivalStats>().titanium, 7);
        assert!(app.world.get_entity(drop).is_none());
        app.update();
        assert_eq!(app.world.resource::<SurvivalStats>().titanium, 7);
        assert_eq!(
            app.world.resource::<SurvivalStats>().generated_science_gb,
            8.5
        );
    }

    #[test]
    fn survey_requires_return_and_rewards_once() {
        let (mut app, player) = test_app();
        app.world.resource_mut::<SurvivalStats>().mineral_catalog = 15;
        app.update();
        assert!(
            !app.world
                .resource::<SurvivalStats>()
                .geology_report_delivered
        );
        app.world
            .get_mut::<AstronautPlayer>(player)
            .unwrap()
            .inside_building = true;
        app.update();
        app.update();
        assert!(
            app.world
                .resource::<SurvivalStats>()
                .geology_report_delivered
        );
        assert_eq!(
            app.world.resource::<SurvivalStats>().generated_science_gb,
            120.0
        );
    }

    #[test]
    fn lander_can_be_entered_without_a_tile_structure() {
        let map = LunarMap::default();
        assert_eq!(
            map.find_nearby_enterable_building(0.0, 0.0, 2.8),
            Some((0, 0, crate::types::StructureKind::HabitatModule))
        );
        assert!(map
            .find_nearby_enterable_building(20.0, 20.0, 2.8)
            .is_none());
    }
}
