pub mod character;
pub mod harmonic_selector;
pub mod hybrid;
pub mod oscillator;
pub mod research;

#[inline]
pub fn finite_or_zero(value: f32) -> f32 {
    if value.is_finite() { value } else { 0.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finite_guard_silences_invalid_values() {
        assert_eq!(finite_or_zero(f32::NAN), 0.0);
        assert_eq!(finite_or_zero(f32::INFINITY), 0.0);
        assert_eq!(finite_or_zero(0.25), 0.25);
    }
}
