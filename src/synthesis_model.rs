use crate::engine::EngineError;
use crate::model_d::voice::{ModelDDiagnostics, ModelDPatch, ModelDVoice};
use crate::preset::{ModelDPatchId, ModelPatchId, SynthesisModelId};
use crate::six_op_pm::live::LiveSixOpVoice;
use crate::strange::{StrangeControls, StrangeInstrument};

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
            _ => Err(EngineError::InvalidModelPatch),
        }
    }

    #[inline]
    pub(crate) fn set_live_controls(&mut self, values: [f32; 8]) {
        match self {
            Self::ModelD(model) => model.set_live_controls(
                values[0], values[1], values[2], values[3], values[4], values[5], values[6],
                values[7],
            ),
            Self::SixOpPm(model) => model.set_live_controls(values),
            Self::StrangeOscillator(model) => model.set_live_controls(values),
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
        }
    }

    pub(crate) fn note_off(&mut self) {
        match self {
            Self::ModelD(model) => model.note_off(),
            Self::SixOpPm(model) => model.note_off(),
            Self::StrangeOscillator(_) => {}
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
        }
    }

    pub(crate) fn reset(&mut self) {
        match self {
            Self::ModelD(model) => model.reset(),
            Self::SixOpPm(model) => model.reset(),
            Self::StrangeOscillator(model) => model.reset(),
        }
    }
}
