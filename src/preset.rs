use crate::control::{MacroId, Normalized};
use crate::envelope::{AdsrConfig, EnvelopeError};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const PRESET_SCHEMA_VERSION: u32 = 1;
pub const MAX_VOICES: usize = 64;

#[derive(Debug, Error)]
pub enum PresetError {
    #[error("invalid TOML preset: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("unsupported preset schema version {0}")]
    UnsupportedVersion(u32),
    #[error("preset name must not be empty")]
    EmptyName,
    #[error("voice count must be between 1 and {MAX_VOICES}")]
    InvalidVoiceCount,
    #[error("output gain must be finite and between 0 and 1")]
    InvalidOutputGain,
    #[error("invalid envelope: {0}")]
    InvalidEnvelope(#[from] EnvelopeError),
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MacroValues {
    pub evolve: Normalized,
    pub shape: Normalized,
    pub color: Normalized,
    pub edge: Normalized,
    pub couple: Normalized,
    pub motion: Normalized,
    pub depth: Normalized,
    pub width: Normalized,
    pub space: Normalized,
    pub attack: Normalized,
    pub decay: Normalized,
    pub sustain: Normalized,
    pub release: Normalized,
}

impl MacroValues {
    pub const fn get(self, id: MacroId) -> Normalized {
        match id {
            MacroId::Evolve => self.evolve,
            MacroId::Shape => self.shape,
            MacroId::Color => self.color,
            MacroId::Edge => self.edge,
            MacroId::Couple => self.couple,
            MacroId::Motion => self.motion,
            MacroId::Depth => self.depth,
            MacroId::Width => self.width,
            MacroId::Space => self.space,
            MacroId::Attack => self.attack,
            MacroId::Decay => self.decay,
            MacroId::Sustain => self.sustain,
            MacroId::Release => self.release,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Preset {
    pub schema_version: u32,
    pub name: String,
    pub voices: usize,
    pub output_gain: f32,
    pub envelope: AdsrConfig,
    pub macros: MacroValues,
}

impl Preset {
    pub fn parse(source: &str) -> Result<Self, PresetError> {
        let preset: Self = toml::from_str(source)?;
        preset.validate()
    }

    pub fn validate(self) -> Result<Self, PresetError> {
        if self.schema_version != PRESET_SCHEMA_VERSION {
            return Err(PresetError::UnsupportedVersion(self.schema_version));
        }
        if self.name.trim().is_empty() {
            return Err(PresetError::EmptyName);
        }
        if !(1..=MAX_VOICES).contains(&self.voices) {
            return Err(PresetError::InvalidVoiceCount);
        }
        if !self.output_gain.is_finite() || !(0.0..=1.0).contains(&self.output_gain) {
            return Err(PresetError::InvalidOutputGain);
        }
        self.envelope.validate()?;
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID: &str = r#"
schema_version = 1
name = "Reference Sine"
voices = 8
output_gain = 0.2

[envelope]
attack_seconds = 0.01
decay_seconds = 0.1
sustain_level = 0.7
release_seconds = 0.2

[macros]
evolve = 0.5
shape = 0.5
color = 0.5
edge = 0.5
couple = 0.5
motion = 0.5
depth = 0.5
width = 0.5
space = 0.5
attack = 0.25
decay = 0.35
sustain = 0.7
release = 0.4
"#;

    #[test]
    fn parses_complete_version_one_preset() {
        let preset = Preset::parse(VALID).unwrap();
        assert_eq!(preset.schema_version, 1);
        assert_eq!(preset.voices, 8);
        assert_eq!(preset.macros.get(MacroId::Evolve).get(), 0.5);
    }

    #[test]
    fn rejects_unknown_fields_and_versions() {
        assert!(Preset::parse(&format!("{VALID}\nunknown = 3")).is_err());
        assert!(Preset::parse(&VALID.replace("schema_version = 1", "schema_version = 2")).is_err());
    }

    #[test]
    fn rejects_invalid_numbers_and_voice_counts() {
        assert!(Preset::parse(&VALID.replace("voices = 8", "voices = 0")).is_err());
        assert!(Preset::parse(&VALID.replace("output_gain = 0.2", "output_gain = nan")).is_err());
        assert!(Preset::parse(&VALID.replace("evolve = 0.5", "evolve = 1.5")).is_err());
        assert!(
            Preset::parse(&VALID.replace("attack_seconds = 0.01", "attack_seconds = 0.0")).is_err()
        );
    }
}
