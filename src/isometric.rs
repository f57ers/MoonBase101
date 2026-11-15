use bevy::prelude::*;
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use crate::types::{HelmetAccessory, StructureKind, TileKind, WorldFeatureKind};

pub const ISO_TILE_W: f32 = 88.0;
pub const ISO_TILE_H: f32 = 44.0;
pub const ISO_ELEV_SCALE: f32 = 28.0;
/// Active streaming radius around the player for the infinite lunar world!
pub const STREAM_RADIUS: i32 = 18;

#[inline]
pub fn iso_to_world(x: f32, y: f32, z: f32) -> Vec2 {
    let sx = (x - y) * (ISO_TILE_W * 0.5);
    let sy = -(x + y) * (ISO_TILE_H * 0.5) + z * ISO_ELEV_SCALE;
    Vec2::new(sx, sy)
}

#[inline]
pub fn world_to_iso(world: Vec2) -> Vec2 {
    let a = world.x / (ISO_TILE_W * 0.5);
    let b = -world.y / (ISO_TILE_H * 0.5);
    let x = 0.5 * (a + b);
    let y = 0.5 * (b - a);
    Vec2::new(x, y)
}

/// Depth for ground tiles (always behind shadows, buildings, rovers, and astronauts).
#[inline]
pub fn iso_tile_depth(x: f32, y: f32, z_height: f32) -> f32 {
    let order = ((x + y) * 0.15).clamp(-60.0, 60.0);
    (80.0 + order + z_height * 0.05).clamp(5.0, 145.0)
}

/// Depth for ground-projected shadows (always directly above ground tiles, behind all 3D objects).
#[inline]
pub fn iso_shadow_depth(x: f32, y: f32) -> f32 {
    let order = ((x + y) * 0.15).clamp(-60.0, 60.0);
    (220.0 + order).clamp(155.0, 285.0)
}

/// Depth for 2.5D surface objects (structures, world features, rovers, astronauts).
#[inline]
pub fn iso_depth(x: f32, y: f32, z_height: f32, layer_bias: f32) -> f32 {
    let order = ((x + y) * 1.8).clamp(-220.0, 220.0);
    (540.0 + order + z_height * 0.12 + layer_bias).clamp(300.0, 840.0)
}

#[derive(Resource, Clone)]
pub struct ProceduralAssets {
    pub deposit_tex: [Handle<Image>; 5],
    pub tile_maria: Handle<Image>,
    pub tile_highland: Handle<Image>,
    pub tile_crater_rim: Handle<Image>,
    pub tile_crater_floor: Handle<Image>,
    pub tile_cursor: Handle<Image>,
    // 4-frame animated astronaut: [0=Idle, 1=StrideA, 2=StrideB, 3=JumpJet]
    #[allow(dead_code)]
    pub astronaut_frames: [Handle<Image>; 4],
    // 8-directional smooth-turning astronaut sprites: [dir 0..8][frame 0..4]
    pub astronaut_dir_frames: [[Handle<Image>; 4]; 8],
    #[allow(dead_code)]
    pub visor_tex: Handle<Image>,
    pub visor_dir_tex: [Handle<Image>; 8],
    pub accessory_tex: [Handle<Image>; 5],
    pub lander_tex: Handle<Image>,
    pub thrust_tex: Handle<Image>,
    pub planet_portraits: [Handle<Image>; 3],
    // 3-frame animated rovers (spinning wheels + suspension + radar)
    pub rover_light_frames: [Handle<Image>; 3],
    pub rover_heavy_frames: [Handle<Image>; 3],
    // 10 Distinct 2.5D Structure Models
    pub structure_tex: [Handle<Image>; 10],
    pub structure_roof_tex: Handle<Image>,
    pub interior_floor_tex: Handle<Image>,
    // 4 Progressive Mining Crack Overlays (25%, 50%, 75%, 95%)
    pub crack_stage_tex: [Handle<Image>; 4],
    // Connected-structure unification bridges (merges adjacent identical buildings into one!)
    pub conn_pressurized_tex: Handle<Image>,
    pub conn_wall_tex: Handle<Image>,
    pub conn_solar_tex: Handle<Image>,
    pub conn_industrial_tex: Handle<Image>,
    // 10 World Feature & Easter Egg Models
    pub feature_tex: [Handle<Image>; 10],
    pub shadow_disc: Handle<Image>,
    // Visual inventory & hotbar icons
    pub icon_pickaxe: Handle<Image>,
    pub icon_regolith: Handle<Image>,
    pub icon_ice: Handle<Image>,
    pub icon_titanium: Handle<Image>,
    pub icon_silicon: Handle<Image>,
    pub icon_he3: Handle<Image>,
    pub icon_rare: Handle<Image>,
    pub icon_ration: Handle<Image>,
    pub icon_drink: Handle<Image>,
    pub icon_sample: Handle<Image>,
    pub icon_medkit: Handle<Image>,
}

impl ProceduralAssets {
    /// Maps a continuous screen-space facing angle (`-PI..PI`, where `-PI/2` = South/Front, `0` = East/Right, `+PI/2` = North/Back, `PI` = West/Left)
    /// into one of 8 smooth directional indices `0..8`.
    #[inline]
    pub fn angle_to_dir_index(angle_rad: f32) -> usize {
        use std::f32::consts::{FRAC_PI_2, TAU};
        // Shift so South (-PI/2) is index 0, going counter-clockwise:
        // 0=S, 1=SE, 2=E, 3=NE, 4=N, 5=NW, 6=W, 7=SW
        let shifted = (angle_rad + FRAC_PI_2).rem_euclid(TAU);
        (((shifted / TAU) * 8.0).round() as usize) % 8
    }

    pub fn astronaut_frame_for_angle(&self, angle_rad: f32, frame: usize) -> Handle<Image> {
        let dir = Self::angle_to_dir_index(angle_rad);
        self.astronaut_dir_frames[dir][frame.min(3)].clone()
    }

    pub fn visor_for_angle(&self, angle_rad: f32) -> Handle<Image> {
        let dir = Self::angle_to_dir_index(angle_rad);
        self.visor_dir_tex[dir].clone()
    }
    pub fn terrain_handle(&self, kind: TileKind, elevation: f32) -> Handle<Image> {
        match kind {
            TileKind::MariaBasalt => self.tile_maria.clone(),
            TileKind::HighlandRegolith => self.tile_highland.clone(),
            TileKind::CraterRim => self.tile_crater_rim.clone(),
            TileKind::CraterFloor => self.tile_crater_floor.clone(),
            _ if elevation < -0.25 => self.tile_crater_floor.clone(),
            _ => self.tile_highland.clone(),
        }
    }

    pub fn structure_handle(&self, kind: StructureKind) -> Handle<Image> {
        let idx = match kind {
            StructureKind::Airlock => 0,
            StructureKind::HabitatModule => 1,
            StructureKind::SolarArray => 2,
            StructureKind::RtgGenerator => 3,
            StructureKind::OxygenElectrolyzer => 4,
            StructureKind::HydroponicsDome => 5,
            StructureKind::StorageBay => 6,
            StructureKind::CommsDish => 7,
            StructureKind::ScienceLab => 8,
            StructureKind::ShieldWall => 9,
        };
        self.structure_tex[idx].clone()
    }

    pub fn connector_handle(&self, kind: StructureKind) -> Handle<Image> {
        match kind {
            StructureKind::Airlock
            | StructureKind::HabitatModule
            | StructureKind::HydroponicsDome
            | StructureKind::ScienceLab => self.conn_pressurized_tex.clone(),
            StructureKind::ShieldWall => self.conn_wall_tex.clone(),
            StructureKind::SolarArray => self.conn_solar_tex.clone(),
            _ => self.conn_industrial_tex.clone(),
        }
    }

    pub fn feature_handle(&self, kind: WorldFeatureKind) -> Handle<Image> {
        let idx = match kind {
            WorldFeatureKind::SupplyDropPod => 0,
            WorldFeatureKind::ApolloLrvWreck => 1,
            WorldFeatureKind::CrashedLunaProbe => 2,
            WorldFeatureKind::TwardowskiRooster => 3,
            WorldFeatureKind::LunarFlagPlaque => 4,
            WorldFeatureKind::GlowingCrystalCluster => 5,
            WorldFeatureKind::KopernikRuinedHub => 6,
            WorldFeatureKind::LemMonolith => 7,
            WorldFeatureKind::HeavyCargoWreck => 8,
            WorldFeatureKind::AutomatedIceRig => 9,
        };
        self.feature_tex[idx].clone()
    }

    pub fn accessory_handle(&self, acc: HelmetAccessory) -> Handle<Image> {
        let idx = match acc {
            HelmetAccessory::HussarWings => 0,
            HelmetAccessory::StandardAntenna => 1,
            HelmetAccessory::SolarHalo => 2,
            HelmetAccessory::CatEars => 3,
            HelmetAccessory::LaserCrest => 4,
        };
        self.accessory_tex[idx].clone()
    }
}

pub fn generate_procedural_assets(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let tile_maria = images.add(make_iso_tile(
        [62, 66, 76],
        [48, 52, 60],
        [82, 86, 98],
        None,
    ));
    let tile_highland = images.add(make_iso_tile(
        [116, 120, 128],
        [92, 96, 104],
        [142, 146, 156],
        None,
    ));
    let tile_crater_rim = images.add(make_iso_tile(
        [138, 142, 152],
        [86, 90, 98],
        [176, 180, 192],
        None,
    ));
    let tile_crater_floor = images.add(make_iso_tile(
        [42, 44, 52],
        [32, 34, 42],
        [58, 60, 70],
        None,
    ));
    let tile_cursor = images.add(make_iso_cursor());

    let astronaut_frames = [
        images.add(make_astronaut_frame(0)),
        images.add(make_astronaut_frame(1)),
        images.add(make_astronaut_frame(2)),
        images.add(make_astronaut_frame(3)),
    ];
    let astronaut_dir_frames = std::array::from_fn(|dir| {
        [
            images.add(make_astronaut_dir_frame(dir, 0)),
            images.add(make_astronaut_dir_frame(dir, 1)),
            images.add(make_astronaut_dir_frame(dir, 2)),
            images.add(make_astronaut_dir_frame(dir, 3)),
        ]
    });
    let visor_tex = images.add(make_visor_sprite());
    let visor_dir_tex = std::array::from_fn(|dir| images.add(make_visor_dir_sprite(dir)));
    let accessory_tex = [
        images.add(make_accessory_sprite(0)),
        images.add(make_accessory_sprite(1)),
        images.add(make_accessory_sprite(2)),
        images.add(make_accessory_sprite(3)),
        images.add(make_accessory_sprite(4)),
    ];
    let lander_tex = images.add(make_lander_sprite());

    let rover_light_frames = [
        images.add(make_rover_frame(false, 0)),
        images.add(make_rover_frame(false, 1)),
        images.add(make_rover_frame(false, 2)),
    ];
    let rover_heavy_frames = [
        images.add(make_rover_frame(true, 0)),
        images.add(make_rover_frame(true, 1)),
        images.add(make_rover_frame(true, 2)),
    ];

    // 10 Distinct Building Models
    let structure_tex = [
        images.add(make_airlock_sprite()),
        images.add(make_habitat_sprite()),
        images.add(make_solar_array_sprite()),
        images.add(make_rtg_generator_sprite()),
        images.add(make_o2_electrolyzer_sprite()),
        images.add(make_hydroponics_dome_sprite()),
        images.add(make_storage_bay_sprite()),
        images.add(make_comms_dish_sprite()),
        images.add(make_science_lab_sprite()),
        images.add(make_shield_wall_sprite()),
    ];
    let structure_roof_tex = images.add(make_habitat_roof_overlay());
    let interior_floor_tex = images.add(make_interior_floor_sprite());
    let crack_stage_tex = [
        images.add(make_crack_overlay_sprite(0)),
        images.add(make_crack_overlay_sprite(1)),
        images.add(make_crack_overlay_sprite(2)),
        images.add(make_crack_overlay_sprite(3)),
    ];

    let conn_pressurized_tex = images.add(make_connector_sprite([225, 232, 244], [75, 215, 255]));
    let conn_wall_tex = images.add(make_connector_sprite([145, 152, 165], [245, 165, 45]));
    let conn_solar_tex = images.add(make_connector_sprite([35, 85, 185], [240, 200, 75]));
    let conn_industrial_tex = images.add(make_connector_sprite([120, 175, 215], [90, 240, 200]));

    let feature_tex = [
        images.add(make_feature_supply_pod()),
        images.add(make_feature_lrv_wreck()),
        images.add(make_feature_luna_probe()),
        images.add(make_feature_twardowski_rooster()),
        images.add(make_feature_polish_flag()),
        images.add(make_feature_crystal_cluster()),
        images.add(make_feature_kopernik_ruin()),
        images.add(make_feature_lem_monolith()),
        images.add(make_feature_cargo_wreck()),
        images.add(make_feature_ice_rig()),
    ];

    let shadow_disc = images.add(make_shadow_disc());

    let icon_pickaxe = images.add(make_pickaxe_icon());
    let icon_regolith = images.add(make_item_icon([145, 150, 164], [90, 95, 108]));
    let icon_ice = images.add(make_item_icon([85, 225, 255], [35, 140, 195]));
    let icon_titanium = images.add(make_item_icon([215, 225, 240], [120, 135, 160]));
    let icon_silicon = images.add(make_item_icon([110, 235, 175], [45, 145, 95]));
    let icon_he3 = images.add(make_item_icon([255, 195, 70], [185, 95, 220]));
    let icon_rare = images.add(make_item_icon([255, 115, 65], [165, 55, 45]));
    let icon_ration = images.add(make_supply_icon(false));
    let icon_drink = images.add(make_supply_icon(true));
    let icon_sample = images.add(make_item_icon([185, 135, 255], [95, 65, 195]));
    let icon_medkit = images.add(make_medkit_icon());

    commands.insert_resource(ProceduralAssets {
        thrust_tex: images.add(make_thruster_plume()),
        planet_portraits: std::array::from_fn(|i| images.add(make_planet_portrait(i))),
        deposit_tex: std::array::from_fn(|i| images.add(make_deposit(i))),
        tile_maria,
        tile_highland,
        tile_crater_rim,
        tile_crater_floor,
        tile_cursor,
        astronaut_frames,
        astronaut_dir_frames,
        visor_tex,
        visor_dir_tex,
        accessory_tex,
        lander_tex,
        rover_light_frames,
        rover_heavy_frames,
        structure_tex,
        structure_roof_tex,
        interior_floor_tex,
        crack_stage_tex,
        conn_pressurized_tex,
        conn_wall_tex,
        conn_solar_tex,
        conn_industrial_tex,
        feature_tex,
        shadow_disc,
        icon_pickaxe,
        icon_regolith,
        icon_ice,
        icon_titanium,
        icon_silicon,
        icon_he3,
        icon_rare,
        icon_ration,
        icon_drink,
        icon_sample,
        icon_medkit,
    });
}

fn new_rgba_image(w: u32, h: u32, data: Vec<u8>) -> Image {
    Image::new(
        Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}

#[inline]
fn put_px(data: &mut [u8], w: u32, h: u32, px: i32, py: i32, rgba: [u8; 4]) {
    if px >= 0 && py >= 0 && (px as u32) < w && (py as u32) < h {
        let idx = ((py as u32 * w + px as u32) * 4) as usize;
        data[idx] = rgba[0];
        data[idx + 1] = rgba[1];
        data[idx + 2] = rgba[2];
        data[idx + 3] = rgba[3];
    }
}

/// High-resolution mineral seams lying flush with the regolith.
fn make_deposit(kind: usize) -> Image {
    let (w, h) = (264u32, 132u32);
    let mut data = vec![0u8; (w * h * 4) as usize];
    let colors = [
        [135.0, 139.0, 148.0],
        [139.0, 201.0, 222.0],
        [176.0, 187.0, 205.0],
        [166.0, 143.0, 108.0],
        [141.0, 109.0, 92.0],
    ];
    for py in 0..h as i32 {
        for px in 0..w as i32 {
            let x = (px as f32 - 132.0) / 132.0;
            let y = (py as f32 - 66.0) / 66.0;
            let border = 1.0 - x.abs() - y.abs();
            if border < 0.14 {
                continue;
            }
            let warp = (x * 9.0 + y * 13.0).sin() * 0.15 + (y * 18.0 - x * 4.0).cos() * 0.10;
            let vein = (y + x * 0.55 + warp).abs();
            let branch = (y - x * 0.7 - 0.21 + warp * 0.72).abs();
            let grain =
                ((px.wrapping_mul(197) ^ py.wrapping_mul(263) ^ px * py) & 255) as f32 / 255.0;
            let speckle = (grain - 0.82).max(0.0) * 1.25;
            let vein_core = (1.0 - vein * 7.0).max(0.0);
            let veins = vein_core + (1.0 - branch * 12.0).max(0.0) * 0.52;
            let stain = (1.0 - vein * 3.1).max(0.0) * 0.38;
            let alpha = ((veins * 0.68 + stain + speckle) * (border * 3.5).min(1.0) * 250.0)
                .min(248.0) as u8;
            let light = 0.67 + grain * 0.31 + vein_core * 0.28;
            let color = colors[kind].map(|v| (v * light).clamp(0.0, 255.0) as u8);
            put_px(
                &mut data,
                w,
                h,
                px,
                py,
                [color[0], color[1], color[2], alpha],
            );
        }
    }
    new_rgba_image(w, h, data)
}

fn make_thruster_plume() -> Image {
    let (w, h) = (32u32, 80u32);
    let mut data = vec![0; (w * h * 4) as usize];
    for py in 0..h as i32 {
        let depth = py as f32 / h as f32;
        let spread = 4.0 + depth * 11.0;
        for px in 0..w as i32 {
            let radius = (px as f32 - 15.5).abs() / spread;
            let intensity = (1.0 - radius).max(0.0).powf(1.6) * (1.0 - depth).powf(1.3);
            let heat = (1.0 - depth - radius * 0.36).clamp(0.0, 1.0);
            put_px(
                &mut data,
                w,
                h,
                px,
                py,
                [
                    255,
                    (92.0 + heat * 159.0) as u8,
                    (35.0 + heat * 211.0) as u8,
                    (intensity * 255.0) as u8,
                ],
            );
        }
    }
    new_rgba_image(w, h, data)
}

fn make_planet_portrait(kind: usize) -> Image {
    let side = 192u32;
    let mut rgba = vec![0; (side * side * 4) as usize];
    let base = [
        [176.0, 182.0, 196.0],
        [214.0, 112.0, 70.0],
        [185.0, 156.0, 130.0],
    ][kind];
    let craters = [
        (-0.33, -0.43, 0.20),
        (0.18, 0.43, 0.17),
        (0.49, -0.12, 0.11),
        (-0.38, 0.25, 0.09),
        (0.07, -0.22, 0.08),
        (-0.58, 0.09, 0.06),
    ];
    for py in 0..side as i32 {
        for px in 0..side as i32 {
            let x = (px as f32 - 95.5) / 80.0;
            let y = (py as f32 - 95.5) / 80.0;
            let radius = x * x + y * y;
            if radius > 1.025 {
                continue;
            }
            let z = (1.0 - radius).max(0.0).sqrt();
            let light = (-x * 0.51 - y * 0.43 + z * 0.74).clamp(0.0, 1.0);
            let detail =
                (x * 9.0 + y * 14.0 + (y * 15.0).sin()).sin() * (x * 16.0 - y * 8.0).cos() * 0.10;
            let grain = (((px * 239 ^ py * 149 ^ px * py) & 255) as f32 / 255.0 - 0.5) * 0.14;
            let mut dent = 0.0;
            if kind != 1 {
                for (cx, cy, r) in craters {
                    let dist = ((x - cx) * (x - cx) + (y - cy) * (y - cy)).sqrt() / r;
                    if dist < 0.90 {
                        dent -= 0.16 * (1.0 - dist * 0.65);
                    } else if dist < 1.06 {
                        dent += 0.12;
                    }
                }
            }
            let frost = if kind == 1 {
                ((-y - 0.73 + detail) * 4.5).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let shade = (0.13 + light * 0.86 + detail + grain + dent).clamp(0.06, 1.15);
            let color = base.map(|value| {
                ((value * (1.0 - frost) + 231.0 * frost) * shade).clamp(0.0, 255.0) as u8
            });
            let alpha = ((1.025 - radius) * 48.0).clamp(0.0, 1.0) * 255.0;
            put_px(
                &mut rgba,
                side,
                side,
                px,
                py,
                [color[0], color[1], color[2], alpha as u8],
            );
        }
    }
    new_rgba_image(side, side, rgba)
}

fn make_pickaxe_icon() -> Image {
    let w = 32u32;
    let h = 32u32;
    let mut data = vec![0u8; (w * h * 4) as usize];
    for i in 5..27 {
        for t in -1..=1 {
            put_px(&mut data, w, h, i + t, 31 - i, [185, 130, 65, 255]);
        }
    }
    for dx in -8..=8 {
        for dy in -2..=2 {
            let hx = 20 + dx;
            let hy = 11 + dx / 2 + dy;
            put_px(&mut data, w, h, hx, hy, [85, 235, 255, 255]);
        }
    }
    new_rgba_image(w, h, data)
}

fn make_supply_icon(water: bool) -> Image {
    let size = 40u32;
    let mut pixels = vec![0; (size * size * 4) as usize];
    for y in 4i32..36 {
        for x in 6i32..34 {
            let color = if water {
                if (4..=10).contains(&y) && (15..=24).contains(&x) {
                    [193, 221, 236, 255]
                } else if (12..=34).contains(&y)
                    && ((x - 20).abs() <= if y == 12 || y >= 33 { 8 } else { 11 })
                {
                    if x <= 12 {
                        [168, 227, 250, 255]
                    } else if (20..=26).contains(&y) {
                        [40, 112, 176, 255]
                    } else {
                        [87, 186, 233, 255]
                    }
                } else {
                    continue;
                }
            } else if (8..=33).contains(&y) && (8..=31).contains(&x) {
                if y <= 11 || y >= 30 {
                    [198, 226, 178, 255]
                } else if (x - 20).abs() + (y - 21).abs() < 6 {
                    [243, 231, 160, 255]
                } else if x <= 11 {
                    [164, 209, 143, 255]
                } else {
                    [92, 153, 108, 255]
                }
            } else {
                continue;
            };
            put_px(&mut pixels, size, size, x, y, color);
        }
    }
    new_rgba_image(size, size, pixels)
}

fn make_medkit_icon() -> Image {
    let w = 32u32;
    let h = 32u32;
    let mut data = vec![0u8; (w * h * 4) as usize];
    for py in 6..=26 {
        for px in 5..=27 {
            let cross = ((14..=18).contains(&px) && (9..=23).contains(&py))
                || ((14..=18).contains(&py) && (9..=23).contains(&px));
            let col = if cross {
                [245, 45, 65, 255]
            } else {
                [240, 245, 252, 255]
            };
            put_px(&mut data, w, h, px, py, col);
        }
    }
    new_rgba_image(w, h, data)
}

fn make_item_icon(primary: [u8; 3], accent: [u8; 3]) -> Image {
    let w = 32u32;
    let h = 32u32;
    let mut data = vec![0u8; (w * h * 4) as usize];
    for py in 0..h as i32 {
        for px in 0..w as i32 {
            let dx = (px - 16).abs();
            let dy = (py - 16).abs();
            if dx + dy <= 13 {
                let col = if dx + dy >= 10 || (px < 15 && py < 15) {
                    primary
                } else {
                    accent
                };
                put_px(&mut data, w, h, px, py, [col[0], col[1], col[2], 255]);
            }
        }
    }
    new_rgba_image(w, h, data)
}

fn make_iso_tile(
    top_rgb: [u8; 3],
    skirt_rgb: [u8; 3],
    edge_rgb: [u8; 3],
    vein_rgb: Option<[u8; 3]>,
) -> Image {
    let w = 88u32;
    let h = 56u32;
    let diamond_h = 44i32;
    let half_w = 44i32;
    let half_h = 22i32;
    let mut data = vec![0u8; (w * h * 4) as usize];

    for py in 0..h as i32 {
        for px in 0..w as i32 {
            let dx = (px - half_w).abs();
            let dy = (py - half_h).abs();

            let diamond_metric = dx + 2 * dy;
            if py <= diamond_h && diamond_metric <= 44 {
                let hash = ((px * 37 + py * 91) ^ (px * py)) & 15;
                let broad_grain =
                    ((px as f32 * 0.17).sin() * (py as f32 * 0.23).cos() * 3.0) as i32;
                let pit_x = (px - 28) as f32 / 7.0;
                let pit_y = (py - 24) as f32 / 3.5;
                let pit = pit_x * pit_x + pit_y * pit_y;
                let relief = if pit < 0.65 {
                    -8
                } else if pit < 1.0 && py < 24 {
                    7
                } else {
                    0
                };
                let noise = hash / 4 - 2 + broad_grain + relief;
                let mut r = (top_rgb[0] as i32 + noise).clamp(0, 255) as u8;
                let mut g = (top_rgb[1] as i32 + noise).clamp(0, 255) as u8;
                let mut b = (top_rgb[2] as i32 + noise).clamp(0, 255) as u8;

                if diamond_metric >= 43 {
                    r = ((top_rgb[0] as u16 * 3 + edge_rgb[0] as u16) / 4) as u8;
                    g = ((top_rgb[1] as u16 * 3 + edge_rgb[1] as u16) / 4) as u8;
                    b = ((top_rgb[2] as u16 * 3 + edge_rgb[2] as u16) / 4) as u8;
                } else if let Some(v) = vein_rgb {
                    // Thin branching mineral seams instead of repeating concentric rings.
                    let seam = 24.0 + py as f32 * 0.65 + (py as f32 * 0.38).sin() * 3.0;
                    let branch = 59.0 - py as f32 * 0.55;
                    let distance = (px as f32 - seam).abs().min((px as f32 - branch).abs());
                    if distance < 1.8 && diamond_metric < 34 && hash > 3 {
                        let strength = if distance < 0.7 { 0.76 } else { 0.32 };
                        r = (r as f32 * (1.0 - strength) + v[0] as f32 * strength) as u8;
                        g = (g as f32 * (1.0 - strength) + v[1] as f32 * strength) as u8;
                        b = (b as f32 * (1.0 - strength) + v[2] as f32 * strength) as u8;
                    }
                }
                put_px(&mut data, w, h, px, py, [r, g, b, 255]);
            } else if py > half_h {
                let bottom_y_at_x = half_h + (44 - dx) / 2;
                if dx <= 44 && py >= bottom_y_at_x && py <= bottom_y_at_x + 10 {
                    let shade = if px < half_w { 1.0 } else { 0.78 };
                    put_px(
                        &mut data,
                        w,
                        h,
                        px,
                        py,
                        [
                            (skirt_rgb[0] as f32 * shade) as u8,
                            (skirt_rgb[1] as f32 * shade) as u8,
                            (skirt_rgb[2] as f32 * shade) as u8,
                            255,
                        ],
                    );
                }
            }
        }
    }
    new_rgba_image(w, h, data)
}

fn make_iso_cursor() -> Image {
    let w = 88u32;
    let h = 44u32;
    let mut data = vec![0u8; (w * h * 4) as usize];
    for py in 0..h as i32 {
        for px in 0..w as i32 {
            let dx = (px - 44).abs();
            let dy = (py - 22).abs();
            let m = dx + 2 * dy;
            if (38..=44).contains(&m) {
                put_px(&mut data, w, h, px, py, [95, 248, 255, 240]);
            }
        }
    }
    new_rgba_image(w, h, data)
}

/// Generates 4 animated frames for the astronaut:
/// frame 0 = idle, 1 = stride A, 2 = stride B, 3 = airborne jump/jetpack with RCS flame
fn make_astronaut_frame(frame: usize) -> Image {
    let w = 48u32;
    let h = 64u32;
    let mut data = vec![0u8; (w * h * 4) as usize];
    let (left_leg_dy, right_leg_dy, arm_swing) = match frame {
        1 => (-4, 3, -3),
        2 => (3, -4, 3),
        3 => (-5, -5, -4),
        _ => (0, 0, 0),
    };

    for py in 0..h as i32 {
        for px in 0..w as i32 {
            let cx = px - 24;

            // PLSS Life-Support Backpack
            if (18..=42).contains(&py) && (-13..=13).contains(&cx) {
                put_px(&mut data, w, h, px, py, [200, 206, 215, 255]);
            }
            // Jetpack RCS flame plumes on frame 3
            if frame == 3 && (42..=55).contains(&py) && (8..=13).contains(&cx.abs()) {
                put_px(&mut data, w, h, px, py, [90, 228, 255, 230]);
            }
            // Helmet sphere
            let h_dist = cx * cx + (py - 15) * (py - 15);
            if h_dist <= 110 {
                put_px(&mut data, w, h, px, py, [245, 248, 255, 255]);
            }
            // Torso & Swinging Arms
            if (24..=45).contains(&py) && cx.abs() <= 10 {
                put_px(&mut data, w, h, px, py, [238, 240, 246, 255]);
                // Chest telemetry module & Polish red-white badge stripe
                if (28..=31).contains(&py) && cx.abs() <= 5 {
                    put_px(&mut data, w, h, px, py, [250, 250, 252, 255]);
                } else if (32..=35).contains(&py) && cx.abs() <= 5 {
                    put_px(&mut data, w, h, px, py, [225, 38, 52, 255]);
                }
            }
            if (26 + arm_swing..=40 + arm_swing).contains(&py) && (10..=13).contains(&cx.abs()) {
                put_px(&mut data, w, h, px, py, [226, 230, 238, 255]);
            }
            // Animated Left & Right Legs
            let leg_offset = if cx < 0 { left_leg_dy } else { right_leg_dy };
            let ly = py - leg_offset;
            if (45..=59).contains(&ly) && (2..=8).contains(&cx.abs()) {
                let col = if ly >= 55 {
                    [85, 92, 104, 255]
                } else {
                    [210, 214, 222, 255]
                };
                put_px(&mut data, w, h, px, py, col);
            }
        }
    }
    new_rgba_image(w, h, data)
}

fn make_visor_sprite() -> Image {
    let w = 48u32;
    let h = 64u32;
    let mut data = vec![0u8; (w * h * 4) as usize];
    for py in 9..=20 {
        for px in 17..=31 {
            let cx = px - 24;
            let cy = py - 15;
            if cx * cx + cy * cy <= 52 {
                let highlight = if cx < -2 && cy < -1 { 255 } else { 225 };
                put_px(
                    &mut data,
                    w,
                    h,
                    px,
                    py,
                    [highlight, highlight, highlight, 255],
                );
            }
        }
    }
    new_rgba_image(w, h, data)
}

/// Generates 5 helmet/backpack accessories:
/// 0 = Polish Winged Hussar Feathers, 1 = Telemetry Antenna, 2 = Solar Halo, 3 = Space Cat Ears, 4 = Commander Crest
fn make_accessory_sprite(kind: usize) -> Image {
    let w = 48u32;
    let h = 64u32;
    let mut data = vec![0u8; (w * h * 4) as usize];
    match kind {
        0 => {
            // Iconic Polish Winged Hussar Wings rising behind the backpack!
            for py in 2..=36 {
                let curve = ((20 - py).max(0) * (20 - py).max(0)) / 28;
                for side in [-1i32, 1i32] {
                    let frame_x = 24 + side * (11 + curve);
                    put_px(&mut data, w, h, frame_x, py, [235, 185, 55, 255]);
                    put_px(&mut data, w, h, frame_x + side, py, [195, 35, 45, 255]);
                    if py % 3 == 0 {
                        for f in 2..=6 {
                            let feather_col = if (py / 3) % 2 == 0 {
                                [250, 252, 255, 245]
                            } else {
                                [235, 45, 58, 245]
                            };
                            put_px(&mut data, w, h, frame_x + side * f, py - 1, feather_col);
                        }
                    }
                }
            }
        }
        1 => {
            // Telemetry whip antenna with glowing cyan beacon
            for py in 2..=16 {
                put_px(&mut data, w, h, 33, py, [190, 205, 220, 255]);
            }
            for dy in -2..=2 {
                for dx in -2..=2 {
                    if dx * dx + dy * dy <= 4 {
                        put_px(&mut data, w, h, 33 + dx, 3 + dy, [75, 245, 255, 255]);
                    }
                }
            }
        }
        2 => {
            // Golden Solar Halo floating above helmet
            for py in 1..=8 {
                for px in 12..=36 {
                    let dx = (px - 24) as f32 / 10.0;
                    let dy = (py - 4) as f32 / 2.5;
                    let r = dx * dx + dy * dy;
                    if (0.55..=1.15).contains(&r) {
                        put_px(&mut data, w, h, px, py, [255, 215, 55, 245]);
                    }
                }
            }
        }
        3 => {
            // Cute Space Cat Ears on helmet
            for side in [-1i32, 1i32] {
                let base_x = 24 + side * 7;
                for dy in 0i32..=6 {
                    let span = (6 - dy) / 2;
                    for dx in -span..=span {
                        let col = if dx.abs() < span && dy > 1 {
                            [255, 130, 175, 255]
                        } else {
                            [240, 245, 255, 255]
                        };
                        put_px(&mut data, w, h, base_x + dx, 8 - dy, col);
                    }
                }
            }
        }
        _ => {
            // Commander Laser Crest
            for py in 2i32..=10 {
                let half = (11 - py) / 2 + 1;
                for dx in -half..=half {
                    let col = if py <= 4 {
                        [70, 235, 255, 255]
                    } else {
                        [245, 195, 45, 255]
                    };
                    put_px(&mut data, w, h, 24 + dx, py, col);
                }
            }
        }
    }
    new_rgba_image(w, h, data)
}

fn make_lander_sprite() -> Image {
    let w = 128u32;
    let h = 128u32;
    let mut data = vec![0u8; (w * h * 4) as usize];
    for py in 0..h as i32 {
        for px in 0..w as i32 {
            let cx = px - 64;
            if (78..=118).contains(&py) {
                let leg_span = (py - 68) * 11 / 10;
                if (cx.abs() - leg_span).abs() <= 3 {
                    put_px(&mut data, w, h, px, py, [150, 155, 165, 255]);
                }
            }
            if (58..=92).contains(&py) && cx.abs() <= 34 {
                let shade = if cx < 0 { 1.0 } else { 0.82 };
                put_px(
                    &mut data,
                    w,
                    h,
                    px,
                    py,
                    [
                        (232.0 * shade) as u8,
                        (178.0 * shade) as u8,
                        (46.0 * shade) as u8,
                        255,
                    ],
                );
            }
            if (22..=58).contains(&py) && cx.abs() <= 26 {
                let shade = if cx < 0 { 1.0 } else { 0.86 };
                let mut col = [
                    (235.0 * shade) as u8,
                    (240.0 * shade) as u8,
                    (248.0 * shade) as u8,
                    255,
                ];
                if (32..=42).contains(&py) && cx.abs() <= 12 {
                    col = [40, 185, 245, 255];
                } else if (46..=50).contains(&py) && cx.abs() <= 16 {
                    col = [220, 38, 50, 255]; // Polish red accent stripe on lander!
                }
                put_px(&mut data, w, h, px, py, col);
            }
            // Insulation folds, docking hatch, ladder and navigation lights.
            if (60..=90).contains(&py) && cx.abs() < 33 && (px + py * 2) % 17 == 0 {
                put_px(&mut data, w, h, px, py, [168, 119, 40, 255]);
            }
            if (51..=78).contains(&py) && cx.abs() <= 8 {
                let edge = cx.abs() >= 6 || py == 51 || py == 78;
                put_px(
                    &mut data,
                    w,
                    h,
                    px,
                    py,
                    if edge {
                        [153, 168, 181, 255]
                    } else {
                        [31, 43, 57, 255]
                    },
                );
            }
            if (79..=117).contains(&py) && cx.abs() <= 6 && (cx.abs() == 6 || py % 6 == 0) {
                put_px(&mut data, w, h, px, py, [181, 190, 201, 255]);
            }
            if (cx.abs() - 24).abs() <= 1 && (26..=28).contains(&py) {
                put_px(&mut data, w, h, px, py, [121, 244, 206, 255]);
            }
        }
    }
    new_rgba_image(w, h, data)
}

fn make_rover_frame(heavy: bool, frame: usize) -> Image {
    let w = 88u32;
    let h = 68u32;
    let mut data = vec![0u8; (w * h * 4) as usize];
    let body_w = if heavy { 30 } else { 22 };
    let susp_bounce = if frame == 1 {
        -1
    } else if frame == 2 {
        1
    } else {
        0
    };

    for py in 0..h as i32 {
        for px in 0..w as i32 {
            let cx = px - 44;

            if (40..=58).contains(&py) && (12..=body_w + 4).contains(&cx.abs()) {
                if cx.abs() % 11 <= 7 {
                    let spoke = ((px + py + frame as i32 * 3) % 5) == 0;
                    let c = if spoke { 125 } else { 52 };
                    put_px(&mut data, w, h, px, py, [c, c + 5, c + 12, 255]);
                }
            }
            let top_y = (if heavy { 16 } else { 28 }) + susp_bounce;
            let bot_y = 46 + susp_bounce;
            if (top_y..=bot_y).contains(&py) && cx.abs() <= body_w {
                let mut col = if heavy {
                    [232, 236, 244, 255]
                } else {
                    [225, 165, 55, 255]
                };
                if heavy && (top_y + 6..=top_y + 16).contains(&py) && cx.abs() <= 16 {
                    col = [55, 210, 250, 255];
                }
                put_px(&mut data, w, h, px, py, col);
            }
            if (34..=40).contains(&py) && (body_w..=body_w + 5).contains(&cx) {
                put_px(&mut data, w, h, px, py, [255, 245, 155, 235]);
            }
        }
    }
    new_rgba_image(w, h, data)
}

// ============================================================================
// 10 DISTINCT 2.5D BUILDING MODELS (Airlock, Habitat, Solar, RTG, O2, Hydro, Storage, Comms, Lab, Wall)
// ============================================================================

/// 1. Airlock: Cylindrical dual-hatch module with yellow/black hazard stripes & green status beacon
fn make_airlock_sprite() -> Image {
    let w = 88u32;
    let h = 88u32;
    let mut data = vec![0u8; (w * h * 4) as usize];
    for py in 24i32..=74 {
        for px in 18i32..=70 {
            let cx: i32 = px - 44;
            if cx.abs() <= 24 {
                let shade = if cx < 0 { 1.0 } else { 0.82 };
                // Yellow & black hazard stripes at top and bottom rims
                let is_hazard = (26..=32).contains(&py) || (66..=72).contains(&py);
                let mut col = if is_hazard {
                    if ((px + py) / 4) % 2 == 0 {
                        [245, 195, 35, 255]
                    } else {
                        [35, 38, 46, 255]
                    }
                } else {
                    [
                        (185.0 * shade) as u8,
                        (195.0 * shade) as u8,
                        (210.0 * shade) as u8,
                        255,
                    ]
                };
                // Heavy round steel airlock hatch in center
                let hatch_d = cx * cx + (py - 50) * (py - 50);
                if hatch_d <= 144 {
                    col = if hatch_d >= 100 {
                        [70, 82, 98, 255]
                    } else {
                        [45, 185, 235, 255]
                    };
                }
                // Green Ready Beacon above hatch
                if (18..=23).contains(&py) && cx.abs() <= 5 {
                    col = [55, 250, 115, 255];
                }
                put_px(&mut data, w, h, px, py, col);
            }
        }
    }
    new_rgba_image(w, h, data)
}

/// 2. Habitat Module: NASA/POLSA White Pressurized Octagonal Living Quarter with panoramic cyan windows
fn make_habitat_sprite() -> Image {
    let w = 88u32;
    let h = 88u32;
    let mut data = vec![0u8; (w * h * 4) as usize];
    for py in 0..h as i32 {
        for px in 0..w as i32 {
            let dx = (px - 44).abs();
            let top_y = 20 + (40 - dx).max(0) / 2;
            let bot_y = 64 + (40 - dx).max(0) / 2;
            if dx <= 40 && py >= top_y && py <= bot_y {
                let shade = if px < 44 { 1.0 } else { 0.84 };
                let mut col = [
                    (236.0 * shade) as u8,
                    (240.0 * shade) as u8,
                    (248.0 * shade) as u8,
                    255,
                ];
                // Gold thermal trim & Polish red-white band
                if (py - top_y) <= 3 {
                    col = [235, 185, 55, 255];
                } else if (py - top_y) >= 12 && (py - top_y) <= 22 && (8..=30).contains(&dx) {
                    col = [65, 225, 255, 255]; // Glowing panoramic window
                } else if (bot_y - py) <= 4 {
                    col = [95, 105, 122, 255];
                }
                put_px(&mut data, w, h, px, py, col);
            }
        }
    }
    new_rgba_image(w, h, data)
}

fn make_habitat_roof_overlay() -> Image {
    let w = 88u32;
    let h = 88u32;
    let mut data = vec![0u8; (w * h * 4) as usize];
    for py in 0..h as i32 {
        for px in 0..w as i32 {
            let dx = (px - 44).abs();
            let dy = (py - 22).abs();
            if dx + 2 * dy <= 40 {
                let col = if dx + 2 * dy >= 36 {
                    [70, 195, 245, 255]
                } else {
                    [230, 236, 246, 255]
                };
                put_px(&mut data, w, h, px, py, col);
            }
        }
    }
    new_rgba_image(w, h, data)
}

/// 3. Solar Array: Tilted deep-blue photovoltaic wings with gold busbars & silver tracking mast
fn make_solar_array_sprite() -> Image {
    let w = 88u32;
    let h = 88u32;
    let mut data = vec![0u8; (w * h * 4) as usize];
    for py in 0..h as i32 {
        for px in 0..w as i32 {
            let dx = (px - 44).abs();
            if (42..=76).contains(&py) && dx <= 4 {
                put_px(&mut data, w, h, px, py, [185, 192, 205, 255]);
            }
            let dy = (py - 32).abs();
            if dx + 2 * dy <= 42 {
                let col = if dx + 2 * dy >= 39 {
                    [240, 195, 65, 255] // Gold frame
                } else if px % 9 == 0 || py % 7 == 0 {
                    [110, 185, 245, 255] // Silver/cyan grid lines
                } else {
                    [24, 62, 155, 255] // Deep photovoltaic cell blue
                };
                put_px(&mut data, w, h, px, py, col);
            }
        }
    }
    new_rgba_image(w, h, data)
}

/// 4. RTG Nuclear Generator: Finned dark cylinder with glowing orange Pu-238 core & radiation trefoil
fn make_rtg_generator_sprite() -> Image {
    let w = 88u32;
    let h = 88u32;
    let mut data = vec![0u8; (w * h * 4) as usize];
    for py in 18i32..=76 {
        for px in 16i32..=72 {
            let cx: i32 = px - 44;
            // Radial cooling fins
            let fin_w = if py % 6 <= 2 { 24 } else { 17 };
            if cx.abs() <= fin_w {
                let mut col = if cx.abs() > 14 {
                    [85, 90, 102, 255] // Radiator fins
                } else if py % 6 == 0 {
                    [255, 115, 28, 255] // Glowing Pu-238 thermal core slots!
                } else {
                    [48, 52, 62, 255]
                };
                // Yellow/black radiation warning badge in center
                if (40..=52).contains(&py) && cx.abs() <= 7 {
                    col = if (cx.abs() + (py - 46).abs()) <= 5 {
                        [255, 215, 35, 255]
                    } else {
                        [25, 25, 30, 255]
                    };
                }
                put_px(&mut data, w, h, px, py, col);
            }
        }
    }
    new_rgba_image(w, h, data)
}

/// 5. Oxygen Electrolyzer: Twin Cryo-Blue vertical O2/H2O tanks with silver pipes & aqua gauges
fn make_o2_electrolyzer_sprite() -> Image {
    let w = 88u32;
    let h = 88u32;
    let mut data = vec![0u8; (w * h * 4) as usize];
    for py in 18i32..=76 {
        for px in 14i32..=74 {
            let cx: i32 = px - 44;
            // Base platform
            if py >= 66 && cx.abs() <= 28 {
                put_px(&mut data, w, h, px, py, [110, 120, 135, 255]);
                continue;
            }
            // Left Tank (-14) and Right Tank (+14)
            let left_d = (cx + 13).abs();
            let right_d = (cx - 13).abs();
            if (22..=66).contains(&py) && (left_d <= 9 || right_d <= 9) {
                let mut col = if left_d <= 9 {
                    [45, 175, 235, 255] // Cryo O2 Tank
                } else {
                    [75, 225, 215, 255] // H2O Electrolysis Tank
                };
                if (36..=42).contains(&py) {
                    col = [235, 245, 255, 255]; // Frost band
                }
                put_px(&mut data, w, h, px, py, col);
            }
            // Connecting overhead manifold pipe
            if (26..=31).contains(&py) && cx.abs() <= 14 {
                put_px(&mut data, w, h, px, py, [210, 220, 232, 255]);
            }
        }
    }
    new_rgba_image(w, h, data)
}

/// 6. Hydroponics Bio-Dome: Geodesic crystal dome with lush green space crops & UV grow-lights
fn make_hydroponics_dome_sprite() -> Image {
    let w = 88u32;
    let h = 88u32;
    let mut data = vec![0u8; (w * h * 4) as usize];
    for py in 0..h as i32 {
        for px in 0..w as i32 {
            let cx = px - 44;
            let cy = py - 58;
            let r2 = cx * cx + cy * cy;
            if py <= 68 && r2 <= 1296 {
                let mut col = if r2 >= 1080 || (px + py) % 11 == 0 || (px - py).abs() % 11 == 0 {
                    [195, 238, 250, 235] // Geodesic glass ribs
                } else if py >= 48 {
                    // Lush green crops + red space tomatoes inside the dome!
                    if (px * 7 + py * 3) % 13 == 0 {
                        [255, 75, 65, 245]
                    } else {
                        [52, 218, 95, 240]
                    }
                } else {
                    [95, 215, 185, 175]
                };
                if (28..=32).contains(&py) && cx.abs() <= 14 {
                    col = [235, 115, 255, 240]; // UV Grow lamp bar
                }
                put_px(&mut data, w, h, px, py, col);
            }
        }
    }
    new_rgba_image(w, h, data)
}

/// 7. Storage Bay: Industrial logistics platform with stacked orange, gold & blue cargo containers
fn make_storage_bay_sprite() -> Image {
    let w = 88u32;
    let h = 88u32;
    let mut data = vec![0u8; (w * h * 4) as usize];
    for py in 24i32..=74 {
        for px in 14i32..=74 {
            let cx: i32 = px - 44;
            if py >= 64 && cx.abs() <= 28 {
                put_px(&mut data, w, h, px, py, [95, 102, 115, 255]);
            } else if (44..=64).contains(&py) && (-25..=-3).contains(&cx) {
                // Left Orange Cargo Container
                let col = if px % 5 == 0 {
                    [195, 105, 28, 255]
                } else {
                    [238, 138, 42, 255]
                };
                put_px(&mut data, w, h, px, py, col);
            } else if (44..=64).contains(&py) && (3..=25).contains(&cx) {
                // Right Cobalt Cargo Container
                let col = if px % 5 == 0 {
                    [38, 115, 185, 255]
                } else {
                    [58, 155, 235, 255]
                };
                put_px(&mut data, w, h, px, py, col);
            } else if (28..=44).contains(&py) && (-15..=15).contains(&cx) {
                // Top Gold Pressurized Crate
                put_px(&mut data, w, h, px, py, [235, 192, 55, 255]);
            }
        }
    }
    new_rgba_image(w, h, data)
}

/// 8. Comms Dish: High-gain parabolic white/gold dish with quad feed horn & skyward laser
fn make_comms_dish_sprite() -> Image {
    let w = 88u32;
    let h = 88u32;
    let mut data = vec![0u8; (w * h * 4) as usize];
    for py in 0..h as i32 {
        for px in 0..w as i32 {
            let cx = px - 44;
            let cy = py - 32;
            if (42..=76).contains(&py) && cx.abs() <= 5 {
                put_px(&mut data, w, h, px, py, [165, 172, 185, 255]);
            }
            let r2 = cx * cx + cy * cy * 2;
            if r2 <= 680 {
                let col = if (540..=680).contains(&r2) {
                    [60, 215, 255, 255]
                } else if r2 <= 55 {
                    [245, 195, 55, 255]
                } else {
                    [235, 242, 250, 255]
                };
                put_px(&mut data, w, h, px, py, col);
            }
            if (8..=28).contains(&py) && cx.abs() <= 2 {
                put_px(&mut data, w, h, px, py, [80, 245, 255, 255]);
            }
        }
    }
    new_rgba_image(w, h, data)
}

/// 9. Science Lab: High-tech silver & violet orbital laboratory with holographic analyzer dome
fn make_science_lab_sprite() -> Image {
    let w = 88u32;
    let h = 88u32;
    let mut data = vec![0u8; (w * h * 4) as usize];
    for py in 14i32..=74 {
        for px in 14i32..=74 {
            let cx: i32 = px - 44;
            // Lower lab hull
            if (38..=70).contains(&py) && cx.abs() <= 26 {
                let mut col = [195, 205, 225, 255];
                if (46..=54).contains(&py) && cx.abs() <= 20 {
                    col = [165, 85, 255, 255]; // Violet quantum scanner band
                }
                put_px(&mut data, w, h, px, py, col);
            }
            // Upper glowing hologram analyzer dome
            let dome_d = cx * cx + (py - 36) * (py - 36);
            if py <= 38 && dome_d <= 320 {
                let col = if dome_d <= 120 {
                    [110, 250, 220, 245]
                } else {
                    [175, 110, 255, 215]
                };
                put_px(&mut data, w, h, px, py, col);
            }
        }
    }
    new_rgba_image(w, h, data)
}

/// 10. Shield Wall: Heavy sloped regolith-titanium armor bastion with orange hazard chevrons
fn make_shield_wall_sprite() -> Image {
    let w = 88u32;
    let h = 88u32;
    let mut data = vec![0u8; (w * h * 4) as usize];
    for py in 26i32..=72 {
        for px in 10i32..=78 {
            let dx = (px - 44).abs();
            if dx <= 32 {
                let shade = if px < 44 { 1.0 } else { 0.80 };
                let mut col = [
                    (138.0 * shade) as u8,
                    (144.0 * shade) as u8,
                    (156.0 * shade) as u8,
                    255,
                ];
                if (26..=31).contains(&py) {
                    col = [205, 215, 230, 255]; // Titanium top armor plate
                } else if (45..=51).contains(&py) && ((px + py) / 5) % 2 == 0 {
                    col = [240, 155, 35, 255]; // Armor chevron
                }
                put_px(&mut data, w, h, px, py, col);
            }
        }
    }
    new_rgba_image(w, h, data)
}

/// Connector sprite that visually merges two adjacent identical buildings into one unified complex!
fn make_connector_sprite(hull: [u8; 3], glow: [u8; 3]) -> Image {
    let w = 64u32;
    let h = 48u32;
    let mut data = vec![0u8; (w * h * 4) as usize];
    for py in 8i32..=40 {
        for px in 6i32..=58 {
            let dx = (px - 32).abs();
            let dy = (py - 24).abs();
            if dx + dy * 2 <= 36 {
                let col = if dy <= 4 {
                    [glow[0], glow[1], glow[2], 245]
                } else {
                    [hull[0], hull[1], hull[2], 245]
                };
                put_px(&mut data, w, h, px, py, col);
            }
        }
    }
    new_rgba_image(w, h, data)
}

// ============================================================================
// 10 WORLD FEATURE & EASTER EGG MODELS (Infinite World Structures & Polish Lore)
// ============================================================================

fn make_feature_supply_pod() -> Image {
    let w = 72u32;
    let h = 72u32;
    let mut data = vec![0u8; (w * h * 4) as usize];
    for py in 20i32..=64 {
        for px in 18i32..=54 {
            let cx: i32 = px - 36;
            let width_at_y = 6 + (py - 20) / 3;
            if cx.abs() <= width_at_y {
                let mut col = if py >= 52 {
                    [245, 130, 35, 255] // Orange flotation/impact airbag
                } else {
                    [235, 240, 248, 255]
                };
                if (22..=27).contains(&py) && cx.abs() <= 4 {
                    col = [65, 255, 120, 255]; // Blinking green locator beacon
                }
                put_px(&mut data, w, h, px, py, col);
            }
        }
    }
    new_rgba_image(w, h, data)
}

fn make_feature_lrv_wreck() -> Image {
    // Mieczysław Bekker's Apollo Lunar Roving Vehicle with gold chassis, wire wheels & umbrella dish
    let w = 88u32;
    let h = 72u32;
    let mut data = vec![0u8; (w * h * 4) as usize];
    for py in 14i32..=62 {
        for px in 14i32..=74 {
            let cx: i32 = px - 44;
            if (44..=58).contains(&py) && (14..=26).contains(&cx.abs()) {
                put_px(&mut data, w, h, px, py, [155, 145, 125, 255]); // Wire-mesh Bekker wheels
            }
            if (36..=46).contains(&py) && cx.abs() <= 24 {
                put_px(&mut data, w, h, px, py, [235, 190, 55, 255]); // Gold Kapton chassis
            }
            // High-gain umbrella antenna
            if (18..=36).contains(&py) && (cx + 10).abs() <= 2 {
                put_px(&mut data, w, h, px, py, [210, 215, 225, 255]);
            }
            if (14..=22).contains(&py) && (cx + 10).abs() <= 12 {
                put_px(&mut data, w, h, px, py, [245, 215, 95, 235]);
            }
        }
    }
    new_rgba_image(w, h, data)
}

fn make_feature_luna_probe() -> Image {
    let w = 72u32;
    let h = 72u32;
    let mut data = vec![0u8; (w * h * 4) as usize];
    for py in 16i32..=62 {
        for px in 12i32..=60 {
            let cx: i32 = px - 36;
            let cy: i32 = py - 42;
            if cx * cx + cy * cy <= 196 {
                let col = if cx * cx + cy * cy <= 36 {
                    [70, 230, 255, 255]
                } else {
                    [165, 175, 190, 255]
                };
                put_px(&mut data, w, h, px, py, col);
            }
            // Splayed whip antennas
            if (cx.abs() - (42 - py)).abs() <= 1 && py < 42 {
                put_px(&mut data, w, h, px, py, [220, 225, 235, 255]);
            }
        }
    }
    new_rgba_image(w, h, data)
}

fn make_feature_twardowski_rooster() -> Image {
    // Easter Egg: Pan Twardowski's Silver & Crimson Space Rooster on a Magic Mirror pedestal!
    let w = 88u32;
    let h = 88u32;
    let mut data = vec![0u8; (w * h * 4) as usize];
    for py in 12i32..=76 {
        for px in 16i32..=72 {
            let cx: i32 = px - 44;
            // Glowing Magic Mirror pedestal at base
            if (58..=72).contains(&py) && cx.abs() + (py - 65).abs() * 2 <= 26 {
                put_px(&mut data, w, h, px, py, [110, 225, 255, 245]);
            }
            // Rooster body & proud tail feathers
            if (32..=56).contains(&py) && (-14..=12).contains(&cx) {
                put_px(&mut data, w, h, px, py, [242, 245, 252, 255]);
            }
            if (24..=46).contains(&py) && (-24..=-10).contains(&cx) {
                put_px(&mut data, w, h, px, py, [225, 35, 52, 255]); // Crimson tail plume
            }
            // Rooster head & red comb
            if (18..=32).contains(&py) && (4..=16).contains(&cx) {
                let col = if py <= 23 {
                    [240, 32, 48, 255] // Red crest
                } else {
                    [248, 248, 255, 255]
                };
                put_px(&mut data, w, h, px, py, col);
            }
            // Golden beak
            if (24..=28).contains(&py) && (16..=22).contains(&cx) {
                put_px(&mut data, w, h, px, py, [255, 200, 40, 255]);
            }
        }
    }
    new_rgba_image(w, h, data)
}

fn make_feature_polish_flag() -> Image {
    // White-and-Red Polish Flag (BIAŁO-CZERWONA) with glowing lunar beacon base!
    let w = 72u32;
    let h = 84u32;
    let mut data = vec![0u8; (w * h * 4) as usize];
    for py in 10i32..=76 {
        // Illuminated base
        if py >= 66 {
            for px in 24i32..=48 {
                put_px(&mut data, w, h, px, py, [90, 235, 255, 255]);
            }
        }
        // Silver flagpole + golden finial
        let pole_col = if py <= 14 {
            [255, 215, 55, 255]
        } else {
            [225, 232, 242, 255]
        };
        put_px(&mut data, w, h, 28, py, pole_col);
        put_px(&mut data, w, h, 29, py, pole_col);

        // Waving White & Red Flag
        if (15..=36).contains(&py) {
            for px in 30i32..=62 {
                let wave = ((px - 30) / 6) % 2;
                let wy = py + wave;
                let col = if wy <= 25 {
                    [250, 252, 255, 255] // White top half
                } else {
                    [220, 28, 48, 255] // Crimson-red bottom half
                };
                put_px(&mut data, w, h, px, wy, col);
            }
        }
    }
    new_rgba_image(w, h, data)
}

fn make_feature_crystal_cluster() -> Image {
    let w = 72u32;
    let h = 72u32;
    let mut data = vec![0u8; (w * h * 4) as usize];
    let spires = [(-12i32, 26i32, 58i32), (0, 14, 60), (13, 24, 58)];
    for (offset_x, top_y, bot_y) in spires {
        for py in top_y..=bot_y {
            let half_w = ((py - top_y) / 4).clamp(1, 7);
            for dx in -half_w..=half_w {
                let col = if dx < 0 {
                    [95, 245, 255, 240]
                } else {
                    [175, 95, 255, 240]
                };
                put_px(&mut data, w, h, 36 + offset_x + dx, py, col);
            }
        }
    }
    new_rgba_image(w, h, data)
}

fn make_feature_kopernik_ruin() -> Image {
    let w = 96u32;
    let h = 96u32;
    let mut data = vec![0u8; (w * h * 4) as usize];
    for py in 22i32..=80 {
        for px in 12i32..=84 {
            let cx: i32 = px - 48;
            let cy: i32 = py - 68;
            let r2 = cx * cx + cy * cy;
            if py <= 74 && r2 <= 1450 {
                // Scorched / cracked outpost hub with Polish red-white stripe & glowing amber core
                if cx > 8 && py < 42 {
                    continue; // Damaged breach in roof!
                }
                let mut col = [145, 152, 166, 255];
                if (52..=56).contains(&py) {
                    col = [205, 42, 52, 255];
                } else if r2 <= 220 {
                    col = [255, 185, 45, 255]; // Emergency black-box beacon!
                }
                put_px(&mut data, w, h, px, py, col);
            }
        }
    }
    new_rgba_image(w, h, data)
}

fn make_feature_lem_monolith() -> Image {
    // Towering 1:4:9 Jet-Black Obelisk ("Niezwyciężony" / Solaris) with pulsing golden-violet aura
    let w = 88u32;
    let h = 104u32;
    let mut data = vec![0u8; (w * h * 4) as usize];
    for py in 8i32..=92 {
        for px in 22i32..=66 {
            let cx: i32 = px - 44;
            if (12..=86).contains(&py) && cx.abs() <= 12 {
                let col = if cx == -12 || py == 12 {
                    [185, 125, 255, 255] // Crisp cosmic edge highlight
                } else if py == 32 && cx.abs() <= 6 {
                    [255, 210, 65, 255] // Mysterious golden glyph
                } else {
                    [18, 20, 32, 255] // Deep obsidian monolith
                };
                put_px(&mut data, w, h, px, py, col);
            } else if cx.abs() <= 16 && (10..=88).contains(&py) && (px + py) % 3 == 0 {
                put_px(&mut data, w, h, px, py, [155, 75, 255, 140]); // Energy aura
            }
        }
    }
    new_rgba_image(w, h, data)
}

fn make_feature_cargo_wreck() -> Image {
    let w = 96u32;
    let h = 88u32;
    let mut data = vec![0u8; (w * h * 4) as usize];
    for py in 24i32..=76 {
        for px in 10i32..=86 {
            let cx: i32 = px - 48;
            if cx.abs() <= 34 && py >= 32 + (cx + 34) / 5 {
                let col = if (px / 6) % 2 == 0 && (46..=56).contains(&py) {
                    [255, 195, 55, 255] // Glowing He-3 cells inside cracked hull
                } else {
                    [125, 134, 148, 255]
                };
                put_px(&mut data, w, h, px, py, col);
            }
        }
    }
    new_rgba_image(w, h, data)
}

fn make_feature_ice_rig() -> Image {
    let w = 88u32;
    let h = 96u32;
    let mut data = vec![0u8; (w * h * 4) as usize];
    for py in 12i32..=82 {
        for px in 18i32..=70 {
            let cx: i32 = px - 44;
            let tower_span = (py - 8) / 4 + 3;
            if (cx.abs() - tower_span).abs() <= 2 || (py % 10 == 0 && cx.abs() <= tower_span) {
                put_px(&mut data, w, h, px, py, [175, 205, 230, 255]);
            }
            // Central cryo-drill bit
            if cx.abs() <= 3 && py >= 20 {
                put_px(&mut data, w, h, px, py, [75, 235, 255, 255]);
            }
        }
    }
    new_rgba_image(w, h, data)
}

fn make_shadow_disc() -> Image {
    let w = 64u32;
    let h = 32u32;
    let mut data = vec![0u8; (w * h * 4) as usize];
    for py in 0..h as i32 {
        for px in 0..w as i32 {
            let dx = (px - 32) as f32 / 30.0;
            let dy = (py - 16) as f32 / 14.0;
            let d = dx * dx + dy * dy;
            if d <= 1.0 {
                let idx = ((py as u32 * w + px as u32) * 4) as usize;
                let alpha = ((1.0 - d) * 155.0) as u8;
                data[idx + 3] = alpha;
            }
        }
    }
    new_rgba_image(w, h, data)
}

/// Generates 8-directional astronaut sprites (`dir: 0..8`, `frame: 0..4`):
/// 0 = S (Front), 1 = SE (Front-Right), 2 = E (Right), 3 = NE (Back-Right),
/// 4 = N (Back), 5 = NW (Back-Left), 6 = W (Left), 7 = SW (Front-Left).
fn make_astronaut_dir_frame(dir: usize, frame: usize) -> Image {
    let w = 48u32;
    let h = 64u32;
    let mut data = vec![0u8; (w * h * 4) as usize];
    let (left_leg_dy, right_leg_dy, arm_swing) = match frame {
        1 => (-4, 3, -3),
        2 => (3, -4, 3),
        3 => (-5, -5, -4),
        _ => (0, 0, 0),
    };

    // Directional offsets for 3D-like rotation!
    // face_dx: where the front of the body/visor points (-7..+7)
    // is_back: whether the PLSS backpack is in the foreground (facing North / away)
    let (face_dx, pack_dx, is_back, torso_half_w): (i32, i32, bool, i32) = match dir % 8 {
        0 => (0, 0, false, 10), // S (Front)
        1 => (4, -4, false, 9), // SE
        2 => (6, -6, false, 8), // E (Right profile)
        3 => (4, -3, true, 9),  // NE (Back-Right)
        4 => (0, 0, true, 10),  // N (Back)
        5 => (-4, 3, true, 9),  // NW (Back-Left)
        6 => (-6, 6, false, 8), // W (Left profile)
        _ => (-4, 4, false, 9), // SW
    };

    for py in 0..h as i32 {
        for px in 0..w as i32 {
            let cx: i32 = px - 24;

            // 1. Background PLSS Backpack (when facing Front / Side)
            if !is_back && (18..=42).contains(&py) && ((cx - pack_dx).abs() <= 11) {
                put_px(&mut data, w, h, px, py, [196, 204, 216, 255]);
            }

            // Jetpack RCS flame plumes on frame 3
            if frame == 3 && (42..=55).contains(&py) && (6..=12).contains(&(cx - pack_dx).abs()) {
                put_px(&mut data, w, h, px, py, [90, 228, 255, 235]);
            }

            // 2. Helmet sphere
            let h_dist = (cx - face_dx / 3) * (cx - face_dx / 3) + (py - 15) * (py - 15);
            if h_dist <= 110 {
                let shade = if is_back && h_dist < 64 { 232 } else { 246 };
                put_px(&mut data, w, h, px, py, [shade, shade + 2, 255, 255]);
            }

            // 3. Torso & Swinging Arms
            if (24..=45).contains(&py) && cx.abs() <= torso_half_w {
                put_px(&mut data, w, h, px, py, [238, 240, 246, 255]);
                // Chest telemetry module & Polish red-white badge stripe (only when front is visible)
                if !is_back {
                    let badge_cx = cx - face_dx / 2;
                    if (28..=31).contains(&py) && badge_cx.abs() <= 4 {
                        put_px(&mut data, w, h, px, py, [252, 252, 255, 255]);
                    } else if (32..=35).contains(&py) && badge_cx.abs() <= 4 {
                        put_px(&mut data, w, h, px, py, [225, 38, 52, 255]);
                    }
                }
            }

            if (26 + arm_swing..=40 + arm_swing).contains(&py)
                && (torso_half_w..=torso_half_w + 3).contains(&cx.abs())
            {
                put_px(&mut data, w, h, px, py, [222, 228, 238, 255]);
            }

            // 4. Foreground PLSS Life-Support Backpack (when facing North / NE / NW!)
            if is_back && (19..=43).contains(&py) && (cx - pack_dx).abs() <= 9 {
                let bx = cx - pack_dx;
                let mut col = [208, 214, 226, 255];
                if (22..=25).contains(&py) && bx.abs() <= 6 {
                    col = [250, 252, 255, 255]; // Polish white stripe on backpack
                } else if (26..=29).contains(&py) && bx.abs() <= 6 {
                    col = [220, 36, 50, 255]; // Polish red stripe on backpack
                } else if (33..=39).contains(&py) && (bx.abs() == 4 || bx.abs() == 5) {
                    col = [75, 215, 255, 255]; // Twin cryo O2 tank gauges
                }
                put_px(&mut data, w, h, px, py, col);
            }

            // 5. Animated Left & Right Legs
            let leg_offset = if cx < 0 { left_leg_dy } else { right_leg_dy };
            let ly = py - leg_offset;
            if (45..=59).contains(&ly) && (2..=7).contains(&cx.abs()) {
                let col = if ly >= 55 {
                    [85, 92, 104, 255]
                } else {
                    [210, 214, 222, 255]
                };
                put_px(&mut data, w, h, px, py, col);
            }
        }
    }
    new_rgba_image(w, h, data)
}

/// Generates 8-directional helmet visor sprites matching `make_astronaut_dir_frame`.
fn make_visor_dir_sprite(dir: usize) -> Image {
    let w = 48u32;
    let h = 64u32;
    let mut data = vec![0u8; (w * h * 4) as usize];
    // Direction 4 (North / straight back) has the visor completely hidden on the far side of the helmet
    if dir % 8 == 4 {
        return new_rgba_image(w, h, data);
    }

    let (visor_cx_center, rx_limit): (i32, i32) = match dir % 8 {
        0 => (24, 7), // S (Full front visor)
        1 => (28, 6), // SE
        2 => (30, 5), // E (Side profile visor)
        3 => (31, 3), // NE (Edge of visor)
        5 => (17, 3), // NW (Edge of visor)
        6 => (18, 5), // W (Side profile visor)
        _ => (20, 6), // SW
    };

    for py in 9i32..=20 {
        for px in 10i32..=38 {
            let dx: i32 = px - visor_cx_center;
            let dy: i32 = py - 15;
            if dx.abs() <= rx_limit && (dx * dx + dy * dy <= 50) {
                let highlight = if dx < 0 && dy < -1 { 255 } else { 225 };
                put_px(
                    &mut data,
                    w,
                    h,
                    px,
                    py,
                    [highlight, highlight, highlight, 255],
                );
            }
        }
    }
    new_rgba_image(w, h, data)
}

/// Warm, illuminated pressurized interior floor revealed when entering a building!
fn make_interior_floor_sprite() -> Image {
    let w = 88u32;
    let h = 56u32;
    let mut data = vec![0u8; (w * h * 4) as usize];
    for py in 0..44i32 {
        for px in 0..88i32 {
            let dx = (px - 44).abs();
            let dy = (py - 22).abs();
            let m = dx + 2 * dy;
            if m <= 42 {
                let mut col = [48, 66, 88, 245];
                if m >= 38 {
                    col = [85, 235, 255, 255]; // Glowing pressurized floor perimeter seal
                } else if (px + py) % 11 == 0 {
                    col = [72, 98, 128, 245]; // Clean modular deck grating
                } else if dx <= 10 && dy <= 5 {
                    col = [65, 215, 155, 250]; // Central life-support floor pad
                }
                put_px(&mut data, w, h, px, py, col);
            }
        }
    }
    new_rgba_image(w, h, data)
}

/// 4-stage progressive block mining crack overlay (`stage: 0..4`) like voxel block breaking!
fn make_crack_overlay_sprite(stage: usize) -> Image {
    let w = 88u32;
    let h = 56u32;
    let mut data = vec![0u8; (w * h * 4) as usize];
    let max_len = 8 + (stage as i32 + 1) * 7;
    let branches: [(i32, i32); 8] = [
        (1, 0),
        (-1, 0),
        (0, 1),
        (0, -1),
        (2, 1),
        (-2, -1),
        (2, -1),
        (-2, 1),
    ];

    for (b_idx, (vx, vy)) in branches.iter().enumerate() {
        if b_idx > (stage + 1) * 2 {
            continue;
        }
        for step in 1..=max_len {
            let jitter = ((step * 7 + b_idx as i32 * 3) % 3) - 1;
            let px = 44 + vx * step + jitter;
            let py = 22 + (vy * step) / 2 - jitter / 2;
            let dx = (px - 44).abs();
            let dy = (py - 22).abs();
            if dx + 2 * dy <= 38 {
                put_px(&mut data, w, h, px, py, [20, 24, 32, 235]);
                put_px(&mut data, w, h, px + 1, py, [95, 240, 255, 220]);
            }
        }
    }
    new_rgba_image(w, h, data)
}

/// Generates a vibrant 64x64 RGBA Window & Taskbar Icon:
/// Deep space badge + Glowing Crescent Moon + Astronaut Helmet with 24K Gold Visor + Polish White-Red Wing Emblem!
pub fn generate_window_icon_rgba_64() -> Vec<u8> {
    let w = 64u32;
    let h = 64u32;
    let mut data = vec![0u8; (w * h * 4) as usize];

    for py in 0..64i32 {
        for px in 0..64i32 {
            let cx = px - 32;
            let cy = py - 32;
            let r2 = cx * cx + cy * cy;

            // Rounded cosmic badge background with glowing cyan-gold border
            if r2 <= 30 * 30 {
                let mut col = if r2 >= 27 * 27 {
                    [65, 220, 255, 255]
                } else {
                    [14, 24, 48, 255]
                };

                // Background stars
                if ((px * 31 + py * 17) % 53 == 0) && r2 < 25 * 25 {
                    col = [230, 245, 255, 255];
                }

                // Glowing Silver-Gold Crescent Moon on upper-left
                let m1 = (px - 24) * (px - 24) + (py - 24) * (py - 24);
                let m2 = (px - 30) * (px - 30) + (py - 20) * (py - 20);
                if m1 <= 18 * 18 && m2 > 15 * 15 {
                    col = [245, 220, 130, 255];
                }

                // Astronaut Helmet Dome on center-right
                let h_dist = (px - 36) * (px - 36) + (py - 36) * (py - 36);
                if h_dist <= 14 * 14 {
                    col = [242, 246, 255, 255];
                    // 24K Golden Reflective Visor
                    let v_dist = (px - 38) * (px - 38) + (py - 35) * (py - 35);
                    if v_dist <= 9 * 9 {
                        col = if px < 37 && py < 34 {
                            [255, 238, 125, 255]
                        } else {
                            [245, 175, 35, 255]
                        };
                    }
                }

                // Polish White-and-Crimson Banner at the bottom of the icon
                if (49..=54).contains(&py) && cx.abs() <= 16 {
                    col = if py <= 51 {
                        [252, 252, 255, 255]
                    } else {
                        [225, 35, 52, 255]
                    };
                }

                put_px(&mut data, w, h, px, py, col);
            }
        }
    }
    data
}
