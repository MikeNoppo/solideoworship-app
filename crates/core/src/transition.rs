use std::ops::RangeInclusive;

use crate::settings::SettingsError;

pub const DURATION_MS: RangeInclusive<u16> = 100..=2000;
pub const DURATION_STEP_MS: u16 = 50;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TransitionMode {
    #[default]
    Cut,
    Fade,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransitionSpec {
    pub mode: TransitionMode,
    pub duration_ms: u16,
}

impl TransitionSpec {
    pub fn validate(self) -> Result<(), SettingsError> {
        if DURATION_MS.contains(&self.duration_ms)
            && self.duration_ms.is_multiple_of(DURATION_STEP_MS)
        {
            Ok(())
        } else {
            Err(SettingsError::InvalidTransitions)
        }
    }

    pub fn clamped(mut self) -> Self {
        let bounded = self
            .duration_ms
            .clamp(*DURATION_MS.start(), *DURATION_MS.end());
        self.duration_ms = ((bounded + DURATION_STEP_MS / 2) / DURATION_STEP_MS) * DURATION_STEP_MS;
        self
    }
}

impl Default for TransitionSpec {
    fn default() -> Self {
        Self {
            mode: TransitionMode::Cut,
            duration_ms: 500,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TransitionSettings {
    pub slides: TransitionSpec,
    pub black: TransitionSpec,
    pub clear: TransitionSpec,
    pub logo: TransitionSpec,
}

impl TransitionSettings {
    pub fn validate(self) -> Result<(), SettingsError> {
        self.slides.validate()?;
        self.black.validate()?;
        self.clear.validate()?;
        self.logo.validate()
    }

    pub fn clamped(self) -> Self {
        Self {
            slides: self.slides.clamped(),
            black: self.black.clamped(),
            clear: self.clear.clamped(),
            logo: self.logo.clamped(),
        }
    }
}
