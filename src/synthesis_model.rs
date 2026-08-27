use crate::bass_matrix::{BassMatrixControls, BassMatrixVoice};
use crate::dual_filter::{DualFilterCore, DualFilterInstrument};
use crate::dual_filter_concept::ConceptControls;
use crate::engine::EngineError;
use crate::micro_machine::{CompiledMicroMachine, MicroMachineGraph, SwarmControls};
use crate::model_d::voice::{ModelDDiagnostics, ModelDPatch, ModelDVoice};
use crate::preset::{DualFilterPatchId, ModelDPatchId, ModelPatchId, SynthesisModelId};
use crate::six_op_pm::live::LiveSixOpVoice;
use crate::strange::{StrangeControls, StrangeInstrument};

const SWARM_GRAPH: &str = include_str!("../experiments/swarm-micro-machine-v1.toml");

#[derive(Debug)]
pub(crate) struct LiveSwarmVoice {
    machine: CompiledMicroMachine,
    velocity: f32,
}

impl LiveSwarmVoice {
    fn new(sample_rate: f32) -> Result<Self, EngineError> {
        let graph =
            MicroMachineGraph::parse(SWARM_GRAPH).map_err(|_| EngineError::InvalidModelPatch)?;
        let machine = graph
            .compile(sample_rate, 440.0, SwarmControls::PAD)
            .map_err(|_| EngineError::InvalidSampleRate)?;
        Ok(Self {
            machine,
            velocity: 0.0,
        })
    }

    fn set_live_controls(&mut self, values: [f32; 8]) {
        let controls = SwarmControls {
            mass: values[0],
            detune: values[1],
            spread: values[2],
            shape: values[3],
            bite: values[4],
            motion: values[5],
            color: values[6],
            space: values[7],
        };
        if self.machine.set_controls(controls).is_err() {
            self.reset();
        }
    }

    fn note_on(&mut self, note: u8, velocity: f32) {
        let frequency = 440.0 * 2.0_f32.powf((f32::from(note) - 69.0) / 12.0);
        if self.machine.set_frequency(frequency).is_err() {
            self.reset();
            return;
        }
        self.machine.reset();
        self.velocity = velocity.clamp(0.0, 1.0);
    }

    #[inline]
    fn sample(&mut self) -> [f32; 2] {
        self.machine.sample().map(|sample| sample * self.velocity)
    }

    fn reset(&mut self) {
        self.machine.reset();
        self.velocity = 0.0;
    }
}

#[derive(Debug)]
pub(crate) struct LiveStrangeVoice {
    sample_rate: f32,
    seed: u32,
    controls: [f32; 8],
    velocity: f32,
    instrument: StrangeInstrument,
}

impl LiveStrangeVoice {
    fn new(sample_rate: f32, seed: u32) -> Result<Self, EngineError> {
        let controls = [0.142_857_15, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5];
        let instrument = StrangeInstrument::new(
            sample_rate,
            440.0,
            seed,
            controls[0],
            strange_controls(controls),
        )
        .map_err(|_| EngineError::InvalidSampleRate)?;
        Ok(Self {
            sample_rate,
            seed,
            controls,
            velocity: 0.0,
            instrument,
        })
    }

    fn set_live_controls(&mut self, controls: [f32; 8]) {
        self.controls = controls.map(|value| value.clamp(0.0, 1.0));
        self.instrument
            .set_controls(strange_controls(self.controls));
        if self
            .instrument
            .set_type_normalized(self.controls[0])
            .is_err()
        {
            self.instrument.reset();
        }
    }

    fn note_on(&mut self, note: u8, velocity: f32) {
        let frequency = 440.0 * 2.0_f32.powf((f32::from(note) - 69.0) / 12.0);
        match StrangeInstrument::new(
            self.sample_rate,
            frequency,
            self.seed ^ u32::from(note).wrapping_mul(0x9e37_79b9),
            self.controls[0],
            strange_controls(self.controls),
        ) {
            Ok(instrument) => {
                self.instrument = instrument;
                self.velocity = velocity.clamp(0.0, 1.0);
            }
            Err(_) => self.reset(),
        }
    }

    fn sample(&mut self) -> [f32; 2] {
        let frame = self.instrument.sample();
        [frame.left * self.velocity, frame.right * self.velocity]
    }

    fn reset(&mut self) {
        self.instrument.reset();
        self.velocity = 0.0;
    }
}

fn strange_controls(values: [f32; 8]) -> StrangeControls {
    StrangeControls {
        form: values[1],
        warp: values[2],
        couple: values[3],
        motion: values[4],
        chaos: values[5],
        color: values[6],
        space: values[7],
    }
}

#[derive(Debug)]
// Both voice families stay fully preallocated so note creation and the audio
// callback never allocate. Boxing the larger family would weaken that owned
// real-time storage contract merely to make the enum smaller.
#[allow(clippy::large_enum_variant)]
pub(crate) enum VoiceModel {
    ModelD(ModelDVoice),
    SixOpPm(LiveSixOpVoice),
    StrangeOscillator(LiveStrangeVoice),
    SwarmMachine(LiveSwarmVoice),
    BassMatrix(BassMatrixVoice),
    DualFilter(DualFilterInstrument),
}

impl VoiceModel {
    pub(crate) fn new(
        sample_rate: f32,
        model_id: SynthesisModelId,
        patch_id: ModelPatchId,
        voice_seed: u32,
    ) -> Result<Self, EngineError> {
        match (model_id, patch_id) {
            (SynthesisModelId::ModelD, ModelPatchId::ModelD(patch_id)) => {
                let mut patch = match patch_id {
                    ModelDPatchId::Bass => ModelDPatch::bass(),
                    ModelDPatchId::Lead => ModelDPatch::lead(),
                    ModelDPatchId::FilterArticulation => ModelDPatch::filter_articulation(),
                };
                patch.loudness_contour.attack_seconds = 0.001;
                patch.loudness_contour.decay_seconds = 0.001;
                patch.loudness_contour.sustain_level = 1.0;
                patch.output_gain = 1.0;
                ModelDVoice::new(sample_rate, patch, ModelDDiagnostics::full())
                    .map(Self::ModelD)
                    .map_err(|_| EngineError::InvalidSampleRate)
            }
            (SynthesisModelId::SixOpPm, ModelPatchId::SixOpPm(patch_id)) => {
                LiveSixOpVoice::new(sample_rate, patch_id)
                    .map(Self::SixOpPm)
                    .map_err(|_| EngineError::InvalidSampleRate)
            }
            (SynthesisModelId::StrangeOscillator, ModelPatchId::StrangeOscillator(_)) => {
                LiveStrangeVoice::new(sample_rate, voice_seed).map(Self::StrangeOscillator)
            }
            (SynthesisModelId::SwarmMachine, ModelPatchId::SwarmMachine(_)) => {
                LiveSwarmVoice::new(sample_rate).map(Self::SwarmMachine)
            }
            (SynthesisModelId::BassMatrix, ModelPatchId::BassMatrix(_)) => {
                BassMatrixVoice::new(sample_rate, voice_seed)
                    .map(Self::BassMatrix)
                    .ok_or(EngineError::InvalidSampleRate)
            }
            (SynthesisModelId::DualFilter, ModelPatchId::DualFilter(core)) => {
                let core = match core {
                    DualFilterPatchId::Industrial => DualFilterCore::Industrial,
                    DualFilterPatchId::Counter => DualFilterCore::Counter,
                };
                DualFilterInstrument::new(sample_rate, ConceptControls::MIDPOINT, core)
                    .map(Self::DualFilter)
                    .map_err(|_| EngineError::InvalidSampleRate)
            }
            _ => Err(EngineError::InvalidModelPatch),
        }
    }

    #[inline]
    pub(crate) fn set_live_controls(&mut self, values: [f32; 15]) {
        let timbral = [
            values[0], values[1], values[2], values[3], values[4], values[5], values[6], values[7],
        ];
        match self {
            Self::ModelD(model) => model.set_live_controls(
                values[0], values[1], values[2], values[3], values[4], values[5], values[6],
                values[7],
            ),
            Self::SixOpPm(model) => model.set_live_controls(timbral),
            Self::StrangeOscillator(model) => model.set_live_controls(timbral),
            Self::SwarmMachine(model) => model.set_live_controls(timbral),
            Self::BassMatrix(model) => {
                model.set_controls(BassMatrixControls::from_macro_values(timbral));
            }
            Self::DualFilter(model) => model.set_controls(ConceptControls::clamped(values)),
        }
    }

    pub(crate) fn note_on(&mut self, note: u8, velocity: f32) {
        match self {
            Self::ModelD(model) => model.note_on(note, velocity),
            Self::SixOpPm(model) => {
                if model.note_on(note, velocity).is_err() {
                    model.reset();
                }
            }
            Self::StrangeOscillator(model) => model.note_on(note, velocity),
            Self::SwarmMachine(model) => model.note_on(note, velocity),
            Self::BassMatrix(model) => model.note_on(note, velocity),
            Self::DualFilter(model) => model.note_on(note, velocity),
        }
    }

    pub(crate) fn note_off(&mut self) {
        match self {
            Self::ModelD(model) => model.note_off(),
            Self::SixOpPm(model) => model.note_off(),
            Self::StrangeOscillator(_) => {}
            Self::SwarmMachine(_) | Self::BassMatrix(_) => {}
            Self::DualFilter(model) => model.note_off(),
        }
    }

    #[inline]
    pub(crate) fn sample(&mut self) -> [f32; 2] {
        match self {
            Self::ModelD(model) => {
                let sample = model.sample();
                [sample, sample]
            }
            Self::SixOpPm(model) => {
                let sample = model.sample();
                [sample, sample]
            }
            Self::StrangeOscillator(model) => model.sample(),
            Self::SwarmMachine(model) => model.sample(),
            Self::BassMatrix(model) => model.sample(),
            Self::DualFilter(model) => {
                let sample = model.sample();
                [sample, sample]
            }
        }
    }

    pub(crate) fn reset(&mut self) {
        match self {
            Self::ModelD(model) => model.reset(),
            Self::SixOpPm(model) => model.reset(),
            Self::StrangeOscillator(model) => model.reset(),
            Self::SwarmMachine(model) => model.reset(),
            Self::BassMatrix(model) => model.reset(),
            Self::DualFilter(model) => model.reset(),
        }
    }

    pub(crate) fn is_idle(&self) -> bool {
        match self {
            Self::DualFilter(model) => model.is_idle(),
            _ => false,
        }
    }

    pub(crate) fn set_dual_filter_core(&mut self, core: DualFilterCore) {
        if let Self::DualFilter(model) = self {
            model.set_core(core);
        }
    }

    pub(crate) fn toggle_dual_filter_core(&mut self) {
        if let Self::DualFilter(model) = self {
            model.toggle_core();
        }
    }
}
