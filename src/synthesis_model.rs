use crate::engine::EngineError;
use crate::model_d::voice::{ModelDDiagnostics, ModelDPatch, ModelDVoice};
use crate::preset::{ModelDPatchId, SynthesisModelId};

#[derive(Debug)]
pub(crate) enum VoiceModel {
    ModelD(ModelDVoice),
}

impl VoiceModel {
    pub(crate) fn new(
        sample_rate: f32,
        model_id: SynthesisModelId,
        patch_id: ModelDPatchId,
    ) -> Result<Self, EngineError> {
        match model_id {
            SynthesisModelId::ModelD => {
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
        }
    }

    #[inline]
    pub(crate) fn set_live_controls(&mut self, values: [f32; 8]) {
        match self {
            Self::ModelD(model) => model.set_live_controls(
                values[0], values[1], values[2], values[3], values[4], values[5], values[6],
                values[7],
            ),
        }
    }

    pub(crate) fn note_on(&mut self, note: u8, velocity: f32) {
        match self {
            Self::ModelD(model) => model.note_on(note, velocity),
        }
    }

    #[inline]
    pub(crate) fn sample(&mut self) -> f32 {
        match self {
            Self::ModelD(model) => model.sample(),
        }
    }

    pub(crate) fn reset(&mut self) {
        match self {
            Self::ModelD(model) => model.reset(),
        }
    }
}
