#[cfg(test)]
mod test {
    use crate::bvar::{AllocVar, AllocationMode, BVar};
    use crate::constraint_system::{ConstraintSystem, ConstraintSystemRef, Element};
    use crate::options::Options;
    use crate::stack::Stack;
    use crate::test_program;
    use crate::treepp::*;
    use anyhow::Result;
    use bitcoin::ScriptBuf;
    use rand::{Rng, SeedableRng};
    use rand_chacha::ChaCha20Rng;
    use std::ops::Mul;

    pub struct M31Var {
        pub variable: usize,
        pub value: u32,
        pub cs: ConstraintSystemRef,
    }

    impl BVar for M31Var {
        type Value = u32;

        fn cs(&self) -> ConstraintSystemRef {
            self.cs.clone()
        }

        fn variable(&self) -> Vec<usize> {
            vec![self.variable]
        }

        fn length() -> usize {
            1
        }

        fn value(&self) -> Result<Self::Value> {
            Ok(self.value)
        }
    }

    impl AllocVar for M31Var {
        fn new_variable(
            cs: &ConstraintSystemRef,
            data: <Self as BVar>::Value,
            mode: AllocationMode,
        ) -> Result<Self> {
            Ok(Self {
                variable: cs.alloc(Element::Num(data as i32), mode)?,
                value: data,
                cs: cs.clone(),
            })
        }
    }

    impl Mul for &M31Var {
        type Output = M31Var;

        fn mul(self, rhs: Self) -> Self::Output {
            let res = ((self.value as i64) * (rhs.value as i64) % ((1i64 << 31) - 1)) as u32;

            let cs = self.cs.and(&rhs.cs);

            cs.insert_script(
                m31_mult_gadget,
                [self.variable, rhs.variable],
                &Options::new(),
            )
            .unwrap();

            let res_var = M31Var::new_variable(&cs, res, AllocationMode::OUTPUT).unwrap();
            res_var
        }
    }

    fn m31_mult_gadget(_: &mut Stack, _: &Options) -> Result<ScriptBuf> {
        Ok(script! {
            { rust_bitcoin_m31::m31_mul() }
        })
    }

    #[test]
    fn test_m31_mult() {
        let mut prng = ChaCha20Rng::seed_from_u64(0);

        let mut a_val = prng.gen_range(0..((1i64 << 31) - 1)) as u32;

        let cs = ConstraintSystem::new_ref();

        let mut a = M31Var::new_input(&cs, a_val).unwrap();

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
