use crate::composite_machine::OriginalLayer;

pub const DURATION_MS: u32 = 1_600;
pub const BASE_HZ: f32 = 73.416_19;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StruckTopology {
    CoupledWire,
    SpectralPlate,
    DualBridge,
}

impl StruckTopology {
    pub const ALL: [Self; 3] = [Self::CoupledWire, Self::SpectralPlate, Self::DualBridge];

    pub const fn exciter(self) -> OriginalLayer {
        match self {
            Self::CoupledWire => OriginalLayer::CrossSingle,
            Self::SpectralPlate => OriginalLayer::SpectralSingle,
            Self::DualBridge => OriginalLayer::DualSingle,
        }
    }

    pub const fn exciter_ms(self) -> u32 {
        match self {
            Self::CoupledWire => 7,
            Self::SpectralPlate => 11,
            Self::DualBridge => 9,
        }
    }

    pub const fn slug(self) -> &'static str {
        match self {
            Self::CoupledWire => "coupled-wire",
            Self::SpectralPlate => "spectral-plate",
            Self::DualBridge => "dual-bridge",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::CoupledWire => "Coupled Wire",
            Self::SpectralPlate => "Spectral Plate",
            Self::DualBridge => "Dual Bridge",
        }
    }

    pub const fn filename(self) -> &'static str {
        match self {
            Self::CoupledWire => "01_coupled_wire.wav",
            Self::SpectralPlate => "02_spectral_plate.wav",
            Self::DualBridge => "03_dual_bridge.wav",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn topologies_are_exact_and_distinct() {
        assert_eq!(
            StruckTopology::ALL,
            [
                StruckTopology::CoupledWire,
                StruckTopology::SpectralPlate,
                StruckTopology::DualBridge,
            ]
        );
        assert_eq!(
            StruckTopology::ALL.map(StruckTopology::exciter),
            [
                OriginalLayer::CrossSingle,
                OriginalLayer::SpectralSingle,
                OriginalLayer::DualSingle,
            ]
        );
        assert_eq!(
            StruckTopology::ALL.map(StruckTopology::exciter_ms),
            [7, 11, 9]
        );
        assert_eq!(DURATION_MS, 1_600);
        assert!((BASE_HZ - 73.416_19).abs() < 1.0e-4);
    }
}
