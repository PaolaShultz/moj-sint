//! Portable DSP and synthesis engine for Moj Sint.

pub const ENGINE_NAME: &str = "Moj Sint";

pub mod analysis;
pub mod compact_composite;
pub mod composite_machine;
pub mod control;
pub mod dsp;
pub mod engine;
pub mod envelope;
pub mod envelope_audition;
pub mod hybrid;
pub mod hybrid_subset;
pub mod offline;
pub mod preset;
pub mod research;
pub mod struck_object;
