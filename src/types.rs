use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
pub const GAME_CREATORS: [&str; 6] = [
    "_wojtasz_",
    "f57ers",
    "hajaxu",
    "melodyxmemorie",
    "rafi2332",
    "wrfghui",
];

pub const GAME_CREATORS_LINE: &str =
    "_wojtasz_  •  f57ers  •  hajaxu  •  melodyxmemorie  •  rafi2332  •  wrfghui";

#[derive(States, Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
pub enum AppState {
    #[default]
    MainMenu,
    NasaApod,
    Settings,
    Controls,
    MissionDesignLab,
    LaunchSequence,
    LunarSurface,
    PlanetAtlas,
    InterplanetaryFlight,
    Paused,
    MissionReport,
    CampaignBriefing,
    Discovery,
    GameOver,
}

/// Generic helper used by every "< value >" selector in the menus.
pub fn cycle_in<T: Copy + PartialEq>(all: &[T], cur: T, dir: i32) -> T {
    let n = all.len() as i32;
    let idx = all.iter().position(|x| *x == cur).unwrap_or(0) as i32;
    all[(idx + dir).rem_euclid(n) as usize]
}

/// Every option that can be changed with "<" / ">" buttons in the Setup Lab or Customizer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MdlOption {
    Scenario,
    Difficulty,
    TeamSeat,
    Launch,
    Orbit,
    Power,
    Comms,
    Rover,
    Lifetime,
    Callsign,
    Suit,
    Visor,
    Accessory,
    TrailColor,
    Patch,
    Backpack,
    Trait,
}

pub const CALLSIGN_PRESETS: [&str; 8] = [
    "KDR. TWARDOWSKI",
    "GEN. HERMASZEWSKI",
    "DR S. UZNAŃSKI",
    "INŻ. M. BEKKER",
    "KDR. S. LEM",
    "PROF. KOPERNIK",
    "CDR. VANCE",
    "DR SKŁODOWSKA",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MissionScenario {
    PolsaKopernikBase,
    ShackletonIceProspecting,
    FarsideRadioObservatory,
    MareTranquillitatisHe3,
    OpenSandbox,
}

impl MissionScenario {
    pub const ALL: [MissionScenario; 5] = [
        MissionScenario::PolsaKopernikBase,
        MissionScenario::ShackletonIceProspecting,
        MissionScenario::FarsideRadioObservatory,
        MissionScenario::MareTranquillitatisHe3,
        MissionScenario::OpenSandbox,
    ];

    #[allow(dead_code)]
    pub fn title(&self) -> &'static str {
        match self {
            Self::PolsaKopernikBase => {
                crate::locale::text("TRYB SURVIVAL — Standardowa Przygoda Księżycowa")
            }
            Self::ShackletonIceProspecting => {
                crate::locale::text("TRYB TRUDNY — Mroźne Kratery i Burze Słoneczne")
            }
            Self::FarsideRadioObservatory => {
                crate::locale::text("TRYB HARDCORE — Jedno Życie po Ciemnej Stronie!")
            }
            Self::MareTranquillitatisHe3 => {
                crate::locale::text("TRYB EKSPLORACJI — Bogate Złoża i Otwarty Świat")
            }
            Self::OpenSandbox => {
                crate::locale::text("TRYB SANDBOX — Kreatywne Budowanie Bez Limitów")
            }
        }
    }

    pub fn short_name(&self) -> &'static str {
        match self {
            Self::PolsaKopernikBase => crate::locale::text("TRYB SURVIVAL (Standard)"),
            Self::ShackletonIceProspecting => crate::locale::text("TRYB TRUDNY (Hard)"),
            Self::FarsideRadioObservatory => crate::locale::text("TRYB HARDCORE (1 Życie!)"),
            Self::MareTranquillitatisHe3 => crate::locale::text("TRYB EKSPLORACJI (Złoża)"),
            Self::OpenSandbox => crate::locale::text("TRYB SANDBOX (Kreatywny)"),
        }
    }

    pub fn objective_summary(&self) -> &'static str {
        match self {
            Self::PolsaKopernikBase => {
                crate::locale::text("Klasyczny Survival! Kop surowce, buduj dom ze śluzą, osiedlaj się w odkrytych strukturach i wykonuj kolejne kroki Mapy Postępu.")
            }
            Self::ShackletonIceProspecting => {
                crate::locale::text("Tryb Trudny! Niskie temperatury, szybszy spadek tlenu i częstsze burze słoneczne. Zbuduj ciepłą bazę i łazik!")
            }
            Self::FarsideRadioObservatory => {
                crate::locale::text("Tryb HARDCORE (Jedno Życie)! Brak klonowania po śmierci i surowe warunki po ciemnej stronie Księżyca.")
            }
            Self::MareTranquillitatisHe3 => {
                crate::locale::text("Swobodna eksploracja otwartego świata z dużą liczbą struktur, kryształów Helu-3 oraz wraków do zasiedlenia.")
            }
            Self::OpenSandbox => {
                crate::locale::text("Tryb Kreatywny (Sandbox)! Nieskończone surowce, brak śmierci, super-szybkie kopanie i pełna swoboda budowy bazy!")
            }
        }
    }

    pub fn target_science_gb(&self) -> f32 {
        match self {
            Self::PolsaKopernikBase => 220.0,
            Self::ShackletonIceProspecting => 250.0,
            Self::FarsideRadioObservatory => 320.0,
            Self::MareTranquillitatisHe3 => 280.0,
            Self::OpenSandbox => 150.0,
        }
    }

    pub fn base_budget_musd(&self, difficulty: DifficultyLevel) -> f32 {
        let base = match self {
            Self::PolsaKopernikBase => 450.0,
            Self::ShackletonIceProspecting => 430.0,
            Self::FarsideRadioObservatory => 470.0,
            Self::MareTranquillitatisHe3 => 450.0,
            Self::OpenSandbox => 9999.0,
        };
        base * difficulty.budget_multiplier()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DifficultyLevel {
    Cadet,
    SystemsEngineer,
    FlightDirector,
}

impl DifficultyLevel {
    pub const ALL: [DifficultyLevel; 3] = [
        DifficultyLevel::Cadet,
        DifficultyLevel::SystemsEngineer,
        DifficultyLevel::FlightDirector,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            Self::Cadet => crate::locale::text("Łatwy"),
            Self::SystemsEngineer => crate::locale::text("Normalny"),
            Self::FlightDirector => crate::locale::text("Trudny"),
        }
    }

    pub fn budget_multiplier(&self) -> f32 {
        match self {
            Self::Cadet => 1.35,
            Self::SystemsEngineer => 1.00,
            Self::FlightDirector => 0.85,
        }
    }

    pub fn survival_drain_multiplier(&self) -> f32 {
        match self {
            Self::Cadet => 0.60,
            Self::SystemsEngineer => 0.95,
            Self::FlightDirector => 1.35,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TeamRole {
    MissionDirector,
    SpacecraftEngineer,
    ChiefScientist,
    CommsOpsOfficer,
}

impl TeamRole {
    pub const ALL: [TeamRole; 4] = [
        TeamRole::MissionDirector,
        TeamRole::SpacecraftEngineer,
        TeamRole::ChiefScientist,
        TeamRole::CommsOpsOfficer,
    ];

    #[allow(dead_code)]
    pub fn title(&self) -> &'static str {
        match self {
            Self::MissionDirector => crate::locale::text("Dowódca Ekspedycji"),
            Self::SpacecraftEngineer => crate::locale::text("Inżynier Bazy"),
            Self::ChiefScientist => crate::locale::text("Odkrywca i Geolog"),
            Self::CommsOpsOfficer => crate::locale::text("Nawigator i Kartograf"),
        }
    }

    pub fn responsibility(&self) -> &'static str {
        match self {
            Self::MissionDirector => {
                crate::locale::text("Dowodzenie bazą, rakieta i planowanie rozwoju kolonii.")
            }
            Self::SpacecraftEngineer => {
                crate::locale::text("Zasilanie, budowa domów, śluz i łazików księżycowych.")
            }
            Self::ChiefScientist => {
                crate::locale::text("Wydobywanie rzadkich minerałów i odkrywanie struktur.")
            }
            Self::CommsOpsOfficer => {
                crate::locale::text("Mapa świata, punkty nawigacyjne (Waypointy) i łączność.")
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LaunchVehicle {
    CommercialLight,
    VulcanCentaur,
    FalconHeavy,
    SlsBlock1B,
}

impl LaunchVehicle {
    pub const ALL: [LaunchVehicle; 4] = [
        LaunchVehicle::CommercialLight,
        LaunchVehicle::VulcanCentaur,
        LaunchVehicle::FalconHeavy,
        LaunchVehicle::SlsBlock1B,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            Self::CommercialLight => crate::locale::text("Lekka Rakieta Nova (1950 kg)"),
            Self::VulcanCentaur => "Vulcan Centaur (2850 kg)",
            Self::FalconHeavy => "Falcon Heavy (3600 kg)",
            Self::SlsBlock1B => crate::locale::text("Ciężka Rakieta SLS (5400 kg)"),
        }
    }

    pub fn max_tli_mass_kg(&self) -> f32 {
        match self {
            Self::CommercialLight => 1950.0,
            Self::VulcanCentaur => 2850.0,
            Self::FalconHeavy => 3600.0,
            Self::SlsBlock1B => 5400.0,
        }
    }

    pub fn cost_musd(&self) -> f32 {
        match self {
            Self::CommercialLight => 105.0,
            Self::VulcanCentaur => 155.0,
            Self::FalconHeavy => 195.0,
            Self::SlsBlock1B => 305.0,
        }
    }

    pub fn supports_nrho_heavy(&self) -> bool {
        !matches!(self, Self::CommercialLight)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LunarOrbit {
    LowLunarOrbit,
    NearRectilinearHalo,
    PolarSunSync,
    DirectSurfaceTli,
}

impl LunarOrbit {
    pub const ALL: [LunarOrbit; 4] = [
        LunarOrbit::LowLunarOrbit,
        LunarOrbit::NearRectilinearHalo,
        LunarOrbit::PolarSunSync,
        LunarOrbit::DirectSurfaceTli,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            Self::LowLunarOrbit => crate::locale::text("Niska Orbita Księżyca (100 km)"),
            Self::NearRectilinearHalo => crate::locale::text("Orbita Stacji Gateway (NRHO)"),
            Self::PolarSunSync => crate::locale::text("Orbita Polarna nad Biegunem"),
            Self::DirectSurfaceTli => crate::locale::text("Lot Bezpośredni (Więcej ładunku)"),
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::LowLunarOrbit => {
                crate::locale::text("+25% punktów odkryć, dobra widoczność powierzchni.")
            }
            Self::NearRectilinearHalo => {
                crate::locale::text("Stała łączność z Ziemią (+40% przesyłu danych).")
            }
            Self::PolarSunSync => crate::locale::text("Świetna nad kraterami lodu (+20% odkryć)."),
            Self::DirectSurfaceTli => {
                crate::locale::text("+15% udźwigu rakiety, ale wymaga satelity po ciemnej stronie.")
            }
        }
    }

    pub fn payload_mass_multiplier(&self) -> f32 {
        match self {
            Self::LowLunarOrbit => 1.00,
            Self::NearRectilinearHalo => 0.92,
            Self::PolarSunSync => 0.95,
            Self::DirectSurfaceTli => 1.15,
        }
    }

    pub fn comms_multiplier(&self) -> f32 {
        match self {
            Self::LowLunarOrbit => 0.90,
            Self::NearRectilinearHalo => 1.40,
            Self::PolarSunSync => 1.15,
            Self::DirectSurfaceTli => 0.75,
        }
    }

    pub fn science_multiplier(&self) -> f32 {
        match self {
            Self::LowLunarOrbit => 1.25,
            Self::NearRectilinearHalo => 1.10,
            Self::PolarSunSync => 1.20,
            Self::DirectSurfaceTli => 0.95,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SciInstrument {
    NeutronSpectrometer,
    SubsurfaceRadar,
    SeismometerArray,
    CryoIceDrill,
    RadioAstronomyArray,
    LaserRetroreflector,
}

impl SciInstrument {
    pub const ALL: [SciInstrument; 6] = [
        SciInstrument::NeutronSpectrometer,
        SciInstrument::SubsurfaceRadar,
        SciInstrument::SeismometerArray,
        SciInstrument::CryoIceDrill,
        SciInstrument::RadioAstronomyArray,
        SciInstrument::LaserRetroreflector,
    ];

    pub fn short_name(&self) -> &'static str {
        match self {
            Self::NeutronSpectrometer => crate::locale::text("Wykrywacz Wody i Lodu"),
            Self::SubsurfaceRadar => crate::locale::text("Radar Podziemnych Żył"),
            Self::SeismometerArray => crate::locale::text("Czujnik Wstrząsów Gruntu"),
            Self::CryoIceDrill => crate::locale::text("Wiertło Lodowe"),
            Self::RadioAstronomyArray => crate::locale::text("Teleskop Kosmiczny"),
            Self::LaserRetroreflector => crate::locale::text("Miernik Laserowy"),
        }
    }

    pub fn mass_kg(&self) -> f32 {
        match self {
            Self::NeutronSpectrometer => 135.0,
            Self::SubsurfaceRadar => 250.0,
            Self::SeismometerArray => 180.0,
            Self::CryoIceDrill => 360.0,
            Self::RadioAstronomyArray => 310.0,
            Self::LaserRetroreflector => 70.0,
        }
    }

    pub fn power_w(&self) -> f32 {
        match self {
            Self::NeutronSpectrometer => 80.0,
            Self::SubsurfaceRadar => 155.0,
            Self::SeismometerArray => 65.0,
            Self::CryoIceDrill => 275.0,
            Self::RadioAstronomyArray => 200.0,
            Self::LaserRetroreflector => 20.0,
        }
    }

    pub fn cost_musd(&self) -> f32 {
        match self {
            Self::NeutronSpectrometer => 22.0,
            Self::SubsurfaceRadar => 36.0,
            Self::SeismometerArray => 28.0,
            Self::CryoIceDrill => 50.0,
            Self::RadioAstronomyArray => 45.0,
            Self::LaserRetroreflector => 12.0,
        }
    }

    pub fn data_rate_mbps(&self) -> f32 {
        match self {
            Self::NeutronSpectrometer => 2.2,
            Self::SubsurfaceRadar => 4.8,
            Self::SeismometerArray => 1.6,
            Self::CryoIceDrill => 3.2,
            Self::RadioAstronomyArray => 6.4,
            Self::LaserRetroreflector => 0.5,
        }
    }

    pub fn base_science_value(&self) -> f32 {
        match self {
            Self::NeutronSpectrometer => 185.0,
            Self::SubsurfaceRadar => 310.0,
            Self::SeismometerArray => 245.0,
            Self::CryoIceDrill => 410.0,
            Self::RadioAstronomyArray => 390.0,
            Self::LaserRetroreflector => 115.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PowerSystemOption {
    BasicSolarArray,
    TripleJunctionSolarPlusBattery,
    HybridRtgNuclearSolar,
}

impl PowerSystemOption {
    pub const ALL: [PowerSystemOption; 3] = [
        PowerSystemOption::BasicSolarArray,
        PowerSystemOption::TripleJunctionSolarPlusBattery,
        PowerSystemOption::HybridRtgNuclearSolar,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            Self::BasicSolarArray => crate::locale::text("Zwykłe Panele Słoneczne (450W)"),
            Self::TripleJunctionSolarPlusBattery => {
                crate::locale::text("Mocne Panele + Bateria (780W)")
            }
            Self::HybridRtgNuclearSolar => crate::locale::text("Reaktor RTG + Panele (960W)"),
        }
    }

    pub fn generation_w(&self) -> f32 {
        match self {
            Self::BasicSolarArray => 450.0,
            Self::TripleJunctionSolarPlusBattery => 780.0,
            Self::HybridRtgNuclearSolar => 960.0,
        }
    }

    pub fn mass_kg(&self) -> f32 {
        match self {
            Self::BasicSolarArray => 180.0,
            Self::TripleJunctionSolarPlusBattery => 340.0,
            Self::HybridRtgNuclearSolar => 490.0,
        }
    }

    pub fn cost_musd(&self) -> f32 {
        match self {
            Self::BasicSolarArray => 18.0,
            Self::TripleJunctionSolarPlusBattery => 38.0,
            Self::HybridRtgNuclearSolar => 68.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommsSystemOption {
    SBandOmni,
    XBandHighGain,
    KaBandOpticalLaser,
}

impl CommsSystemOption {
    pub const ALL: [CommsSystemOption; 3] = [
        CommsSystemOption::SBandOmni,
        CommsSystemOption::XBandHighGain,
        CommsSystemOption::KaBandOpticalLaser,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            Self::SBandOmni => crate::locale::text("Antena Podstawowa (5 Mbps)"),
            Self::XBandHighGain => crate::locale::text("Talerz Dalekiego Zasięgu (12.5 Mbps)"),
            Self::KaBandOpticalLaser => crate::locale::text("Szybki Laser Kosmiczny (24 Mbps)"),
        }
    }

    pub fn capacity_mbps(&self) -> f32 {
        match self {
            Self::SBandOmni => 5.0,
            Self::XBandHighGain => 12.5,
            Self::KaBandOpticalLaser => 24.0,
        }
    }

    pub fn mass_kg(&self) -> f32 {
        match self {
            Self::SBandOmni => 65.0,
            Self::XBandHighGain => 140.0,
            Self::KaBandOpticalLaser => 210.0,
        }
    }

    pub fn power_w(&self) -> f32 {
        match self {
            Self::SBandOmni => 45.0,
            Self::XBandHighGain => 110.0,
            Self::KaBandOpticalLaser => 165.0,
        }
    }

    pub fn cost_musd(&self) -> f32 {
        match self {
            Self::SBandOmni => 12.0,
            Self::XBandHighGain => 28.0,
            Self::KaBandOpticalLaser => 48.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrelandedRoverChoice {
    NoneBuildOnSurface,
    LightUtilityRover,
    HeavyPressurizedCruiser,
}

impl PrelandedRoverChoice {
    pub const ALL: [PrelandedRoverChoice; 3] = [
        PrelandedRoverChoice::NoneBuildOnSurface,
        PrelandedRoverChoice::LightUtilityRover,
        PrelandedRoverChoice::HeavyPressurizedCruiser,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            Self::NoneBuildOnSurface => crate::locale::text("Brak (zbuduj sam na miejscu)"),
            Self::LightUtilityRover => crate::locale::text("Szybki Łazik Bekkera LUR-1"),
            Self::HeavyPressurizedCruiser => {
                crate::locale::text("Ciężki Łazik z Kabiną Tlenową LSC-2")
            }
        }
    }

    pub fn mass_kg(&self) -> f32 {
        match self {
            Self::NoneBuildOnSurface => 0.0,
            Self::LightUtilityRover => 320.0,
            Self::HeavyPressurizedCruiser => 820.0,
        }
    }

    pub fn cost_musd(&self) -> f32 {
        match self {
            Self::NoneBuildOnSurface => 0.0,
            Self::LightUtilityRover => 26.0,
            Self::HeavyPressurizedCruiser => 64.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SuitColor {
    ApolloWhite,
    HusariaCrimson,
    ArtemisOrange,
    KopernikCobalt,
    StealthGrey,
    CyberViolet,
    EmeraldBio,
    GoldThermal,
}

impl SuitColor {
    pub const ALL: [SuitColor; 8] = [
        SuitColor::ApolloWhite,
        SuitColor::HusariaCrimson,
        SuitColor::ArtemisOrange,
        SuitColor::KopernikCobalt,
        SuitColor::StealthGrey,
        SuitColor::CyberViolet,
        SuitColor::EmeraldBio,
        SuitColor::GoldThermal,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            Self::ApolloWhite => crate::locale::text("Biel Apollo EVA"),
            Self::HusariaCrimson => crate::locale::text("Karmazyn Husarii (PL)"),
            Self::ArtemisOrange => crate::locale::text("Pomarańcz Ratunkowy"),
            Self::KopernikCobalt => crate::locale::text("Kobalt Kopernika"),
            Self::StealthGrey => crate::locale::text("Ciemny Bazalt"),
            Self::CyberViolet => crate::locale::text("Neonowy Fiolet"),
            Self::EmeraldBio => crate::locale::text("Szmaragd Botanika"),
            Self::GoldThermal => crate::locale::text("Złoty Solaris"),
        }
    }

    pub fn to_color(&self) -> Color {
        match self {
            Self::ApolloWhite => Color::rgb(0.96, 0.97, 1.0),
            Self::HusariaCrimson => Color::rgb(0.96, 0.26, 0.34),
            Self::ArtemisOrange => Color::rgb(1.0, 0.58, 0.25),
            Self::KopernikCobalt => Color::rgb(0.32, 0.68, 1.0),
            Self::StealthGrey => Color::rgb(0.52, 0.56, 0.64),
            Self::CyberViolet => Color::rgb(0.78, 0.42, 1.0),
            Self::EmeraldBio => Color::rgb(0.32, 0.94, 0.62),
            Self::GoldThermal => Color::rgb(1.0, 0.84, 0.36),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VisorTint {
    GoldReflective,
    CobaltBlue,
    EmeraldHud,
    RubyLaser,
    SilverMirror,
    AmethystVoid,
}

impl VisorTint {
    pub const ALL: [VisorTint; 6] = [
        VisorTint::GoldReflective,
        VisorTint::CobaltBlue,
        VisorTint::EmeraldHud,
        VisorTint::RubyLaser,
        VisorTint::SilverMirror,
        VisorTint::AmethystVoid,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            Self::GoldReflective => crate::locale::text("24K Złoty Słoneczny"),
            Self::CobaltBlue => crate::locale::text("Błękitny UV"),
            Self::EmeraldHud => crate::locale::text("Szmaragdowy HUD"),
            Self::RubyLaser => crate::locale::text("Rubinowy Laser"),
            Self::SilverMirror => crate::locale::text("Chromowane Lustro"),
            Self::AmethystVoid => crate::locale::text("Fioletowa Plazma"),
        }
    }

    pub fn to_color(&self) -> Color {
        match self {
            Self::GoldReflective => Color::rgb(0.98, 0.76, 0.18),
            Self::CobaltBlue => Color::rgb(0.22, 0.55, 0.98),
            Self::EmeraldHud => Color::rgb(0.18, 0.92, 0.52),
            Self::RubyLaser => Color::rgb(1.0, 0.25, 0.38),
            Self::SilverMirror => Color::rgb(0.82, 0.88, 0.96),
            Self::AmethystVoid => Color::rgb(0.75, 0.35, 1.0),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum HelmetAccessory {
    #[default]
    HussarWings,
    StandardAntenna,
    SolarHalo,
    CatEars,
    LaserCrest,
}

impl HelmetAccessory {
    pub const ALL: [HelmetAccessory; 5] = [
        HelmetAccessory::HussarWings,
        HelmetAccessory::StandardAntenna,
        HelmetAccessory::SolarHalo,
        HelmetAccessory::CatEars,
        HelmetAccessory::LaserCrest,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            Self::HussarWings => crate::locale::text("Skrzydła Husarskie (PL)"),
            Self::StandardAntenna => crate::locale::text("Antena Telemetryczna"),
            Self::SolarHalo => crate::locale::text("Złota Korona Solarna"),
            Self::CatEars => crate::locale::text("Kosmiczne Koci-Uszy"),
            Self::LaserCrest => crate::locale::text("Grzebień Komandorski"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum JetpackTrailColor {
    #[default]
    PolishWhiteRed,
    CyanColdGas,
    SolarGold,
    IonViolet,
    EmeraldBio,
}

impl JetpackTrailColor {
    pub const ALL: [JetpackTrailColor; 5] = [
        JetpackTrailColor::PolishWhiteRed,
        JetpackTrailColor::CyanColdGas,
        JetpackTrailColor::SolarGold,
        JetpackTrailColor::IonViolet,
        JetpackTrailColor::EmeraldBio,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            Self::PolishWhiteRed => crate::locale::text("Biało-Czerwona Plazma (PL)"),
            Self::CyanColdGas => crate::locale::text("Błękitny Gaz RCS"),
            Self::SolarGold => crate::locale::text("Złoty Płomień Słoneczny"),
            Self::IonViolet => crate::locale::text("Fioletowy Napęd Jonowy"),
            Self::EmeraldBio => crate::locale::text("Zielony Bio-Napęd"),
        }
    }

    pub fn to_color(&self) -> Color {
        match self {
            Self::PolishWhiteRed => Color::rgba(1.0, 0.32, 0.42, 0.90),
            Self::CyanColdGas => Color::rgba(0.35, 0.92, 1.0, 0.88),
            Self::SolarGold => Color::rgba(1.0, 0.82, 0.22, 0.90),
            Self::IonViolet => Color::rgba(0.80, 0.38, 1.0, 0.90),
            Self::EmeraldBio => Color::rgba(0.28, 0.98, 0.55, 0.88),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MissionPatch {
    PolsaWingedEagle,
    ArtemisAnchor,
    CsaCanadarmStar,
    ViperPolarScout,
    DeepSpacePioneer,
}

impl MissionPatch {
    pub const ALL: [MissionPatch; 5] = [
        MissionPatch::PolsaWingedEagle,
        MissionPatch::ArtemisAnchor,
        MissionPatch::CsaCanadarmStar,
        MissionPatch::ViperPolarScout,
        MissionPatch::DeepSpacePioneer,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            Self::PolsaWingedEagle => crate::locale::text("POLSA Orzeł Księżycowy"),
            Self::ArtemisAnchor => crate::locale::text("Artemis Horyzont"),
            Self::CsaCanadarmStar => crate::locale::text("Gwiazda Północy"),
            Self::ViperPolarScout => crate::locale::text("VIPER Łowca Lodu"),
            Self::DeepSpacePioneer => crate::locale::text("Pionier Czarnej Dziury"),
        }
    }

    pub fn color(&self) -> Color {
        match self {
            Self::PolsaWingedEagle => Color::rgb(0.95, 0.22, 0.30),
            Self::ArtemisAnchor => Color::rgb(0.25, 0.45, 0.95),
            Self::CsaCanadarmStar => Color::rgb(0.95, 0.35, 0.25),
            Self::ViperPolarScout => Color::rgb(0.30, 0.85, 0.95),
            Self::DeepSpacePioneer => Color::rgb(0.95, 0.75, 0.25),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackpackModule {
    CryoO2TwinTank,
    MicroRtgCell,
    HighImpulseRcsJet,
    RadiationLeadShield,
}

impl BackpackModule {
    pub const ALL: [BackpackModule; 4] = [
        BackpackModule::CryoO2TwinTank,
        BackpackModule::MicroRtgCell,
        BackpackModule::HighImpulseRcsJet,
        BackpackModule::RadiationLeadShield,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            Self::CryoO2TwinTank => crate::locale::text("Podwójna Butla O2 (+50% Tlenu)"),
            Self::MicroRtgCell => crate::locale::text("Bateria Atomowa (+50% Energii)"),
            Self::HighImpulseRcsJet => crate::locale::text("Turbo-Jetpack (+45% Wyższy Skok)"),
            Self::RadiationLeadShield => {
                crate::locale::text("Tarcza Ołowiana (-65% Promieniowania)")
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AstronautTrait {
    SystemsArchitect,
    AstroGeologist,
    ClosedLoopBotanist,
    RoverTestPilot,
}

impl AstronautTrait {
    pub const ALL: [AstronautTrait; 4] = [
        AstronautTrait::SystemsArchitect,
        AstronautTrait::AstroGeologist,
        AstronautTrait::ClosedLoopBotanist,
        AstronautTrait::RoverTestPilot,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            Self::SystemsArchitect => crate::locale::text("Budowniczy (-25% kosztu budynków)"),
            Self::AstroGeologist => crate::locale::text("Górnik (2x więcej surowców z kopania)"),
            Self::ClosedLoopBotanist => crate::locale::text("Biolog (-30% zużycia tlenu i głodu)"),
            Self::RoverTestPilot => crate::locale::text("Kierowca Bekkera (+35% szybszy łazik)"),
        }
    }
}

pub const BUILD_LABEL: &str = "ORBIT-013 · 2026.11.15";
fn quests_default() -> bool {
    true
}
fn default_fps_limit() -> Option<u16> {
    Some(60)
}

#[derive(Resource, Debug, Clone, Serialize, Deserialize)]
pub struct GameSettings {
    #[serde(default)]
    pub language: crate::locale::Language,
    #[serde(default)]
    pub bindings: std::collections::BTreeMap<usize, String>,
    #[serde(default)]
    pub astronaut_name: String,
    #[serde(default = "default_fps_limit")]
    pub fps_limit: Option<u16>,
    #[serde(skip)]
    pub hide_hud: bool,
    #[serde(skip)]
    pub windowed_placement: Option<(bevy::window::WindowResolution, bevy::window::WindowPosition)>,
    pub master_volume: f32,
    pub music_volume: f32,
    pub sfx_volume: f32,
    pub show_corner_radar: bool,
    pub show_horizon_planets: bool,
    pub show_touch_controls: bool,
    pub camera_zoom: f32,
    /// When false in Solo mode, the other 3 astronauts stay at their base/outpost stations
    /// (visible on the Map [M] and switchable with [Z]) without crowding around the player.
    pub co_op_ai_crew: bool,
}

impl Default for GameSettings {
    fn default() -> Self {
        Self {
            language: Default::default(),
            bindings: default(),
            astronaut_name: String::new(),
            fps_limit: Some(60),
            hide_hud: false,
            windowed_placement: None,
            master_volume: 0.85,
            music_volume: 0.35,
            sfx_volume: 0.80,
            show_corner_radar: true,
            show_horizon_planets: false,
            show_touch_controls: false,
            camera_zoom: 1.0,
            co_op_ai_crew: true, // Stationed at their posts & visible on the Map [M]!
        }
    }
}

impl GameSettings {
    pub fn binding(&self, index: usize) -> KeyCode {
        self.bindings
            .get(&index)
            .and_then(|name| crate::controls::key_code(name))
            .unwrap_or(crate::controls::ACTIONS[index].0)
    }
    pub fn save(&self) {
        if let Ok(text) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write("settings.json", text);
        }
    }
    pub fn load() -> Self {
        let mut value = std::fs::read_to_string("settings.json")
            .ok()
            .and_then(|text| serde_json::from_str::<Self>(&text).ok())
            .unwrap_or_default();
        if value
            .fps_limit
            .is_some_and(|limit| !(30..=300).contains(&limit))
        {
            value.fps_limit = Some(60);
        }
        value
    }
}

#[derive(Resource, Debug, Clone, Serialize, Deserialize)]
pub struct MissionConfig {
    #[serde(default = "quests_default")]
    pub sandbox_quests: bool,
    pub scenario: MissionScenario,
    pub difficulty: DifficultyLevel,
    pub team_mode_players: u8,
    pub active_role: TeamRole,
    pub launch_vehicle: LaunchVehicle,
    pub orbit: LunarOrbit,
    pub power_system: PowerSystemOption,
    pub comms_system: CommsSystemOption,
    pub prelanded_rover: PrelandedRoverChoice,
    pub selected_instruments: Vec<SciInstrument>,
    pub planned_lifetime_sols: u32,
    pub dilemma_resolved_bonus: f32,
    pub dilemma_index: usize,
    pub callsign: String,
    pub suit_color: SuitColor,
    pub visor_tint: VisorTint,
    pub accessory: HelmetAccessory,
    pub trail_color: JetpackTrailColor,
    pub mission_patch: MissionPatch,
    pub backpack_module: BackpackModule,
    pub astronaut_trait: AstronautTrait,
}

impl Default for MissionConfig {
    fn default() -> Self {
        Self {
            sandbox_quests: true,
            scenario: MissionScenario::PolsaKopernikBase,
            difficulty: DifficultyLevel::SystemsEngineer,
            team_mode_players: 1,
            active_role: TeamRole::MissionDirector,
            launch_vehicle: LaunchVehicle::VulcanCentaur,
            orbit: LunarOrbit::NearRectilinearHalo,
            power_system: PowerSystemOption::TripleJunctionSolarPlusBattery,
            comms_system: CommsSystemOption::XBandHighGain,
            prelanded_rover: PrelandedRoverChoice::LightUtilityRover,
            selected_instruments: vec![
                SciInstrument::NeutronSpectrometer,
                SciInstrument::SubsurfaceRadar,
                SciInstrument::CryoIceDrill,
            ],
            planned_lifetime_sols: 14,
            dilemma_resolved_bonus: 1.0,
            dilemma_index: 0,
            callsign: CALLSIGN_PRESETS[0].to_string(),
            suit_color: SuitColor::ApolloWhite,
            visor_tint: VisorTint::GoldReflective,
            accessory: HelmetAccessory::HussarWings,
            trail_color: JetpackTrailColor::PolishWhiteRed,
            mission_patch: MissionPatch::PolsaWingedEagle,
            backpack_module: BackpackModule::CryoO2TwinTank,
            astronaut_trait: AstronautTrait::AstroGeologist,
        }
    }
}

impl MissionConfig {
    pub fn normalize(&mut self) {
        self.planned_lifetime_sols = self.planned_lifetime_sols.clamp(5, 45);
        self.team_mode_players = self.team_mode_players.clamp(1, 4);
        self.dilemma_index %= 4;
        self.dilemma_resolved_bonus = if self.dilemma_resolved_bonus.is_finite() {
            self.dilemma_resolved_bonus.clamp(1.0, 1.5)
        } else {
            1.0
        };
        self.callsign = self
            .callsign
            .chars()
            .filter(|c| !c.is_control())
            .take(48)
            .collect();
        if self.callsign.trim().is_empty() {
            self.callsign = CALLSIGN_PRESETS[0].to_string();
        }
        let mut instruments = Vec::new();
        for instrument in &self.selected_instruments {
            if !instruments.contains(instrument) {
                instruments.push(*instrument);
            }
        }
        self.selected_instruments = instruments;
    }
    pub fn is_hardcore(&self) -> bool {
        self.scenario == MissionScenario::FarsideRadioObservatory
    }

    pub fn cycle(&mut self, opt: MdlOption, dir: i32) {
        match opt {
            MdlOption::Scenario => {
                self.scenario = cycle_in(&MissionScenario::ALL, self.scenario, dir)
            }
            MdlOption::Difficulty => {
                self.difficulty = cycle_in(&DifficultyLevel::ALL, self.difficulty, dir)
            }
            MdlOption::TeamSeat => {
                self.active_role = cycle_in(&TeamRole::ALL, self.active_role, dir)
            }
            MdlOption::Launch => {
                self.launch_vehicle = cycle_in(&LaunchVehicle::ALL, self.launch_vehicle, dir)
            }
            MdlOption::Orbit => self.orbit = cycle_in(&LunarOrbit::ALL, self.orbit, dir),
            MdlOption::Power => {
                self.power_system = cycle_in(&PowerSystemOption::ALL, self.power_system, dir)
            }
            MdlOption::Comms => {
                self.comms_system = cycle_in(&CommsSystemOption::ALL, self.comms_system, dir)
            }
            MdlOption::Rover => {
                self.prelanded_rover =
                    cycle_in(&PrelandedRoverChoice::ALL, self.prelanded_rover, dir)
            }
            MdlOption::Lifetime => {
                let v = self.planned_lifetime_sols as i32 + dir * 3;
                self.planned_lifetime_sols = v.clamp(5, 45) as u32;
            }
            MdlOption::Callsign => {
                let idx = CALLSIGN_PRESETS
                    .iter()
                    .position(|c| *c == self.callsign)
                    .unwrap_or(0) as i32;
                let n = CALLSIGN_PRESETS.len() as i32;
                self.callsign = CALLSIGN_PRESETS[(idx + dir).rem_euclid(n) as usize].to_string();
            }
            MdlOption::Suit => self.suit_color = cycle_in(&SuitColor::ALL, self.suit_color, dir),
            MdlOption::Visor => self.visor_tint = cycle_in(&VisorTint::ALL, self.visor_tint, dir),
            MdlOption::Accessory => {
                self.accessory = cycle_in(&HelmetAccessory::ALL, self.accessory, dir)
            }
            MdlOption::TrailColor => {
                self.trail_color = cycle_in(&JetpackTrailColor::ALL, self.trail_color, dir)
            }
            MdlOption::Patch => {
                self.mission_patch = cycle_in(&MissionPatch::ALL, self.mission_patch, dir)
            }
            MdlOption::Backpack => {
                self.backpack_module = cycle_in(&BackpackModule::ALL, self.backpack_module, dir)
            }
            MdlOption::Trait => {
                self.astronaut_trait = cycle_in(&AstronautTrait::ALL, self.astronaut_trait, dir)
            }
        }
    }

    /// 4 One-Click Game Mode Presets: Survival, Sandbox, Tryb Trudny, Hardcore!
    pub fn apply_preset(&mut self, preset_idx: usize) {
        match preset_idx {
            0 => {
                // 1. TRYB SURVIVAL (Standardowy)
                *self = Self::default();
                self.scenario = MissionScenario::PolsaKopernikBase;
                self.difficulty = DifficultyLevel::SystemsEngineer;
                self.callsign = "KDR. TWARDOWSKI".to_string();
                self.suit_color = SuitColor::HusariaCrimson;
                self.visor_tint = VisorTint::GoldReflective;
                self.accessory = HelmetAccessory::HussarWings;
                self.trail_color = JetpackTrailColor::PolishWhiteRed;
            }
            1 => {
                // 2. TRYB SANDBOX (Kreatywny — Nieskończone Surowce!)
                *self = Self::default();
                self.scenario = MissionScenario::OpenSandbox;
                self.difficulty = DifficultyLevel::Cadet;
                self.launch_vehicle = LaunchVehicle::SlsBlock1B;
                self.power_system = PowerSystemOption::HybridRtgNuclearSolar;
                self.comms_system = CommsSystemOption::KaBandOpticalLaser;
                self.prelanded_rover = PrelandedRoverChoice::HeavyPressurizedCruiser;
                self.callsign = "GEN. HERMASZEWSKI".to_string();
                self.suit_color = SuitColor::GoldThermal;
                self.visor_tint = VisorTint::CobaltBlue;
                self.accessory = HelmetAccessory::SolarHalo;
            }
            2 => {
                // 3. TRYB TRUDNY (Hard)
                *self = Self::default();
                self.scenario = MissionScenario::ShackletonIceProspecting;
                self.difficulty = DifficultyLevel::FlightDirector;
                self.launch_vehicle = LaunchVehicle::FalconHeavy;
                self.power_system = PowerSystemOption::TripleJunctionSolarPlusBattery;
                self.prelanded_rover = PrelandedRoverChoice::LightUtilityRover;
                self.callsign = crate::locale::text("INŻ. M. BEKKER").to_string();
                self.suit_color = SuitColor::ArtemisOrange;
                self.astronaut_trait = AstronautTrait::RoverTestPilot;
            }
            _ => {
                // 4. TRYB HARDCORE (Jedno Życie!)
                *self = Self::default();
                self.scenario = MissionScenario::FarsideRadioObservatory;
                self.difficulty = DifficultyLevel::FlightDirector;
                self.launch_vehicle = LaunchVehicle::SlsBlock1B;
                self.orbit = LunarOrbit::NearRectilinearHalo;
                self.power_system = PowerSystemOption::HybridRtgNuclearSolar;
                self.comms_system = CommsSystemOption::KaBandOpticalLaser;
                self.prelanded_rover = PrelandedRoverChoice::NoneBuildOnSurface;
                self.callsign = "KDR. S. LEM".to_string();
                self.suit_color = SuitColor::StealthGrey;
                self.visor_tint = VisorTint::RubyLaser;
                self.accessory = HelmetAccessory::LaserCrest;
            }
        }
    }

    pub fn toggle_instrument(&mut self, inst: SciInstrument) {
        if let Some(pos) = self.selected_instruments.iter().position(|x| *x == inst) {
            self.selected_instruments.remove(pos);
        } else {
            self.selected_instruments.push(inst);
        }
    }

    pub fn negotiate_dilemma(&mut self) {
        self.dilemma_index = (self.dilemma_index + 1) % 4;
        self.dilemma_resolved_bonus = (self.dilemma_resolved_bonus + 0.08).min(1.32);
    }

    pub fn dry_bus_mass_kg(&self) -> f32 {
        760.0 + (self.planned_lifetime_sols as f32) * 16.0
    }

    pub fn total_mass_kg(&self) -> f32 {
        let inst_mass: f32 = self.selected_instruments.iter().map(|i| i.mass_kg()).sum();
        self.dry_bus_mass_kg()
            + self.power_system.mass_kg()
            + self.comms_system.mass_kg()
            + self.prelanded_rover.mass_kg()
            + inst_mass
    }

    pub fn effective_max_mass_kg(&self) -> f32 {
        if self.scenario == MissionScenario::OpenSandbox {
            99999.0
        } else {
            self.launch_vehicle.max_tli_mass_kg() * self.orbit.payload_mass_multiplier()
        }
    }

    pub fn total_power_demand_w(&self) -> f32 {
        let inst_power: f32 = self.selected_instruments.iter().map(|i| i.power_w()).sum();
        160.0 + self.comms_system.power_w() + inst_power
    }

    pub fn total_power_supply_w(&self) -> f32 {
        self.power_system.generation_w()
    }

    pub fn ops_cost_musd(&self) -> f32 {
        (self.planned_lifetime_sols as f32) * 2.5
    }

    pub fn total_cost_musd(&self) -> f32 {
        let inst_cost: f32 = self
            .selected_instruments
            .iter()
            .map(|i| i.cost_musd())
            .sum();
        self.launch_vehicle.cost_musd()
            + self.power_system.cost_musd()
            + self.comms_system.cost_musd()
            + self.prelanded_rover.cost_musd()
            + inst_cost
            + self.ops_cost_musd()
    }

    pub fn max_budget_musd(&self) -> f32 {
        self.scenario.base_budget_musd(self.difficulty)
    }

    pub fn total_data_rate_mbps(&self) -> f32 {
        let inst_data: f32 = self
            .selected_instruments
            .iter()
            .map(|i| i.data_rate_mbps())
            .sum();
        0.8 + inst_data
    }

    pub fn total_payload_science_value(&self) -> f32 {
        let raw: f32 = self
            .selected_instruments
            .iter()
            .map(|i| i.base_science_value())
            .sum();
        raw * self.orbit.science_multiplier()
    }

    pub fn effective_comms_capacity_mbps(&self) -> f32 {
        self.comms_system.capacity_mbps() * self.orbit.comms_multiplier()
    }

    pub fn total_power_demand_ok(&self) -> bool {
        self.total_power_demand_w() <= self.total_power_supply_w()
    }

    pub fn cml_level(&self) -> u8 {
        let mut level = 1;
        if !self.selected_instruments.is_empty() {
            level = 2;
        }
        if level == 2
            && self.total_mass_kg() <= self.effective_max_mass_kg()
            && self.total_power_demand_ok()
        {
            level = 3;
        }
        if self.validate_launch_readiness().is_ok() {
            level = 4;
        }
        level
    }

    pub fn validate_launch_readiness(&self) -> Result<(), String> {
        if self.scenario == MissionScenario::OpenSandbox {
            return Ok(());
        }
        if self.selected_instruments.is_empty() {
            return Err(
                crate::locale::text("Zaznacz przynajmniej 1 przyrząd badawczy na liście.")
                    .to_string(),
            );
        }
        if self.total_mass_kg() > self.effective_max_mass_kg() {
            return Err(moon_base_localize::localized_format!(
                "Za ciężki ładunek: {:.0} kg / {:.0} kg! Wybierz większą rakietę lub odepnij sprzęt.",
                self.total_mass_kg(),
                self.effective_max_mass_kg()
            ));
        }
        if self.total_cost_musd() > self.max_budget_musd() {
            return Err(moon_base_localize::localized_format!(
                "Przekroczono budżet: ${:.0}M / ${:.0}M!",
                self.total_cost_musd(),
                self.max_budget_musd()
            ));
        }
        if self.total_power_demand_w() > self.total_power_supply_w() {
            return Err(moon_base_localize::localized_format!(
                "Za mało prądu: potrzeba {:.0}W, a zasilanie daje {:.0}W! Wybierz mocniejsze panele.",
                self.total_power_demand_w(),
                self.total_power_supply_w()
            ));
        }
        if self.scenario == MissionScenario::FarsideRadioObservatory
            && self.orbit == LunarOrbit::DirectSurfaceTli
        {
            return Err(crate::locale::text(
                "Ciemna strona Księżyca wymaga satelity na orbicie (NRHO, Niska lub Polarna)!",
            )
            .to_string());
        }
        if self.orbit == LunarOrbit::NearRectilinearHalo
            && !self.launch_vehicle.supports_nrho_heavy()
            && self.total_mass_kg() > 1650.0
        {
            return Err(crate::locale::text(
                "Lekka rakieta jest za słaba na ciężki ładunek na orbicie NRHO!",
            )
            .to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TileKind {
    MariaBasalt,
    HighlandRegolith,
    CraterRim,
    CraterFloor,
    SubsurfaceIceVein,
    Helium3Deposit,
    TitaniumOutcrop,
    RareMeteoriteDeposit,
}

impl TileKind {
    pub fn name(&self) -> &'static str {
        match self {
            Self::MariaBasalt => crate::locale::text("Bazalt"),
            Self::HighlandRegolith => crate::locale::text("Regolit"),
            Self::CraterRim => crate::locale::text("Brzeg krateru"),
            Self::CraterFloor => crate::locale::text("Dno krateru"),
            Self::SubsurfaceIceVein => crate::locale::text("Lód"),
            Self::Helium3Deposit => "Hel-3",
            Self::TitaniumOutcrop => crate::locale::text("Ruda tytanu"),
            Self::RareMeteoriteDeposit => crate::locale::text("Meteoryt"),
        }
    }

    /// Base time in seconds required to mine 100% of this block with the starter tool (Mk.I).
    pub fn base_mine_time_sec(&self) -> f32 {
        match self {
            Self::MariaBasalt | Self::HighlandRegolith | Self::CraterFloor => 0.85,
            Self::CraterRim => 1.05,
            Self::SubsurfaceIceVein => 1.15,
            Self::TitaniumOutcrop => 1.35,
            Self::Helium3Deposit => 1.45,
            Self::RareMeteoriteDeposit => 1.65,
        }
    }
}

/// Procedural large & small world features, landmarks, loot pods, and Polish space easter eggs!
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WorldFeatureKind {
    SupplyDropPod,
    ApolloLrvWreck,
    CrashedLunaProbe,
    TwardowskiRooster,
    LunarFlagPlaque,
    GlowingCrystalCluster,
    KopernikRuinedHub,
    LemMonolith,
    HeavyCargoWreck,
    AutomatedIceRig,
}

pub const LUR_REPAIR_REGOLITH: u32 = 7;
pub const LUR_REPAIR_SILICON: u32 = 3;
pub const LUR_REPAIR_HINT: &str = "Naprawa LUR-1 wymaga 3 szt. krzemu i 7 szt. regolitu.";

impl WorldFeatureKind {
    pub fn name(&self) -> &'static str {
        match self {
            Self::SupplyDropPod => crate::locale::text(
                "Kapsuła Zaopatrzeniowa (Przytrzymaj LPM by otworzyć skrzynię!)",
            ),
            Self::ApolloLrvWreck => crate::locale::text(
                "Wrak LUR-1 · 3 szt. krzemu + 7 szt. regolitu · przytrzymaj LPM",
            ),
            Self::CrashedLunaProbe => {
                crate::locale::text("Rozbita Sonda (Przytrzymaj LPM by pobrać części)")
            }
            Self::TwardowskiRooster => {
                crate::locale::text("Kogut i Lusterko Pana Twardowskiego (Legendarny Skarb!)")
            }
            Self::LunarFlagPlaque => crate::locale::text("Biało-Czerwona Flaga Kosmiczna PL"),
            Self::GlowingCrystalCluster => {
                crate::locale::text("Skupisko Kryształów Księżycowych (Przytrzymaj LPM)")
            }
            Self::KopernikRuinedHub => {
                crate::locale::text("Stacja 'Kopernik-0' — Osiedl się lub wejdź przez Śluzę [E]!")
            }
            Self::LemMonolith => {
                crate::locale::text("Czarny Obelisk 'Niezwyciężony' S. Lema (Tajemnicza Energia)")
            }
            Self::HeavyCargoWreck => {
                crate::locale::text("Wrak Ciężkiego Frachtowca (Skarbiec Tytanu i Części)")
            }
            Self::AutomatedIceRig => {
                crate::locale::text("Automatyczna Kopalnia Lodu (Przytrzymaj LPM by zebrać Lód)")
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ToolTier {
    GeoHammer = 0,
    ThermalIceExtractor = 1,
    PlasmaCutter = 2,
    LaserCoreDrill = 3,
    ArcWeldingTorch = 4,
    QuantumScanner = 5,
}

impl ToolTier {
    pub const ALL: [ToolTier; 6] = [
        ToolTier::GeoHammer,
        ToolTier::ThermalIceExtractor,
        ToolTier::PlasmaCutter,
        ToolTier::LaserCoreDrill,
        ToolTier::ArcWeldingTorch,
        ToolTier::QuantumScanner,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            Self::GeoHammer => crate::locale::text("Kilof Geologiczny Mk.I (1.0x szybkości)"),
            Self::ThermalIceExtractor => {
                crate::locale::text("Termo-Wiertło Mk.II (1.7x szybciej!)")
            }
            Self::PlasmaCutter => crate::locale::text("Przecinak Plazmowy Mk.III (2.5x szybciej!)"),
            Self::LaserCoreDrill => crate::locale::text("Laser Górniczy Mk.IV (3.4x szybciej!)"),
            Self::ArcWeldingTorch => crate::locale::text("Spawarka Łukowa Mk.V (4.5x szybciej!)"),
            Self::QuantumScanner => {
                crate::locale::text("Wiertło Kwantowe Mk.VI (6.0x MAX SZYBKOŚĆ!)")
            }
        }
    }

    pub fn short_name(&self) -> &'static str {
        match self {
            Self::GeoHammer => crate::locale::text("Kilof Mk.I (1.0x)"),
            Self::ThermalIceExtractor => crate::locale::text("Wiertło Mk.II (1.7x)"),
            Self::PlasmaCutter => crate::locale::text("Plazma Mk.III (2.5x)"),
            Self::LaserCoreDrill => crate::locale::text("Laser Mk.IV (3.4x)"),
            Self::ArcWeldingTorch => crate::locale::text("Spawarka Mk.V (4.5x)"),
            Self::QuantumScanner => crate::locale::text("Kwant Mk.VI (6.0x)"),
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::GeoHammer => {
                crate::locale::text("Podstawowy kilof do kopania skał, lodu i rozbierania bloków.")
            }
            Self::ThermalIceExtractor => {
                crate::locale::text("Kopie o 70% szybciej i daje więcej lodu wodnego!")
            }
            Self::PlasmaCutter => crate::locale::text(
                "Kopie 2.5x szybciej, tnie tytan i odblokowuje zaawansowane bloki!",
            ),
            Self::LaserCoreDrill => {
                crate::locale::text("Kopie 3.4x szybciej i wydobywa czysty Hel-3!")
            }
            Self::ArcWeldingTorch => {
                crate::locale::text("Kopie 4.5x szybciej i podwaja uzysk rzadkich rud!")
            }
            Self::QuantumScanner => crate::locale::text(
                "Najwyższy poziom! Kopie błyskawicznie (6x) i daje +50% odkryć!",
            ),
        }
    }

    /// How much faster this tool mines blocks compared to Mk.I.
    pub fn mining_speed_multiplier(&self) -> f32 {
        match self {
            Self::GeoHammer => 1.0,
            Self::ThermalIceExtractor => 1.70,
            Self::PlasmaCutter => 2.50,
            Self::LaserCoreDrill => 3.40,
            Self::ArcWeldingTorch => 4.50,
            Self::QuantumScanner => 6.00,
        }
    }

    pub fn next(&self) -> Option<ToolTier> {
        ToolTier::ALL.get(*self as usize + 1).copied()
    }

    pub fn craft_cost(&self) -> (u32, u32, u32, u32) {
        // (Regolith, WaterIce, Titanium, Silicon)
        let (a, b, c, d) = match self {
            Self::GeoHammer => (0, 0, 0, 0),
            Self::ThermalIceExtractor => (10, 5, 0, 0),
            Self::PlasmaCutter => (14, 6, 6, 4),
            Self::LaserCoreDrill => (18, 10, 10, 8),
            Self::ArcWeldingTorch => (16, 8, 12, 10),
            Self::QuantumScanner => (22, 12, 16, 14),
        };
        (
            (a * 6 + 4) / 5,
            (b * 6 + 4) / 5,
            (c * 6 + 4) / 5,
            (d * 6 + 4) / 5,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StructureKind {
    Airlock,
    HabitatModule,
    SolarArray,
    RtgGenerator,
    OxygenElectrolyzer,
    HydroponicsDome,
    StorageBay,
    CommsDish,
    ScienceLab,
    ShieldWall,
}

impl StructureKind {
    pub const ALL: [StructureKind; 10] = [
        StructureKind::Airlock,
        StructureKind::HabitatModule,
        StructureKind::SolarArray,
        StructureKind::RtgGenerator,
        StructureKind::OxygenElectrolyzer,
        StructureKind::HydroponicsDome,
        StructureKind::StorageBay,
        StructureKind::CommsDish,
        StructureKind::ScienceLab,
        StructureKind::ShieldWall,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            Self::Airlock => {
                crate::locale::text("Śluza Powietrzna (Drzwi do Bazy — wejdź klawiszem [E])")
            }
            Self::HabitatModule => {
                crate::locale::text("Dom Mieszkalny (Habitat — daje tlen, ciepło i ochronę)")
            }
            Self::SolarArray => crate::locale::text("Panel Słoneczny (+190W prądu w dzień)"),
            Self::RtgGenerator => {
                crate::locale::text("Reaktor Atomowy RTG (+155W prądu w dzień i w nocy!)")
            }
            Self::OxygenElectrolyzer => {
                crate::locale::text("Generator Tlenu O2 (szybkie ładowanie butli)")
            }
            Self::HydroponicsDome => crate::locale::text("Szklarnia (hoduje jedzenie)"),
            Self::StorageBay => crate::locale::text("Magazyn Zasobów (powiększa zapasy bazy)"),
            Self::CommsDish => crate::locale::text("Antena Satelitarna (szybszy przesył odkryć)"),
            Self::ScienceLab => {
                crate::locale::text("Laboratorium Badawcze (odkrycia i technologie)")
            }
            Self::ShieldWall => crate::locale::text("Mur Obronny (chroni bazę przed meteorytami!)"),
        }
    }

    pub fn short_name(&self) -> &'static str {
        match self {
            Self::Airlock => crate::locale::text("Śluza"),
            Self::HabitatModule => crate::locale::text("Habitat"),
            Self::SolarArray => "Solar",
            Self::RtgGenerator => crate::locale::text("Reaktor"),
            Self::OxygenElectrolyzer => crate::locale::text("Tlen O2"),
            Self::HydroponicsDome => crate::locale::text("Szklarnia"),
            Self::StorageBay => crate::locale::text("Magazyn"),
            Self::CommsDish => crate::locale::text("Antena"),
            Self::ScienceLab => "Lab",
            Self::ShieldWall => crate::locale::text("Mur"),
        }
    }

    pub fn simple_kid_hint(&self) -> &'static str {
        match self {
            Self::Airlock => crate::locale::text("Drzwi do Twojego domu! Postaw obok Habitatu i naciśnij [E], aby wejść do środka przez śluzę."),
            Self::HabitatModule => crate::locale::text("Twój bezpieczny dom na Księżycu! Stawiaj kilka obok siebie, by połączyć je w dużą bazę."),
            Self::SolarArray => crate::locale::text("Daje darmowy prąd ze Słońca. Postaw kilka obok siebie dla +15% bonusu mocy!"),
            Self::RtgGenerator => crate::locale::text("Daje prąd bez przerwy — nawet podczas mroźnej nocy księżycowej!"),
            Self::OxygenElectrolyzer => crate::locale::text("Produkuje świeży tlen z lodu wodnego dla całej Twojej bazy."),
            Self::HydroponicsDome => crate::locale::text("Podłącz szklarnię do zasilonej bazy. Przynieś wodę i wybierz „Zrób posiłek”."),
            Self::StorageBay => crate::locale::text("Bezpieczny skarbiec na wykopane surowce i narzędzia."),
            Self::CommsDish => crate::locale::text("Wielki talerz łączności z Ziemią i satelitami na orbicie."),
            Self::ScienceLab => crate::locale::text("Badaj próbki skał i odblokowuj kolejne osiągnięcia naukowe!"),
            Self::ShieldWall => crate::locale::text("Chroni sąsiednie budynki przed deszczem meteorytów!"),
        }
    }

    pub fn required_tool(&self) -> ToolTier {
        match self {
            Self::Airlock
            | Self::HabitatModule
            | Self::SolarArray
            | Self::ShieldWall
            | Self::StorageBay
            | Self::OxygenElectrolyzer => ToolTier::GeoHammer,
            Self::CommsDish | Self::HydroponicsDome => ToolTier::ThermalIceExtractor,
            Self::RtgGenerator | Self::ScienceLab => ToolTier::PlasmaCutter,
        }
    }

    pub fn build_cost(&self) -> (u32, u32, u32, u32) {
        // (Regolith, Titanium, Silicon, Helium3)
        let (a, b, c, d) = match self {
            Self::Airlock => (8, 3, 2, 0),
            Self::HabitatModule => (12, 6, 4, 0),
            Self::SolarArray => (6, 3, 6, 0),
            Self::RtgGenerator => (10, 8, 6, 2),
            Self::OxygenElectrolyzer => (10, 4, 4, 0),
            Self::HydroponicsDome => (14, 6, 8, 1),
            Self::StorageBay => (10, 4, 2, 0),
            Self::CommsDish => (8, 6, 8, 0),
            Self::ScienceLab => (14, 8, 8, 1),
            Self::ShieldWall => (6, 1, 0, 0),
        };
        (
            (a * 3 + 1) / 2,
            (b * 3 + 1) / 2,
            (c * 3 + 1) / 2,
            (d * 3 + 1) / 2,
        )
    }

    pub fn power_delta_w(&self, sun_factor: f32) -> f32 {
        match self {
            Self::Airlock => -10.0,
            Self::HabitatModule => -35.0,
            Self::SolarArray => 190.0 * sun_factor,
            Self::RtgGenerator => 155.0,
            Self::OxygenElectrolyzer => -50.0,
            Self::HydroponicsDome => -45.0,
            Self::StorageBay => -5.0,
            Self::CommsDish => -40.0,
            Self::ScienceLab => -60.0,
            Self::ShieldWall => 0.0,
        }
    }

    pub fn is_pressurized_room(&self) -> bool {
        matches!(
            self,
            Self::Airlock | Self::HabitatModule | Self::HydroponicsDome | Self::ScienceLab
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RoverKind {
    LightBuggy,
    HeavyCruiser,
}

impl RoverKind {
    pub fn name(&self) -> &'static str {
        match self {
            Self::LightBuggy => crate::locale::text(
                "Łazik Bekkera LUR-1 (Chroni przed promieniowaniem, -75% zużycia O2)",
            ),
            Self::HeavyCruiser => {
                crate::locale::text("Krążownik Ciśnieniowy LSC-2 (Pełny tlen O2 i ochrona kabiny!)")
            }
        }
    }

    pub fn short_name(&self) -> &'static str {
        match self {
            Self::LightBuggy => crate::locale::text("Łazik LUR-1"),
            Self::HeavyCruiser => crate::locale::text("Krążownik LSC-2"),
        }
    }

    pub fn max_speed(&self) -> f32 {
        match self {
            Self::LightBuggy => 8.2,
            Self::HeavyCruiser => 6.2,
        }
    }

    pub fn max_battery(&self) -> f32 {
        match self {
            Self::LightBuggy => 140.0,
            Self::HeavyCruiser => 260.0,
        }
    }

    #[allow(dead_code)]
    pub fn cargo_bonus(&self) -> u32 {
        match self {
            Self::LightBuggy => 120,
            Self::HeavyCruiser => 240,
        }
    }

    pub fn is_pressurized(&self) -> bool {
        matches!(self, Self::HeavyCruiser)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LunarEventKind {
    DustStorm,
    Nominal,
    MeteoriteShower,
    SolarFlareStorm,
    Moonquake,
    LunarNightDeepFreeze,
    OrbitalSupplyDrop,
}

impl LunarEventKind {
    #[allow(dead_code)]
    pub fn title(&self) -> &'static str {
        match self {
            Self::DustStorm => crate::locale::text("Burza pyłowa"),
            Self::Nominal => crate::locale::text("POGODA KOSMICZNA: SPOKOJNIE"),
            Self::MeteoriteShower => crate::locale::text("DESZCZ METEORYTÓW!"),
            Self::SolarFlareStorm => crate::locale::text("BURZA SŁoneczna (PROMIENIOWANIE!)"),
            Self::Moonquake => crate::locale::text("WSTRZĄS SEJSMICZNY KSIĘŻYCA!"),
            Self::LunarNightDeepFreeze => crate::locale::text("NOC KSIĘŻYCOWA (-172°C)"),
            Self::OrbitalSupplyDrop => crate::locale::text("ZRZUT KAPSUŁY ZAOPATRZENIA!"),
        }
    }

    pub fn color(&self) -> Color {
        match self {
            Self::DustStorm => Color::rgb(0.98, 0.65, 0.39),
            Self::Nominal => Color::rgb(0.45, 0.95, 0.65),
            Self::MeteoriteShower => Color::rgb(1.0, 0.58, 0.22),
            Self::SolarFlareStorm => Color::rgb(1.0, 0.32, 0.45),
            Self::Moonquake => Color::rgb(0.95, 0.72, 0.35),
            Self::LunarNightDeepFreeze => Color::rgb(0.48, 0.78, 1.0),
            Self::OrbitalSupplyDrop => Color::rgb(0.32, 0.98, 0.88),
        }
    }
}
