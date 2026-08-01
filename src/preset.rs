use crate::control::{MacroId, Normalized};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const PRESET_SCHEMA_VERSION: u32 = 5;
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
    #[error("schema-5 preset requires a known model")]
    InvalidModel,
    #[error("version-1 envelope must contain finite positive times and a sustain in 0..=1")]
    InvalidLegacyEnvelope,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelDPatchId {
    Bass,
    Lead,
    FilterArticulation,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SixOpPatchId {
    BellMetal,
    FracturedMetal,
    ElectricPianoMallet,
    GlassWood,
    BrassBass,
    MechanicalStab,
}

impl SixOpPatchId {
    pub const ALL: [Self; 6] = [
        Self::BellMetal,
        Self::FracturedMetal,
        Self::ElectricPianoMallet,
        Self::GlassWood,
        Self::BrassBass,
        Self::MechanicalStab,
    ];
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModelPatchId {
    ModelD(ModelDPatchId),
    SixOpPm(SixOpPatchId),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SynthesisModelId {
    ModelD,
    SixOpPm,
}

impl SynthesisModelId {
    pub const fn stable_id(self) -> &'static str {
        match self {
            Self::ModelD => "model_d",
            Self::SixOpPm => "six_op_pm",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::ModelD => "Model D",
            Self::SixOpPm => "Six-Op PM",
        }
    }
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
            MacroId::Space => self.space,
            MacroId::Attack => self.attack,
            MacroId::Decay => self.decay,
            MacroId::Sustain => self.sustain,
            MacroId::Release => self.release,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Preset {
    pub schema_version: u32,
    pub name: String,
    pub voices: usize,
    pub output_gain: f32,
    pub model: SynthesisModelId,
    pub model_patch: ModelPatchId,
    pub macros: MacroValues,
}

impl Preset {
    pub fn parse(source: &str) -> Result<Self, PresetError> {
        let value: toml::Value = toml::from_str(source)?;
        let version = value
            .get("schema_version")
            .and_then(toml::Value::as_integer)
            .and_then(|version| u32::try_from(version).ok())
            .ok_or(PresetError::UnsupportedVersion(0))?;
        let preset = match version {
            PRESET_SCHEMA_VERSION => VersionFivePreset::parse(source)?,
            4 => VersionFourPreset::parse(source)?.into_current(),
            3 => VersionThreePreset::parse(source)?.into_current(),
            2 => VersionTwoPreset::parse(source)?.into_current(),
            1 => LegacyPreset::parse(source)?.into_current(),
            unsupported => return Err(PresetError::UnsupportedVersion(unsupported)),
        };
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
        Ok(self)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct VersionFiveModelDPreset {
    schema_version: u32,
    name: String,
    voices: usize,
    output_gain: f32,
    model: SynthesisModelId,
    model_d_patch: ModelDPatchId,
    macros: MacroValues,
}

#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
struct SixOpMacroValues {
    index: Normalized,
    ratio: Normalized,
    feedback: Normalized,
    operator_decay: Normalized,
    balance: Normalized,
    key_scale: Normalized,
    velocity: Normalized,
    motion: Normalized,
    attack: Normalized,
    decay: Normalized,
    sustain: Normalized,
    release: Normalized,
}

impl From<SixOpMacroValues> for MacroValues {
    fn from(values: SixOpMacroValues) -> Self {
        Self {
            evolve: values.index,
            shape: values.ratio,
            color: values.feedback,
            edge: values.operator_decay,
            couple: values.balance,
            motion: values.key_scale,
            depth: values.velocity,
            space: values.motion,
            attack: values.attack,
            decay: values.decay,
            sustain: values.sustain,
            release: values.release,
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct VersionFiveSixOpPreset {
    schema_version: u32,
    name: String,
    voices: usize,
    output_gain: f32,
    model: SynthesisModelId,
    six_op_patch: SixOpPatchId,
    macros: SixOpMacroValues,
}

struct VersionFivePreset;

impl VersionFivePreset {
    fn parse(source: &str) -> Result<Preset, PresetError> {
        let value: toml::Value = toml::from_str(source)?;
        let model = value
            .get("model")
            .and_then(toml::Value::as_str)
            .ok_or(PresetError::InvalidModel)?;
        match model {
            "model_d" => {
                let preset: VersionFiveModelDPreset = toml::from_str(source)?;
                if preset.schema_version != PRESET_SCHEMA_VERSION
                    || preset.model != SynthesisModelId::ModelD
                {
                    return Err(PresetError::UnsupportedVersion(preset.schema_version));
                }
                Ok(Preset {
                    schema_version: PRESET_SCHEMA_VERSION,
                    name: preset.name,
                    voices: preset.voices,
                    output_gain: preset.output_gain,
                    model: preset.model,
                    model_patch: ModelPatchId::ModelD(preset.model_d_patch),
                    macros: preset.macros,
                })
            }
            "six_op_pm" => {
                let preset: VersionFiveSixOpPreset = toml::from_str(source)?;
                if preset.schema_version != PRESET_SCHEMA_VERSION
                    || preset.model != SynthesisModelId::SixOpPm
                {
                    return Err(PresetError::UnsupportedVersion(preset.schema_version));
                }
                Ok(Preset {
                    schema_version: PRESET_SCHEMA_VERSION,
                    name: preset.name,
                    voices: preset.voices,
                    output_gain: preset.output_gain,
                    model: preset.model,
                    model_patch: ModelPatchId::SixOpPm(preset.six_op_patch),
                    macros: preset.macros.into(),
                })
            }
            _ => Err(PresetError::InvalidModel),
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct VersionFourPreset {
    schema_version: u32,
    name: String,
    voices: usize,
    output_gain: f32,
    model: SynthesisModelId,
    model_d_patch: ModelDPatchId,
    macros: MacroValues,
}

impl VersionFourPreset {
    fn parse(source: &str) -> Result<Self, PresetError> {
        let preset: Self = toml::from_str(source)?;
        if preset.schema_version != 4 || preset.model != SynthesisModelId::ModelD {
            return Err(PresetError::UnsupportedVersion(preset.schema_version));
        }
        Ok(preset)
    }

    fn into_current(self) -> Preset {
        Preset {
            schema_version: PRESET_SCHEMA_VERSION,
            name: self.name,
            voices: self.voices,
            output_gain: self.output_gain,
            model: SynthesisModelId::ModelD,
            model_patch: ModelPatchId::ModelD(self.model_d_patch),
            macros: self.macros,
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct VersionThreePreset {
    schema_version: u32,
    name: String,
    voices: usize,
    output_gain: f32,
    model_d_patch: ModelDPatchId,
    macros: MacroValues,
}

impl VersionThreePreset {
    fn parse(source: &str) -> Result<Self, PresetError> {
        let preset: Self = toml::from_str(source)?;
        if preset.schema_version != 3 {
            return Err(PresetError::UnsupportedVersion(preset.schema_version));
        }
        Ok(preset)
    }

    fn into_current(self) -> Preset {
        Preset {
            schema_version: PRESET_SCHEMA_VERSION,
            name: self.name,
            voices: self.voices,
            output_gain: self.output_gain,
            model: SynthesisModelId::ModelD,
            model_patch: ModelPatchId::ModelD(self.model_d_patch),
            macros: self.macros,
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct VersionTwoPreset {
    schema_version: u32,
    name: String,
    voices: usize,
    output_gain: f32,
    macros: MacroValues,
}

impl VersionTwoPreset {
    fn parse(source: &str) -> Result<Self, PresetError> {
        let preset: Self = toml::from_str(source)?;
        if preset.schema_version != 2 {
            return Err(PresetError::UnsupportedVersion(preset.schema_version));
        }
        Ok(preset)
    }

    fn into_current(self) -> Preset {
        Preset {
            schema_version: PRESET_SCHEMA_VERSION,
            name: self.name,
            voices: self.voices,
            output_gain: self.output_gain,
            model: SynthesisModelId::ModelD,
            model_patch: ModelPatchId::ModelD(ModelDPatchId::Bass),
            macros: self.macros,
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyPreset {
    schema_version: u32,
    name: String,
    voices: usize,
    output_gain: f32,
    envelope: LegacyEnvelope,
    macros: LegacyMacroValues,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyEnvelope {
    attack_seconds: f32,
    decay_seconds: f32,
    sustain_level: f32,
    release_seconds: f32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyMacroValues {
    evolve: Normalized,
    shape: Normalized,
    color: Normalized,
    edge: Normalized,
    couple: Normalized,
    motion: Normalized,
    depth: Normalized,
    width: Normalized,
    space: Normalized,
    attack: Normalized,
    decay: Normalized,
    sustain: Normalized,
    release: Normalized,
}

impl LegacyPreset {
    fn parse(source: &str) -> Result<Self, PresetError> {
        let preset: Self = toml::from_str(source)?;
        if preset.schema_version != 1
            || !preset.envelope.attack_seconds.is_finite()
            || preset.envelope.attack_seconds <= 0.0
            || !preset.envelope.decay_seconds.is_finite()
            || preset.envelope.decay_seconds <= 0.0
            || !preset.envelope.release_seconds.is_finite()
            || preset.envelope.release_seconds <= 0.0
            || !preset.envelope.sustain_level.is_finite()
            || !(0.0..=1.0).contains(&preset.envelope.sustain_level)
        {
            return Err(PresetError::InvalidLegacyEnvelope);
        }
        Ok(preset)
    }

    fn into_current(self) -> Preset {
        debug_assert_eq!(self.schema_version, 1);
        let _discarded_width = self.macros.width;
        let _legacy_envelope = (
            self.envelope.attack_seconds,
            self.envelope.decay_seconds,
            self.envelope.sustain_level,
            self.envelope.release_seconds,
        );
        Preset {
            schema_version: PRESET_SCHEMA_VERSION,
            name: self.name,
            voices: self.voices,
            output_gain: self.output_gain,
            model: SynthesisModelId::ModelD,
            model_patch: ModelPatchId::ModelD(ModelDPatchId::Bass),
            macros: MacroValues {
                evolve: self.macros.evolve,
                shape: self.macros.shape,
                color: self.macros.color,
                edge: self.macros.edge,
                couple: self.macros.couple,
                motion: self.macros.motion,
                depth: self.macros.depth,
                space: self.macros.space,
                attack: self.macros.attack,
                decay: self.macros.decay,
                sustain: self.macros.sustain,
                release: self.macros.release,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID: &str = r#"
schema_version = 4
name = "Reference Sine"
voices = 8
output_gain = 0.2
model = "model_d"
model_d_patch = "bass"

[macros]
evolve = 0.5
shape = 0.5
color = 0.5
edge = 0.5
couple = 0.5
motion = 0.5
depth = 0.5
space = 0.5
attack = 0.25
decay = 0.35
sustain = 0.7
release = 0.4
"#;

    #[test]
    fn parses_complete_version_four_preset() {
        let preset = Preset::parse(VALID).unwrap();
        assert_eq!(preset.schema_version, 5);
        assert_eq!(preset.voices, 8);
        assert_eq!(preset.model, SynthesisModelId::ModelD);
        assert_eq!(
            preset.model_patch,
            ModelPatchId::ModelD(ModelDPatchId::Bass)
        );
        assert_eq!(preset.macros.get(MacroId::Evolve).get(), 0.5);
    }

    #[test]
    fn rejects_unknown_fields_and_versions() {
        assert!(Preset::parse(&format!("{VALID}\nunknown = 3")).is_err());
        assert!(
            Preset::parse(&VALID.replace("schema_version = 4", "schema_version = 99")).is_err()
        );
        assert!(Preset::parse(&VALID.replace("\"bass\"", "\"unknown\"")).is_err());
    }

    #[test]
    fn rejects_invalid_numbers_and_voice_counts() {
        assert!(Preset::parse(&VALID.replace("voices = 8", "voices = 0")).is_err());
        assert!(Preset::parse(&VALID.replace("output_gain = 0.2", "output_gain = nan")).is_err());
        assert!(Preset::parse(&VALID.replace("evolve = 0.5", "evolve = 1.5")).is_err());
    }

    #[test]
    fn migrates_strict_version_one_and_discards_only_width() {
        let legacy = VALID
            .replace("schema_version = 4", "schema_version = 1")
            .replace("model = \"model_d\"\n", "")
            .replace("model_d_patch = \"bass\"\n", "")
            .replace(
                "[macros]",
                "[envelope]\nattack_seconds = 0.01\ndecay_seconds = 0.1\nsustain_level = 0.7\nrelease_seconds = 0.2\n\n[macros]",
            )
            .replace("space = 0.5", "width = 0.9\nspace = 0.5");
        let preset = Preset::parse(&legacy).unwrap();
        assert_eq!(preset.schema_version, 5);
        assert_eq!(preset.model, SynthesisModelId::ModelD);
        assert_eq!(
            preset.model_patch,
            ModelPatchId::ModelD(ModelDPatchId::Bass)
        );
        assert_eq!(preset.macros.space.get(), 0.5);
        assert!(Preset::parse(&format!("{legacy}\nunknown = 3")).is_err());
    }

    #[test]
    fn migrates_strict_version_two_to_the_bass_patch() {
        let version_two = VALID
            .replace("schema_version = 4", "schema_version = 2")
            .replace("model = \"model_d\"\n", "")
            .replace("model_d_patch = \"bass\"\n", "");
        let preset = Preset::parse(&version_two).unwrap();
        assert_eq!(preset.schema_version, 5);
        assert_eq!(preset.model, SynthesisModelId::ModelD);
        assert_eq!(
            preset.model_patch,
            ModelPatchId::ModelD(ModelDPatchId::Bass)
        );
        assert!(Preset::parse(&format!("{version_two}\nunknown = 3")).is_err());
    }

    #[test]
    fn migrates_strict_version_three_to_model_d() {
        let version_three = VALID
            .replace("schema_version = 4", "schema_version = 3")
            .replace("model = \"model_d\"\n", "");
        let preset = Preset::parse(&version_three).unwrap();
        assert_eq!(preset.schema_version, 5);
        assert_eq!(preset.model, SynthesisModelId::ModelD);
        assert_eq!(
            preset.model_patch,
            ModelPatchId::ModelD(ModelDPatchId::Bass)
        );
        assert!(Preset::parse(&format!("{version_three}\nunknown = 3")).is_err());
    }

    #[test]
    fn factory_presets_are_strict_and_cover_both_models() {
        let sources = [
            include_str!("../presets/01-full-bass.mojsint"),
            include_str!("../presets/02-full-lead.mojsint"),
            include_str!("../presets/03-full-filter-articulation.mojsint"),
            include_str!("../presets/reference.mojsint"),
            include_str!("../presets/05-matched-linear-mixer.mojsint"),
            include_str!("../presets/06-matched-linear-ladder.mojsint"),
            include_str!("../presets/07-matched-no-drift-or-feedback.mojsint"),
            include_str!("../presets/08-six-op-bell-metal.mojsint"),
            include_str!("../presets/09-six-op-fractured-metal.mojsint"),
            include_str!("../presets/10-six-op-electric-piano-mallet.mojsint"),
            include_str!("../presets/11-six-op-glass-wood.mojsint"),
            include_str!("../presets/12-six-op-brass-bass.mojsint"),
            include_str!("../presets/13-six-op-mechanical-stab.mojsint"),
        ];
        let presets = sources.map(|source| Preset::parse(source).unwrap());
        let mut names = presets
            .iter()
            .map(|preset| preset.name.as_str())
            .collect::<Vec<_>>();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), 13);
        assert!(
            presets[..7]
                .iter()
                .all(|preset| preset.model == SynthesisModelId::ModelD)
        );
        assert!(
            presets[7..]
                .iter()
                .all(|preset| preset.model == SynthesisModelId::SixOpPm)
        );
        assert_eq!(
            presets[0].model_patch,
            ModelPatchId::ModelD(ModelDPatchId::Bass)
        );
        assert_eq!(
            presets[1].model_patch,
            ModelPatchId::ModelD(ModelDPatchId::Lead)
        );
        assert_eq!(
            presets[2].model_patch,
            ModelPatchId::ModelD(ModelDPatchId::FilterArticulation)
        );
        for (preset, patch) in presets[7..].iter().zip(SixOpPatchId::ALL) {
            assert_eq!(preset.model_patch, ModelPatchId::SixOpPm(patch));
        }
    }

    #[test]
    fn parses_strict_version_five_six_op_preset_and_rejects_mixed_fields() {
        let valid = include_str!("../presets/08-six-op-bell-metal.mojsint");
        let preset = Preset::parse(valid).unwrap();
        assert_eq!(preset.schema_version, 5);
        assert_eq!(preset.model, SynthesisModelId::SixOpPm);
        assert_eq!(
            preset.model_patch,
            ModelPatchId::SixOpPm(SixOpPatchId::BellMetal)
        );
        assert_eq!(preset.macros.get(MacroId::Evolve).get(), 0.5);
        assert!(Preset::parse(&format!("{valid}\nmodel_d_patch = \"bass\"")).is_err());
        assert!(Preset::parse(&valid.replace("six_op_patch", "model_d_patch")).is_err());
    }
}
