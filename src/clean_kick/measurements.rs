use super::{KickConfig, KickError, KickRender, KickTopology, render_solo};
use crate::research::fitted_residual_db;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KickMetrics {
    pub sample_peak_dbfs: f64,
    pub true_peak_dbfs: f64,
    pub rms_dbfs: f64,
    pub absolute_dc: f64,
    pub maximum_jump: f64,
    pub ceiling_contacts: u64,
    pub finite: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KickRejection {
    NonFinite,
    Level,
    TruePeak,
    CeilingContact,
    Dc,
    Jump,
    CausalAblation,
    HighRateResidual,
}

#[derive(Clone, Debug, PartialEq)]
pub struct KickEvidence {
    pub topology: KickTopology,
    pub metrics: KickMetrics,
    pub high_rate_residual_db: f64,
    pub onset_ablation_db: f64,
    pub body_ablation_db: f64,
}

impl KickEvidence {
    pub fn rejection_reasons(&self) -> Vec<KickRejection> {
        let mut reasons = Vec::new();
        if !self.metrics.finite {
            reasons.push(KickRejection::NonFinite);
        }
        if !(-6.0..=-1.0).contains(&self.metrics.sample_peak_dbfs) {
            reasons.push(KickRejection::Level);
        }
        if self.metrics.true_peak_dbfs > -1.0 {
            reasons.push(KickRejection::TruePeak);
        }
        if self.metrics.ceiling_contacts != 0 {
            reasons.push(KickRejection::CeilingContact);
        }
        if self.metrics.absolute_dc >= 1.0e-5 {
            reasons.push(KickRejection::Dc);
        }
        if self.metrics.maximum_jump >= 0.10 {
            reasons.push(KickRejection::Jump);
        }
        match self.topology {
            KickTopology::HouseImpact => {
                if self.onset_ablation_db < -6.0 || self.body_ablation_db.abs() > 1.0 {
                    reasons.push(KickRejection::CausalAblation);
                }
            }
            KickTopology::LongPressure => {
                if self.onset_ablation_db < -4.0 || self.body_ablation_db < -45.0 {
                    reasons.push(KickRejection::CausalAblation);
                }
            }
        }
        if self.high_rate_residual_db > -60.0 {
            reasons.push(KickRejection::HighRateResidual);
        }
        reasons
    }
}

pub fn select(topology: KickTopology, sample_rate: u32) -> Result<KickConfig, KickError> {
    let candidates: &[f32] = match topology {
        KickTopology::HouseImpact => &[0.4, 0.6, 0.9, 1.2],
        KickTopology::LongPressure => &[0.5, 0.75, 1.0, 1.25],
    };
    for &modifier_amount in candidates {
        let mut config = KickConfig::default_for(topology);
        config.modifier_amount = modifier_amount;
        let render = render_solo(config, sample_rate)?;
        if evaluate(&render)?.rejection_reasons().is_empty() {
            return Ok(config);
        }
    }
    Err(KickError::Unavailable)
}

pub fn evaluate(render: &KickRender) -> Result<KickEvidence, KickError> {
    if render.samples.is_empty() || render.sample_rate < 8_000 {
        return Err(KickError::InvalidConfig);
    }
    let mut metrics = measure(&render.samples);
    let (high_rate_residual_db, true_peak) = measure_high_rate(render.config, render.sample_rate)?;
    metrics.true_peak_dbfs =
        amplitude_db(true_peak.max(10.0_f64.powf(metrics.sample_peak_dbfs / 20.0)));
    let (onset_ablation_db, body_ablation_db) = measure_ablation(render)?;
    Ok(KickEvidence {
        topology: render.config.topology,
        metrics,
        high_rate_residual_db,
        onset_ablation_db,
        body_ablation_db,
    })
}

fn measure(samples: &[f32]) -> KickMetrics {
    let finite = samples.iter().all(|sample| sample.is_finite());
    let peak = samples
        .iter()
        .map(|sample| f64::from(sample.abs()))
        .fold(0.0_f64, f64::max);
    let sum = samples.iter().map(|sample| f64::from(*sample)).sum::<f64>();
    let energy = samples
        .iter()
        .map(|sample| f64::from(*sample).powi(2))
        .sum::<f64>();
    let maximum_jump = samples
        .windows(2)
        .map(|pair| f64::from((pair[1] - pair[0]).abs()))
        .fold(0.0_f64, f64::max);
    KickMetrics {
        sample_peak_dbfs: amplitude_db(peak),
        true_peak_dbfs: amplitude_db(peak),
        rms_dbfs: amplitude_db((energy / samples.len() as f64).sqrt()),
        absolute_dc: (sum / samples.len() as f64).abs(),
        maximum_jump,
        ceiling_contacts: samples
            .iter()
            .filter(|sample| sample.abs() >= 0.999)
            .count() as u64,
        finite,
    }
}

fn measure_high_rate(config: KickConfig, sample_rate: u32) -> Result<(f64, f64), KickError> {
    const FACTOR: usize = 8;
    let target = render_solo(config, sample_rate)?;
    let high = render_solo(config, sample_rate * FACTOR as u32)?;
    let true_peak = high
        .samples
        .iter()
        .map(|sample| f64::from(sample.abs()))
        .fold(0.0_f64, f64::max);
    let edge = sample_rate as usize / 100;
    let count = (sample_rate as usize / 2).min(target.samples.len() - edge);
    let target = &target.samples[edge..edge + count];
    let mut best = f64::INFINITY;
    for center_offset in -8..=8 {
        let reference = downsample_8x(&high.samples, edge + count, center_offset);
        best = best.min(fitted_residual_db(target, &reference[edge..edge + count]));
    }
    Ok((best, true_peak))
}

fn downsample_8x(high_rate: &[f32], output_frames: usize, center_offset: isize) -> Vec<f32> {
    const FACTOR: isize = 8;
    const RADIUS: isize = 32;
    (0..output_frames)
        .map(|frame| {
            let center = frame as isize * FACTOR + center_offset;
            let mut output = 0.0;
            let mut weight = 0.0;
            for offset in -RADIUS..=RADIUS {
                let index = center + offset;
                if !(0..high_rate.len() as isize).contains(&index) {
                    continue;
                }
                let distance = offset as f64 / FACTOR as f64;
                let sinc = if offset == 0 {
                    1.0
                } else {
                    (std::f64::consts::PI * distance).sin() / (std::f64::consts::PI * distance)
                };
                let window =
                    0.5 + 0.5 * (std::f64::consts::PI * offset as f64 / RADIUS as f64).cos();
                let coefficient = sinc * window;
                output += f64::from(high_rate[index as usize]) * coefficient;
                weight += coefficient;
            }
            (output / weight.max(1.0e-18)) as f32
        })
        .collect()
}

fn measure_ablation(render: &KickRender) -> Result<(f64, f64), KickError> {
    let mut muted = render.config;
    muted.modifier_amount = 0.0;
    let muted = render_solo(muted, render.sample_rate)?;
    Ok(match render.config.topology {
        KickTopology::HouseImpact => {
            let complete_onset = band_rms(&render.samples, render.sample_rate, 0, 80, 100.0, 800.0);
            let onset_difference = band_difference_rms(
                &render.samples,
                &muted.samples,
                render.sample_rate,
                0,
                80,
                100.0,
                800.0,
            );
            let complete_body = band_rms(&render.samples, render.sample_rate, 180, 320, 35.0, 90.0);
            let muted_body = band_rms(&muted.samples, render.sample_rate, 180, 320, 35.0, 90.0);
            (
                ratio_db(onset_difference, complete_onset),
                ratio_db(complete_body, muted_body),
            )
        }
        KickTopology::LongPressure => {
            let complete_onset = band_rms(&render.samples, render.sample_rate, 0, 140, 70.0, 250.0);
            let onset_difference = band_difference_rms(
                &render.samples,
                &muted.samples,
                render.sample_rate,
                0,
                140,
                70.0,
                250.0,
            );
            let body = band_rms(&render.samples, render.sample_rate, 250, 760, 44.0, 54.0);
            (
                ratio_db(onset_difference, complete_onset),
                amplitude_db(body),
            )
        }
    })
}

#[derive(Clone, Copy)]
struct OnePole {
    coefficient: f32,
    state: f32,
}

impl OnePole {
    fn new(cutoff_hz: f32, sample_rate: u32) -> Self {
        Self {
            coefficient: (-std::f32::consts::TAU * cutoff_hz / sample_rate as f32).exp(),
            state: 0.0,
        }
    }

    fn sample(&mut self, input: f32) -> f32 {
        self.state = (1.0 - self.coefficient) * input + self.coefficient * self.state;
        self.state
    }
}

fn band_rms(
    samples: &[f32],
    sample_rate: u32,
    start_ms: usize,
    end_ms: usize,
    low_hz: f32,
    high_hz: f32,
) -> f64 {
    let mut low = OnePole::new(low_hz, sample_rate);
    let mut high = OnePole::new(high_hz, sample_rate);
    let start = start_ms * sample_rate as usize / 1_000;
    let end = (end_ms * sample_rate as usize / 1_000).min(samples.len());
    let mut energy = 0.0;
    for (frame, &sample) in samples[..end].iter().enumerate() {
        let above_low = sample - low.sample(sample);
        let band = high.sample(above_low);
        if frame >= start {
            energy += f64::from(band).powi(2);
        }
    }
    (energy / (end - start).max(1) as f64).sqrt()
}

fn band_difference_rms(
    complete: &[f32],
    muted: &[f32],
    sample_rate: u32,
    start_ms: usize,
    end_ms: usize,
    low_hz: f32,
    high_hz: f32,
) -> f64 {
    let difference = complete
        .iter()
        .zip(muted)
        .map(|(&complete, &muted)| complete - muted)
        .collect::<Vec<_>>();
    band_rms(&difference, sample_rate, start_ms, end_ms, low_hz, high_hz)
}

fn ratio_db(numerator: f64, denominator: f64) -> f64 {
    20.0 * (numerator.max(1.0e-12) / denominator.max(1.0e-12)).log10()
}

fn amplitude_db(value: f64) -> f64 {
    20.0 * value.max(1.0e-12).log10()
}
