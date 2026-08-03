mod house;
mod pressure;

use thiserror::Error;

pub const SAMPLE_RATE: u32 = 48_000;

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
        match topology {
            KickTopology::HouseImpact => Ok(Self::House(house::HouseImpact::new(sample_rate)?)),
            KickTopology::LongPressure => {
                Ok(Self::Pressure(pressure::LongPressure::new(sample_rate)?))
            }
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
