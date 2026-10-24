//! R3TRØ CORE's exclusive (bit-perfect) mode setting, kept apart from Sonora's own audio files so
//! that a Sonora update never has to be merged into it.
//!
//! This holds the setting and the "is it active" flag the rest of the app reads. The output that
//! actually takes the device exclusively is not wired into Sonora's new output code yet, so for
//! now the flag stays off and the audio plays through the normal shared path.

use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ExclusiveMode {
    Off,
    #[default]
    Auto,
    Always,
}

impl ExclusiveMode {
    pub const ALL: [Self; 3] = [Self::Off, Self::Auto, Self::Always];

    pub fn id(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Auto => "auto",
            Self::Always => "always",
        }
    }

    pub fn from_id(id: &str) -> Self {
        match id {
            "off" => Self::Off,
            "always" => Self::Always,
            _ => Self::Auto,
        }
    }

    fn code(self) -> u8 {
        match self {
            Self::Off => 0,
            Self::Auto => 1,
            Self::Always => 2,
        }
    }

    fn from_code(code: u8) -> Self {
        match code {
            0 => Self::Off,
            2 => Self::Always,
            _ => Self::Auto,
        }
    }
}

static EXCLUSIVE_MODE: AtomicU8 = AtomicU8::new(1);
/// Whether the output now playing is the exclusive, bit-perfect one.
static EXCLUSIVE_ACTIVE: AtomicBool = AtomicBool::new(false);

/// Whether sound is leaving the device untouched by the system mixer right now.
pub fn exclusive_active() -> bool {
    EXCLUSIVE_ACTIVE.load(Ordering::Relaxed)
}

pub fn set_exclusive_mode(mode: ExclusiveMode) {
    EXCLUSIVE_MODE.store(mode.code(), Ordering::Relaxed);
}

/// Whether audio from `source` should take the device exclusively: never in `Off`, always in
/// `Always`, and in `Auto` for every source but YouTube, whose audio is compressed anyway.
pub fn exclusive_for(source: &str) -> bool {
    if !cfg!(windows) {
        return false;
    }
    match ExclusiveMode::from_code(EXCLUSIVE_MODE.load(Ordering::Relaxed)) {
        ExclusiveMode::Off => false,
        ExclusiveMode::Always => true,
        ExclusiveMode::Auto => source != "youtube",
    }
}
