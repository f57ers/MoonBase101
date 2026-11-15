use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        OnceLock,
    },
};

pub const ENGLISH: &str = include_str!("../locales/en.json");
pub const POLISH: &str = include_str!("../locales/pl.json");

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    #[default]
    English,
    Polish,
}

static ENGLISH_ACTIVE: AtomicBool = AtomicBool::new(false);

pub fn set_language(language: Language) {
    ENGLISH_ACTIVE.store(language == Language::English, Ordering::Relaxed);
}

pub fn language() -> Language {
    if ENGLISH_ACTIVE.load(Ordering::Relaxed) {
        Language::English
    } else {
        Language::Polish
    }
}

pub fn text(source: &'static str) -> &'static str {
    static EN: OnceLock<HashMap<String, String>> = OnceLock::new();
    static PL: OnceLock<HashMap<String, String>> = OnceLock::new();
    let translated = match language() {
        Language::English => {
            EN.get_or_init(|| serde_json::from_str(ENGLISH).expect("Valid English locale"))
        }
        Language::Polish => {
            PL.get_or_init(|| serde_json::from_str(POLISH).expect("Valid Polish locale"))
        }
    };
    translated.get(source).map_or(source, String::as_str)
}
