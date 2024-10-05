#[cfg(test)]
mod test {
    use crate::builtins::m31::M31Var;
    use crate::builtins::table::utils::rand_m31;
    use crate::bvar::AllocVar;
    use crate::constraint_system::ConstraintSystem;
    use crate::test_program;
    use bitcoin_circle_stark::treepp::*;
    use rand::SeedableRng;
    use rand_chacha::ChaCha20Rng;

    #[test]
    fn test_m31_mult() {
        let mut prng = ChaCha20Rng::seed_from_u64(0);

        let mut a_val = rand_m31(&mut prng);

        let cs = ConstraintSystem::new_ref();

        let mut a = M31Var::new_program_input(&cs, a_val).unwrap();

        for _ in 0..10 {
            let b_val = rand_m31(&mut prng);

            let b = M31Var::new_constant(&cs, b_val).unwrap();

            let c = &a * &b;
            let c_val = a_val * b_val;
            assert_eq!(c.value, c_val);

            a = c;
            a_val = c_val;
        }

        cs.set_program_output(&a).unwrap();

        test_program(
            cs,
            script! {
                { a_val.0 }
            },
        )
        .unwrap();
    }
}
