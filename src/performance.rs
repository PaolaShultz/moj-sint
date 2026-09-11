//! Transient performance controls, independent of preset timbre and note gates.
use crate::control::Smoother;

#[derive(Debug)]
pub(crate) struct Performance {
    bend: Smoother,
    depth: Smoother,
    phase: f32,
    step: f32,
}

impl Performance {
    pub fn new(rate: f32) -> Self {
        Self {
            bend: Smoother::new(0.0, rate, 0.010).expect("validated sample rate"),
            depth: Smoother::new(0.0, rate, 0.010).expect("validated sample rate"),
            phase: 0.0,
            step: 5.0 / rate,
        }
    }
    pub fn bend(&mut self, value: i16) {
        let value = value.clamp(-8192, 8191);
        let target = 2.0 * f32::from(value) / if value < 0 { 8192.0 } else { 8191.0 };
        self.bend.set_target(target);
    }
    pub fn modulation(&mut self, value: u8) {
        self.depth
            .set_target(0.5 * f32::from(value.min(127)) / 127.0);
    }
    pub fn reset(&mut self) {
        self.bend(0);
        self.modulation(0);
    }
    pub fn advance(&mut self) -> f32 {
        self.phase += self.step;
        self.phase -= self.phase.floor();
        // Smooth parabolic sine approximation; no per-sample transcendental setup.
        let x = 2.0 * self.phase - 1.0;
        let y = 4.0 * x * (1.0 - x.abs());
        let sine = y + 0.225 * (y * y.abs() - y);
        self.bend.advance() + self.depth.advance() * sine
    }
}

pub(crate) fn ratio(semitones: f32) -> f32 {
    // exp(x), |x| <= ln(2)*2.5/12. Error below f32 precision here.
    let x = semitones * (std::f32::consts::LN_2 / 12.0);
    1.0 + x * (1.0 + x * (0.5 + x * (1.0 / 6.0 + x * (1.0 / 24.0 + x * (1.0 / 120.0 + x / 720.0)))))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bend_endpoints_center_vibrato_and_reset() {
        let mut p = Performance::new(48_000.0);
        for (value, target) in [(-8192, -2.0), (8191, 2.0), (0, 0.0)] {
            p.bend(value);
            for _ in 0..48000 {
                p.advance();
            }
            assert!((p.advance() - target).abs() < 0.0001);
        }
        p.modulation(127);
        let mut lo = 0.0_f32;
        let mut hi = 0.0_f32;
        for _ in 0..48000 {
            let v = p.advance();
            lo = lo.min(v);
            hi = hi.max(v);
        }
        assert!(lo < -0.49 && hi > 0.49);
        p.reset();
        for _ in 0..48000 {
            p.advance();
        }
        assert!(p.advance().abs() < 0.0001);
        for step in -250..=250 {
            let semitones = step as f32 / 100.0;
            assert!((ratio(semitones) - 2.0_f32.powf(semitones / 12.0)).abs() < 0.000001);
        }
    }
}
