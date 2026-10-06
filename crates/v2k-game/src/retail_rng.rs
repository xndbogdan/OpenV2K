//! Retail V2000's process-global pseudo-random recurrence.
//!
//! `Random_Next` / `FUN_00457930` owns one state word at `0x004F7308`,
//! advances the Microsoft C-runtime linear congruential generator, and
//! returns its high word.  This module deliberately centralizes only that
//! arithmetic.  Individual port systems still own separate state until the
//! original cross-system call order has been recovered.

/// Advance one `FUN_00457930` state word and return the retail 16-bit sample.
pub fn retail_random_u16(state: &mut u32) -> u16 {
    *state = state.wrapping_mul(214_013).wrapping_add(2_531_011);
    (*state >> 16) as u16
}

#[cfg(test)]
mod tests {
    use super::retail_random_u16;

    #[test]
    fn matches_fun_00457930_recurrence_and_returned_high_word() {
        let mut state = 0;

        assert_eq!(retail_random_u16(&mut state), 0x0026);
        assert_eq!(state, 0x0026_9EC3);
        assert_eq!(retail_random_u16(&mut state), 0x1E27);
        assert_eq!(state, 0x1E27_8E7A);
    }
}
