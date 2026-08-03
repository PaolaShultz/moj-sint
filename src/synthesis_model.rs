use crate::engine::EngineError;
use crate::model_d::voice::{ModelDDiagnostics, ModelDPatch, ModelDVoice};
use crate::preset::{ModelDPatchId, ModelPatchId, SynthesisModelId};
use crate::six_op_pm::live::LiveSixOpVoice;

#[derive(Debug)]
// Both voice families stay fully preallocated so note creation and the audio
// callback never allocate. Boxing the larger family would weaken that owned
// real-time storage contract merely to make the enum smaller.
#[allow(clippy::large_enum_variant)]
pub(crate) enum VoiceModel {
    ModelD(ModelDVoice),
    SixOpPm(LiveSixOpVoice),
}

impl VoiceModel {
    pub(crate) fn new(
        sample_rate: f32,
        model_id: SynthesisModelId,
        patch_id: ModelPatchId,
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
        }
    }

    pub(crate) fn note_off(&mut self) {
        match self {
            Self::ModelD(model) => model.note_off(),
            Self::SixOpPm(model) => model.note_off(),
        }
    }

    #[inline]
    pub(crate) fn sample(&mut self) -> f32 {
        match self {
            Self::ModelD(model) => model.sample(),
            Self::SixOpPm(model) => model.sample(),
        }
    }

    pub(crate) fn reset(&mut self) {
        match self {
            Self::ModelD(model) => model.reset(),
            Self::SixOpPm(model) => model.reset(),
        }
    }
}
