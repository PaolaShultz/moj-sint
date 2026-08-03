mod house;
mod measurements;
mod pressure;

use thiserror::Error;

pub const SAMPLE_RATE: u32 = 48_000;

pub use measurements::{KickEvidence, KickMetrics, KickRejection, evaluate, select};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KickConfig {
    pub topology: KickTopology,
    pub output_gain: f32,
    pub modifier_amount: f32,
}

impl KickConfig {
    fn default_for(topology: KickTopology) -> Self {
        match topology {
            KickTopology::HouseImpact => Self {
                topology,
                output_gain: 0.8,
                modifier_amount: 0.9,
            },
            KickTopology::LongPressure => Self {
                topology,
                output_gain: 1.5,
                modifier_amount: 1.0,
            },
        }
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
            coefficient: (-std::f32::consts::TAU * 5.0 / sample_rate as f32).exp(),
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KickTopology {
    HouseImpact,
    LongPressure,
}

#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum KickError {
    #[error("invalid sample rate")]
    InvalidSampleRate,
    #[error("invalid kick configuration")]
    InvalidConfig,
    #[error("kick topology is not implemented")]
    Unavailable,
}

pub enum PreparedKick {
    House(house::HouseImpact),
    Pressure(pressure::LongPressure),
}

impl PreparedKick {
    pub fn new(topology: KickTopology, sample_rate: u32) -> Result<Self, KickError> {
        Self::from_config(KickConfig::default_for(topology), sample_rate)
    }

    fn from_config(config: KickConfig, sample_rate: u32) -> Result<Self, KickError> {
        match config.topology {
            KickTopology::HouseImpact => {
                Ok(Self::House(house::HouseImpact::new(config, sample_rate)?))
            }
            KickTopology::LongPressure => Ok(Self::Pressure(pressure::LongPressure::new(
                config,
                sample_rate,
            )?)),
        }
    }

    pub fn trigger(&mut self) {
        match self {
            Self::House(voice) => voice.trigger(),
            Self::Pressure(voice) => voice.trigger(),
        }
    }

    #[inline]
    pub fn sample(&mut self) -> f32 {
        match self {
            Self::House(voice) => voice.sample(),
            Self::Pressure(voice) => voice.sample(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct KickRender {
    pub config: KickConfig,
    pub sample_rate: u32,
    pub samples: Vec<f32>,
}

pub fn render_solo(config: KickConfig, sample_rate: u32) -> Result<KickRender, KickError> {
    let seconds = match config.topology {
        KickTopology::HouseImpact => 1.0,
        KickTopology::LongPressure => 1.5,
    };
    let frames = (seconds * sample_rate as f32).round() as usize;
    let mut voice = PreparedKick::from_config(config, sample_rate)?;
    voice.trigger();
    let samples = (0..frames).map(|_| voice.sample()).collect();
    Ok(KickRender {
        config,
        sample_rate,
        samples,
    })
}
