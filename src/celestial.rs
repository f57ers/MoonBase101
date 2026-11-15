use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use std::f32::consts::TAU;

use crate::types::{AppState, GameSettings, LunarOrbit, MissionConfig};

#[derive(Clone, Debug)]
pub struct PlanetBody {
    pub name: &'static str,
    pub semi_major_au: f32,
    pub eccentricity: f32,
    pub orbital_period_days: f32,
    pub inclination_deg: f32,
    pub mean_longitude_rad: f32,
    pub radius_px: f32,
    pub color: Color,
    pub has_rings: bool,
}

#[derive(Clone, Debug)]
pub struct InfallingMatter {
    pub angle_rad: f32,
    pub radius_norm: f32,
    pub speed_mult: f32,
    pub inclination_offset: f32,
    pub color: Color,
    pub size: f32,
}

#[derive(Resource)]
pub struct SolarSystemState {
    pub sim_time_days: f32,
    pub anim_time_sec: f32,
    pub time_warp: f32,
    pub lunar_sun_angle_rad: f32,
    pub relay_orbit_angle_rad: f32,
    pub show_gmat_overlay: bool,
    pub planets: Vec<PlanetBody>,
    pub stars: Vec<(Vec2, f32, Color)>,
    pub black_hole_matter: Vec<InfallingMatter>,
}

impl Default for SolarSystemState {
    fn default() -> Self {
        let planets = vec![
            PlanetBody {
                name: "Mercury",
                semi_major_au: 0.387,
                eccentricity: 0.205,
                orbital_period_days: 87.97,
                inclination_deg: 7.0,
                mean_longitude_rad: 1.1,
                radius_px: 2.5,
                color: Color::rgb(0.76, 0.73, 0.70),
                has_rings: false,
            },
            PlanetBody {
                name: "Venus",
                semi_major_au: 0.723,
                eccentricity: 0.007,
                orbital_period_days: 224.70,
                inclination_deg: 3.4,
                mean_longitude_rad: 2.4,
                radius_px: 3.5,
                color: Color::rgb(0.96, 0.88, 0.65),
                has_rings: false,
            },
            PlanetBody {
                name: "Earth",
                semi_major_au: 1.000,
                eccentricity: 0.017,
                orbital_period_days: 365.25,
                inclination_deg: 0.0,
                mean_longitude_rad: 0.6,
                radius_px: 5.5,
                color: Color::rgb(0.22, 0.62, 0.98),
                has_rings: false,
            },
            PlanetBody {
                name: crate::locale::text("Mars"),
                semi_major_au: 1.524,
                eccentricity: 0.093,
                orbital_period_days: 686.98,
                inclination_deg: 1.85,
                mean_longitude_rad: 4.1,
                radius_px: 3.2,
                color: Color::rgb(0.92, 0.42, 0.26),
                has_rings: false,
            },
            PlanetBody {
                name: "Jupiter",
                semi_major_au: 5.203,
                eccentricity: 0.048,
                orbital_period_days: 4332.59,
                inclination_deg: 1.3,
                mean_longitude_rad: 1.9,
                radius_px: 5.0,
                color: Color::rgb(0.88, 0.74, 0.56),
                has_rings: false,
            },
            PlanetBody {
                name: "Saturn",
                semi_major_au: 9.537,
                eccentricity: 0.054,
                orbital_period_days: 10759.22,
                inclination_deg: 2.49,
                mean_longitude_rad: 5.2,
                radius_px: 4.5,
                color: Color::rgb(0.92, 0.84, 0.60),
                has_rings: true,
            },
            PlanetBody {
                name: "Uranus",
                semi_major_au: 19.19,
                eccentricity: 0.047,
                orbital_period_days: 30687.0,
                inclination_deg: 0.77,
                mean_longitude_rad: 3.3,
                radius_px: 3.0,
                color: Color::rgb(0.48, 0.88, 0.93),
                has_rings: false,
            },
            PlanetBody {
                name: "Neptune",
                semi_major_au: 30.07,
                eccentricity: 0.009,
                orbital_period_days: 60190.0,
                inclination_deg: 1.77,
                mean_longitude_rad: 5.8,
                radius_px: 3.0,
                color: Color::rgb(0.25, 0.43, 0.98),
                has_rings: false,
            },
        ];

        let mut stars = Vec::with_capacity(140);
        for i in 0..140 {
            let fi = i as f32;
            let sx = ((fi * 127.1).sin() * 43758.5453).fract() * 2400.0 - 1200.0;
            let sy = ((fi * 311.7).cos() * 21983.132).fract() * 1400.0 - 700.0;
            let mag = 0.8 + ((fi * 73.3).sin().abs()) * 1.6;
            let tint = match i % 4 {
                0 => Color::rgb(0.88, 0.92, 1.0),
                1 => Color::rgb(1.0, 0.96, 0.86),
                2 => Color::rgb(0.70, 0.86, 1.0),
                _ => Color::rgb(0.98, 0.80, 0.72),
            };
            stars.push((Vec2::new(sx, sy), mag, tint));
        }

        // 56 3D-orbiting matter clumps / stars being pulled across the Black Hole Event Horizon!
        let mut black_hole_matter = Vec::with_capacity(56);
        for i in 0..56 {
            let fi = i as f32;
            let golden_ratio = (1.0 + 5.0_f32.sqrt()) * 0.5;
            let angle_rad = (fi * golden_ratio * TAU) % TAU;
            let radius_norm = 0.16 + ((fi * 0.37).fract()) * 0.84;
            let speed_mult = 0.75 + ((fi * 0.53).fract()) * 0.90;
            let inclination_offset = ((fi * 2.1).sin()) * 0.22;
            let color = match i % 5 {
                0 => Color::rgb(0.45, 0.92, 1.0),
                1 => Color::rgb(1.0, 0.82, 0.32),
                2 => Color::rgb(1.0, 0.45, 0.22),
                3 => Color::rgb(0.92, 0.32, 0.65),
                _ => Color::rgb(0.95, 0.98, 1.0),
            };
            let size = 2.0 + (i % 4) as f32 * 1.1;
            black_hole_matter.push(InfallingMatter {
                angle_rad,
                radius_norm,
                speed_mult,
                inclination_offset,
                color,
                size,
            });
        }

        Self {
            sim_time_days: 0.0,
            anim_time_sec: 0.0,
            time_warp: 12.0,
            lunar_sun_angle_rad: 0.85,
            relay_orbit_angle_rad: 0.0,
            show_gmat_overlay: false,
            planets,
            stars,
            black_hole_matter,
        }
    }
}

impl SolarSystemState {
    pub fn sun_illumination(&self) -> f32 {
        (self.lunar_sun_angle_rad.sin() * 0.5 + 0.5).clamp(0.0, 1.0)
    }

    pub fn surface_temperature_c(&self, scenario_polar: bool) -> f32 {
        let sun = self.sun_illumination();
        if scenario_polar {
            -155.0 + sun * 135.0
        } else {
            -172.0 + sun * 293.0
        }
    }

    pub fn sun_shadow_offset(&self) -> Vec2 {
        let angle = self.lunar_sun_angle_rad;
        let len = (1.35 - self.sun_illumination() * 0.75).clamp(0.45, 1.45) * 32.0;
        Vec2::new(-angle.cos() * len, -angle.sin().abs() * 12.0 - 4.0)
    }

    pub fn relay_link_active(&self, orbit: LunarOrbit) -> bool {
        match orbit {
            LunarOrbit::NearRectilinearHalo => true,
            LunarOrbit::PolarSunSync => self.relay_orbit_angle_rad.sin() > -0.65,
            LunarOrbit::LowLunarOrbit => self.relay_orbit_angle_rad.cos() > -0.25,
            LunarOrbit::DirectSurfaceTli => self.lunar_sun_angle_rad.cos() > -0.45,
        }
    }
}

pub fn update_celestial_mechanics(
    time: Res<Time>,
    lan: Option<Res<crate::net_lan::LanMultiplayer>>,
    mut solar: ResMut<SolarSystemState>,
    mut settings: ResMut<GameSettings>,
    keys: Res<ButtonInput<KeyCode>>,
    state: Res<State<AppState>>,
) {
    if keys.just_pressed(KeyCode::KeyO) && *state.get() == AppState::LunarSurface {
        settings.show_corner_radar = !settings.show_corner_radar;
        solar.show_gmat_overlay = settings.show_corner_radar;
    }

    if matches!(
        state.get(),
        AppState::Paused | AppState::MissionReport | AppState::GameOver | AppState::Discovery
    ) && !lan.as_ref().is_some_and(|l| l.is_active)
    {
        return;
    }

    let dt = time.delta_seconds().min(0.05);
    solar.anim_time_sec += dt;
    solar.sim_time_days += dt * solar.time_warp * 0.10;
    solar.lunar_sun_angle_rad = (solar.lunar_sun_angle_rad + dt * 0.032) % TAU;
    solar.relay_orbit_angle_rad = (solar.relay_orbit_angle_rad + dt * 0.50) % TAU;

    for planet in &mut solar.planets {
        let angular_vel = TAU / planet.orbital_period_days.sqrt();
        planet.mean_longitude_rad = (planet.mean_longitude_rad + dt * angular_vel * 2.5) % TAU;
    }

    // Update 3D Black Hole Infalling Matter spiraling across the Event Horizon!
    for m in &mut solar.black_hole_matter {
        // Relativistic Keplerian angular speed increases dramatically near the event horizon (r -> 0.12)
        let r_clamped = m.radius_norm.max(0.13);
        let omega = (0.85 / (r_clamped * r_clamped.sqrt())) * m.speed_mult;
        m.angle_rad = (m.angle_rad + dt * omega) % TAU;
        // Radial inspiral toward the Schwarzschild Event Horizon
        m.radius_norm -= dt * (0.065 + (1.0 - r_clamped) * 0.14) * m.speed_mult;
        if m.radius_norm <= 0.13 {
            // Re-inject at the outer tidal capture boundary
            m.radius_norm = 0.98;
        }
    }
}

/// Projects a 3D point `(x, y, z)` around `center` onto 2D screen space with pitch & roll tilt.
#[inline]
fn project_3d_disk(center: Vec2, p3: Vec3, tilt_pitch: f32, tilt_roll: f32) -> (Vec2, f32) {
    // Rotate around X (pitch)
    let (sp, cp) = tilt_pitch.sin_cos();
    let y1 = p3.y * cp - p3.z * sp;
    let z1 = p3.y * sp + p3.z * cp;
    // Rotate around Z (roll)
    let (sr, cr) = tilt_roll.sin_cos();
    let x2 = p3.x * cr - y1 * sr;
    let y2 = p3.x * sr + y1 * cr;
    // Perspective foreshortening
    let persp = 900.0 / (900.0 - z1.clamp(-400.0, 400.0));
    (center + Vec2::new(x2 * persp, y2 * persp), z1)
}

pub fn render_celestial_backdrop_and_gmat(
    mut gizmos: Gizmos,
    solar: Res<SolarSystemState>,
    _config: Res<MissionConfig>,
    settings: Res<GameSettings>,
    state: Res<State<AppState>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_q: Query<(&Transform, &OrthographicProjection), With<Camera2d>>,
) {
    let (cam_pos, camera_scale) = camera_q
        .get_single()
        .map(|(t, p)| (t.translation.truncate(), p.scale))
        .unwrap_or((Vec2::ZERO, 1.0));

    let (win_w, win_h) = windows
        .get_single()
        .map(|w| (w.width(), w.height()))
        .unwrap_or((1440.0, 900.0));

    let cur_state = *state.get();

    // 1. Deep-space starfield background
    if cur_state != AppState::LunarSurface || settings.show_horizon_planets {
        for (i, (star_offset, mag, color)) in solar.stars.iter().enumerate() {
            let parallax = cam_pos + *star_offset * 0.55;
            let twinkle = 0.65 + 0.35 * (solar.anim_time_sec * 2.6 + i as f32).sin();
            gizmos.circle_2d(
                parallax,
                *mag * 0.78,
                color.with_a((twinkle * 0.55).clamp(0.18, 0.75)),
            );
        }
    }

    // 2. 3D RELATIVISTIC BLACK HOLE ("GARGANTUA") & EVENT HORIZON IN MENU BACKDROPS!
    if matches!(
        cur_state,
        AppState::MainMenu | AppState::Settings | AppState::Controls | AppState::MissionDesignLab
    ) {
        let t = solar.anim_time_sec;
        let bh_center = if cur_state == AppState::MainMenu {
            cam_pos
        } else {
            cam_pos + Vec2::new(-win_w * 0.34, win_h * 0.18)
        };
        let bh_scale = if cur_state == AppState::MainMenu {
            0.68
        } else {
            0.78
        };

        let horizon_r = 58.0 * bh_scale;
        let photon_r = 65.0 * bh_scale;
        let max_disk_r = 390.0 * bh_scale;
        let pitch = 1.24 + (t * 0.28).sin() * 0.06; // ~71 deg 3D viewing inclination
        let roll = -0.32 + (t * 0.17).cos() * 0.05;

        // A) 3D Spacetime Gravitational Funnel Grid Warping Into the Singularity
        for sector in 0..16 {
            let a = (sector as f32 / 16.0) * TAU + t * 0.08;
            let mut prev: Option<Vec2> = None;
            for step in 2..=12 {
                let frac = step as f32 / 12.0;
                let r = horizon_r * 0.8 + frac * max_disk_r * 0.95;
                let gravity_dip = -140.0 * bh_scale / (frac * 3.2 + 0.35);
                let p3 = Vec3::new(a.cos() * r, a.sin() * r, gravity_dip);
                let (p2, _) = project_3d_disk(bh_center, p3, pitch, roll);
                if let Some(p0) = prev {
                    gizmos.line_2d(
                        p0,
                        p2,
                        Color::rgba(
                            0.18,
                            0.52,
                            0.92,
                            (0.24 * (1.0 - frac * 0.5)).clamp(0.05, 0.25),
                        ),
                    );
                }
                prev = Some(p2);
            }
        }

        // B) Relativistic Twin Polar Plasma Jets (shooting along the 3D Z-axis of the Black Hole)
        for dir_sign in [-1.0f32, 1.0f32] {
            for seg in 1..=18 {
                let z_dist = dir_sign * (horizon_r * 0.9 + seg as f32 * 22.0 * bh_scale);
                let cone_spread =
                    (seg as f32 * 2.8 + (t * 8.0 - seg as f32 * 0.6).sin() * 4.0) * bh_scale;
                let (jet_pt, _) =
                    project_3d_disk(bh_center, Vec3::new(0.0, 0.0, z_dist), pitch, roll);
                let alpha = (1.0 - seg as f32 / 19.0) * 0.45;
                let jet_col = if seg % 2 == 0 {
                    Color::rgba(0.38, 0.88, 1.0, alpha)
                } else {
                    Color::rgba(0.75, 0.45, 1.0, alpha)
                };
                gizmos.circle_2d(jet_pt, cone_spread.max(2.0), jet_col);
            }
        }

        // C) Gravitationally Lensed Upper & Lower Einstein Halo (Bent light from back of disk!)
        for i in 0..8 {
            let lensed_r = photon_r + 4.0 + i as f32 * 5.2 * bh_scale;
            let alpha = (0.48 - i as f32 * 0.05).max(0.06);
            let col = if i < 3 {
                Color::rgba(1.0, 0.92, 0.72, alpha)
            } else {
                Color::rgba(1.0, 0.48, 0.18, alpha * 0.8)
            };
            gizmos.ellipse_2d(
                bh_center,
                roll,
                Vec2::new(lensed_r * 1.18, lensed_r * 0.92),
                col,
            );
        }

        // D) 3D Multi-Ring Relativistic Accretion Disk with Doppler Beaming
        for ring_idx in 0..22 {
            let frac = ring_idx as f32 / 21.0;
            let r = photon_r * 1.06 + frac * (max_disk_r - photon_r);
            let ring_speed = (1.8 / (0.3 + frac * 1.4)) * t;
            let segments = 40;
            for s in 0..segments {
                let a0 = (s as f32 / segments as f32) * TAU + ring_speed;
                let a1 = ((s as f32 + 0.75) / segments as f32) * TAU + ring_speed;
                let z_warp = (a0 * 2.0 + t).sin() * 4.5 * frac * bh_scale;

                let (p0, z0) = project_3d_disk(
                    bh_center,
                    Vec3::new(a0.cos() * r, a0.sin() * r, z_warp),
                    pitch,
                    roll,
                );
                let (p1, _) = project_3d_disk(
                    bh_center,
                    Vec3::new(a1.cos() * r, a1.sin() * r, z_warp),
                    pitch,
                    roll,
                );

                // Hide back-of-disk segments that pass directly behind the Black Hole shadow
                if z0 < 0.0 && p0.distance(bh_center) < horizon_r * 0.95 {
                    continue;
                }

                // Relativistic Doppler shift: left/approaching side is blue-white hot, receding side is deep orange-red
                let doppler = (-a0.sin() * 0.5 + 0.5).clamp(0.0, 1.0);
                let heat = (1.0 - frac).powf(0.65);
                let col = Color::rgba(
                    (0.95 * heat + 0.45 * (1.0 - doppler)).clamp(0.25, 1.0),
                    (0.32 + 0.62 * heat * doppler).clamp(0.15, 1.0),
                    (0.12 + 0.88 * doppler * heat).clamp(0.08, 1.0),
                    (0.18 + 0.65 * heat).clamp(0.12, 0.85),
                );
                gizmos.line_2d(p0, p1, col);
            }
        }

        // E) Infalling Stars & Spaghettified Matter Streams Crossing the Event Horizon!
        for m in &solar.black_hole_matter {
            let r = horizon_r * 0.92 + (m.radius_norm - 0.13).max(0.0) * (max_disk_r - horizon_r);
            let z_spiral = m.inclination_offset * r * (m.angle_rad * 1.5).cos();
            let p3 = Vec3::new(m.angle_rad.cos() * r, m.angle_rad.sin() * r, z_spiral);
            let (head_2d, z_depth) = project_3d_disk(bh_center, p3, pitch, roll);

            if z_depth < 0.0 && head_2d.distance(bh_center) < horizon_r * 0.92 {
                continue;
            }

            // Draw elongated spaghettification tail as matter approaches the Event Horizon
            let stretch = (1.0 / m.radius_norm.max(0.14)).clamp(1.0, 6.5);
            let mut prev_pt = head_2d;
            for tail_step in 1..=6 {
                let dt_ang = tail_step as f32 * 0.055 * stretch;
                let tail_a = m.angle_rad - dt_ang;
                let tail_r = r + tail_step as f32 * 4.5 * bh_scale;
                let tail_3d = Vec3::new(
                    tail_a.cos() * tail_r,
                    tail_a.sin() * tail_r,
                    z_spiral * 1.05,
                );
                let (tail_2d, _) = project_3d_disk(bh_center, tail_3d, pitch, roll);
                let tail_alpha = (1.0 - tail_step as f32 / 7.0) * 0.78;
                gizmos.line_2d(prev_pt, tail_2d, m.color.with_a(tail_alpha));
                prev_pt = tail_2d;
            }

            gizmos.circle_2d(head_2d, m.size * bh_scale, m.color);

            // Bright X-ray / Hawking tidal disruption flash right at the Event Horizon boundary!
            if m.radius_norm < 0.19 {
                let flash_r = (0.19 - m.radius_norm) * 210.0 * bh_scale;
                gizmos.circle_2d(
                    head_2d,
                    flash_r.max(3.5),
                    Color::rgba(0.65, 0.96, 1.0, 0.85),
                );
            }
        }

        // F) Pitch-Black Schwarzschild Event Horizon Core & Blazing Photon Ring
        for dark_r in (2..=(horizon_r as i32)).step_by(3) {
            gizmos.circle_2d(
                bh_center,
                dark_r as f32,
                Color::rgba(0.005, 0.008, 0.02, 0.96),
            );
        }
        gizmos.circle_2d(bh_center, horizon_r, Color::rgba(1.0, 0.92, 0.68, 0.95));
        gizmos.circle_2d(
            bh_center,
            horizon_r + 2.5 * bh_scale,
            Color::rgba(0.38, 0.90, 1.0, 0.82),
        );
        gizmos.circle_2d(bh_center, photon_r, Color::rgba(1.0, 0.56, 0.20, 0.65));

        // Also draw a secondary 3D Planetary Orrery on the right side of the Main Menu
        if cur_state == AppState::LaunchSequence {
            let orrery_center = cam_pos + Vec2::new(win_w * 0.33, -win_h * 0.10);
            for (idx, planet) in solar.planets.iter().enumerate() {
                let r = 34.0 + idx as f32 * 24.0;
                gizmos.ellipse_2d(
                    orrery_center,
                    0.25,
                    Vec2::new(r, r * 0.52),
                    Color::rgba(0.24, 0.55, 0.86, 0.28),
                );
                let theta = planet.mean_longitude_rad + idx as f32 * 0.5;
                let px = theta.cos() * r;
                let py = theta.sin() * r * 0.52;
                let rot = 0.25f32;
                let p_pos = orrery_center
                    + Vec2::new(
                        px * rot.cos() - py * rot.sin(),
                        px * rot.sin() + py * rot.cos(),
                    );
                gizmos.circle_2d(p_pos, planet.radius_px * 1.35, planet.color);
                if planet.has_rings {
                    gizmos.ellipse_2d(
                        p_pos,
                        0.35,
                        Vec2::new(planet.radius_px * 2.8, planet.radius_px * 0.9),
                        Color::rgba(0.96, 0.88, 0.62, 0.80),
                    );
                }
            }
        }
    }

    // 3. Compact Corner Mini-Orrery & Orbital Relay Radar (on Surface or during Launch Sequence)
    let show_mini_orrery =
        (cur_state == AppState::LunarSurface && settings.show_corner_radar && !settings.hide_hud)
            || cur_state == AppState::LaunchSequence;

    if show_mini_orrery {
        let radar_center = if cur_state == AppState::LaunchSequence {
            cam_pos + Vec2::new(0.0, 35.0)
        } else {
            cam_pos + Vec2::new(win_w * 0.5 - 98.0, win_h * 0.5 - 148.0) * camera_scale
        };
        let scale = if cur_state == AppState::LaunchSequence {
            1.65
        } else {
            0.78 * camera_scale
        };

        gizmos.circle_2d(
            radar_center,
            92.0 * scale,
            Color::rgba(0.10, 0.65, 0.95, 0.42),
        );
        gizmos.circle_2d(
            radar_center,
            88.0 * scale,
            Color::rgba(0.04, 0.14, 0.26, 0.55),
        );

        // Schematic distances and sizes keep every planet legible.
        for (idx, planet) in solar.planets.iter().enumerate() {
            let radius = (23.0 + idx as f32 * 8.5 + planet.semi_major_au.ln_1p() * 0.05) * scale;
            gizmos.circle_2d(radar_center, radius, Color::rgba(0.32, 0.52, 0.72, 0.48));
            let theta = planet.mean_longitude_rad + planet.inclination_deg.to_radians() * 0.02;
            let pos = radar_center + Vec2::new(theta.cos(), theta.sin()) * radius;
            let size =
                (if idx < 4 { 2.3 } else { 3.0 }) * scale * (1.0 - planet.eccentricity * 0.05);
            for ring in 1..=5 {
                gizmos.circle_2d(pos, size * ring as f32 / 5.0, planet.color);
            }
            if planet.has_rings {
                gizmos.ellipse_2d(
                    pos,
                    0.35,
                    Vec2::new(size * 1.9, size * 0.6),
                    Color::rgb(0.95, 0.84, 0.60),
                );
            }
            if planet.name == "Earth" {
                let moon = pos
                    + Vec2::new(
                        (solar.anim_time_sec * 0.35).cos(),
                        (solar.anim_time_sec * 0.35).sin(),
                    ) * 5.0
                        * scale;
                gizmos.circle_2d(moon, 0.9 * scale, Color::rgb(0.82, 0.86, 0.9));
            }
        }
        for ring in 1..=10 {
            gizmos.circle_2d(
                radar_center,
                ring as f32 * 0.8 * scale,
                Color::rgb(1.0, 0.78, 0.18),
            );
        }
    }
}
