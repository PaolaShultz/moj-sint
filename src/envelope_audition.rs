use crate::composite_machine::OriginalLayer;

pub const MONOPHONIC_LAYERS: [OriginalLayer; 3] = [
    OriginalLayer::CrossSingle,
    OriginalLayer::SpectralSingle,
    OriginalLayer::DualSingle,
];
pub const LAYER_OFFSETS_MS: [u32; 3] = [0, 2, 5];
pub const DURATION_MS: u32 = 2_400;
pub const DECAY_MS: u32 = 220;
pub const SUSTAIN: f32 = 0.58;
pub const RELEASE_MS: u32 = 500;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AttackProfile {
    VeryShort,
    Moderate,
    Slow,
}

impl AttackProfile {
    pub const ALL: [Self; 3] = [Self::VeryShort, Self::Moderate, Self::Slow];

    pub const fn attack_ms(self) -> u32 {
        match self {
            Self::VeryShort => 6,
            Self::Moderate => 35,
            Self::Slow => 140,
        }
    }

    pub const fn duration_ms(self) -> u32 {
        DURATION_MS
    }

    pub const fn decay_ms(self) -> u32 {
        DECAY_MS
    }

    pub const fn sustain(self) -> f32 {
        SUSTAIN
    }

    pub const fn release_ms(self) -> u32 {
        RELEASE_MS
    }

    pub const fn slug(self) -> &'static str {
        match self {
            Self::VeryShort => "attack-006ms",
            Self::Moderate => "attack-035ms",
            Self::Slow => "attack-140ms",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::VeryShort => "very short attack",
            Self::Moderate => "moderately longer attack",
            Self::Slow => "slow attack",
        }
    }

    pub const fn filename(self) -> &'static str {
        match self {
            Self::VeryShort => "01_monophonic_attack_006ms.wav",
            Self::Moderate => "02_monophonic_attack_035ms.wav",
            Self::Slow => "03_monophonic_attack_140ms.wav",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hybrid::HybridCondition;

    #[test]
    fn audition_uses_only_exact_single_note_layers() {
        assert_eq!(
            MONOPHONIC_LAYERS,
            [
                OriginalLayer::CrossSingle,
                OriginalLayer::SpectralSingle,
                OriginalLayer::DualSingle,
            ]
        );
        assert_eq!(LAYER_OFFSETS_MS, [0, 2, 5]);
        assert!(
            MONOPHONIC_LAYERS
                .iter()
                .all(|layer| layer.condition() == HybridCondition::Single)
        );
    }

    #[test]
    fn attack_profiles_are_short_fixed_and_share_the_remaining_shape() {
        assert_eq!(
            AttackProfile::ALL.map(AttackProfile::attack_ms),
            [6, 35, 140]
        );
        for profile in AttackProfile::ALL {
            assert_eq!(profile.duration_ms(), 2_400);
            assert_eq!(profile.decay_ms(), 220);
            assert_eq!(profile.sustain(), 0.58);
            assert_eq!(profile.release_ms(), 500);
            assert!(
                LAYER_OFFSETS_MS
                    .iter()
                    .all(|offset| *offset < profile.attack_ms())
            );
        }
    }
}
