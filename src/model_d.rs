//! Isolated circuit-informed Model D character research modules.

pub mod contour;
pub mod ladder;
pub mod mixer;
pub mod vco;
pub mod voice;

use thiserror::Error;

#[derive(Clone, Copy, Debug, Error, PartialEq)]
pub enum ModelDError {
    #[error("sample rate must be finite and positive")]
    InvalidSampleRate,
    #[error("Model D configuration is invalid")]
    InvalidConfig,
}
