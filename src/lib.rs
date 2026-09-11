//! Portable DSP and synthesis engine for Moj Sint.

pub const ENGINE_NAME: &str = "Moj Sint";

pub mod analysis;
pub mod bass_matrix;
pub mod clean_kick;
pub mod compact_composite;
pub mod composite_machine;
pub mod control;
pub mod coupled_wire_motion;
pub mod coupled_wire_thump;
pub mod dsp;
pub mod dual_filter;
pub mod dual_filter_concept;
pub mod engine;
pub mod envelope;
pub mod envelope_audition;
pub mod host;
pub mod hybrid;
pub mod hybrid_subset;
pub mod micro_machine;
pub mod micro_machine_lab;
pub mod model_d;
pub mod model_d_lab;
pub mod native_bench;
pub mod offline;
#[cfg(feature = "open303")]
pub mod open303;
mod performance;
pub mod preset;
pub mod pressure_chain;
pub mod research;
pub mod six_op_pm;
pub mod strange;
pub mod strange_lab;
pub mod struck_object;
mod synthesis_model;

#[cfg(feature = "open303")]
mod open303_live;
