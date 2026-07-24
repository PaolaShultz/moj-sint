use crate::dsp::hybrid::HybridFrame as StereoFrame;
use crate::struck_object::{StruckError, StruckObject, StruckTopology};
use std::f32::consts::TAU;

pub const LOW_SPLIT_HZ: f32 = 105.0;
pub const THUMP_SPLIT_HZ: f32 = 500.0;
pub const LOW_STRIKE_GAIN: f32 = 0.794_328_2;
pub const WET_START_MS: u32 = 8;
pub const WET_FULL_MS: u32 = 12;
pub const WET_HOLD_END_MS: u32 = 45;
pub const WET_END_MS: u32 = 70;
pub const LOW_HOLD_END_MS: u32 = 80;
pub const LOW_RECOVERY_END_MS: u32 = 150;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThumpConfig {
    pub slug: &'static str,
    pub drive: f32,
    pub threshold: f32,
    pub wet: f32,
}

pub const CONFIGS: [ThumpConfig; 3] = [
    ThumpConfig {
        slug: "gentle",
        drive: 1.5,
        threshold: 0.16,
        wet: 0.25,
    },
    ThumpConfig {
        slug: "controlled",
        drive: 1.8,
        threshold: 0.15,
        wet: 0.30,
    },
    ThumpConfig {
        slug: "hard",
        drive: 2.1,
        threshold: 0.14,
        wet: 0.35,
    },
];

#[derive(Clone, Copy, Debug, Default)]
pub struct ThumpFrame {
    pub source: StereoFrame,
    pub output: StereoFrame,
    pub clean_low: f32,
    pub nonlinear_residual: StereoFrame,
}

#[derive(Clone, Copy)]
struct OnePole {
    coefficient: f32,
    state: f32,
}

impl OnePole {
    fn new(hz: f32, sample_rate: u32) -> Self {
        Self {
            coefficient: (-TAU * hz / sample_rate as f32).exp(),
            state: 0.0,
        }
    }

    #[inline]
    fn sample(&mut self, input: f32) -> f32 {
        self.state = (1.0 - self.coefficient) * input + self.coefficient * self.state;
        self.state
    }
}

#[derive(Clone, Copy)]
struct DcBlocker {
    coefficient: f32,
    previous_input: f32,
    previous_output: f32,
}

impl DcBlocker {
    fn new(sample_rate: u32) -> Self {
        Self {
            coefficient: (-TAU * 35.0 / sample_rate as f32).exp(),
            previous_input: 0.0,
            previous_output: 0.0,
        }
    }

    #[inline]
    fn sample(&mut self, input: f32) -> f32 {
        let output = input - self.previous_input + self.coefficient * self.previous_output;
        self.previous_input = input;
        self.previous_output = output;
        output
    }
}

pub struct ControlledThumpVoice {
    body: StruckObject,
    config: ThumpConfig,
    sample_rate: u32,
    frame: usize,
    duration_frames: usize,
    low_split: OnePole,
    thump_split: [OnePole; 2],
    residual_low: [OnePole; 2],
    post_dc: [DcBlocker; 2],
}

impl ControlledThumpVoice {
    pub fn new(config: ThumpConfig, sample_rate: u32) -> Result<Self, StruckError> {
        if sample_rate == 0
            || !config.drive.is_finite()
            || config.drive <= 0.0
            || !config.threshold.is_finite()
            || config.threshold <= 0.0
            || !config.wet.is_finite()
            || !(0.0..=0.35).contains(&config.wet)
        {
            return Err(StruckError::InvalidConfiguration);
        }
        let body = StruckObject::new(StruckTopology::CoupledWire, sample_rate)?;
        let duration_frames = body.duration_frames();
        Ok(Self {
            body,
            config,
            sample_rate,
            frame: 0,
            duration_frames,
            low_split: OnePole::new(LOW_SPLIT_HZ, sample_rate),
            thump_split: [OnePole::new(THUMP_SPLIT_HZ, sample_rate); 2],
            residual_low: [OnePole::new(LOW_SPLIT_HZ, sample_rate); 2],
            post_dc: [DcBlocker::new(sample_rate); 2],
        })
    }

    #[inline]
    pub fn sample(&mut self) -> StereoFrame {
        self.sample_trace().output
    }

    #[inline]
    pub fn sample_trace(&mut self) -> ThumpFrame {
        if self.frame >= self.duration_frames {
            return ThumpFrame::default();
        }
        let source = self.body.sample();
        let mid = 0.5 * (source.left + source.right);
        let clean_low = self.low_split.sample(mid);
        let upper_left = source.left - clean_low;
        let upper_right = source.right - clean_low;
        let thump_left = self.thump_split[0].sample(upper_left);
        let thump_right = self.thump_split[1].sample(upper_right);
        let bright_left = upper_left - thump_left;
        let bright_right = upper_right - thump_right;

        let wet = self.wet_envelope();
        let (residual_left, residual_right) = if wet > 0.0 {
            let shaped_left = thump_left
                + wet
                    * self.config.wet
                    * (hard_crest(thump_left, self.config) - thump_left);
            let shaped_right = thump_right
                + wet
                    * self.config.wet
                    * (hard_crest(thump_right, self.config) - thump_right);
            let raw_left = shaped_left - thump_left;
            let raw_right = shaped_right - thump_right;
            (
                raw_left - self.residual_low[0].sample(raw_left),
                raw_right - self.residual_low[1].sample(raw_right),
            )
        } else {
            (0.0, 0.0)
        };
        let low = clean_low * self.low_contour();
        let output = StereoFrame {
            left: self.post_dc[0].sample(low + bright_left + thump_left + residual_left),
            right: self.post_dc[1].sample(low + bright_right + thump_right + residual_right),
        };
        self.frame += 1;
        ThumpFrame {
            source,
            output,
            clean_low,
            nonlinear_residual: StereoFrame {
                left: residual_left,
                right: residual_right,
            },
        }
    }

    pub fn duration_frames(&self) -> usize {
        self.duration_frames
    }

    #[inline]
    fn wet_envelope(&self) -> f32 {
        let milliseconds = self.frame as f32 * 1_000.0 / self.sample_rate as f32;
        if milliseconds < WET_START_MS as f32 {
            0.0
        } else if milliseconds < WET_FULL_MS as f32 {
            smoothstep(
                (milliseconds - WET_START_MS as f32)
                    / (WET_FULL_MS - WET_START_MS) as f32,
            )
        } else if milliseconds < WET_HOLD_END_MS as f32 {
            1.0
        } else if milliseconds < WET_END_MS as f32 {
            1.0 - smoothstep(
                (milliseconds - WET_HOLD_END_MS as f32)
                    / (WET_END_MS - WET_HOLD_END_MS) as f32,
            )
        } else {
            0.0
        }
    }

    #[inline]
    fn low_contour(&self) -> f32 {
        let milliseconds = self.frame as f32 * 1_000.0 / self.sample_rate as f32;
        if milliseconds < LOW_HOLD_END_MS as f32 {
            LOW_STRIKE_GAIN
        } else if milliseconds < LOW_RECOVERY_END_MS as f32 {
            let phase = (milliseconds - LOW_HOLD_END_MS as f32)
                / (LOW_RECOVERY_END_MS - LOW_HOLD_END_MS) as f32;
            LOW_STRIKE_GAIN + (1.0 - LOW_STRIKE_GAIN) * smoothstep(phase)
        } else {
            1.0
        }
    }
}

#[inline]
fn hard_crest(input: f32, config: ThumpConfig) -> f32 {
    (input * config.drive).clamp(-config.threshold, config.threshold)
}

#[inline]
fn smoothstep(value: f32) -> f32 {
    value * value * (3.0 - 2.0 * value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_no_alloc::assert_no_alloc;

    #[test]
    fn source_before_presentation_is_sample_identical() {
        let sample_rate = 48_000;
        let mut expected =
            StruckObject::new(StruckTopology::CoupledWire, sample_rate).unwrap();
        let mut voice = ControlledThumpVoice::new(CONFIGS[0], sample_rate).unwrap();
        for _ in 0..voice.duration_frames() {
            let expected = expected.sample();
            let trace = voice.sample_trace();
            assert_eq!(trace.source.left.to_bits(), expected.left.to_bits());
            assert_eq!(trace.source.right.to_bits(), expected.right.to_bits());
        }
    }

    #[test]
    fn sample_path_is_finite_allocation_free_and_returns_to_zero() {
        let mut voice = ControlledThumpVoice::new(CONFIGS[1], 48_000).unwrap();
        assert_no_alloc(|| {
            for _ in 0..voice.duration_frames() {
                let frame = voice.sample();
                assert!(frame.left.is_finite() && frame.right.is_finite());
            }
            assert_eq!(voice.sample(), StereoFrame::default());
        });
    }

    #[test]
    fn nonlinear_residual_is_confined_to_the_declared_window() {
        let sample_rate = 48_000;
        let mut voice = ControlledThumpVoice::new(CONFIGS[2], sample_rate).unwrap();
        for frame in 0..voice.duration_frames() {
            let trace = voice.sample_trace();
            let residual =
                trace.nonlinear_residual.left.abs() + trace.nonlinear_residual.right.abs();
            if frame < WET_START_MS as usize * sample_rate as usize / 1_000
                || frame >= WET_END_MS as usize * sample_rate as usize / 1_000
            {
                assert_eq!(residual.to_bits(), 0.0_f32.to_bits());
            }
        }
    }
}
