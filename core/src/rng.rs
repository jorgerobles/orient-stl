//! Deterministic PRNG for reproducible refinement.
//!
//! xorshift32 — no crate dependency, ~5 ns/call, full 2³²-1 period. Seeded
//! from a u32; the same seed always yields the same sequence, which makes
//! `refine_orientation` reproducible per input (fixes the click-to-click
//! non-determinism from `js_sys::Math::random()`).

pub(crate) struct Rng {
    state: u32,
}

impl Rng {
    pub(crate) fn new(seed: u32) -> Self {
        // xorshift cannot use 0 (absorbing state); map any zero to a constant.
        Self { state: if seed == 0 { 0xDEAD_BEEF } else { seed } }
    }

    /// Next u32 in the sequence.
    pub(crate) fn next_u32(&mut self) -> u32 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.state = x;
        x
    }

    /// Uniform float in [0, 1).
    pub(crate) fn next_f32(&mut self) -> f32 {
        // 24-bit mantissa for a stable [0,1) range.
        (self.next_u32() >> 8) as f32 * (1.0f32 / ((1u32 << 24) as f32))
    }

    /// Uniform float in [-1, 1).
    pub(crate) fn next_signed_f32(&mut self) -> f32 {
        self.next_f32() * 2.0 - 1.0
    }
}

/// Hash a direction + an integer index into a u32 seed. Cheap mixing of the
/// three direction components (expressed as scaled integers) and the index,
/// so each (direction, index) pair maps to a distinct reproducible seed.
pub(crate) fn seed_from_direction(dir: &[f32; 3], salt: u32) -> u32 {
    // Quantise each component to 12 fractional bits (~0.00024° resolution).
    let qx = (dir[0] * 4096.0) as i32 as u32;
    let qy = (dir[1] * 4096.0) as i32 as u32;
    let qz = (dir[2] * 4096.0) as i32 as u32;
    let mut h = salt;
    h = h.wrapping_add(qx).wrapping_mul(0x9E37_79B1);
    h = h.wrapping_add(qy).wrapping_mul(0x85EB_CA77);
    h = h.wrapping_add(qz).wrapping_mul(0xC2B2_AE3D);
    h ^= h >> 16;
    if h == 0 { 0xDEAD_BEEF } else { h }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_same_sequence() {
        let mut a = Rng::new(42);
        let mut b = Rng::new(42);
        for _ in 0..100 {
            assert_eq!(a.next_u32(), b.next_u32(), "sequences diverged");
        }
    }

    #[test]
    fn different_seeds_different_sequence() {
        let mut a = Rng::new(1);
        let mut b = Rng::new(2);
        let mut differ = 0;
        for _ in 0..10 {
            if a.next_u32() != b.next_u32() {
                differ += 1;
            }
        }
        assert!(differ > 5, "different seeds should produce mostly-different sequences");
    }

    #[test]
    fn zero_seed_does_not_absorb() {
        let mut r = Rng::new(0);
        let v1 = r.next_u32();
        let v2 = r.next_u32();
        assert_ne!(v1, 0, "zero seed must not produce zero state");
        assert_ne!(v1, v2, "sequence must progress");
    }

    #[test]
    fn next_f32_in_unit_range() {
        let mut r = Rng::new(7);
        for _ in 0..1000 {
            let f = r.next_f32();
            assert!(f >= 0.0 && f < 1.0, "next_f32 out of [0,1): {}", f);
        }
    }

    #[test]
    fn next_signed_f32_in_signed_unit_range() {
        let mut r = Rng::new(99);
        for _ in 0..1000 {
            let f = r.next_signed_f32();
            assert!(f >= -1.0 && f < 1.0, "next_signed_f32 out of [-1,1): {}", f);
        }
    }

    #[test]
    fn seed_from_direction_is_deterministic() {
        let d = [0.0, 0.0, -1.0];
        assert_eq!(seed_from_direction(&d, 0), seed_from_direction(&d, 0));
    }

    #[test]
    fn seed_from_direction_differs_for_different_dirs() {
        let a = seed_from_direction(&[0.0, 0.0, -1.0], 0);
        let b = seed_from_direction(&[1.0, 0.0, 0.0], 0);
        assert_ne!(a, b);
    }

    #[test]
    fn seed_from_direction_differs_for_different_salts() {
        let a = seed_from_direction(&[0.0, 0.0, -1.0], 0);
        let b = seed_from_direction(&[0.0, 0.0, -1.0], 1);
        assert_ne!(a, b);
    }
}
