//! Production dual-filter instrument with two reversible backstage cores.

use crate::dual_filter_concept::{
    ConceptControl, ConceptControls, ConceptError, ConceptTopology, ConceptVariant,
    DualFilterConceptVoice,
};

const CORE_CROSSFADE_SECONDS: f32 = 0.030;
const INDUSTRIAL_INTERNAL_ROUTING: f32 = 0.18;
pub const CORE_TOGGLE_CC: u8 = 35;
pub const CORE_STATE_CC: u8 = 36;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DualFilterCore {
    Industrial,
    Counter,
}

impl DualFilterCore {
    pub const fn toggled(self) -> Self {
        match self {
            Self::Industrial => Self::Counter,
            Self::Counter => Self::Industrial,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Industrial => "INDUSTRIAL",
            Self::Counter => "COUNTER",
        }
    }

    const fn mix(self) -> f32 {
        match self {
            Self::Industrial => 0.0,
            Self::Counter => 1.0,
        }
    }
}

#[derive(Debug)]
pub struct DualFilterInstrument {
    industrial_serial: DualFilterConceptVoice,
    industrial_parallel: DualFilterConceptVoice,
    counter: DualFilterConceptVoice,
    controls: ConceptControls,
    core: DualFilterCore,
    core_mix: f32,
    core_mix_step: f32,
}

impl DualFilterInstrument {
    pub fn new(
        sample_rate: f32,
        controls: ConceptControls,
        core: DualFilterCore,
    ) -> Result<Self, ConceptError> {
        let industrial_controls = industrial_controls(controls);
        let industrial_serial = DualFilterConceptVoice::new(
            sample_rate,
            ConceptVariant::SweetSerial,
            industrial_controls,
        )?;
        let mut industrial_parallel = DualFilterConceptVoice::new(
            sample_rate,
            ConceptVariant::SweetSerial,
            industrial_controls,
        )?;
        industrial_parallel.set_topology_immediate(ConceptTopology::Parallel);
        let counter =
            DualFilterConceptVoice::new(sample_rate, ConceptVariant::CounterMotion, controls)?;
        Ok(Self {
            industrial_serial,
            industrial_parallel,
            counter,
            controls,
            core,
            core_mix: core.mix(),
            core_mix_step: (sample_rate * CORE_CROSSFADE_SECONDS).recip(),
        })
    }

    pub const fn core(&self) -> DualFilterCore {
        self.core
    }

    pub fn set_core(&mut self, core: DualFilterCore) {
        self.core = core;
        if self.is_idle() {
            self.core_mix = core.mix();
        }
    }

    pub fn toggle_core(&mut self) {
        self.set_core(self.core.toggled());
    }

    pub fn set_controls(&mut self, controls: ConceptControls) {
        self.controls = controls;
        let industrial = industrial_controls(controls);
        self.industrial_serial.set_controls(industrial);
        self.industrial_parallel.set_controls(industrial);
        self.counter.set_controls(controls);
    }

    pub fn note_on(&mut self, note: u8, velocity: f32) {
        self.industrial_serial.note_on(note, velocity);
        self.industrial_parallel.note_on(note, velocity);
        self.counter.note_on(note, velocity);
    }

    pub fn note_off(&mut self) {
        self.industrial_serial.note_off();
        self.industrial_parallel.note_off();
        self.counter.note_off();
    }

    pub fn is_idle(&self) -> bool {
        self.industrial_serial.is_idle()
            && self.industrial_parallel.is_idle()
            && self.counter.is_idle()
    }

    pub fn reset(&mut self) {
        self.industrial_serial.reset();
        self.industrial_parallel.reset();
        self.counter.reset();
        self.core_mix = self.core.mix();
    }

    #[inline]
    pub fn sample(&mut self) -> f32 {
        let serial = self.industrial_serial.sample();
        let parallel = self.industrial_parallel.sample();
        let counter = self.counter.sample();
        let structure = self.controls.get(ConceptControl::Routing);
        let industrial = serial + structure * (parallel - serial);
        self.advance_core_mix();
        industrial + self.core_mix * (counter - industrial)
    }

    #[inline]
    fn advance_core_mix(&mut self) {
        let target = self.core.mix();
        if self.core_mix < target {
            self.core_mix = (self.core_mix + self.core_mix_step).min(target);
        } else if self.core_mix > target {
            self.core_mix = (self.core_mix - self.core_mix_step).max(target);
        }
    }
}

fn industrial_controls(mut controls: ConceptControls) -> ConceptControls {
    controls.set(ConceptControl::Routing, INDUSTRIAL_INTERNAL_ROUTING);
    controls
}
