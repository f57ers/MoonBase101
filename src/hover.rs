use crate::isometric::iso_to_world;
use crate::world::{AstronautPlayer, CoOpCrewMember, LunarMap, RoverVehicle, TouchAndUiInput};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
#[derive(Component)]
pub struct Tooltip;
#[derive(Component)]
pub struct TooltipText;

pub fn setup(mut commands: Commands) {
    commands
        .spawn((
            crate::ui_screens::ScreenUiRoot,
            Tooltip,
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    padding: UiRect::all(Val::Px(10.0)),
                    max_width: Val::Px(280.0),
                    display: Display::None,
                    ..default()
                },
                background_color: Color::rgba(0.025, 0.045, 0.075, 0.97).into(),
                z_index: ZIndex::Global(500),
                focus_policy: bevy::ui::FocusPolicy::Pass,
                ..default()
            },
        ))
        .with_children(|root| {
            root.spawn((
                TooltipText,
                TextBundle::from_section(
                    "",
                    TextStyle {
                        font_size: 15.0,
                        color: Color::rgb(0.91, 0.96, 1.0),
                        ..default()
                    },
                ),
            ));
        });
}

pub fn update(
    windows: Query<&Window, With<PrimaryWindow>>,
    camera: Query<(&Camera, &GlobalTransform, &OrthographicProjection)>,
    map: Res<LunarMap>,
    touch: Res<TouchAndUiInput>,
    config: Res<crate::types::MissionConfig>,
    lan: Res<crate::net_lan::LanMultiplayer>,
    drops: Query<&crate::mineral::ResourcePickup>,
    rovers: Query<(&RoverVehicle, Option<&crate::vehicles::SeatInfo>)>,
    players: Query<&AstronautPlayer>,
    crew: Query<&CoOpCrewMember>,
    mut tooltip: Query<&mut Style, With<Tooltip>>,
    mut texts: Query<&mut Text, With<TooltipText>>,
    scale: Res<UiScale>,
) {
    let Ok(mut style) = tooltip.get_single_mut() else {
        return;
    };
    style.display = Display::None;
    if map.any_modal_open() || touch.pointer_over_ui {
        return;
    }
    let Ok(window) = windows.get_single() else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let Ok((camera, transform, projection)) = camera.get_single() else {
        return;
    };
    let Some(point) = camera.viewport_to_world_2d(transform, cursor) else {
        return;
    };
    let mut best = 30.0 * projection.scale;
    let mut name = String::new();
    let mut consider = |pos: Vec2, label: String| {
        let distance = pos.distance(point);
        if distance < best {
            best = distance;
            name = label;
        }
    };
    let minerals = [
        crate::locale::text("Regolit"),
        crate::locale::text("Krzem"),
        crate::locale::text("Lód wodny"),
        crate::locale::text("Tytan"),
        "Hel-3",
        crate::locale::text("Rzadkie minerały"),
        crate::locale::text("Próbka skały"),
        crate::locale::text("Racja żywnościowa"),
        crate::locale::text("Posiłek"),
        crate::locale::text("Woda do picia"),
        crate::locale::text("Apteczka"),
    ];
    for item in &drops {
        consider(
            iso_to_world(item.position.x, item.position.y, item.position.z) + Vec2::Y * 8.0,
            moon_base_localize::localized_format!(
                "{} ×{}\n{:.1} kg · podejdź, by zebrać",
                minerals[item.kind.min(10)],
                item.amount,
                crate::world::SurvivalStats::item_weight(item.kind) * item.amount as f32
            ),
        );
    }
    for (rover, seats) in &rovers {
        consider(
            iso_to_world(rover.iso_pos.x, rover.iso_pos.y, rover.iso_pos.z) + Vec2::Y * 22.0,
            moon_base_localize::localized_format!(
                "{} · miejsca {}/{}\nE — kierowca lub pasażer · bateria {:.0}%",
                rover.kind.short_name(),
                seats.map_or(0, |s| usize::from(s.driver.is_some()) + s.passengers.len()),
                if rover.kind == crate::types::RoverKind::HeavyCruiser {
                    4
                } else {
                    2
                },
                100.0 * rover.battery / rover.kind.max_battery()
            ),
        );
    }
    for player in &players {
        consider(
            iso_to_world(player.iso_pos.x, player.iso_pos.y, player.iso_pos.z) + Vec2::Y * 28.0,
            moon_base_localize::localized_format!("{} · Twój astronauta", config.callsign),
        );
    }
    for member in &crew {
        if let Some(peer) = lan.peers.get(member.peer_index) {
            if !peer.is_ai_bot && peer.body == map.body && lan.is_active {
                consider(
                    iso_to_world(peer.iso_pos.x, peer.iso_pos.y, peer.iso_pos.z) + Vec2::Y * 28.0,
                    moon_base_localize::localized_format!(
                        "{}\nF8 — piątka · F9 — wspólny taniec",
                        peer.callsign
                    ),
                );
            }
        }
    }
    consider(
        iso_to_world(
            map.station_position.x,
            map.station_position.y,
            map.station_position.z,
        ) + Vec2::Y * 50.0,
        crate::locale::text("Stacja kosmiczna\nG — pilotuj · E — wejdź do kabiny").into(),
    );
    for research in map.survey_points() {
        let p = research.pos.as_vec2();
        consider(
            iso_to_world(p.x, p.y, map.get_elevation(p.x, p.y)) + Vec2::Y * 30.0,
            moon_base_localize::localized_format!(
                "Punkt badań: {}\nPodejdź i naciśnij P",
                research.name
            ),
        );
    }
    if name.is_empty() {
        let pos = map.pick_tile_at_world(point);
        if let Some(cell) = map.cells.get(&pos) {
            name = if let Some(kind) = cell.structure {
                format!("{}\nStan {:.0}%", kind.name(), cell.structure_hp)
            } else if let Some(feature) = cell.feature {
                feature.name().into()
            } else {
                moon_base_localize::localized_format!(
                    "{}\nZapas złoża: {} · przytrzymaj mysz, by kopać",
                    cell.kind.name(),
                    cell.resource_yield
                )
            };
        }
    }
    if name.is_empty() {
        return;
    }
    style.display = Display::Flex;
    style.left = Val::Px((cursor.x + 18.0).min(window.width() - 290.0).max(0.0) / scale.0);
    style.top = Val::Px((cursor.y + 22.0).min(window.height() - 95.0).max(0.0) / scale.0);
    if let Ok(mut text) = texts.get_single_mut() {
        text.sections[0].value = name;
    }
}
