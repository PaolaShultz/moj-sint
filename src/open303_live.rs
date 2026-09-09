//! Monophonic Open303 adapter. All storage/table preparation occurs in new.
use crate::{
    engine::EngineError,
    open303::{Controls, FilterMode, Open303},
    preset::Open303PatchId,
};

#[derive(Debug)]
pub(crate) struct LiveOpen303 {
    core: Open303,
    values: [f32; 15],
    until_control: u8,
}
impl LiveOpen303 {
    pub fn new(rate: f32, patch: Open303PatchId) -> Result<Self, EngineError> {
        let mode = match patch {
            Open303PatchId::Tb303 => FilterMode::Tb303,
            Open303PatchId::Lowpass18 => FilterMode::Lowpass18,
        };
        Ok(Self {
            core: Open303::new(f64::from(rate), mode, Controls::default())
                .map_err(|_| EngineError::InvalidSampleRate)?,
            values: [0.5; 15],
            until_control: 0,
        })
    }
    pub fn set_controls(&mut self, values: [f32; 15]) {
        self.values = values;
        if self.until_control == 0 {
            self.apply_controls();
            self.until_control = 32;
        }
    }
    fn apply_controls(&mut self) {
        let v = self.values.map(f64::from);
        let controls = Controls {
            waveform: v[0],
            cutoff_hz: 314.0 + 2080.0 * v[1],
            resonance: 100.0 * v[2],
            env_mod: 100.0 * v[3],
            decay_ms: 200.0 + 1800.0 * v[5],
            accent: 100.0 * v[6],
            slide_ms: 300.0 * v[7],
            volume_db: -18.0,
            tuning_hz: 440.0,
        };
        if self.core.set_controls(controls).is_err()
            || self
                .core
                .set_articulation(
                    0.3 + 29.7 * v[8],
                    0.3 + 29.7 * v[9],
                    30.0 + 2970.0 * v[10],
                    16.0 + 2984.0 * v[11],
                )
                .is_err()
        {
            self.core.reset();
        }
    }
    pub fn note_on(&mut self, note: u8, velocity: f32) {
        self.apply_controls();
        // Keep zero-velocity handling in Engine; positive notes remain at least velocity 1.
        let vel = (velocity.clamp(0.0, 1.0) * 127.0).round().clamp(1.0, 127.0) as u8;
        if self.core.note_on(note, vel).is_err() {
            self.core.reset();
        }
    }
    pub fn note_off(&mut self, note: u8) {
        self.core.note_off(note);
    }
    pub fn sample(&mut self) -> [f32; 2] {
        self.until_control = self.until_control.saturating_sub(1);
        let mut mono = [0.0];
        self.core.render(&mut mono);
        [mono[0]; 2]
    }
    pub fn reset(&mut self) {
        self.core.reset();
        self.until_control = 0;
    }
    pub fn is_idle(&self) -> bool {
        self.core.is_idle()
    }
}
