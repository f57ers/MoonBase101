use bevy::prelude::*;
use std::collections::{HashMap, HashSet};
use std::f32::consts::FRAC_PI_2;

use crate::celestial::SolarSystemState;
use crate::isometric::{
    iso_depth, iso_shadow_depth, iso_tile_depth, iso_to_world, world_to_iso, ProceduralAssets,
    STREAM_RADIUS,
};
use crate::net_lan::LanMultiplayer;
use crate::types::{
    BackpackModule, MissionConfig, MissionScenario, PrelandedRoverChoice, RoverKind, StructureKind,
    TileKind, ToolTier, WorldFeatureKind,
};

#[derive(Clone, Debug)]
pub struct Waypoint {
    pub name: String,
    pub gx: i32,
    pub gy: i32,
    pub color: Color,
}

#[derive(Clone, Debug)]
pub struct TileCell {
    pub kind: TileKind,
    pub elevation: f32,
    pub resource_yield: u32,
    pub scanned: bool,
    pub structure: Option<StructureKind>,
    pub structure_hp: f32,
    pub charge_wh: f32,
    /// Exact paid cost; changing crew must not create resources on demolition.
    pub paid_cost: Option<(u32, u32, u32, u32)>,
    pub feature: Option<WorldFeatureKind>,
    pub is_world_generated_base: bool,
    pub tile_entity: Option<Entity>,
    pub feature_entity: Option<Entity>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct FlareLight {
    pub x: i32,
    pub y: i32,
    pub until: f32,
}

#[derive(Resource, Default)]
pub struct LunarMap {
    pub flare_lights: Vec<FlareLight>,
    pub quests_disabled: bool,
    pub campaign_route: bool,
    pub scan_progress: f32,
    pub scan_target: Option<usize>,
    pub last_discovery: Option<usize>,
    pub water_job: Option<crate::base::SupplyJob>,
    pub meal_job: Option<crate::base::SupplyJob>,
    pub body: crate::planets::WorldBody,
    pub surveyed: u8,
    pub storage: [u32; 11],
    pub networks: HashMap<(i32, i32), crate::base::BaseStatus>,
    pub network_elapsed: f32,
    pub station_position: Vec3,
    pub station_previous: Vec3,
    pub station_parked: Vec3,
    pub piloting_station: bool,
    pub cells: HashMap<(i32, i32), TileCell>,
    pub generated_sectors: HashSet<(i32, i32)>,
    pub hovered_tile: (i32, i32),
    pub selected_build: usize,
    pub build_mode: bool,
    pub demolition_mode: bool,
    pub show_inventory_window: bool,
    pub quest_panel_collapsed: bool,
    pub show_customizer_window: bool,
    pub show_map_window: bool,
    pub show_progression_window: bool,
    pub last_stream_center: (i32, i32),
    pub world_seed: u64,
    // Progressive block mining state (0.0 -> 1.0)
    pub mining_target: Option<(i32, i32)>,
    pub mining_progress: f32,
    pub mining_label: String,
    // Interactive World Map & Waypoints
    pub waypoints: Vec<Waypoint>,
}

impl LunarMap {
    pub fn flare_lit(&self, x: i32, y: i32, now: f32) -> bool {
        self.flare_lights
            .iter()
            .any(|light| light.until > now && (light.x - x).abs() <= 2 && (light.y - y).abs() <= 2)
    }
    pub fn safe_exit(&self, from: Vec2, max_distance: f32) -> Option<Vec2> {
        let center = from.round().as_ivec2();
        let radius = max_distance.ceil() as i32;
        (-radius..=radius)
            .flat_map(|dy| (-radius..=radius).map(move |dx| center + IVec2::new(dx, dy)))
            .map(|tile| tile.as_vec2())
            .filter(|pos| {
                pos.distance(from) <= max_distance
                    && !self.blocks_movement(pos.x, pos.y)
                    && !self.is_inside_pressurized(pos.x, pos.y)
            })
            .min_by(|a, b| {
                a.distance_squared(from)
                    .total_cmp(&b.distance_squared(from))
            })
    }
    pub fn survey_points(&self) -> Vec<crate::planets::SurveyPoint> {
        crate::campaign::survey_points(self.body).to_vec()
    }
    pub fn blocks_movement(&self, x: f32, y: f32) -> bool {
        self.cells
            .get(&(x.round() as i32, y.round() as i32))
            .map_or(true, |cell| {
                cell.structure.is_some() && cell.structure_hp > 0.0
            })
    }
    pub fn any_modal_open(&self) -> bool {
        self.show_inventory_window
            || self.show_customizer_window
            || self.show_map_window
            || self.show_progression_window
    }

    pub fn close_all_modals(&mut self) {
        self.show_inventory_window = false;
        self.show_customizer_window = false;
        self.show_map_window = false;
        self.show_progression_window = false;
    }

    pub fn get_elevation(&self, x: f32, y: f32) -> f32 {
        let gx = x.round() as i32;
        let gy = y.round() as i32;
        self.cells
            .get(&(gx, gy))
            .map(|c| c.elevation)
            .unwrap_or(0.0)
    }

    pub fn is_inside_pressurized(&self, x: f32, y: f32) -> bool {
        let gx = x.round() as i32;
        let gy = y.round() as i32;
        if (x - self.station_position.x).abs() <= 1.0 && (y - self.station_position.y).abs() <= 1.0
        {
            return true;
        }
        if let Some(cell) = self.cells.get(&(gx, gy)) {
            if let Some(st) = cell.structure {
                return st.is_pressurized_room() && cell.structure_hp > 10.0;
            }
        }
        false
    }

    /// Finds the nearest enterable building (`Airlock` or pressurized room) within `max_dist` tiles of `(x, y)`.
    pub fn find_nearby_enterable_building(
        &self,
        x: f32,
        y: f32,
        max_dist: f32,
    ) -> Option<(i32, i32, StructureKind)> {
        let px = x.round() as i32;
        let py = y.round() as i32;
        let mut best: Option<(i32, i32, StructureKind, f32)> = None;
        let lander_distance = Vec2::new(x, y).distance(self.station_position.truncate());
        if lander_distance <= max_dist {
            best = Some((
                self.station_position.x.round() as i32,
                self.station_position.y.round() as i32,
                StructureKind::HabitatModule,
                lander_distance,
            ));
        }

        // First priority: Airlocks! Second priority: any pressurized module or Lander (0,0).
        for dy in -3..=3 {
            for dx in -3..=3 {
                let gx = px + dx;
                let gy = py + dy;
                let d = Vec2::new(x, y).distance(Vec2::new(gx as f32, gy as f32));
                if d > max_dist {
                    continue;
                }
                if let Some(cell) = self.cells.get(&(gx, gy)) {
                    if cell.structure_hp > 10.0 {
                        if let Some(st) = cell.structure {
                            if st.is_pressurized_room() {
                                // Prefer Airlock slightly by giving it an effective distance bonus
                                let score = if st == StructureKind::Airlock {
                                    d - 1.2
                                } else {
                                    d
                                };
                                if best.map_or(true, |b| score < b.3) {
                                    best = Some((gx, gy, st, score));
                                }
                            }
                        }
                    }
                }
            }
        }
        best.map(|(gx, gy, st, _)| (gx, gy, st))
    }

    pub fn has_adjacent_shield_wall(&self, gx: i32, gy: i32) -> bool {
        for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
            if let Some(c) = self.cells.get(&(gx + dx, gy + dy)) {
                if c.structure == Some(StructureKind::ShieldWall) && c.structure_hp > 10.0 {
                    return true;
                }
            }
        }
        false
    }

    /// Counts how many orthogonal neighbor tiles share the same structure (or pressurized complex)
    /// so adjacent buildings unite into a single mega-structure!
    pub fn connected_neighbors_count(&self, gx: i32, gy: i32, kind: StructureKind) -> u32 {
        let mut count = 0;
        for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
            if let Some(c) = self.cells.get(&(gx + dx, gy + dy)) {
                if c.structure_hp > 0.0 {
                    if let Some(other) = c.structure {
                        if other == kind
                            || (kind.is_pressurized_room() && other.is_pressurized_room())
                        {
                            count += 1;
                        }
                    }
                }
            }
        }
        count
    }

    /// Elevation-aware isometric tile picking so clicking elevated rims or deep craters is 100% accurate.
    pub fn pick_tile_at_world(&self, world_pos: Vec2) -> (i32, i32) {
        let approx = world_to_iso(world_pos);
        let base_gx = approx.x.round() as i32;
        let base_gy = approx.y.round() as i32;

        let mut best_tile = (base_gx, base_gy);
        let mut best_dist = f32::MAX;

        for dy in -2..=2 {
            for dx in -2..=2 {
                let gx = base_gx + dx;
                let gy = base_gy + dy;
                let elev = self.get_elevation(gx as f32, gy as f32);
                let center = iso_to_world(gx as f32, gy as f32, elev);
                let diff = world_pos - center;
                let metric = diff.x.abs() / 44.0 + diff.y.abs() / 22.0;
                if metric < best_dist {
                    best_dist = metric;
                    best_tile = (gx, gy);
                }
            }
        }
        best_tile
    }

    #[inline]
    fn tile_hash(&self, gx: i32, gy: i32, salt: u64) -> u64 {
        let mut h = self.world_seed.wrapping_add(salt);
        h ^= (gx as i64 as u64).wrapping_mul(0x9E3779B97F4A7C15);
        h = h.rotate_left(27).wrapping_mul(0x94D049BB133111EB);
        h ^= (gy as i64 as u64).wrapping_mul(0xBF58476D1CE4E5B9);
        h ^ (h >> 31)
    }

    /// Generates a 12x12 sector of the infinite lunar world, including craters, resource veins,
    /// and INHABITABLE multi-block bases & structures you can walk into and settle!
    pub fn generate_sector(&mut self, sx: i32, sy: i32, _scenario: MissionScenario) {
        if !self.generated_sectors.insert((sx, sy)) {
            return;
        }

        let x0 = sx * 12;
        let y0 = sy * 12;
        let sec_hash = self.tile_hash(sx, sy, 0xCAFE_BABE);
        let is_origin_area = sx.abs() <= 1 && sy.abs() <= 1;

        let crater_cx = x0 + 6 + ((sec_hash % 5) as i32 - 2);
        let crater_cy = y0 + 6 + (((sec_hash >> 8) % 5) as i32 - 2);
        use crate::planets::WorldBody;
        let (frequency, radius, depth) = match self.body {
            WorldBody::Moon => (68, 3, 0.50),
            WorldBody::Mars => (36, 3, 0.35),
            WorldBody::Venus => (86, 4, 0.90),
        };
        let has_crater = !is_origin_area && (sec_hash % 100) < frequency;
        let crater_r = radius + ((sec_hash >> 16) % 3) as i32;
        let crater_depth = -depth - ((sec_hash >> 24) % 20) as f32 * 0.01;

        let campaign_points = self.survey_points();
        for gy in y0..(y0 + 12) {
            for gx in x0..(x0 + 12) {
                if self.cells.contains_key(&(gx, gy)) {
                    continue;
                }
                let dist_origin = ((gx * gx + gy * gy) as f32).sqrt();
                let h = (self.tile_hash(gx, gy, 1) % 100) as u32;

                let mut kind = if (gx + gy * 2).rem_euclid(7) == 0 || dist_origin > 15.0 && h < 35 {
                    TileKind::HighlandRegolith
                } else {
                    TileKind::MariaBasalt
                };
                let phase = (self.world_seed % 1024) as f32 * 0.013;
                let mut elevation = match self.body {
                    WorldBody::Moon => {
                        (gx as f32 * 0.23 + phase).sin() * (gy as f32 * 0.23 - phase).cos() * 0.22
                    }
                    WorldBody::Mars => {
                        (gx as f32 * 0.16 + phase).sin() * 0.14 + (gy as f32 * 0.30).cos() * 0.08
                    }
                    WorldBody::Venus => {
                        (gx as f32 * 0.09 + phase).sin() * (gy as f32 * 0.11 + phase).cos() * 0.42
                            + ((gx + gy) as f32 * 0.8).sin() * 0.07
                    }
                };

                let starter_craters = match self.body {
                    WorldBody::Moon => [
                        (-9, -7, 4, -0.55),
                        (10, -8, 5, -0.60),
                        (-10, 9, 4, -0.50),
                        (9, 10, 4, -0.48),
                    ],
                    WorldBody::Mars => [
                        (-14, 8, 3, -0.28),
                        (16, 13, 4, -0.32),
                        (-8, -17, 3, -0.25),
                        (23, -9, 4, -0.35),
                    ],
                    WorldBody::Venus => [
                        (12, 11, 6, -0.85),
                        (-17, -12, 5, -0.95),
                        (21, -16, 5, -0.75),
                        (-20, 18, 6, -0.90),
                    ],
                };
                for (cx, cy, r, depth) in starter_craters {
                    let d = (((gx - cx).pow(2) + (gy - cy).pow(2)) as f32).sqrt();
                    if (d - r as f32).abs() <= 1.1 {
                        kind = TileKind::CraterRim;
                        elevation = 0.52;
                    } else if d < (r - 1) as f32 {
                        kind = TileKind::CraterFloor;
                        elevation = depth;
                    }
                }

                if has_crater && dist_origin > 12.0 {
                    let d = (((gx - crater_cx).pow(2) + (gy - crater_cy).pow(2)) as f32).sqrt();
                    if (d - crater_r as f32).abs() <= 1.1 {
                        kind = TileKind::CraterRim;
                        elevation = 0.55;
                    } else if d < (crater_r - 1) as f32 {
                        kind = TileKind::CraterFloor;
                        elevation = crater_depth;
                    }
                }

                // Sparse geological regions: tiles in a seam share the same mineral.
                let rx = gx.div_euclid(18);
                let ry = gy.div_euclid(18);
                let vein = self.tile_hash(rx, ry, 0xDE90517);
                let center = Vec2::new(
                    (rx * 18 + 3 + (vein % 12) as i32) as f32,
                    (ry * 18 + 3 + ((vein >> 12) % 12) as i32) as f32,
                );
                let radius = 1.3 + ((vein >> 24) % 12) as f32 * 0.1;
                let mineral = match (vein >> 32) % 10 {
                    0..=3 => TileKind::SubsurfaceIceVein,
                    4..=6 => TileKind::TitaniumOutcrop,
                    7..=8 => TileKind::Helium3Deposit,
                    _ => TileKind::RareMeteoriteDeposit,
                };
                if dist_origin > 5.0
                    && vein % 100 < 42
                    && center.distance(Vec2::new(gx as f32, gy as f32)) <= radius
                {
                    kind = mineral;
                }
                // Two small seams keep a first expedition possible.
                let offset = self.body.portrait_index() as i32 * 3;
                if IVec2::new(gx, gy).distance_squared(IVec2::new(9 + offset, -3 - offset)) <= 1 {
                    kind = TileKind::SubsurfaceIceVein;
                }
                if IVec2::new(gx, gy).distance_squared(IVec2::new(-8 - offset, 9 + offset)) <= 1 {
                    kind = TileKind::TitaniumOutcrop;
                }
                if dist_origin <= 3.5 {
                    kind = TileKind::MariaBasalt;
                    elevation = 0.0;
                }
                if self.body == WorldBody::Mars && dist_origin > 4.0 {
                    elevation += 0.33 * (gx as f32 * 0.32 + gy as f32 * 0.11).sin();
                }
                if IVec2::new(gx, gy) == self.body.survey()[1].pos && self.body == WorldBody::Mars {
                    kind = TileKind::SubsurfaceIceVein;
                    if self.body == WorldBody::Venus {
                        elevation = -1.05;
                    }
                }
                if campaign_points
                    .iter()
                    .any(|point| point.pos.distance_squared(IVec2::new(gx, gy)) <= 4)
                {
                    elevation = if (gx, gy) == (6, 4) && self.body == WorldBody::Venus {
                        -0.92
                    } else {
                        0.0
                    };
                }

                if self.body == WorldBody::Venus && kind == TileKind::SubsurfaceIceVein {
                    kind = TileKind::MariaBasalt;
                }
                let resource_yield = crate::mineral::initial_yield(kind);

                self.cells.insert(
                    (gx, gy),
                    TileCell {
                        kind,
                        elevation,
                        resource_yield,
                        scanned: dist_origin <= 10.0,
                        structure: None,
                        structure_hp: 0.0,
                        charge_wh: 0.0,
                        paid_cost: None,
                        feature: None,
                        is_world_generated_base: false,
                        tile_entity: None,
                        feature_entity: None,
                    },
                );
            }
        }

        // Starter discoveries & a ready-to-claim Outpost near the landing zone!
        if self.body != WorldBody::Moon {
            if !is_origin_area && sec_hash % 1000 < 35 {
                self.set_pre_structure(crater_cx, crater_cy, StructureKind::HabitatModule);
                self.set_pre_structure(crater_cx - 1, crater_cy, StructureKind::Airlock);
                self.set_pre_structure(
                    crater_cx,
                    crater_cy + 1,
                    if self.body == WorldBody::Venus {
                        StructureKind::RtgGenerator
                    } else {
                        StructureKind::SolarArray
                    },
                );
                self.set_pre_structure(crater_cx + 1, crater_cy, StructureKind::OxygenElectrolyzer);
            }
            if sx == 0 && sy == 0 {
                self.set_feature(4, 3, WorldFeatureKind::SupplyDropPod);
            }
            return;
        }
        if sx == 0 && sy == 0 {
            self.set_feature(4, 3, WorldFeatureKind::SupplyDropPod);
            self.set_feature(7, 5, WorldFeatureKind::ApolloLrvWreck);
        } else if sx == -1 && sy == 0 {
            self.set_feature(-4, 3, WorldFeatureKind::LunarFlagPlaque);
            self.set_feature(-7, 6, WorldFeatureKind::TwardowskiRooster);
        } else if sx == 0 && sy == -1 {
            // A nearby abandoned Polish Outpost you can walk right into via its Airlock and settle!
            self.set_pre_structure(6, -6, StructureKind::HabitatModule);
            self.set_pre_structure(5, -6, StructureKind::Airlock);
            self.set_pre_structure(6, -7, StructureKind::SolarArray);
            self.set_feature(4, -6, WorldFeatureKind::LunarFlagPlaque);
            self.set_feature(7, -6, WorldFeatureKind::SupplyDropPod);
        } else if sx == -1 && sy == -1 {
            self.set_feature(-6, -6, WorldFeatureKind::CrashedLunaProbe);
        }

        // Spawn Inhabitable Open-World Structures & Landmarks across outer sectors!
        if !is_origin_area {
            let poi_roll = sec_hash % 100;
            if poi_roll < 3 {
                // SETTLEABLE STRUCTURE 1: Baza Badawcza "Kopernik-0" (Full enterable habitat + airlock + solar + storage!)
                self.set_pre_structure(crater_cx, crater_cy, StructureKind::HabitatModule);
                self.set_pre_structure(crater_cx + 1, crater_cy, StructureKind::HabitatModule);
                self.set_pre_structure(crater_cx - 1, crater_cy, StructureKind::Airlock);
                self.set_pre_structure(crater_cx, crater_cy + 1, StructureKind::SolarArray);
                self.set_pre_structure(crater_cx + 1, crater_cy + 1, StructureKind::StorageBay);
                self.set_feature(
                    crater_cx - 1,
                    crater_cy + 1,
                    WorldFeatureKind::LunarFlagPlaque,
                );
                self.set_feature(crater_cx + 2, crater_cy, WorldFeatureKind::SupplyDropPod);
            } else if poi_roll < 5 {
                // SETTLEABLE STRUCTURE 2: Stacja Badawcza przy Obelisku "Niezwyciężony" S. Lema
                self.set_feature(crater_cx, crater_cy, WorldFeatureKind::LemMonolith);
                self.set_pre_structure(crater_cx - 2, crater_cy, StructureKind::ScienceLab);
                self.set_pre_structure(crater_cx - 3, crater_cy, StructureKind::Airlock);
                self.set_pre_structure(crater_cx - 2, crater_cy + 1, StructureKind::RtgGenerator);
                self.set_feature(
                    crater_cx + 2,
                    crater_cy,
                    WorldFeatureKind::GlowingCrystalCluster,
                );
                self.set_feature(crater_cx, crater_cy + 2, WorldFeatureKind::SupplyDropPod);
            } else if poi_roll < 7 {
                // SETTLEABLE STRUCTURE 3: Obozowisko Ocalałych przy Wraku Frachtowca
                self.set_feature(crater_cx, crater_cy, WorldFeatureKind::HeavyCargoWreck);
                self.set_pre_structure(crater_cx + 1, crater_cy, StructureKind::HabitatModule);
                self.set_pre_structure(crater_cx + 2, crater_cy, StructureKind::Airlock);
                self.set_pre_structure(crater_cx + 1, crater_cy - 1, StructureKind::StorageBay);
                self.set_feature(
                    crater_cx - 1,
                    crater_cy + 1,
                    WorldFeatureKind::SupplyDropPod,
                );
                self.set_feature(
                    crater_cx + 2,
                    crater_cy + 1,
                    WorldFeatureKind::ApolloLrvWreck,
                );
            } else if poi_roll < 9 {
                // SETTLEABLE STRUCTURE 4: Kopalnia Lodu z Domkiem Górniczym i Elektrolizerem O2
                self.set_feature(crater_cx, crater_cy, WorldFeatureKind::AutomatedIceRig);
                self.set_pre_structure(crater_cx + 1, crater_cy, StructureKind::OxygenElectrolyzer);
                self.set_pre_structure(crater_cx + 1, crater_cy + 1, StructureKind::HabitatModule);
                self.set_pre_structure(crater_cx + 2, crater_cy + 1, StructureKind::Airlock);
                self.set_feature(crater_cx - 1, crater_cy, WorldFeatureKind::SupplyDropPod);
            } else if poi_roll < 19 {
                let small_kind = match (sec_hash >> 7) % 6 {
                    0 => WorldFeatureKind::SupplyDropPod,
                    1 => WorldFeatureKind::ApolloLrvWreck,
                    2 => WorldFeatureKind::CrashedLunaProbe,
                    3 => WorldFeatureKind::TwardowskiRooster,
                    4 => WorldFeatureKind::GlowingCrystalCluster,
                    _ => WorldFeatureKind::LunarFlagPlaque,
                };
                self.set_feature(crater_cx, crater_cy, small_kind);
                if (sec_hash >> 13) % 10 == 0 {
                    self.set_feature(
                        x0 + 2 + ((sec_hash >> 19) % 8) as i32,
                        y0 + 2 + ((sec_hash >> 23) % 8) as i32,
                        WorldFeatureKind::SupplyDropPod,
                    );
                }
            }
        }
    }

    fn set_feature(&mut self, gx: i32, gy: i32, feat: WorldFeatureKind) {
        if gx.abs() <= 2 && gy.abs() <= 2 {
            return;
        }
        if let Some(cell) = self.cells.get_mut(&(gx, gy)) {
            if cell.structure.is_none() {
                cell.feature = Some(feat);
            }
        }
    }

    fn set_pre_structure(&mut self, gx: i32, gy: i32, st: StructureKind) {
        if self
            .survey_points()
            .iter()
            .any(|point| point.pos.distance_squared(IVec2::new(gx, gy)) <= 4)
        {
            return;
        }
        if gx.abs() <= 2 && gy.abs() <= 2 {
            return;
        }
        if let Some(cell) = self.cells.get_mut(&(gx, gy)) {
            cell.feature = None;
            cell.elevation = 0.0; // Flatten foundation so generated bases look clean and level!
            cell.structure = Some(st);
            cell.structure_hp = 100.0;
            cell.is_world_generated_base = true;
        }
    }
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
#[allow(dead_code)]
pub enum SurfaceUiCmd {
    UpgradeBackpack,
    GrantCargo(usize, u32),
    SetDemolitionMode,
    SetBuildMode(bool),
    SelectStructure(usize),
    CraftNextTool,
    UseRation,
    DrinkWater,
    MakeWater,
    MakeMeal,
    UseMedkit,
    PatchSuit,
    PlantPolishFlag,
    SwitchCrewMember,
    MountOrExitRover,
    EnterOrExitBuilding,
    PilotStation,
    AddWaypointAtPlayer,
    MarkAt(i32, i32),
    RemoveLastWaypoint,
    FabricateRover,
    ThrowSurveyFlare,
    MineOrBuildAtPlayer,
    TriggerJump,
    StoreCargo,
    TakeCargo,
    DropCargo(usize, bool),
    ServerMine(i32, i32),
    ServerBuild(i32, i32, usize),
}

#[derive(Resource, Default)]
pub struct TouchAndUiInput {
    pub move_dir: Vec2,
    pub jetpack_held: bool,
    pub pointer_over_ui: bool,
    pub action_held: bool,
    pub cmds: Vec<SurfaceUiCmd>,
}

#[derive(Component)]
pub struct SurfaceEntity;

#[derive(Component)]
pub struct IsoTileMarker {
    pub gx: i32,
    pub gy: i32,
}

#[derive(Component)]
pub struct WorldFeatureSpriteMarker {
    pub gx: i32,
    pub gy: i32,
}

#[derive(Component)]
pub struct StructureSpriteMarker {
    pub gx: i32,
    pub gy: i32,
    pub is_roof: bool,
    pub is_interior_floor: bool,
}

#[derive(Component)]
pub struct StructureConnectorMarker {
    pub gx: i32,
    pub gy: i32,
}

#[derive(Component)]
pub struct BuildGhostMarker;

#[derive(Component)]
pub struct MiningCrackMarker;

#[derive(Clone, Copy, Debug)]
pub enum ShadowOwnerKind {
    Player,
    Rover(Entity),
    Crew(Entity),
    Station,
}

#[derive(Component)]
pub struct CastShadowMarker {
    pub owner_kind: ShadowOwnerKind,
    pub base_scale: Vec2,
}

#[derive(Component)]
pub struct CursorMarker;

#[derive(Component, Clone)]
pub struct AstronautPlayer {
    pub iso_pos: Vec3,
    pub iso_vel: Vec3,
    pub grounded: bool,
    pub mounted_rover: Option<Entity>,
    pub is_passenger: bool,
    pub facing_right: bool,
    /// Continuous screen-space facing angle in radians (`-PI/2` = Front/South, `0` = Right/East, `+PI/2` = Back/North, `PI` = Left/West)
    pub facing_angle_rad: f32,
    pub target_facing_rad: f32,
    pub walk_cycle: f32,
    pub action_swing: f32,
    /// True when the astronaut has entered inside a pressurized building via an Airlock!
    pub inside_building: bool,
    pub airlock_anim_timer: f32,
    pub airlock_entering: bool,
    pub airlock_target_iso: Vec2,
}

#[derive(Component)]
pub struct CoOpCrewMember {
    pub peer_index: usize,
    pub walk_cycle: f32,
}

#[derive(Component)]
pub struct AstronautVisorMarker;

#[derive(Component)]
pub struct AstronautAccessoryMarker;

#[derive(Component, Clone)]
pub struct RoverVehicle {
    pub kind: RoverKind,
    pub iso_pos: Vec3,
    pub iso_vel: Vec2,
    pub heading_rad: f32,
    pub battery: f32,
    pub cargo_stored: u32,
    pub wheel_anim: f32,
}

#[derive(Component, Clone)]
pub struct ThrownObject {
    pub iso_pos: Vec3,
    pub iso_vel: Vec3,
    pub ttl: f32,
    pub is_flare: bool,
    pub is_meteor: bool,
}

#[derive(Component)]
pub struct DustParticle {
    pub iso_pos: Vec3,
    pub iso_vel: Vec3,
    pub life: f32,
    pub max_life: f32,
    pub size: f32,
    pub color: Color,
}

#[derive(Resource, Clone, serde::Serialize, serde::Deserialize)]
pub struct SurvivalStats {
    #[serde(default)]
    pub placement_ready_at: f32,
    #[serde(default)]
    pub creative_cargo: bool,
    #[serde(default)]
    pub backpack_level: u8,
    pub campaign: Option<crate::campaign::Campaign>,
    pub health: f32,
    pub oxygen: f32,
    pub max_oxygen: f32,
    pub oxygen_use_per_sec: f32,
    pub suit_power: f32,
    pub max_suit_power: f32,
    pub body_temp_c: f32,
    pub outside_temperature_c: f32,
    pub local_power_w: f32,
    pub local_powered: bool,
    pub local_oxygen: bool,
    pub local_charge: f32,
    pub lab_progress: f32,
    pub hunger: f32,
    pub hydration: f32,
    pub radiation: f32,
    pub suit_integrity: f32,
    pub flare_ready_at: f32,
    pub regolith: u32,
    pub water_ice: u32,
    pub titanium: u32,
    pub silicon: u32,
    pub helium3: u32,
    pub rare_earth: u32,
    pub rations: u32,
    pub fresh_meals: u32,
    pub drinking_water: u32,
    pub medkits: u32,
    pub raw_samples: u32,
    pub mineral_catalog: u8,
    pub geology_report_delivered: bool,
    pub collected_minerals: bool,
    pub flew_station: bool,
    pub highest_tool: ToolTier,
    pub generated_science_gb: f32,
    pub transmitted_science_gb: f32,
    pub base_power_gen_w: f32,
    pub base_power_load_w: f32,
    pub base_battery_wh: f32,
    pub max_base_battery_wh: f32,
    pub comms_downlink_mbps: f32,
    pub elapsed_sols: f32,
    pub death_reason: String,
    pub status_log: String,
    // Progression Tree & Achievements tracking
    pub blocks_mined: u32,
    pub entered_building_once: bool,
    pub built_habitat_once: bool,
    pub built_power_and_o2: bool,
    pub placed_waypoint_once: bool,
    pub settled_world_structure: bool,
    pub drove_rover_once: bool,
    pub planted_flag_once: bool,
}

impl SurvivalStats {
    pub const CARGO_LIMIT_KG: f32 = 35.0;
    pub fn cargo_limit(&self) -> f32 {
        if self.creative_cargo {
            f32::INFINITY
        } else {
            Self::CARGO_LIMIT_KG + 15.0 * f32::from(self.backpack_level.min(5))
        }
    }

    pub fn cargo_counts(&self) -> [u32; 11] {
        [
            self.regolith,
            self.silicon,
            self.water_ice,
            self.titanium,
            self.helium3,
            self.rare_earth,
            self.raw_samples,
            self.rations,
            self.fresh_meals,
            self.drinking_water,
            self.medkits,
        ]
    }

    pub fn cargo_weight(&self) -> f32 {
        self.cargo_counts()
            .iter()
            .enumerate()
            .map(|(i, n)| *n as f32 * Self::item_weight(i))
            .sum()
    }

    pub fn item_weight(kind: usize) -> f32 {
        [0.25, 0.35, 0.5, 0.6, 0.2, 0.6, 0.3, 0.4, 0.5, 0.5, 0.3][kind.min(10)]
    }

    pub fn change_cargo(&mut self, kind: usize, delta: i32) {
        let slot = match kind {
            0 => &mut self.regolith,
            1 => &mut self.silicon,
            2 => &mut self.water_ice,
            3 => &mut self.titanium,
            4 => &mut self.helium3,
            5 => &mut self.rare_earth,
            6 => &mut self.raw_samples,
            7 => &mut self.rations,
            8 => &mut self.fresh_meals,
            9 => &mut self.drinking_water,
            _ => &mut self.medkits,
        };
        *slot = if delta < 0 {
            slot.saturating_sub(delta.unsigned_abs())
        } else {
            slot.saturating_add(delta as u32)
        };
    }

    pub fn fits_cargo(&self, kind: usize, amount: u32) -> bool {
        self.cargo_weight() + Self::item_weight(kind) * amount as f32 <= self.cargo_limit() + 0.001
    }

    pub fn from_config(config: &MissionConfig) -> Self {
        let max_o2 = if config.backpack_module == BackpackModule::CryoO2TwinTank {
            150.0
        } else {
            100.0
        };
        let max_pwr = if config.backpack_module == BackpackModule::MicroRtgCell {
            150.0
        } else {
            100.0
        };

        Self {
            campaign: None,
            placement_ready_at: 0.0,
            creative_cargo: config.scenario == MissionScenario::OpenSandbox,
            backpack_level: 0,
            health: 100.0,
            oxygen: max_o2,
            max_oxygen: max_o2,
            oxygen_use_per_sec: 0.0,
            suit_power: max_pwr,
            max_suit_power: max_pwr,
            body_temp_c: 37.0,
            outside_temperature_c: 0.0,
            local_power_w: 0.0,
            local_powered: false,
            local_oxygen: false,
            local_charge: 0.0,
            lab_progress: 0.0,
            hunger: 100.0,
            hydration: 100.0,
            radiation: 0.0,
            suit_integrity: 100.0,
            flare_ready_at: 0.0,
            regolith: 0,
            water_ice: 0,
            titanium: 0,
            silicon: 0,
            helium3: 0,
            rare_earth: 0,
            rations: 2,
            fresh_meals: 0,
            drinking_water: 2,
            medkits: 0,
            raw_samples: 0,
            mineral_catalog: 0,
            geology_report_delivered: false,
            collected_minerals: false,
            flew_station: false,
            highest_tool: ToolTier::GeoHammer,
            generated_science_gb: 0.0,
            transmitted_science_gb: 0.0,
            base_power_gen_w: config.total_power_supply_w(),
            base_power_load_w: config.total_power_demand_w(),
            base_battery_wh: 720.0,
            max_base_battery_wh: 1000.0,
            comms_downlink_mbps: config.effective_comms_capacity_mbps(),
            elapsed_sols: 0.1,
            death_reason: String::new(),
            status_log: crate::locale::text("Witaj na Księżycu! Ruszaj klawiszami W, A, S, D.")
                .into(),
            blocks_mined: 0,
            entered_building_once: false,
            built_habitat_once: false,
            built_power_and_o2: false,
            placed_waypoint_once: false,
            settled_world_structure: false,
            drove_rover_once: false,
            planted_flag_once: false,
        }
    }

    /// 8 sequential steps on the Progression Map (Mapa Postępu):
    /// Step 0..8 — returns whether the given step has been completed.
    pub fn is_progression_step_done(&self, step_idx: usize, map: &LunarMap) -> bool {
        match step_idx {
            0 => self.blocks_mined >= 2,
            1 => self.entered_building_once,
            2 => self.built_habitat_once,
            3 => self.built_power_and_o2,
            4 => (self.highest_tool as u8) >= (ToolTier::ThermalIceExtractor as u8),
            5 => self.placed_waypoint_once || self.settled_world_structure,
            6 => self.drove_rover_once,
            7 => {
                let total_buildings = map
                    .cells
                    .values()
                    .filter(|c| c.structure.is_some() && c.structure_hp > 0.0)
                    .count();
                total_buildings >= 7 && (self.highest_tool as u8) >= (ToolTier::PlasmaCutter as u8)
            }
            _ => false,
        }
    }

    /// Returns the index `0..8` of the current active step that the player should do next!
    /// Any step > `current_active_step` that is not yet done is rendered in GRAY (locked) on the Progression Map!
    pub fn current_active_step(&self, map: &LunarMap) -> usize {
        for i in 0..8 {
            if !self.is_progression_step_done(i, map) {
                return i;
            }
        }
        8
    }
}

pub fn setup_lunar_surface(
    mut commands: Commands,
    assets: Res<ProceduralAssets>,
    config: Res<MissionConfig>,
    lan: Res<LanMultiplayer>,
    mut map: ResMut<LunarMap>,
) {
    let mut stats = SurvivalStats::from_config(&config);
    map.quests_disabled = config.scenario == MissionScenario::OpenSandbox && !config.sandbox_quests;
    if map.campaign_route {
        stats.campaign = Some(crate::campaign::Campaign::default());
        stats.status_log =
            crate::locale::text("Witaj! Najpierw otwórz skrzynię wskazaną złotym znacznikiem.")
                .into();
    }
    commands.insert_resource(stats);
    spawn_planet_surface(&mut commands, &assets, &config, &lan, &mut map, true);
}

pub fn spawn_planet_surface(
    commands: &mut Commands,
    assets: &ProceduralAssets,
    config: &MissionConfig,
    lan: &LanMultiplayer,
    map: &mut LunarMap,
    new_world: bool,
) {
    map.networks.clear();
    map.network_elapsed = 0.0;
    if new_world {
        map.cells.clear();
        map.generated_sectors.clear();
        map.selected_build = 1; // Default to Habitat on slot [2]
        map.station_position = Vec3::ZERO;
        map.station_parked = Vec3::ZERO;
        map.piloting_station = false;
        map.build_mode = false;
        map.close_all_modals();
        map.last_stream_center = (9999, 9999);
        map.world_seed = map.body.seed();
        map.mining_target = None;
        map.mining_progress = 0.0;
        map.mining_label.clear();

        // Starter Waypoints on the Map [M] so the player immediately sees landmarks & outposts!
        map.waypoints = vec![
            Waypoint {
                name: crate::locale::text("Lądownik i Dom Startowy").to_string(),
                gx: 0,
                gy: 0,
                color: Color::rgb(0.35, 0.96, 1.0),
            },
            Waypoint {
                name: crate::locale::text("Porzucona Baza 'Kopernik-0' (Osiedl się!)").to_string(),
                gx: 6,
                gy: -6,
                color: Color::rgb(0.42, 1.0, 0.65),
            },
            Waypoint {
                name: crate::locale::text("Wrak Łazika inż. Bekkera").to_string(),
                gx: 7,
                gy: 5,
                color: Color::rgb(1.0, 0.84, 0.32),
            },
            Waypoint {
                name: crate::locale::text("Kogut Pana Twardowskiego").to_string(),
                gx: -7,
                gy: 6,
                color: Color::rgb(1.0, 0.45, 0.55),
            },
        ];

        if map.body != crate::planets::WorldBody::Moon {
            map.waypoints.truncate(1);
        }
        for point in map.survey_points() {
            map.waypoints.push(Waypoint {
                name: point.name.into(),
                gx: point.pos.x,
                gy: point.pos.y,
                color: Color::rgb(0.73, 0.52, 1.0),
            });
        }
    } else {
        map.piloting_station = false;
        map.close_all_modals();
        map.mining_target = None;
        map.mining_progress = 0.0;
        map.last_stream_center = (9999, 9999);
    }

    let station = map.station_position;
    map.station_previous = station;
    stream_infinite_chunks_around(
        commands,
        assets,
        map,
        station.x.round() as i32,
        station.y.round() as i32,
        config.scenario,
    );

    if new_world {
        // Spawn a connected starter House (Airlock at (2,0) + Habitat at (3,0)) + Solar Array at (0,2)
        // so the player can immediately test entering their house through the Airlock with [E]!
        spawn_structure_at(commands, assets, map, 2, 0, StructureKind::Airlock);
        spawn_structure_at(commands, assets, map, 3, 0, StructureKind::HabitatModule);
        spawn_structure_at(commands, assets, map, 2, 1, StructureKind::SolarArray);
        if config.difficulty == crate::types::DifficultyLevel::Cadet
            || map.body != crate::planets::WorldBody::Moon
        {
            spawn_structure_at(
                commands,
                assets,
                map,
                3,
                1,
                StructureKind::OxygenElectrolyzer,
            );
        }
        if config.difficulty == crate::types::DifficultyLevel::Cadet {
            spawn_structure_at(commands, assets, map, 4, 0, StructureKind::HydroponicsDome);
        }
        if map.campaign_route
            || map.body == crate::planets::WorldBody::Mars
            || (map.body == crate::planets::WorldBody::Moon
                && config.difficulty == crate::types::DifficultyLevel::Cadet)
        {
            spawn_structure_at(commands, assets, map, 3, -1, StructureKind::RtgGenerator);
        }
        if map.campaign_route {
            spawn_structure_at(commands, assets, map, 2, -1, StructureKind::StorageBay);
            if map.body == crate::planets::WorldBody::Moon {
                spawn_structure_at(commands, assets, map, 5, 0, StructureKind::RtgGenerator);
                map.cells.get_mut(&(5, 0)).unwrap().is_world_generated_base = true;
            }
            for (x, y) in [(2, 0), (3, 0), (2, 1), (3, 1), (4, 0), (3, -1), (4, -1)] {
                if let Some(cell) = map.cells.get_mut(&(x, y)) {
                    if cell.structure.is_some() {
                        cell.is_world_generated_base = true;
                    }
                }
            }
            map.cells.get_mut(&(2, -1)).unwrap().is_world_generated_base = true;
        }
    }

    let lander_world = iso_to_world(station.x, station.y, station.z);
    commands
        .spawn((
            SurfaceEntity,
            crate::flight::FlyingStation,
            SpriteBundle {
                texture: assets.lander_tex.clone(),
                transform: Transform::from_xyz(
                    lander_world.x,
                    lander_world.y + 36.0,
                    iso_depth(station.x, station.y, station.z, 2.2),
                ),
                ..default()
            },
        ))
        .with_children(|ship| {
            for x in [-16.0, 16.0] {
                ship.spawn((
                    crate::flight::StationThruster(x),
                    SpriteBundle {
                        texture: assets.thrust_tex.clone(),
                        sprite: Sprite {
                            custom_size: Some(Vec2::new(25.0, 32.0)),
                            ..default()
                        },
                        transform: Transform::from_xyz(x, -43.0, -0.3),
                        visibility: Visibility::Hidden,
                        ..default()
                    },
                ));
            }
        });

    commands.spawn((
        SurfaceEntity,
        CastShadowMarker {
            owner_kind: ShadowOwnerKind::Station,
            base_scale: Vec2::new(1.85, 1.45),
        },
        SpriteBundle {
            texture: assets.shadow_disc.clone(),
            transform: Transform::from_xyz(
                lander_world.x,
                lander_world.y,
                iso_shadow_depth(0.0, 0.0),
            ),
            ..default()
        },
    ));

    // Spawn Pre-Landed Rover
    let initial_rover_kind = match config.prelanded_rover {
        PrelandedRoverChoice::HeavyPressurizedCruiser => Some(RoverKind::HeavyCruiser),
        PrelandedRoverChoice::LightUtilityRover => Some(RoverKind::LightBuggy),
        PrelandedRoverChoice::NoneBuildOnSurface => None,
    };
    let initial_rover_kind = if map.campaign_route {
        Some(RoverKind::HeavyCruiser)
    } else {
        initial_rover_kind
    };

    if let Some(r_kind) = initial_rover_kind.filter(|_| new_world) {
        spawn_rover_entity(commands, assets, r_kind, Vec3::new(-2.5, 1.5, 0.0));
    }

    // Spawn Co-Op / LAN Crewmates
    for idx in 0..6 {
        let fallback = crate::net_lan::default_ai_crewmates();
        let peer = lan.peers.get(idx).unwrap_or(&fallback[0]);
        let p_world = iso_to_world(peer.iso_pos.x, peer.iso_pos.y, peer.iso_pos.z);
        let p_z = iso_depth(peer.iso_pos.x, peer.iso_pos.y, peer.iso_pos.z, 1.95);
        let crew_ent = commands
            .spawn((
                SurfaceEntity,
                CoOpCrewMember {
                    peer_index: idx,
                    walk_cycle: idx as f32 * 1.4,
                },
                SpriteBundle {
                    texture: assets.astronaut_dir_frames[0][0].clone(),
                    sprite: Sprite {
                        color: peer.suit.to_color(),
                        ..default()
                    },
                    transform: Transform::from_xyz(p_world.x, p_world.y + 22.0, p_z),
                    ..default()
                },
            ))
            .with_children(|parent| {
                parent.spawn((
                    crate::net_visuals::CrewVisor,
                    SpriteBundle {
                        texture: assets.visor_dir_tex[0].clone(),
                        transform: Transform::from_xyz(0.0, 0.0, 0.2),
                        ..default()
                    },
                ));
                parent.spawn((
                    crate::net_visuals::CrewAccessory,
                    SpriteBundle {
                        texture: assets.accessory_handle(peer.accessory),
                        transform: Transform::from_xyz(0.0, 0.0, 0.3),
                        ..default()
                    },
                ));
            })
            .id();

        commands.spawn((
            SurfaceEntity,
            CastShadowMarker {
                owner_kind: ShadowOwnerKind::Crew(crew_ent),
                base_scale: Vec2::new(0.80, 0.70),
            },
            SpriteBundle {
                texture: assets.shadow_disc.clone(),
                transform: Transform::from_xyz(
                    p_world.x,
                    p_world.y,
                    iso_shadow_depth(peer.iso_pos.x, peer.iso_pos.y),
                ),
                ..default()
            },
        ));
    }

    // Spawn Player Astronaut at (1.5, 1.0, 0.0) facing South (-FRAC_PI_2) with Visor + Helmet Accessory
    let astro_xy = station.truncate() + Vec2::new(1.5, 1.0);
    let astro_xy = if map.blocks_movement(astro_xy.x, astro_xy.y) {
        station.truncate() + Vec2::new(-1.5, 0.0)
    } else {
        astro_xy
    };
    let astro_pos = astro_xy.extend(map.get_elevation(astro_xy.x, astro_xy.y));
    let astro_world = iso_to_world(astro_pos.x, astro_pos.y, astro_pos.z);
    let astro_z = iso_depth(astro_pos.x, astro_pos.y, astro_pos.z, 2.0);

    commands
        .spawn((
            SurfaceEntity,
            AstronautPlayer {
                iso_pos: astro_pos,
                iso_vel: Vec3::ZERO,
                grounded: true,
                mounted_rover: None,
                is_passenger: false,
                facing_right: true,
                facing_angle_rad: -FRAC_PI_2,
                target_facing_rad: -FRAC_PI_2,
                walk_cycle: 0.0,
                action_swing: 0.0,
                inside_building: false,
                airlock_anim_timer: 0.0,
                airlock_entering: false,
                airlock_target_iso: Vec2::ZERO,
            },
            SpriteBundle {
                texture: assets.astronaut_dir_frames[0][0].clone(),
                sprite: Sprite {
                    color: config.suit_color.to_color(),
                    ..default()
                },
                transform: Transform::from_xyz(astro_world.x, astro_world.y + 22.0, astro_z),
                ..default()
            },
        ))
        .with_children(|parent| {
            parent.spawn((
                AstronautVisorMarker,
                SpriteBundle {
                    texture: assets.visor_dir_tex[0].clone(),
                    sprite: Sprite {
                        color: config.visor_tint.to_color(),
                        ..default()
                    },
                    transform: Transform::from_xyz(0.0, 0.0, 0.2),
                    ..default()
                },
            ));
            parent.spawn((
                AstronautAccessoryMarker,
                SpriteBundle {
                    texture: assets.accessory_handle(config.accessory),
                    transform: Transform::from_xyz(0.0, 0.0, 0.3),
                    ..default()
                },
            ));
        });

    commands.spawn((
        SurfaceEntity,
        CastShadowMarker {
            owner_kind: ShadowOwnerKind::Player,
            base_scale: Vec2::new(0.86, 0.76),
        },
        SpriteBundle {
            texture: assets.shadow_disc.clone(),
            transform: Transform::from_xyz(
                astro_world.x,
                astro_world.y,
                iso_shadow_depth(astro_pos.x, astro_pos.y),
            ),
            ..default()
        },
    ));

    // Tile selection cursor + translucent Build Ghost Preview + Progressive Mining Crack Overlay!
    commands.spawn((
        SurfaceEntity,
        CursorMarker,
        SpriteBundle {
            texture: assets.tile_cursor.clone(),
            transform: Transform::from_xyz(0.0, 0.0, 860.0),
            ..default()
        },
    ));

    commands.spawn((
        SurfaceEntity,
        BuildGhostMarker,
        SpriteBundle {
            texture: assets.structure_handle(StructureKind::HabitatModule),
            sprite: Sprite {
                color: Color::rgba(0.35, 1.0, 0.65, 0.55),
                ..default()
            },
            visibility: Visibility::Hidden,
            transform: Transform::from_xyz(0.0, 0.0, 855.0),
            ..default()
        },
    ));

    commands.spawn((
        SurfaceEntity,
        MiningCrackMarker,
        SpriteBundle {
            texture: assets.crack_stage_tex[0].clone(),
            visibility: Visibility::Hidden,
            transform: Transform::from_xyz(0.0, 0.0, 858.0),
            ..default()
        },
    ));
}

/// Streams infinite lunar sectors & tile/feature sprites around `(center_gx, center_gy)`.
pub fn stream_infinite_chunks_around(
    commands: &mut Commands,
    assets: &ProceduralAssets,
    map: &mut LunarMap,
    center_gx: i32,
    center_gy: i32,
    scenario: MissionScenario,
) {
    if (center_gx - map.last_stream_center.0).abs() < 3
        && (center_gy - map.last_stream_center.1).abs() < 3
    {
        return;
    }
    map.last_stream_center = (center_gx, center_gy);

    let min_sx = (center_gx - STREAM_RADIUS - 6).div_euclid(12);
    let max_sx = (center_gx + STREAM_RADIUS + 6).div_euclid(12);
    let min_sy = (center_gy - STREAM_RADIUS - 6).div_euclid(12);
    let max_sy = (center_gy + STREAM_RADIUS + 6).div_euclid(12);

    for sy in min_sy..=max_sy {
        for sx in min_sx..=max_sx {
            map.generate_sector(sx, sy, scenario);
        }
    }

    let mut newly_visible_structures = Vec::new();

    for gy in (center_gy - STREAM_RADIUS)..=(center_gy + STREAM_RADIUS) {
        for gx in (center_gx - STREAM_RADIUS)..=(center_gx + STREAM_RADIUS) {
            let Some(cell) = map.cells.get_mut(&(gx, gy)) else {
                continue;
            };
            if (gx - center_gx).abs() <= 7 && (gy - center_gy).abs() <= 7 {
                cell.scanned = true;
            }
            if cell.tile_entity.is_none() {
                let world_pos = iso_to_world(gx as f32, gy as f32, cell.elevation);
                let z = iso_tile_depth(gx as f32, gy as f32, cell.elevation);
                let t_ent = commands
                    .spawn((
                        SurfaceEntity,
                        IsoTileMarker { gx, gy },
                        SpriteBundle {
                            texture: assets.terrain_handle(cell.kind, cell.elevation),
                            transform: Transform::from_xyz(world_pos.x, world_pos.y - 6.0, z),
                            ..default()
                        },
                    ))
                    .id();
                cell.tile_entity = Some(t_ent);
                commands.entity(t_ent).with_children(|parent| {
                    parent.spawn((
                        crate::mineral::DepositVisual { gx, gy },
                        SpriteBundle {
                            sprite: Sprite {
                                custom_size: Some(Vec2::new(88.0, 44.0)),
                                ..default()
                            },
                            texture: assets.deposit_tex[crate::mineral::deposit_index(cell.kind)]
                                .clone(),
                            transform: Transform::from_xyz(0.0, 6.0, 0.1),
                            visibility: Visibility::Hidden,
                            ..default()
                        },
                    ));
                });

                if let Some(feat) = cell.feature {
                    let crystal = feat == WorldFeatureKind::GlowingCrystalCluster;
                    let (texture, position, sprite) = if crystal {
                        (
                            assets.deposit_tex[1].clone(),
                            world_pos.extend(z + 0.2),
                            Sprite {
                                custom_size: Some(Vec2::new(84.0, 42.0)),
                                ..default()
                            },
                        )
                    } else {
                        (
                            assets.feature_handle(feat),
                            (world_pos + Vec2::Y * 22.0).extend(iso_depth(
                                gx as f32,
                                gy as f32,
                                cell.elevation,
                                1.5,
                            )),
                            Sprite::default(),
                        )
                    };
                    let f_ent = commands
                        .spawn((
                            SurfaceEntity,
                            WorldFeatureSpriteMarker { gx, gy },
                            SpriteBundle {
                                texture,
                                sprite,
                                transform: Transform::from_translation(position),
                                ..default()
                            },
                        ))
                        .id();
                    cell.feature_entity = Some(f_ent);
                }

                if let Some(st) = cell.structure {
                    newly_visible_structures.push((gx, gy, st));
                }
            }
        }
    }

    for (gx, gy, st) in newly_visible_structures {
        spawn_single_structure_sprites(commands, assets, map, gx, gy, st);
    }
}

pub fn spawn_rover_entity(
    commands: &mut Commands,
    assets: &ProceduralAssets,
    kind: RoverKind,
    iso_pos: Vec3,
) -> Entity {
    let w_pos = iso_to_world(iso_pos.x, iso_pos.y, iso_pos.z);
    let z = iso_depth(iso_pos.x, iso_pos.y, iso_pos.z, 2.1);
    let tex = match kind {
        RoverKind::LightBuggy => assets.rover_light_frames[0].clone(),
        RoverKind::HeavyCruiser => assets.rover_heavy_frames[0].clone(),
    };
    let rover_ent = commands
        .spawn((
            SurfaceEntity,
            RoverVehicle {
                kind,
                iso_pos,
                iso_vel: Vec2::ZERO,
                heading_rad: 0.0,
                battery: kind.max_battery(),
                cargo_stored: 0,
                wheel_anim: 0.0,
            },
            SpriteBundle {
                texture: tex,
                transform: Transform::from_xyz(w_pos.x, w_pos.y + 18.0, z),
                ..default()
            },
        ))
        .id();

    commands.spawn((
        SurfaceEntity,
        CastShadowMarker {
            owner_kind: ShadowOwnerKind::Rover(rover_ent),
            base_scale: Vec2::new(1.35, 1.05),
        },
        SpriteBundle {
            texture: assets.shadow_disc.clone(),
            transform: Transform::from_xyz(
                w_pos.x,
                w_pos.y,
                iso_shadow_depth(iso_pos.x, iso_pos.y),
            ),
            ..default()
        },
    ));

    rover_ent
}

fn spawn_single_structure_sprites(
    commands: &mut Commands,
    assets: &ProceduralAssets,
    map: &LunarMap,
    gx: i32,
    gy: i32,
    kind: StructureKind,
) {
    let elev = map.get_elevation(gx as f32, gy as f32);
    let w_pos = iso_to_world(gx as f32, gy as f32, elev);
    let base_z = iso_depth(gx as f32, gy as f32, elev, 1.4);

    let connected_neighbors = map.connected_neighbors_count(gx, gy, kind);
    let scale_boost = if connected_neighbors > 0 { 1.06 } else { 1.0 };

    // If this is a pressurized room (Airlock, Habitat, Hydroponics, Lab), spawn an Interior Floor layer
    // so when the astronaut walks inside and the roof cuts away, they walk on a warm lit deck!
    if kind.is_pressurized_room() {
        commands.spawn((
            SurfaceEntity,
            StructureSpriteMarker {
                gx,
                gy,
                is_roof: false,
                is_interior_floor: true,
            },
            SpriteBundle {
                texture: assets.interior_floor_tex.clone(),
                transform: Transform::from_xyz(w_pos.x, w_pos.y - 2.0, base_z - 0.5),
                ..default()
            },
        ));
    }

    commands.spawn((
        SurfaceEntity,
        StructureSpriteMarker {
            gx,
            gy,
            is_roof: false,
            is_interior_floor: false,
        },
        SpriteBundle {
            texture: assets.structure_handle(kind),
            transform: Transform::from_xyz(w_pos.x, w_pos.y + 20.0, base_z)
                .with_scale(Vec3::splat(scale_boost)),
            ..default()
        },
    ));

    if kind.is_pressurized_room() {
        commands.spawn((
            SurfaceEntity,
            StructureSpriteMarker {
                gx,
                gy,
                is_roof: true,
                is_interior_floor: false,
            },
            SpriteBundle {
                texture: assets.structure_roof_tex.clone(),
                transform: Transform::from_xyz(w_pos.x, w_pos.y + 22.0, base_z + 0.9)
                    .with_scale(Vec3::splat(scale_boost)),
                ..default()
            },
        ));
    }

    for (dx, dy) in [(1, 0), (0, 1)] {
        let nx = gx + dx;
        let ny = gy + dy;
        if let Some(ncell) = map.cells.get(&(nx, ny)) {
            if ncell.structure_hp > 0.0 {
                if let Some(nkind) = ncell.structure {
                    let unites = nkind == kind
                        || (kind.is_pressurized_room() && nkind.is_pressurized_room());
                    if unites {
                        let mid_x = gx as f32 + dx as f32 * 0.5;
                        let mid_y = gy as f32 + dy as f32 * 0.5;
                        let mid_elev = (elev + ncell.elevation) * 0.5;
                        let mid_world = iso_to_world(mid_x, mid_y, mid_elev);
                        let mid_z = iso_depth(mid_x, mid_y, mid_elev, 1.35);

                        commands.spawn((
                            SurfaceEntity,
                            StructureConnectorMarker { gx, gy },
                            SpriteBundle {
                                texture: assets.connector_handle(kind),
                                transform: Transform::from_xyz(
                                    mid_world.x,
                                    mid_world.y + 18.0,
                                    mid_z,
                                ),
                                ..default()
                            },
                        ));
                    }
                }
            }
        }
    }
}

pub fn spawn_structure_at(
    commands: &mut Commands,
    assets: &ProceduralAssets,
    map: &mut LunarMap,
    gx: i32,
    gy: i32,
    kind: StructureKind,
) {
    let mut existing = Vec::new();
    for (&(ax, ay), cell) in &map.cells {
        if (ax - gx).abs() <= 1 && (ay - gy).abs() <= 1 && cell.structure.is_some() {
            existing.push((ax, ay, cell.structure.unwrap()));
        }
    }
    if let Some(cell) = map.cells.get_mut(&(gx, gy)) {
        if let Some(f_ent) = cell.feature_entity.take() {
            commands.entity(f_ent).despawn_recursive();
        }
        cell.feature = None;
        cell.structure = Some(kind);
        cell.structure_hp = 100.0;
        cell.charge_wh = crate::base::capacity_wh(kind) * 0.75;
    }
    spawn_single_structure_sprites(commands, assets, map, gx, gy, kind);
}

/// Rebuilds the sprites and connectors for `(gx, gy)` and its 4 neighbors cleanly without glitches.
pub fn refresh_structure_cluster_around(
    commands: &mut Commands,
    assets: &ProceduralAssets,
    map: &LunarMap,
    gx: i32,
    gy: i32,
    struct_q: &Query<(Entity, &StructureSpriteMarker)>,
    conn_q: &Query<(Entity, &StructureConnectorMarker)>,
) {
    let affected = [
        (gx, gy),
        (gx - 1, gy),
        (gx + 1, gy),
        (gx, gy - 1),
        (gx, gy + 1),
    ];

    for (ent, sm) in struct_q.iter() {
        if affected.contains(&(sm.gx, sm.gy)) {
            commands.entity(ent).despawn_recursive();
        }
    }
    for (ent, cm) in conn_q.iter() {
        if affected.contains(&(cm.gx, cm.gy)) {
            commands.entity(ent).despawn_recursive();
        }
    }

    for &(ax, ay) in &affected {
        if let Some(cell) = map.cells.get(&(ax, ay)) {
            if cell.structure_hp > 0.0 {
                if let Some(kind) = cell.structure {
                    spawn_single_structure_sprites(commands, assets, map, ax, ay, kind);
                }
            }
        }
    }
}

#[allow(clippy::type_complexity)]
pub fn update_lighting_shadows_and_cutaways(
    settings: Res<crate::types::GameSettings>,
    time: Res<Time>,
    solar: Res<SolarSystemState>,
    map: Res<LunarMap>,
    mut gizmos: Gizmos,
    player_q: Query<&AstronautPlayer>,
    rover_q: Query<&RoverVehicle>,
    crew_q: Query<(&CoOpCrewMember, &Visibility)>,
    lan: Res<LanMultiplayer>,
    mut tile_q: Query<
        (&IsoTileMarker, &mut Sprite, &mut Visibility),
        (
            Without<StructureSpriteMarker>,
            Without<WorldFeatureSpriteMarker>,
            Without<CastShadowMarker>,
            Without<CoOpCrewMember>,
        ),
    >,
    mut feat_q: Query<
        (&WorldFeatureSpriteMarker, &mut Sprite, &mut Visibility),
        (
            Without<IsoTileMarker>,
            Without<StructureSpriteMarker>,
            Without<CastShadowMarker>,
            Without<CoOpCrewMember>,
        ),
    >,
    mut struct_q: Query<
        (&StructureSpriteMarker, &mut Sprite, &mut Visibility),
        (
            Without<IsoTileMarker>,
            Without<WorldFeatureSpriteMarker>,
            Without<CastShadowMarker>,
            Without<CoOpCrewMember>,
        ),
    >,
    mut shadow_q: Query<
        (&CastShadowMarker, &mut Transform, &mut Visibility),
        (
            Without<AstronautPlayer>,
            Without<IsoTileMarker>,
            Without<WorldFeatureSpriteMarker>,
            Without<StructureSpriteMarker>,
            Without<CoOpCrewMember>,
        ),
    >,
) {
    let sun = solar.sun_illumination();
    let ambient = (0.38 + sun * 0.62).clamp(0.36, 1.0);
    let night_blue = (1.0 - sun) * 0.16;
    let [r, g, b] = map.body.ground_tint();
    let tint = Color::rgb(
        ambient * r,
        (ambient + night_blue * 0.3).min(1.0) * g,
        (ambient + night_blue).min(1.0) * b,
    );
    let now = time.elapsed_seconds()
        + if lan.is_active && !lan.is_host {
            lan.server_offset
        } else {
            0.0
        };

    let (player_iso, inside_building, airlock_timer) = player_q
        .get_single()
        .map(|p| (p.iso_pos, p.inside_building, p.airlock_anim_timer))
        .unwrap_or((Vec3::ZERO, false, 0.0));
    let px = player_iso.x.round() as i32;
    let py = player_iso.y.round() as i32;

    for (marker, mut sprite, mut vis) in &mut tile_q {
        if (marker.gx - px).abs() > STREAM_RADIUS + 4 || (marker.gy - py).abs() > STREAM_RADIUS + 4
        {
            *vis = Visibility::Hidden;
            continue;
        }
        *vis = Visibility::Inherited;
        if let Some(cell) = map.cells.get(&(marker.gx, marker.gy)) {
            if cell.scanned {
                sprite.color = tint;
            } else {
                sprite.color = Color::rgb(tint.r() * 0.85, tint.g() * 0.85, tint.b() * 0.90);
            }
            if map.flare_lit(marker.gx, marker.gy, now) {
                sprite.color = Color::rgb(1.0, 0.87, 0.62);
            }
        }
    }

    for (f_marker, mut sprite, mut vis) in &mut feat_q {
        let alive = map
            .cells
            .get(&(f_marker.gx, f_marker.gy))
            .map(|c| c.feature.is_some())
            .unwrap_or(false);
        if !alive
            || (f_marker.gx - px).abs() > STREAM_RADIUS + 4
            || (f_marker.gy - py).abs() > STREAM_RADIUS + 4
        {
            *vis = Visibility::Hidden;
            continue;
        }
        *vis = Visibility::Inherited;
        sprite.color = tint;
    }

    // Building Cutaway when entering inside via Airlock!
    for (st_marker, mut sprite, mut vis) in &mut struct_q {
        let cell_alive = map
            .cells
            .get(&(st_marker.gx, st_marker.gy))
            .map(|c| c.structure.is_some() && c.structure_hp > 0.0)
            .unwrap_or(false);

        if !cell_alive {
            *vis = Visibility::Hidden;
            continue;
        }
        *vis = Visibility::Inherited;

        let dist_to_player = (st_marker.gx - px).abs().max((st_marker.gy - py).abs());
        let is_pressurized = map
            .cells
            .get(&(st_marker.gx, st_marker.gy))
            .and_then(|c| c.structure)
            .map(|s| s.is_pressurized_room())
            .unwrap_or(false);

        if st_marker.is_interior_floor {
            sprite.color = Color::rgb(1.0, 1.0, 1.0);
        } else if st_marker.is_roof {
            let alpha = if inside_building && dist_to_player <= 4 {
                0.06
            } else if dist_to_player <= 1 {
                0.22
            } else {
                0.94
            };
            sprite.color = Color::rgba(tint.r(), tint.g(), tint.b(), alpha);
        } else if inside_building && is_pressurized && dist_to_player <= 3 {
            // Cut away the outer shell to 28% opacity so you see the interior floor and your astronaut inside!
            sprite.color = Color::rgba(tint.r(), tint.g(), tint.b(), 0.28);
        } else {
            sprite.color = tint;
        }
    }

    // Draw Airlock Decompression / Compression Door Ring Animation when entering or exiting!
    if airlock_timer > 0.0 {
        let p_world = iso_to_world(player_iso.x, player_iso.y, player_iso.z) + Vec2::new(0.0, 18.0);
        let prog = 1.0 - (airlock_timer / 0.85).clamp(0.0, 1.0);
        gizmos.circle_2d(
            p_world,
            16.0 + prog * 22.0,
            Color::rgba(0.28, 0.96, 1.0, 0.88),
        );
        gizmos.circle_2d(
            p_world,
            26.0 * (1.0 - prog * 0.5),
            Color::rgba(0.35, 1.0, 0.65, 0.85),
        );
        // Sliding airlock door bars
        let door_gap = 4.0 + prog * 18.0;
        gizmos.line_2d(
            p_world + Vec2::new(-door_gap, -16.0),
            p_world + Vec2::new(-door_gap, 16.0),
            Color::rgb(1.0, 0.82, 0.28),
        );
        gizmos.line_2d(
            p_world + Vec2::new(door_gap, -16.0),
            p_world + Vec2::new(door_gap, 16.0),
            Color::rgb(1.0, 0.82, 0.28),
        );
    }

    // Draw On-Screen Waypoint Beacons & Crew/LAN Player Indicators!
    let p_world_center = iso_to_world(player_iso.x, player_iso.y, player_iso.z);
    for wp in &map.waypoints {
        if settings.hide_hud {
            break;
        }
        let wp_elev = map.get_elevation(wp.gx as f32, wp.gy as f32);
        let wp_world = iso_to_world(wp.gx as f32, wp.gy as f32, wp_elev);
        let dist_tiles = player_iso
            .truncate()
            .distance(Vec2::new(wp.gx as f32, wp.gy as f32));
        if dist_tiles <= 24.0 {
            // Draw pulsing beacon ring at the waypoint tile
            let pulse = 10.0 + (solar.anim_time_sec * 4.0).sin().abs() * 6.0;
            gizmos.circle_2d(wp_world + Vec2::new(0.0, 8.0), pulse, wp.color.with_a(0.65));
            gizmos.line_2d(
                wp_world + Vec2::new(0.0, 8.0),
                wp_world + Vec2::new(0.0, 52.0),
                wp.color.with_a(0.75),
            );
        }
        // Directional compass pointer around the astronaut toward distant waypoints
        if dist_tiles > 4.5 {
            let dir = (wp_world - p_world_center).normalize_or_zero();
            let arrow_start = p_world_center + Vec2::new(0.0, 20.0) + dir * 48.0;
            let arrow_tip = p_world_center + Vec2::new(0.0, 20.0) + dir * 64.0;
            gizmos.line_2d(arrow_start, arrow_tip, wp.color.with_a(0.82));
            gizmos.circle_2d(arrow_tip, 3.5, wp.color);
        }
    }

    // Draw directional indicator to other astronauts (Solo Crew / LAN Players)
    for peer in &lan.peers {
        if settings.hide_hud {
            break;
        }
        let peer_world = iso_to_world(peer.iso_pos.x, peer.iso_pos.y, peer.iso_pos.z);
        if !peer.is_ai_bot && peer.body != map.body {
            continue;
        }
        let dist = player_iso.truncate().distance(peer.iso_pos.truncate());
        if dist > 4.0 {
            let dir = (peer_world - p_world_center).normalize_or_zero();
            let tip = p_world_center + Vec2::new(0.0, 20.0) + dir * 56.0;
            gizmos.circle_2d(tip, 4.0, peer.suit.to_color());
        }
    }

    // Accurate ground-projected shadows!
    let shadow_offset = solar.sun_shadow_offset();
    for (shadow, mut transform, mut vis) in &mut shadow_q {
        match shadow.owner_kind {
            ShadowOwnerKind::Player => {
                if let Ok(player) = player_q.get_single() {
                    if player.mounted_rover.is_some() || map.piloting_station {
                        *vis = Visibility::Hidden;
                        continue;
                    }
                    *vis = Visibility::Inherited;
                    let ground_z = map.get_elevation(player.iso_pos.x, player.iso_pos.y);
                    let height_above_ground = (player.iso_pos.z - ground_z).max(0.0);
                    let ground_world = iso_to_world(player.iso_pos.x, player.iso_pos.y, ground_z);

                    transform.translation.x = ground_world.x + shadow_offset.x * 0.18;
                    transform.translation.y = ground_world.y + 4.0 + shadow_offset.y * 0.12;
                    transform.translation.z = iso_shadow_depth(player.iso_pos.x, player.iso_pos.y);

                    let air_shrink = (1.0 / (1.0 + height_above_ground * 0.28)).clamp(0.42, 1.0);
                    transform.scale = Vec3::new(
                        shadow.base_scale.x * air_shrink * (1.0 + (1.0 - sun) * 0.35),
                        shadow.base_scale.y * air_shrink,
                        1.0,
                    );

                    if height_above_ground > 0.06 {
                        let land_spot = Vec2::new(ground_world.x, ground_world.y + 4.0);
                        let air_spot =
                            iso_to_world(player.iso_pos.x, player.iso_pos.y, player.iso_pos.z)
                                + Vec2::new(0.0, 6.0);
                        gizmos.circle_2d(
                            land_spot,
                            (13.0 * air_shrink).max(6.0),
                            Color::rgba(0.35, 0.96, 1.0, 0.72),
                        );
                        gizmos.circle_2d(land_spot, 3.0, Color::rgba(1.0, 0.92, 0.35, 0.85));
                        gizmos.line_2d(air_spot, land_spot, Color::rgba(0.35, 0.96, 1.0, 0.38));
                    }
                }
            }
            ShadowOwnerKind::Rover(r_ent) => {
                if let Ok(rover) = rover_q.get(r_ent) {
                    *vis = Visibility::Inherited;
                    let ground_z = map.get_elevation(rover.iso_pos.x, rover.iso_pos.y);
                    let ground_world = iso_to_world(rover.iso_pos.x, rover.iso_pos.y, ground_z);
                    transform.translation.x = ground_world.x + shadow_offset.x * 0.25;
                    transform.translation.y = ground_world.y + 3.0 + shadow_offset.y * 0.18;
                    transform.translation.z = iso_shadow_depth(rover.iso_pos.x, rover.iso_pos.y);
                    transform.scale = Vec3::new(
                        shadow.base_scale.x * (1.0 + (1.0 - sun) * 0.45),
                        shadow.base_scale.y,
                        1.0,
                    );
                } else {
                    *vis = Visibility::Hidden;
                }
            }
            ShadowOwnerKind::Crew(c_ent) => {
                if let Ok((crew, c_vis)) = crew_q.get(c_ent) {
                    if *c_vis == Visibility::Hidden {
                        *vis = Visibility::Hidden;
                        continue;
                    }
                    if let Some(peer) = lan.peers.get(crew.peer_index) {
                        *vis = Visibility::Inherited;
                        let ground_z = map.get_elevation(peer.iso_pos.x, peer.iso_pos.y);
                        let ground_world = iso_to_world(peer.iso_pos.x, peer.iso_pos.y, ground_z);
                        transform.translation.x = ground_world.x + shadow_offset.x * 0.18;
                        transform.translation.y = ground_world.y + 4.0 + shadow_offset.y * 0.12;
                        transform.translation.z = iso_shadow_depth(peer.iso_pos.x, peer.iso_pos.y);
                    }
                } else {
                    *vis = Visibility::Hidden;
                }
            }
            ShadowOwnerKind::Station => {
                let iso_xy = map.station_position.truncate();
                *vis = Visibility::Inherited;
                let ground_z = map.get_elevation(iso_xy.x, iso_xy.y);
                let ground_world = iso_to_world(iso_xy.x, iso_xy.y, ground_z);
                transform.translation.x = ground_world.x + shadow_offset.x * 0.35;
                transform.translation.y = ground_world.y + 2.0 + shadow_offset.y * 0.25;
                transform.translation.z = iso_shadow_depth(iso_xy.x, iso_xy.y);
                transform.scale = Vec3::new(
                    shadow.base_scale.x * (1.0 + (1.0 - sun) * 0.50),
                    shadow.base_scale.y,
                    1.0,
                );
            }
        }
    }
}

pub fn cleanup_surface_entities(mut commands: Commands, q: Query<Entity, With<SurfaceEntity>>) {
    for ent in &q {
        commands.entity(ent).despawn_recursive();
    }
}

/// Keep explored map data, but release distant render entities for long expeditions.
#[allow(clippy::type_complexity)]
pub fn unload_distant_surface_sprites(
    mut commands: Commands,
    mut map: ResMut<LunarMap>,
    mut previous_center: Local<Option<(i32, i32)>>,
    renderables: Query<
        (
            Entity,
            Option<&IsoTileMarker>,
            Option<&WorldFeatureSpriteMarker>,
            Option<&StructureSpriteMarker>,
            Option<&StructureConnectorMarker>,
        ),
        Or<(
            With<IsoTileMarker>,
            With<WorldFeatureSpriteMarker>,
            With<StructureSpriteMarker>,
            With<StructureConnectorMarker>,
        )>,
    >,
) {
    let center = map.last_stream_center;
    if *previous_center == Some(center) {
        return;
    }
    *previous_center = Some(center);
    for (entity, tile, feature, structure, connector) in &renderables {
        let (gx, gy) = if let Some(m) = tile {
            (m.gx, m.gy)
        } else if let Some(m) = feature {
            (m.gx, m.gy)
        } else if let Some(m) = structure {
            (m.gx, m.gy)
        } else if let Some(m) = connector {
            (m.gx, m.gy)
        } else {
            continue;
        };
        if (gx - center.0).abs() <= STREAM_RADIUS + 6 && (gy - center.1).abs() <= STREAM_RADIUS + 6
        {
            continue;
        }
        if let Some(cell) = map.cells.get_mut(&(gx, gy)) {
            if tile.is_some() {
                cell.tile_entity = None;
            }
            if feature.is_some() {
                cell.feature_entity = None;
            }
        }
        commands.entity(entity).despawn_recursive();
    }
}
