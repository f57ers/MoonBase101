use std::collections::HashSet;

use crate::types::StructureKind;
use crate::world::LunarMap;
use crate::world::SurvivalStats;

#[derive(Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct SupplyJob {
    pub station: (i32, i32),
    pub remaining: f32,
}

#[derive(Clone, Copy)]
pub enum SupplyRecipe {
    Water,
    Meal,
}

impl SupplyRecipe {
    fn duration(self) -> f32 {
        match self {
            Self::Water => 4.0,
            Self::Meal => 12.0,
        }
    }

    pub fn job(self, map: &LunarMap) -> Option<SupplyJob> {
        match self {
            Self::Water => map.water_job,
            Self::Meal => map.meal_job,
        }
    }

    fn job_slot(self, map: &mut LunarMap) -> &mut Option<SupplyJob> {
        match self {
            Self::Water => &mut map.water_job,
            Self::Meal => &mut map.meal_job,
        }
    }

    pub fn can_run(self, map: &LunarMap, station: (i32, i32)) -> bool {
        if map
            .cells
            .get(&station)
            .is_some_and(|c| c.structure == Some(StructureKind::Airlock))
        {
            return false;
        }
        let required = match self {
            Self::Water => StructureKind::OxygenElectrolyzer,
            Self::Meal => StructureKind::HydroponicsDome,
        };
        let near_workstation = map.cells.iter().any(|(&(x, y), cell)| {
            cell.structure == Some(required)
                && cell.structure_hp > 10.0
                && (x - station.0).pow(2) + (y - station.1).pow(2) <= 2
        });
        if !near_workstation {
            return false;
        }

        map.networks.get(&station).is_some_and(|base| {
            base.powered
                && match self {
                    Self::Water => true,
                    Self::Meal => base.greenhouse,
                }
        })
    }

    pub fn start(
        self,
        map: &mut LunarMap,
        stats: &mut SurvivalStats,
        station: (i32, i32),
    ) -> Result<&'static str, &'static str> {
        if self.job(map).is_some() {
            return Err(crate::locale::text("Ta porcja jest już w przygotowaniu."));
        }
        if !self.can_run(map, station) {
            return Err(match self {
                Self::Water => crate::locale::text(
                    "Podejdź do elektrolizera w zasilonej bazie. Śluza nie produkuje wody.",
                ),
                Self::Meal => {
                    crate::locale::text("Wejdź do szklarni lub podejdź do niej od środka bazy.")
                }
            });
        }
        let (stock, message) = match self {
            Self::Water => (
                &mut stats.water_ice,
                crate::locale::text("Filtr przygotowuje wodę."),
            ),
            Self::Meal => (
                &mut stats.drinking_water,
                crate::locale::text("Szklarnia przygotowuje posiłek."),
            ),
        };
        if *stock == 0 {
            return Err(match self {
                Self::Water => crate::locale::text("Brak lodu. Wykop go i zbierz odłamki."),
                Self::Meal => crate::locale::text("Brak wody. Najpierw przygotuj wodę z lodu."),
            });
        }
        *stock -= 1;
        *self.job_slot(map) = Some(SupplyJob {
            station,
            remaining: self.duration(),
        });
        Ok(message)
    }

    pub fn advance(self, map: &mut LunarMap, stats: &mut SurvivalStats, dt: f32) {
        let Some(job) = self.job(map) else {
            return;
        };
        if !map
            .cells
            .get(&job.station)
            .is_some_and(|cell| cell.structure.is_some() && cell.structure_hp > 10.0)
        {
            *self.job_slot(map) = None;
            match self {
                Self::Water => stats.water_ice += 1,
                Self::Meal => stats.drinking_water += 1,
            }
            stats.status_log =
                crate::locale::text("Przerwano przygotowanie. Składnik wrócił do plecaka.").into();
            return;
        }
        if !self.can_run(map, job.station) {
            return;
        }
        let remaining = job.remaining - dt;
        if remaining > 0.0 {
            *self.job_slot(map) = Some(SupplyJob { remaining, ..job });
        } else {
            *self.job_slot(map) = None;
            stats.status_log = match self {
                Self::Water => {
                    stats.drinking_water += 2;
                    if let Some(campaign) = &mut stats.campaign {
                        campaign.made_water = true;
                    }
                    crate::locale::text("Gotowe: 2 porcje wody w plecaku.")
                }
                Self::Meal => {
                    stats.fresh_meals += 1;
                    if let Some(campaign) = &mut stats.campaign {
                        campaign.made_meal = true;
                    }
                    crate::locale::text("Gotowe: świeży posiłek w plecaku.")
                }
            }
            .into();
        }
    }
}

#[derive(Clone, Copy, Default)]
pub struct BaseStatus {
    pub net_watts: f32,
    pub powered: bool,
    pub oxygen: bool,
    pub lab: bool,
    pub greenhouse: bool,
    pub relay: bool,
    pub charge_percent: f32,
}

/// Adjacent, intact modules share power. Each network keeps charge in its own modules.
fn operate_local_base(
    map: &mut LunarMap,
    start: (i32, i32),
    sun: f32,
    dt: f32,
) -> (BaseStatus, Vec<(i32, i32)>) {
    use StructureKind::*;
    let mut pending = vec![start];
    let mut visited = HashSet::new();
    let mut modules = Vec::new();
    let mut status = BaseStatus::default();
    while let Some(pos @ (x, y)) = pending.pop() {
        if !visited.insert(pos) {
            continue;
        }
        let Some(cell) = map.cells.get(&pos).filter(|c| c.structure_hp > 10.0) else {
            continue;
        };
        let Some(kind) = cell.structure.filter(|kind| *kind != ShieldWall) else {
            continue;
        };
        status.net_watts += kind.power_delta_w(sun);
        status.oxygen |= kind == OxygenElectrolyzer;
        status.lab |= kind == ScienceLab;
        status.greenhouse |= kind == HydroponicsDome;
        status.relay |= kind == CommsDish;
        modules.push((pos, kind));
        pending.extend([(x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)]);
    }
    let capacity: f32 = modules.iter().map(|(_, kind)| capacity_wh(*kind)).sum();
    let stored: f32 = modules
        .iter()
        .map(|(pos, _)| map.cells[pos].charge_wh)
        .sum();
    let updated = (stored + status.net_watts * dt / 60.0).clamp(0.0, capacity);
    status.powered = !modules.is_empty() && (status.net_watts >= 0.0 || updated > 0.001);
    if capacity > 0.0 {
        status.charge_percent = updated / capacity * 100.0;
        for (pos, kind) in &modules {
            if let Some(cell) = map.cells.get_mut(pos) {
                cell.charge_wh = updated / capacity * capacity_wh(*kind);
            }
        }
    }
    (status, modules.into_iter().map(|(pos, _)| pos).collect())
}

pub fn update_networks(map: &mut LunarMap, sun: f32, dt: f32) {
    map.network_elapsed += dt;
    if map.network_elapsed < 0.5 && !map.networks.is_empty() {
        return;
    }
    let elapsed = std::mem::take(&mut map.network_elapsed);
    let candidates: Vec<_> = map
        .cells
        .iter()
        .filter_map(|(&pos, cell)| {
            (cell.structure_hp > 10.0
                && cell
                    .structure
                    .is_some_and(|kind| kind != StructureKind::ShieldWall))
            .then_some(pos)
        })
        .collect();
    map.networks.clear();
    for pos in candidates {
        if map.networks.contains_key(&pos) {
            continue;
        }
        let (status, members) = operate_local_base(map, pos, sun, elapsed);
        map.networks
            .extend(members.into_iter().map(|pos| (pos, status)));
    }
}

pub fn capacity_wh(kind: StructureKind) -> f32 {
    match kind {
        StructureKind::StorageBay => 180.0,
        StructureKind::HabitatModule => 45.0,
        _ => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{MissionConfig, MissionScenario};

    #[test]
    fn supply_jobs_reserve_ingredients_and_wait_for_power() {
        let mut map = LunarMap::default();
        let config = MissionConfig::default();
        let mut stats = SurvivalStats::from_config(&config);
        map.generate_sector(0, 0, config.scenario);
        let station = (3, 0);
        let tile = map.cells.get_mut(&station).unwrap();
        tile.structure = Some(StructureKind::HabitatModule);
        tile.structure_hp = 100.0;
        for (pos, kind) in [
            ((3, 1), StructureKind::OxygenElectrolyzer),
            ((4, 0), StructureKind::HydroponicsDome),
        ] {
            let cell = map.cells.get_mut(&pos).unwrap();
            cell.structure = Some(kind);
            cell.structure_hp = 100.0;
        }
        let base = BaseStatus {
            powered: true,
            greenhouse: false,
            ..Default::default()
        };
        map.networks.insert(station, base);
        stats.water_ice = 1;
        stats.drinking_water = 0;
        assert!(SupplyRecipe::Water
            .start(&mut map, &mut stats, station)
            .is_ok());
        assert_eq!(stats.water_ice, 0);
        assert!(SupplyRecipe::Water
            .start(&mut map, &mut stats, station)
            .is_err());
        map.networks.get_mut(&station).unwrap().powered = false;
        SupplyRecipe::Water.advance(&mut map, &mut stats, 4.0);
        assert_eq!(stats.drinking_water, 0);
        assert_eq!(map.water_job.unwrap().remaining, 4.0);
        map.networks.insert(station, base);
        SupplyRecipe::Water.advance(&mut map, &mut stats, 4.0);
        SupplyRecipe::Water.advance(&mut map, &mut stats, 4.0);
        assert_eq!(stats.drinking_water, 2);

        assert!(SupplyRecipe::Meal
            .start(&mut map, &mut stats, station)
            .is_err());
        assert_eq!(stats.drinking_water, 2);
        map.networks.get_mut(&station).unwrap().greenhouse = true;
        assert!(SupplyRecipe::Meal
            .start(&mut map, &mut stats, station)
            .is_ok());
        SupplyRecipe::Meal.advance(&mut map, &mut stats, 12.0);
        assert_eq!((stats.drinking_water, stats.fresh_meals), (1, 1));
    }

    #[test]
    fn oxygen_requires_a_connected_power_source_or_charged_battery() {
        let mut map = LunarMap::default();
        map.generate_sector(0, 0, MissionScenario::PolsaKopernikBase);
        for (pos, kind) in [
            ((2, 0), StructureKind::HabitatModule),
            ((3, 0), StructureKind::OxygenElectrolyzer),
            ((5, 0), StructureKind::RtgGenerator),
        ] {
            let cell = map.cells.get_mut(&pos).unwrap();
            cell.structure = Some(kind);
            cell.structure_hp = 100.0;
        }
        let (status, _) = operate_local_base(&mut map, (2, 0), 1.0, 1.0);
        assert!(status.oxygen && !status.powered);
        let bridge = map.cells.get_mut(&(4, 0)).unwrap();
        bridge.structure = Some(StructureKind::Airlock);
        bridge.structure_hp = 100.0;
        assert!(operate_local_base(&mut map, (2, 0), 1.0, 1.0).0.powered);
        map.cells.get_mut(&(4, 0)).unwrap().structure_hp = 0.0;
        assert!(operate_local_base(&mut map, (2, 0), 1.0, 0.1).0.powered);
        assert!(!operate_local_base(&mut map, (2, 0), 1.0, 100.0).0.powered);
    }
}
