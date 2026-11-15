use bevy::prelude::*;

use crate::isometric::ProceduralAssets;
use crate::planets::WorldBody;
use crate::types::{AppState, StructureKind};
use crate::world::{AstronautPlayer, LunarMap, SurvivalStats};

pub const GOAL: &str = "Zbadaj trzy światy i wyślij raport na Ziemię.";
pub const SUPPLIES: IVec2 = IVec2::new(4, 3);
pub const BASE: IVec2 = IVec2::new(3, 0);
pub const RELAY: (i32, i32) = (4, -1);
pub const ROVER_SPEED: f32 = 9.0;
pub const SCAN_SECONDS: f32 = 25.0;
const UPLINK_SECONDS: f32 = 18.0;

#[derive(Resource, Default)]
pub struct CampaignLaunch(pub bool);

pub fn survey_points(body: WorldBody) -> [crate::planets::SurveyPoint; 4] {
    use crate::planets::SurveyPoint;
    let [first, second] = body.survey();
    let (third,fourth) = match body {
        WorldBody::Moon => (
            SurveyPoint {pos:IVec2::new(-147,54),name:crate::locale::text("Cień krateru"),fact:crate::locale::text("W głębokich kraterach blisko biegunów światło może nigdy nie docierać do dna.")},
            SurveyPoint {pos:IVec2::new(64,165),name:crate::locale::text("Ślady misji"),fact:crate::locale::text("Na Księżycu ślady astronautów mogą przetrwać bardzo długo, bo nie ma tam deszczu ani zwykłego wiatru.")}),
        WorldBody::Mars => (
            SurveyPoint {pos:IVec2::new(-153,73),name:crate::locale::text("Pole pyłu"),fact:crate::locale::text("Pył w atmosferze Marsa ogranicza światło, które dociera do paneli słonecznych.")},
            SurveyPoint {pos:IVec2::new(60,171),name:crate::locale::text("Ślad dawnej wody"),fact:crate::locale::text("Na Marsie widać koryta, które wskazują, że dawno temu płynęła tam woda.")}),
        WorldBody::Venus => (
            SurveyPoint {pos:IVec2::new(-143,85),name:crate::locale::text("Mapa radarowa"),fact:crate::locale::text("Gęste chmury zasłaniają powierzchnię Wenus. Radar pomaga tworzyć mapy gór i równin pod chmurami.")},
            SurveyPoint {pos:IVec2::new(73,166),name:crate::locale::text("Gorąca równina"),fact:crate::locale::text("Wenus jest gorętsza od Merkurego mimo większej odległości od Słońca. Przyczyną jest jej gęsta atmosfera.")}),
    };
    [first, second, third, fourth]
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub enum Quest {
    Supplies,
    Water,
    Meal,
    Relay,
    Moon,
    Mars,
    Venus,
    Report,
    Complete,
}

impl Quest {
    pub const ALL: [Self; 8] = [
        Self::Supplies,
        Self::Water,
        Self::Meal,
        Self::Relay,
        Self::Moon,
        Self::Mars,
        Self::Venus,
        Self::Report,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Self::Supplies => crate::locale::text("Otwórz skrzynię z zapasami"),
            Self::Water => crate::locale::text("Zrób wodę w bazie"),
            Self::Meal => crate::locale::text("Zrób posiłek w bazie"),
            Self::Relay => crate::locale::text("Uruchom antenę bazy"),
            Self::Moon => crate::locale::text("Zbadaj Księżyc"),
            Self::Mars => crate::locale::text("Zbadaj Marsa"),
            Self::Venus => crate::locale::text("Zbadaj Wenus"),
            Self::Report => crate::locale::text("Wyślij raport na Ziemię"),
            Self::Complete => crate::locale::text("Misja ukończona"),
        }
    }

    pub fn target_body(self) -> Option<WorldBody> {
        match self {
            Self::Moon | Self::Report => Some(WorldBody::Moon),
            Self::Mars => Some(WorldBody::Mars),
            Self::Venus => Some(WorldBody::Venus),
            _ => None,
        }
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Campaign {
    pub quest: Quest,
    pub seconds: f32,
    pub action_requested: bool,
    pub opened_supplies: bool,
    pub made_water: bool,
    pub made_meal: bool,
    pub uplink: Option<f32>,
}

impl Default for Campaign {
    fn default() -> Self {
        Self {
            quest: Quest::Supplies,
            seconds: 0.0,
            action_requested: false,
            opened_supplies: false,
            made_water: false,
            made_meal: false,
            uplink: None,
        }
    }
}

pub fn travel_allowed(campaign: Option<&Campaign>, destination: WorldBody) -> bool {
    campaign.is_none_or(|c| {
        matches!(c.quest, Quest::Mars | Quest::Venus | Quest::Report)
            && c.quest.target_body() == Some(destination)
    })
}

pub fn indoors_at_base(map: &LunarMap, player: &AstronautPlayer) -> bool {
    let pos = player.iso_pos.truncate().round().as_ivec2();
    player.inside_building
        && player.airlock_anim_timer <= 0.0
        && !map.piloting_station
        && matches!((pos.x, pos.y), (2, 0) | (3, 0) | (4, 0))
}

pub fn available_action(
    campaign: &Campaign,
    map: &LunarMap,
    player: &AstronautPlayer,
) -> Option<&'static str> {
    if map.any_modal_open()
        || campaign
            .quest
            .target_body()
            .is_some_and(|body| body != map.body)
    {
        return None;
    }
    match campaign.quest {
        Quest::Relay if indoors_at_base(map, player) => {
            Some(crate::locale::text("Podłącz antenę [P]"))
        }
        Quest::Report if indoors_at_base(map, player) && campaign.uplink.is_none() => {
            Some(crate::locale::text("Wyślij raport [P]"))
        }
        Quest::Moon | Quest::Mars | Quest::Venus
            if !player.inside_building
                && player.mounted_rover.is_none()
                && !map.piloting_station
                && map.scan_target.is_none()
                && map.survey_points().iter().enumerate().any(|(i, p)| {
                    map.surveyed & (1 << i) == 0
                        && player.iso_pos.truncate().distance(p.pos.as_vec2()) < 1.9
                }) =>
        {
            Some(crate::locale::text("Zbadaj [P]"))
        }
        _ => None,
    }
}

pub fn current_target(quest: Quest, map: &LunarMap) -> Option<Vec2> {
    if quest.target_body().is_some_and(|body| body != map.body) {
        return Some(map.station_position.truncate());
    }
    match quest {
        Quest::Supplies => Some(SUPPLIES.as_vec2()),
        Quest::Water | Quest::Meal | Quest::Relay | Quest::Report => Some(BASE.as_vec2()),
        Quest::Moon | Quest::Mars | Quest::Venus => map
            .survey_points()
            .iter()
            .enumerate()
            .find(|(i, _)| map.surveyed & (1 << i) == 0)
            .map(|(_, p)| p.pos.as_vec2()),
        Quest::Complete => None,
    }
}

pub fn draw_target(
    settings: Res<crate::types::GameSettings>,
    map: Res<LunarMap>,
    stats: Res<SurvivalStats>,
    player: Query<&AstronautPlayer>,
    mut gizmos: Gizmos,
) {
    if settings.hide_hud || map.quests_disabled {
        return;
    }
    let Some(campaign) = &stats.campaign else {
        return;
    };
    let Some(target) = current_target(campaign.quest, &map) else {
        return;
    };
    let Ok(player) = player.get_single() else {
        return;
    };
    if map.any_modal_open() {
        return;
    }
    let start =
        crate::isometric::iso_to_world(player.iso_pos.x, player.iso_pos.y, player.iso_pos.z)
            + Vec2::Y * 27.0;
    let end =
        crate::isometric::iso_to_world(target.x, target.y, map.get_elevation(target.x, target.y));
    let color = Color::rgb(1.0, 0.80, 0.36);
    let diff = end - start;
    if player.iso_pos.truncate().distance(target) <= 4.0 {
        let tip = end + Vec2::Y * 46.0;
        gizmos.line_2d(tip + Vec2::new(-9.0, 9.0), tip, color);
        gizmos.line_2d(tip + Vec2::new(9.0, 9.0), tip, color);
    } else {
        let direction = diff.normalize_or_zero();
        let tip = start + direction * 70.0;
        let tail = tip - direction * 14.0;
        gizmos.line_2d(start + direction * 43.0, tip, color);
        gizmos.line_2d(tail + direction.perp() * 8.0, tip, color);
        gizmos.line_2d(tail - direction.perp() * 8.0, tip, color);
    }
}

pub fn guidance(
    campaign: &Campaign,
    map: &LunarMap,
    player: Option<&AstronautPlayer>,
    network: bool,
) -> String {
    let quest = campaign.quest;
    if let Some(destination) = quest.target_body().filter(|body| *body != map.body) {
        return if network {
            moon_base_localize::localized_format!(
                "Podejdź do stacji, otwórz dziennik [N] i wybierz: {}",
                destination.name()
            )
        } else if map.piloting_station {
            moon_base_localize::localized_format!(
                "Otwórz dziennik [N] i wybierz: {}",
                destination.name()
            )
        } else {
            moon_base_localize::localized_format!(
                "Wsiądź do stacji [G], aby polecieć: {}",
                destination.name()
            )
        };
    }
    let at_base = player.is_some_and(|p| indoors_at_base(map, p));
    match quest {
        Quest::Supplies => {
            crate::locale::text("Idź do złotego znacznika. Przytrzymaj mysz na skrzyni.").into()
        }
        Quest::Water if at_base => crate::locale::text("Kliknij „Zrób wodę” po lewej.").into(),
        Quest::Meal if at_base => crate::locale::text("Kliknij „Zrób posiłek” po lewej.").into(),
        Quest::Relay if at_base => {
            crate::locale::text("Naciśnij P, by podłączyć antenę zestawem ze skrzyni.").into()
        }
        Quest::Report if at_base => campaign.uplink.map_or_else(
            || crate::locale::text("Naciśnij P, aby wysłać raport.").into(),
            |left| {
                moon_base_localize::localized_format!("Wysyłanie raportu · {:.0} s", left.ceil())
            },
        ),
        Quest::Water | Quest::Meal | Quest::Relay | Quest::Report => {
            crate::locale::text("Wróć do oznaczonej bazy i wejdź przez śluzę [E].").into()
        }
        Quest::Moon | Quest::Mars | Quest::Venus => {
            if map.scan_progress > 0.0 {
                moon_base_localize::localized_format!(
                    "Pomiar trwa · {:.0}% · Zostań przy znaczniku",
                    map.scan_progress * 100.0
                )
            } else {
                let distance = player
                    .and_then(|p| {
                        current_target(quest, map)
                            .map(|target| target.distance(p.iso_pos.truncate()))
                    })
                    .unwrap_or_default();
                let count = map.surveyed.count_ones();
                if distance > 45.0 {
                    moon_base_localize::localized_format!(
                        "Jedź łazikiem za złotą strzałką · {:.0} m · {}/4",
                        distance,
                        count
                    )
                } else if distance > 2.0 {
                    moon_base_localize::localized_format!(
                        "Podążaj za złotą strzałką · {:.0} m · {}/4",
                        distance,
                        count
                    )
                } else {
                    moon_base_localize::localized_format!(
                        "Wysiądź, jeśli jedziesz. Naciśnij P, aby zbadać. Zbadano {}/4.",
                        count
                    )
                }
            }
        }
        Quest::Complete => crate::locale::text("Raport dotarł na Ziemię.").into(),
    }
}

pub fn update_campaign(
    mut commands: Commands,
    assets: Res<ProceduralAssets>,
    time: Res<Time>,
    lan: Option<Res<crate::net_lan::LanMultiplayer>>,
    ledger: Option<Res<crate::dimensions::SurveyLedger>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut map: ResMut<LunarMap>,
    mut stats: ResMut<SurvivalStats>,
    player: Query<&AstronautPlayer>,
    mut next: ResMut<NextState<AppState>>,
) {
    if lan
        .as_ref()
        .is_some_and(|lan| lan.is_active && !lan.is_host)
    {
        return;
    }
    let Some(mut campaign) = stats.campaign.take() else {
        return;
    };
    campaign.seconds += time.delta_seconds();
    let action = (std::mem::take(&mut campaign.action_requested)
        || keys.just_pressed(KeyCode::KeyP))
        && !map.any_modal_open();
    let at_base = (map.body == WorldBody::Moon
        && player.get_single().is_ok_and(|p| indoors_at_base(&map, p)))
        || lan.as_ref().is_some_and(|lan| {
            lan.is_host
                && lan.peers.iter().any(|p| {
                    !p.is_ai_bot
                        && p.body == WorldBody::Moon
                        && (p.iso_pos.x - 3.0).abs() < 1.1
                        && p.iso_pos.y.abs() < 0.8
                })
        });
    let surveyed = |body: WorldBody| {
        if body == map.body {
            map.surveyed.count_ones() == 4
        } else {
            ledger
                .as_ref()
                .is_some_and(|l| l.surveys[body.portrait_index()].count_ones() == 4)
        }
    };
    let previous = campaign.quest;
    match campaign.quest {
        Quest::Supplies if campaign.opened_supplies => campaign.quest = Quest::Water,
        Quest::Water if campaign.made_water => campaign.quest = Quest::Meal,
        Quest::Meal if campaign.made_meal => campaign.quest = Quest::Relay,
        Quest::Relay if action && at_base => {
            if let Some(relay) = map.cells.get_mut(&RELAY) {
                relay.is_world_generated_base = true;
                crate::world::spawn_structure_at(
                    &mut commands,
                    &assets,
                    &mut map,
                    RELAY.0,
                    RELAY.1,
                    StructureKind::CommsDish,
                );
                map.network_elapsed = 0.5;
                campaign.quest = Quest::Moon;
            }
        }
        Quest::Moon if surveyed(WorldBody::Moon) => campaign.quest = Quest::Mars,
        Quest::Mars if surveyed(WorldBody::Mars) => campaign.quest = Quest::Venus,
        Quest::Venus if surveyed(WorldBody::Venus) => campaign.quest = Quest::Report,
        Quest::Report => {
            let link = if map.body == WorldBody::Moon {
                map.networks
                    .get(&(BASE.x, BASE.y))
                    .is_some_and(|base| base.powered && base.relay)
            } else {
                ledger.as_ref().is_some_and(|l| l.moon_link)
            };
            if action && at_base && link && campaign.uplink.is_none() {
                campaign.uplink = Some(UPLINK_SECONDS);
            }
            if action && at_base && !link {
                stats.status_log =
                    crate::locale::text("Antena czeka na prąd. Sprawdź zasilanie bazy.").into();
            }
            if at_base && link {
                if let Some(left) = &mut campaign.uplink {
                    *left -= time.delta_seconds().min(0.05);
                    if *left <= 0.0 {
                        campaign.quest = Quest::Complete;
                        stats.transmitted_science_gb = stats.generated_science_gb;
                        next.set(AppState::MissionReport);
                    }
                }
            }
        }
        _ => {}
    }
    if previous != campaign.quest {
        stats.status_log = format!(
            "Zrobione: {}. Teraz: {}.",
            previous.title(),
            campaign.quest.title()
        );
    }
    stats.campaign = Some(campaign);
}
