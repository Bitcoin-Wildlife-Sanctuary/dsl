#[cfg(test)]
mod test {
    use crate::builtins::m31::M31Var;
    use crate::bvar::AllocVar;
    use crate::constraint_system::ConstraintSystem;
    use crate::test_program;
    use crate::treepp::*;
    use rand::{Rng, SeedableRng};
    use rand_chacha::ChaCha20Rng;

    #[test]
    fn test_m31_mult() {
        let mut prng = ChaCha20Rng::seed_from_u64(0);

        let mut a_val = prng.gen_range(0..((1i64 << 31) - 1)) as u32;

        let cs = ConstraintSystem::new_ref();

        let mut a = M31Var::new_program_input(&cs, a_val).unwrap();

        for _ in 0..10 {
            let b_val = prng.gen_range(0..((1i64 << 31) - 1)) as u32;

            let b = M31Var::new_constant(&cs, b_val).unwrap();

            let c = &a * &b;
            let c_val = ((a_val as i64) * (b_val as i64) % ((1i64 << 31) - 1)) as u32;
            assert_eq!(c.value, c_val);

            a = c;
            a_val = c_val;
        }

        cs.set_execution_output(&a).unwrap();

        test_program(
            cs,
            script! {
                { a_val }
            },
        )
        .unwrap();
    }
}
