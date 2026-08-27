use crate::control::{MacroId, Normalized};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const PRESET_SCHEMA_VERSION: u32 = 8;
pub const MAX_VOICES: usize = 64;

#[derive(Debug, Error)]
pub enum PresetError {
    #[error("invalid TOML preset: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("cannot serialize preset: {0}")]
    Serialize(#[from] toml::ser::Error),
    #[error("unsupported preset schema version {0}")]
    UnsupportedVersion(u32),
    #[error("preset name must not be empty")]
    EmptyName,
    #[error("voice count must be between 1 and {MAX_VOICES}")]
    InvalidVoiceCount,
    #[error("output gain must be finite and between 0 and 1")]
    InvalidOutputGain,
    #[error("current preset schema requires a known model")]
    InvalidModel,
    #[error("current preset model and patch identity do not match")]
    InvalidPatch,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StrangePatchId {
    Unified,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SwarmPatchId {
    WarmPad,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BassMatrixPatchId {
    Transformer,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DualFilterPatchId {
    Industrial,
    Counter,
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
    StrangeOscillator(StrangePatchId),
    SwarmMachine(SwarmPatchId),
    BassMatrix(BassMatrixPatchId),
    DualFilter(DualFilterPatchId),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SynthesisModelId {
    ModelD,
    SixOpPm,
    StrangeOscillator,
    SwarmMachine,
    BassMatrix,
    DualFilter,
}

impl SynthesisModelId {
    pub const fn stable_id(self) -> &'static str {
        match self {
            Self::ModelD => "model_d",
            Self::SixOpPm => "six_op_pm",
            Self::StrangeOscillator => "strange_oscillator",
            Self::SwarmMachine => "swarm_machine",
            Self::BassMatrix => "bass_matrix",
            Self::DualFilter => "dual_filter",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::ModelD => "Model D",
            Self::SixOpPm => "Six-Op PM",
            Self::StrangeOscillator => "Strange Osc",
            Self::SwarmMachine => "Swarm Machine",
            Self::BassMatrix => "Bass Matrix",
            Self::DualFilter => "Dual Filter",
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
    #[serde(default = "midpoint", skip_serializing)]
    pub control_13: Normalized,
    #[serde(default = "midpoint", skip_serializing)]
    pub control_14: Normalized,
    #[serde(default = "midpoint", skip_serializing)]
    pub control_15: Normalized,
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
            MacroId::Control13 => self.control_13,
            MacroId::Control14 => self.control_14,
            MacroId::Control15 => self.control_15,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Preset {
    pub schema_version: u32,
    pub name: String,
    pub voices: usize,
    pub output_gain: f32,
    pub instrument_volume: Normalized,
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
            PRESET_SCHEMA_VERSION => VersionEightPreset::parse(source)?,
            7 => VersionSevenPreset::parse(source)?,
            6 => VersionSixPreset::parse(source)?,
            5 => VersionFivePreset::parse(source)?,
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
        if !matches!(
            (self.model, self.model_patch),
            (SynthesisModelId::ModelD, ModelPatchId::ModelD(_))
                | (SynthesisModelId::SixOpPm, ModelPatchId::SixOpPm(_))
                | (
                    SynthesisModelId::StrangeOscillator,
                    ModelPatchId::StrangeOscillator(_)
                )
                | (
                    SynthesisModelId::SwarmMachine,
                    ModelPatchId::SwarmMachine(_)
                )
                | (SynthesisModelId::BassMatrix, ModelPatchId::BassMatrix(_))
                | (SynthesisModelId::DualFilter, ModelPatchId::DualFilter(_))
        ) {
            return Err(PresetError::InvalidPatch);
        }
        Ok(self)
    }

    /// Encode the current preset as the strict public schema. Older presets
    /// are migrated by `parse`, so saving always publishes schema 8 with the
    /// exact model-specific patch and macro field names.
    pub fn to_toml(&self) -> Result<String, PresetError> {
        let preset = self.clone().validate()?;
        match preset.model_patch {
            ModelPatchId::ModelD(model_d_patch) => {
                Ok(toml::to_string_pretty(&VersionFiveModelDPreset {
                    schema_version: PRESET_SCHEMA_VERSION,
                    name: preset.name,
                    voices: preset.voices,
                    output_gain: preset.output_gain,
                    instrument_volume: preset.instrument_volume,
                    model: SynthesisModelId::ModelD,
                    model_d_patch,
                    macros: preset.macros,
                })?)
            }
            ModelPatchId::SixOpPm(six_op_patch) => {
                Ok(toml::to_string_pretty(&VersionFiveSixOpPreset {
                    schema_version: PRESET_SCHEMA_VERSION,
                    name: preset.name,
                    voices: preset.voices,
                    output_gain: preset.output_gain,
                    instrument_volume: preset.instrument_volume,
                    model: SynthesisModelId::SixOpPm,
                    six_op_patch,
                    macros: preset.macros.into(),
                })?)
            }
            ModelPatchId::StrangeOscillator(strange_patch) => {
                Ok(toml::to_string_pretty(&VersionSixStrangePreset {
                    schema_version: PRESET_SCHEMA_VERSION,
                    name: preset.name,
                    voices: preset.voices,
                    output_gain: preset.output_gain,
                    instrument_volume: preset.instrument_volume,
                    model: SynthesisModelId::StrangeOscillator,
                    strange_patch,
                    macros: preset.macros.into(),
                })?)
            }
            ModelPatchId::SwarmMachine(swarm_patch) => {
                Ok(toml::to_string_pretty(&VersionSevenSwarmPreset {
                    schema_version: PRESET_SCHEMA_VERSION,
                    name: preset.name,
                    voices: preset.voices,
                    output_gain: preset.output_gain,
                    instrument_volume: preset.instrument_volume,
                    model: SynthesisModelId::SwarmMachine,
                    swarm_patch,
                    macros: preset.macros.into(),
                })?)
            }
            ModelPatchId::BassMatrix(bass_matrix_patch) => {
                Ok(toml::to_string_pretty(&VersionSevenBassPreset {
                    schema_version: PRESET_SCHEMA_VERSION,
                    name: preset.name,
                    voices: preset.voices,
                    output_gain: preset.output_gain,
                    instrument_volume: preset.instrument_volume,
                    model: SynthesisModelId::BassMatrix,
                    bass_matrix_patch,
                    macros: preset.macros.into(),
                })?)
            }
            ModelPatchId::DualFilter(dual_filter_core) => {
                Ok(toml::to_string_pretty(&VersionEightDualFilterPreset {
                    schema_version: PRESET_SCHEMA_VERSION,
                    name: preset.name,
                    voices: preset.voices,
                    output_gain: preset.output_gain,
                    instrument_volume: preset.instrument_volume,
                    model: SynthesisModelId::DualFilter,
                    dual_filter_core,
                    controls: preset.macros.into(),
                })?)
            }
        }
    }
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct VersionFiveModelDPreset {
    schema_version: u32,
    name: String,
    voices: usize,
    output_gain: f32,
    #[serde(default = "full_volume")]
    instrument_volume: Normalized,
    model: SynthesisModelId,
    model_d_patch: ModelDPatchId,
    macros: MacroValues,
}

#[derive(Clone, Copy, Debug, PartialEq, Deserialize, Serialize)]
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
            control_13: midpoint(),
            control_14: midpoint(),
            control_15: midpoint(),
        }
    }
}

impl From<MacroValues> for SixOpMacroValues {
    fn from(values: MacroValues) -> Self {
        Self {
            index: values.evolve,
            ratio: values.shape,
            feedback: values.color,
            operator_decay: values.edge,
            balance: values.couple,
            key_scale: values.motion,
            velocity: values.depth,
            motion: values.space,
            attack: values.attack,
            decay: values.decay,
            sustain: values.sustain,
            release: values.release,
        }
    }
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct VersionFiveSixOpPreset {
    schema_version: u32,
    name: String,
    voices: usize,
    output_gain: f32,
    #[serde(default = "full_volume")]
    instrument_volume: Normalized,
    model: SynthesisModelId,
    six_op_patch: SixOpPatchId,
    macros: SixOpMacroValues,
}

#[derive(Clone, Copy, Debug, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct StrangeMacroValues {
    #[serde(rename = "type")]
    type_: Normalized,
    form: Normalized,
    warp: Normalized,
    couple: Normalized,
    motion: Normalized,
    chaos: Normalized,
    color: Normalized,
    space: Normalized,
    attack: Normalized,
    decay: Normalized,
    sustain: Normalized,
    release: Normalized,
}

impl From<StrangeMacroValues> for MacroValues {
    fn from(values: StrangeMacroValues) -> Self {
        Self {
            evolve: values.type_,
            shape: values.form,
            color: values.warp,
            edge: values.couple,
            couple: values.motion,
            motion: values.chaos,
            depth: values.color,
            space: values.space,
            attack: values.attack,
            decay: values.decay,
            sustain: values.sustain,
            release: values.release,
            control_13: midpoint(),
            control_14: midpoint(),
            control_15: midpoint(),
        }
    }
}

impl From<MacroValues> for StrangeMacroValues {
    fn from(values: MacroValues) -> Self {
        Self {
            type_: values.evolve,
            form: values.shape,
            warp: values.color,
            couple: values.edge,
            motion: values.couple,
            chaos: values.motion,
            color: values.depth,
            space: values.space,
            attack: values.attack,
            decay: values.decay,
            sustain: values.sustain,
            release: values.release,
        }
    }
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct VersionSixStrangePreset {
    schema_version: u32,
    name: String,
    voices: usize,
    output_gain: f32,
    #[serde(default = "full_volume")]
    instrument_volume: Normalized,
    model: SynthesisModelId,
    strange_patch: StrangePatchId,
    macros: StrangeMacroValues,
}

fn full_volume() -> Normalized {
    Normalized::new(1.0).expect("one is normalized")
}

fn midpoint() -> Normalized {
    Normalized::new(0.5).expect("one half is normalized")
}

#[derive(Clone, Copy, Debug, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct SwarmMacroValues {
    mass: Normalized,
    detune: Normalized,
    spread: Normalized,
    shape: Normalized,
    bite: Normalized,
    motion: Normalized,
    color: Normalized,
    space: Normalized,
    attack: Normalized,
    decay: Normalized,
    sustain: Normalized,
    release: Normalized,
}

impl From<SwarmMacroValues> for MacroValues {
    fn from(v: SwarmMacroValues) -> Self {
        Self {
            evolve: v.mass,
            shape: v.detune,
            color: v.spread,
            edge: v.shape,
            couple: v.bite,
            motion: v.motion,
            depth: v.color,
            space: v.space,
            attack: v.attack,
            decay: v.decay,
            sustain: v.sustain,
            release: v.release,
            control_13: midpoint(),
            control_14: midpoint(),
            control_15: midpoint(),
        }
    }
}

impl From<MacroValues> for SwarmMacroValues {
    fn from(v: MacroValues) -> Self {
        Self {
            mass: v.evolve,
            detune: v.shape,
            spread: v.color,
            shape: v.edge,
            bite: v.couple,
            motion: v.motion,
            color: v.depth,
            space: v.space,
            attack: v.attack,
            decay: v.decay,
            sustain: v.sustain,
            release: v.release,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct BassMacroValues {
    body: Normalized,
    growl: Normalized,
    metal: Normalized,
    punch: Normalized,
    character: Normalized,
    drive: Normalized,
    filter: Normalized,
    unstable: Normalized,
    attack: Normalized,
    decay: Normalized,
    sustain: Normalized,
    release: Normalized,
}

impl From<BassMacroValues> for MacroValues {
    fn from(v: BassMacroValues) -> Self {
        Self {
            evolve: v.body,
            shape: v.growl,
            color: v.metal,
            edge: v.punch,
            couple: v.character,
            motion: v.drive,
            depth: v.filter,
            space: v.unstable,
            attack: v.attack,
            decay: v.decay,
            sustain: v.sustain,
            release: v.release,
            control_13: midpoint(),
            control_14: midpoint(),
            control_15: midpoint(),
        }
    }
}

impl From<MacroValues> for BassMacroValues {
    fn from(v: MacroValues) -> Self {
        Self {
            body: v.evolve,
            growl: v.shape,
            metal: v.color,
            punch: v.edge,
            character: v.couple,
            drive: v.motion,
            filter: v.depth,
            unstable: v.space,
            attack: v.attack,
            decay: v.decay,
            sustain: v.sustain,
            release: v.release,
        }
    }
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct VersionSevenSwarmPreset {
    schema_version: u32,
    name: String,
    voices: usize,
    output_gain: f32,
    instrument_volume: Normalized,
    model: SynthesisModelId,
    swarm_patch: SwarmPatchId,
    macros: SwarmMacroValues,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct VersionSevenBassPreset {
    schema_version: u32,
    name: String,
    voices: usize,
    output_gain: f32,
    instrument_volume: Normalized,
    model: SynthesisModelId,
    bass_matrix_patch: BassMatrixPatchId,
    macros: BassMacroValues,
}

#[derive(Clone, Copy, Debug, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct DualFilterControlValues {
    filter_a_cutoff: Normalized,
    filter_a_resonance: Normalized,
    filter_a_envelope_depth: Normalized,
    filter_b_cutoff: Normalized,
    filter_b_resonance: Normalized,
    filter_b_envelope_depth: Normalized,
    structure: Normalized,
    filter_attack: Normalized,
    filter_decay: Normalized,
    filter_sustain: Normalized,
    filter_release: Normalized,
    amp_attack: Normalized,
    amp_decay: Normalized,
    amp_sustain: Normalized,
    amp_release: Normalized,
}

impl From<DualFilterControlValues> for MacroValues {
    fn from(v: DualFilterControlValues) -> Self {
        Self {
            evolve: v.filter_a_cutoff,
            shape: v.filter_a_resonance,
            color: v.filter_a_envelope_depth,
            edge: v.filter_b_cutoff,
            couple: v.filter_b_resonance,
            motion: v.filter_b_envelope_depth,
            depth: v.structure,
            space: v.filter_attack,
            attack: v.filter_decay,
            decay: v.filter_sustain,
            sustain: v.filter_release,
            release: v.amp_attack,
            control_13: v.amp_decay,
            control_14: v.amp_sustain,
            control_15: v.amp_release,
        }
    }
}

impl From<MacroValues> for DualFilterControlValues {
    fn from(v: MacroValues) -> Self {
        Self {
            filter_a_cutoff: v.evolve,
            filter_a_resonance: v.shape,
            filter_a_envelope_depth: v.color,
            filter_b_cutoff: v.edge,
            filter_b_resonance: v.couple,
            filter_b_envelope_depth: v.motion,
            structure: v.depth,
            filter_attack: v.space,
            filter_decay: v.attack,
            filter_sustain: v.decay,
            filter_release: v.sustain,
            amp_attack: v.release,
            amp_decay: v.control_13,
            amp_sustain: v.control_14,
            amp_release: v.control_15,
        }
    }
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct VersionEightDualFilterPreset {
    schema_version: u32,
    name: String,
    voices: usize,
    output_gain: f32,
    instrument_volume: Normalized,
    model: SynthesisModelId,
    dual_filter_core: DualFilterPatchId,
    controls: DualFilterControlValues,
}

struct VersionEightPreset;

impl VersionEightPreset {
    fn parse(source: &str) -> Result<Preset, PresetError> {
        let value: toml::Value = toml::from_str(source)?;
        match value
            .get("model")
            .and_then(toml::Value::as_str)
            .ok_or(PresetError::InvalidModel)?
        {
            "dual_filter" => {
                let p: VersionEightDualFilterPreset = toml::from_str(source)?;
                if p.schema_version != PRESET_SCHEMA_VERSION
                    || p.model != SynthesisModelId::DualFilter
                {
                    return Err(PresetError::InvalidModel);
                }
                Ok(Preset {
                    schema_version: PRESET_SCHEMA_VERSION,
                    name: p.name,
                    voices: p.voices,
                    output_gain: p.output_gain,
                    instrument_volume: p.instrument_volume,
                    model: p.model,
                    model_patch: ModelPatchId::DualFilter(p.dual_filter_core),
                    macros: p.controls.into(),
                })
            }
            "swarm_machine" => {
                let p: VersionSevenSwarmPreset = toml::from_str(source)?;
                if p.schema_version != PRESET_SCHEMA_VERSION
                    || p.model != SynthesisModelId::SwarmMachine
                {
                    return Err(PresetError::InvalidModel);
                }
                Ok(Preset {
                    schema_version: PRESET_SCHEMA_VERSION,
                    name: p.name,
                    voices: p.voices,
                    output_gain: p.output_gain,
                    instrument_volume: p.instrument_volume,
                    model: p.model,
                    model_patch: ModelPatchId::SwarmMachine(p.swarm_patch),
                    macros: p.macros.into(),
                })
            }
            "bass_matrix" => {
                let p: VersionSevenBassPreset = toml::from_str(source)?;
                if p.schema_version != PRESET_SCHEMA_VERSION
                    || p.model != SynthesisModelId::BassMatrix
                {
                    return Err(PresetError::InvalidModel);
                }
                Ok(Preset {
                    schema_version: PRESET_SCHEMA_VERSION,
                    name: p.name,
                    voices: p.voices,
                    output_gain: p.output_gain,
                    instrument_volume: p.instrument_volume,
                    model: p.model,
                    model_patch: ModelPatchId::BassMatrix(p.bass_matrix_patch),
                    macros: p.macros.into(),
                })
            }
            _ => VersionSixPreset::parse_for(source, PRESET_SCHEMA_VERSION),
        }
    }
}

struct VersionSevenPreset;

impl VersionSevenPreset {
    fn parse(source: &str) -> Result<Preset, PresetError> {
        let value: toml::Value = toml::from_str(source)?;
        match value
            .get("model")
            .and_then(toml::Value::as_str)
            .ok_or(PresetError::InvalidModel)?
        {
            "swarm_machine" => {
                let p: VersionSevenSwarmPreset = toml::from_str(source)?;
                if p.schema_version != 7 || p.model != SynthesisModelId::SwarmMachine {
                    return Err(PresetError::InvalidModel);
                }
                Ok(Preset {
                    schema_version: PRESET_SCHEMA_VERSION,
                    name: p.name,
                    voices: p.voices,
                    output_gain: p.output_gain,
                    instrument_volume: p.instrument_volume,
                    model: p.model,
                    model_patch: ModelPatchId::SwarmMachine(p.swarm_patch),
                    macros: p.macros.into(),
                })
            }
            "bass_matrix" => {
                let p: VersionSevenBassPreset = toml::from_str(source)?;
                if p.schema_version != 7 || p.model != SynthesisModelId::BassMatrix {
                    return Err(PresetError::InvalidModel);
                }
                Ok(Preset {
                    schema_version: PRESET_SCHEMA_VERSION,
                    name: p.name,
                    voices: p.voices,
                    output_gain: p.output_gain,
                    instrument_volume: p.instrument_volume,
                    model: p.model,
                    model_patch: ModelPatchId::BassMatrix(p.bass_matrix_patch),
                    macros: p.macros.into(),
                })
            }
            _ => VersionSixPreset::parse_for(source, 7),
        }
    }
}

struct VersionSixPreset;

impl VersionSixPreset {
    fn parse(source: &str) -> Result<Preset, PresetError> {
        Self::parse_for(source, 6)
    }

    fn parse_for(source: &str, expected_schema: u32) -> Result<Preset, PresetError> {
        let value: toml::Value = toml::from_str(source)?;
        let model = value
            .get("model")
            .and_then(toml::Value::as_str)
            .ok_or(PresetError::InvalidModel)?;
        match model {
            "model_d" => {
                let preset: VersionFiveModelDPreset = toml::from_str(source)?;
                if preset.schema_version != expected_schema
                    || preset.model != SynthesisModelId::ModelD
                {
                    return Err(PresetError::UnsupportedVersion(preset.schema_version));
                }
                Ok(Preset {
                    schema_version: PRESET_SCHEMA_VERSION,
                    name: preset.name,
                    voices: preset.voices,
                    output_gain: preset.output_gain,
                    instrument_volume: preset.instrument_volume,
                    model: preset.model,
                    model_patch: ModelPatchId::ModelD(preset.model_d_patch),
                    macros: preset.macros,
                })
            }
            "six_op_pm" => {
                let preset: VersionFiveSixOpPreset = toml::from_str(source)?;
                if preset.schema_version != expected_schema
                    || preset.model != SynthesisModelId::SixOpPm
                {
                    return Err(PresetError::UnsupportedVersion(preset.schema_version));
                }
                Ok(Preset {
                    schema_version: PRESET_SCHEMA_VERSION,
                    name: preset.name,
                    voices: preset.voices,
                    output_gain: preset.output_gain,
                    instrument_volume: preset.instrument_volume,
                    model: preset.model,
                    model_patch: ModelPatchId::SixOpPm(preset.six_op_patch),
                    macros: preset.macros.into(),
                })
            }
            "strange_oscillator" => {
                let preset: VersionSixStrangePreset = toml::from_str(source)?;
                if preset.schema_version != expected_schema
                    || preset.model != SynthesisModelId::StrangeOscillator
                {
                    return Err(PresetError::UnsupportedVersion(preset.schema_version));
                }
                Ok(Preset {
                    schema_version: PRESET_SCHEMA_VERSION,
                    name: preset.name,
                    voices: preset.voices,
                    output_gain: preset.output_gain,
                    instrument_volume: preset.instrument_volume,
                    model: preset.model,
                    model_patch: ModelPatchId::StrangeOscillator(preset.strange_patch),
                    macros: preset.macros.into(),
                })
            }
            _ => Err(PresetError::InvalidModel),
        }
    }
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
                if preset.schema_version != 5 || preset.model != SynthesisModelId::ModelD {
                    return Err(PresetError::UnsupportedVersion(preset.schema_version));
                }
                Ok(Preset {
                    schema_version: PRESET_SCHEMA_VERSION,
                    name: preset.name,
                    voices: preset.voices,
                    output_gain: preset.output_gain,
                    instrument_volume: full_volume(),
                    model: preset.model,
                    model_patch: ModelPatchId::ModelD(preset.model_d_patch),
                    macros: preset.macros,
                })
            }
            "six_op_pm" => {
                let preset: VersionFiveSixOpPreset = toml::from_str(source)?;
                if preset.schema_version != 5 || preset.model != SynthesisModelId::SixOpPm {
                    return Err(PresetError::UnsupportedVersion(preset.schema_version));
                }
                Ok(Preset {
                    schema_version: PRESET_SCHEMA_VERSION,
                    name: preset.name,
                    voices: preset.voices,
                    output_gain: preset.output_gain,
                    instrument_volume: full_volume(),
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
            instrument_volume: full_volume(),
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
            instrument_volume: full_volume(),
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
            instrument_volume: full_volume(),
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
            instrument_volume: full_volume(),
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
                control_13: midpoint(),
                control_14: midpoint(),
                control_15: midpoint(),
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
        assert_eq!(preset.schema_version, PRESET_SCHEMA_VERSION);
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
        assert_eq!(preset.schema_version, PRESET_SCHEMA_VERSION);
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
        assert_eq!(preset.schema_version, PRESET_SCHEMA_VERSION);
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
        assert_eq!(preset.schema_version, PRESET_SCHEMA_VERSION);
        assert_eq!(preset.model, SynthesisModelId::ModelD);
        assert_eq!(
            preset.model_patch,
            ModelPatchId::ModelD(ModelDPatchId::Bass)
        );
        assert!(Preset::parse(&format!("{version_three}\nunknown = 3")).is_err());
    }

    #[test]
    fn factory_presets_are_strict_and_cover_all_models() {
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
            include_str!("../presets/14-strange-oscillator.mojsint"),
            include_str!("../presets/15-swarm-warm-pad.mojsint"),
            include_str!("../presets/16-bass-matrix.mojsint"),
            include_str!("../presets/17-dual-filter-industrial-lead.mojsint"),
            include_str!("../presets/18-dual-filter-serial-bass.mojsint"),
            include_str!("../presets/19-dual-filter-counter-growl.mojsint"),
            include_str!("../presets/20-dual-filter-envelope-punch.mojsint"),
            include_str!("../presets/21-dual-filter-topology-motion.mojsint"),
        ];
        let presets = sources.map(|source| Preset::parse(source).unwrap());
        let mut names = presets
            .iter()
            .map(|preset| preset.name.as_str())
            .collect::<Vec<_>>();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), 21);
        assert!(
            presets[..7]
                .iter()
                .all(|preset| preset.model == SynthesisModelId::ModelD)
        );
        assert!(
            presets[7..13]
                .iter()
                .all(|preset| preset.model == SynthesisModelId::SixOpPm)
        );
        assert_eq!(presets[13].model, SynthesisModelId::StrangeOscillator);
        assert_eq!(presets[14].model, SynthesisModelId::SwarmMachine);
        assert_eq!(presets[15].model, SynthesisModelId::BassMatrix);
        assert!(
            presets[16..]
                .iter()
                .all(|preset| preset.model == SynthesisModelId::DualFilter)
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
        for (preset, patch) in presets[7..13].iter().zip(SixOpPatchId::ALL) {
            assert_eq!(preset.model_patch, ModelPatchId::SixOpPm(patch));
        }
        assert_eq!(
            presets[13].model_patch,
            ModelPatchId::StrangeOscillator(StrangePatchId::Unified)
        );
        assert_eq!(
            presets[14].model_patch,
            ModelPatchId::SwarmMachine(SwarmPatchId::WarmPad)
        );
        assert_eq!(
            presets[15].model_patch,
            ModelPatchId::BassMatrix(BassMatrixPatchId::Transformer)
        );
        assert_eq!(
            presets[18].model_patch,
            ModelPatchId::DualFilter(DualFilterPatchId::Counter)
        );
        assert!(
            presets
                .iter()
                .all(|preset| preset.instrument_volume == Normalized::new(1.0).unwrap())
        );
    }

    #[test]
    fn parses_strict_version_five_six_op_preset_and_rejects_mixed_fields() {
        let valid = include_str!("../presets/08-six-op-bell-metal.mojsint");
        let preset = Preset::parse(valid).unwrap();
        assert_eq!(preset.schema_version, PRESET_SCHEMA_VERSION);
        assert_eq!(preset.model, SynthesisModelId::SixOpPm);
        assert_eq!(
            preset.model_patch,
            ModelPatchId::SixOpPm(SixOpPatchId::BellMetal)
        );
        assert_eq!(preset.macros.get(MacroId::Evolve).get(), 0.5);
        assert!(Preset::parse(&format!("{valid}\nmodel_d_patch = \"bass\"")).is_err());
        assert!(Preset::parse(&valid.replace("six_op_patch", "model_d_patch")).is_err());
    }

    #[test]
    fn serializes_model_d_as_strict_current_schema_and_round_trips() {
        let preset = Preset::parse(VALID).unwrap();
        let encoded = preset.to_toml().unwrap();

        assert!(encoded.contains("schema_version = 8"));
        assert!(encoded.contains("model = \"model_d\""));
        assert!(encoded.contains("model_d_patch = \"bass\""));
        assert!(!encoded.contains("six_op_patch"));
        assert_eq!(Preset::parse(&encoded).unwrap(), preset);
    }

    #[test]
    fn serializes_six_op_with_its_exact_current_macro_names() {
        let preset =
            Preset::parse(include_str!("../presets/08-six-op-bell-metal.mojsint")).unwrap();
        let encoded = preset.to_toml().unwrap();

        assert!(encoded.contains("schema_version = 8"));
        assert!(encoded.contains("model = \"six_op_pm\""));
        assert!(encoded.contains("six_op_patch = \"bell_metal\""));
        for field in [
            "index",
            "ratio",
            "feedback",
            "operator_decay",
            "balance",
            "key_scale",
            "velocity",
            "motion",
            "attack",
            "decay",
            "sustain",
            "release",
        ] {
            assert!(encoded.contains(&format!("{field} =")), "missing {field}");
        }
        assert!(!encoded.contains("evolve ="));
        assert!(!encoded.contains("model_d_patch"));
        assert_eq!(Preset::parse(&encoded).unwrap(), preset);
    }

    #[test]
    fn new_models_round_trip_with_exact_schema_seven_identity() {
        for (source, model, patch_field) in [
            (
                include_str!("../presets/15-swarm-warm-pad.mojsint"),
                SynthesisModelId::SwarmMachine,
                "swarm_patch = \"warm_pad\"",
            ),
            (
                include_str!("../presets/16-bass-matrix.mojsint"),
                SynthesisModelId::BassMatrix,
                "bass_matrix_patch = \"transformer\"",
            ),
        ] {
            let preset = Preset::parse(source).unwrap();
            assert_eq!(preset.model, model);
            let encoded = preset.to_toml().unwrap();
            assert!(encoded.contains("schema_version = 8"));
            assert!(encoded.contains("instrument_volume = 1.0"));
            assert!(encoded.contains(patch_field));
            assert_eq!(Preset::parse(&encoded).unwrap(), preset);
        }
    }

    #[test]
    fn schema_six_presets_migrate_at_unity_volume_without_changing_macros() {
        let source = include_str!("../presets/14-strange-oscillator.mojsint");
        let migrated = Preset::parse(source).unwrap();
        assert_eq!(migrated.schema_version, PRESET_SCHEMA_VERSION);
        assert_eq!(migrated.instrument_volume, Normalized::new(1.0).unwrap());
        assert_eq!(migrated.macros.get(MacroId::Couple).get(), 0.5);
        assert_eq!(migrated.macros.get(MacroId::Motion).get(), 0.5);
    }

    #[test]
    fn parses_and_round_trips_strange_oscillator_with_exact_macro_names() {
        let preset =
            Preset::parse(include_str!("../presets/14-strange-oscillator.mojsint")).unwrap();
        assert_eq!(preset.model, SynthesisModelId::StrangeOscillator);
        assert_eq!(
            preset.model_patch,
            ModelPatchId::StrangeOscillator(StrangePatchId::Unified)
        );
        let encoded = preset.to_toml().unwrap();
        for field in [
            "type", "form", "warp", "couple", "motion", "chaos", "color", "space", "attack",
            "decay", "sustain", "release",
        ] {
            assert!(encoded.contains(&format!("{field} =")), "missing {field}");
        }
        assert!(!encoded.contains("evolve ="));
        assert!(!encoded.contains("six_op_patch"));
        assert_eq!(Preset::parse(&encoded).unwrap(), preset);
    }

    #[test]
    fn serialization_rejects_a_model_patch_mismatch() {
        let mut preset = Preset::parse(VALID).unwrap();
        preset.model = SynthesisModelId::SixOpPm;

        assert!(matches!(preset.to_toml(), Err(PresetError::InvalidPatch)));
    }

    #[test]
    fn dual_filter_round_trips_all_controls_and_persisted_core() {
        let preset = Preset::parse(include_str!(
            "../presets/19-dual-filter-counter-growl.mojsint"
        ))
        .unwrap();
        assert_eq!(preset.model, SynthesisModelId::DualFilter);
        assert_eq!(
            preset.model_patch,
            ModelPatchId::DualFilter(DualFilterPatchId::Counter)
        );
        assert_eq!(preset.macros.get(MacroId::Control15).get(), 0.26);
        let encoded = preset.to_toml().unwrap();
        assert!(encoded.contains("schema_version = 8"));
        assert!(encoded.contains("dual_filter_core = \"counter\""));
        assert!(encoded.contains("amp_release = 0.26"));
        assert_eq!(Preset::parse(&encoded).unwrap(), preset);
    }
}
