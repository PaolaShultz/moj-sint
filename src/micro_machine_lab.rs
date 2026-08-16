//! Non-real-time measurements for the isolated typed swarm micro-machine.

use crate::micro_machine::{MicroMachineError, MicroMachineGraph, SwarmControls};
use crate::research::fitted_residual_db;

const REFERENCE_FACTOR: usize = 8;

pub fn midi_frequency(note: u8) -> f32 {
    440.0 * 2.0_f32.powf((f32::from(note) - 69.0) / 12.0)
}

pub fn measure_high_rate_residual(
    graph: &MicroMachineGraph,
    controls: SwarmControls,
    note: u8,
    sample_count: usize,
) -> Result<f64, MicroMachineError> {
    let frequency_hz = midi_frequency(note);
    let mut target = graph.compile(48_000.0, frequency_hz, controls)?;
    let mut reference =
        graph.compile(48_000.0 * REFERENCE_FACTOR as f32, frequency_hz, controls)?;
    const WARMUP: usize = 4_096;
    for _ in 0..WARMUP {
        target.sample();
    }
    for _ in 0..WARMUP * REFERENCE_FACTOR {
        reference.sample();
    }
    let mut target_samples = Vec::with_capacity(sample_count);
    let mut reference_samples = Vec::with_capacity(sample_count);
    for _ in 0..sample_count {
        let target_frame = target.sample();
        target_samples.push(0.5 * (target_frame[0] + target_frame[1]));
        let mut sum = 0.0;
        for _ in 0..REFERENCE_FACTOR {
            let frame = reference.sample();
            sum += 0.5 * (frame[0] + frame[1]);
        }
        reference_samples.push(sum / REFERENCE_FACTOR as f32);
    }
    Ok(fitted_residual_db(&target_samples, &reference_samples))
}
