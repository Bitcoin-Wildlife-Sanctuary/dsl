use crate::builtins::cm31::CM31Var;
use crate::builtins::m31_limbs::M31LimbsVar;
use crate::builtins::table::m31::{M31Limbs, M31LimbsGadget};
use crate::builtins::table::TableVar;
use crate::bvar::{AllocVar, BVar};
use crate::constraint_system::ConstraintSystemRef;
use crate::options::Options;
use crate::stack::Stack;
use crate::treepp::Script;
use anyhow::Result;
use std::ops::{Add, Mul};

pub struct CM31LimbsVar {
    pub real: M31LimbsVar,
    pub imag: M31LimbsVar,
}

impl BVar for CM31LimbsVar {
    type Value = ([u32; 4], [u32; 4]);

    fn cs(&self) -> ConstraintSystemRef {
        self.real.cs.and(&self.imag.cs)
    }

    fn variables(&self) -> Vec<usize> {
        let mut variables = self.real.variables();
        variables.extend(self.imag.variables());
        variables
    }

    fn length() -> usize {
        8
    }

    fn value(&self) -> Result<Self::Value> {
        Ok((self.real.value, self.imag.value))
    }
}

impl From<&CM31Var> for CM31LimbsVar {
    fn from(var: &CM31Var) -> Self {
        let real = M31LimbsVar::from(&var.real);
        let imag = M31LimbsVar::from(&var.imag);

        Self { real, imag }
    }
}

impl CM31LimbsVar {
    pub fn equalverify(&self, rhs: &Self) -> Result<()> {
        assert_eq!(self.value()?, rhs.value()?);
        self.real.equalverify(&rhs.real)?;
        self.imag.equalverify(&rhs.imag)?;
        Ok(())
    }
}

impl Mul<(&TableVar, &CM31LimbsVar)> for &CM31LimbsVar {
    type Output = CM31Var;

    fn mul(self, rhs: (&TableVar, &CM31LimbsVar)) -> Self::Output {
        let table = rhs.0;
        let rhs = rhs.1;

        let self_sum = &self.real + &self.imag;
        let rhs_sum = &rhs.real + &rhs.imag;

        let sum_product = &self_sum * (&table, &rhs_sum);
        let real_product = &self.real * (&table, &rhs.real);
        let imag_product = &self.imag * (&table, &rhs.imag);

        let new_real = &real_product - &imag_product;
        let new_imag = &(&sum_product - &real_product) - &imag_product;

        CM31Var {
            real: new_real,
            imag: new_imag,
        }
    }
}

impl Add<&CM31LimbsVar> for &CM31LimbsVar {
    type Output = CM31LimbsVar;

    fn add(self, rhs: &CM31LimbsVar) -> Self::Output {
        let new_real_limbs = M31Limbs::add_limbs_with_reduction(&self.real.value, &rhs.real.value);
        let new_imag_limbs = M31Limbs::add_limbs_with_reduction(&self.imag.value, &rhs.imag.value);

        let cs = self.cs().and(&rhs.cs());
        cs.insert_script(
            m31_limbs_add_with_reduction_gadget,
            self.real
                .variables()
                .iter()
                .chain(rhs.real.variables.iter())
                .copied(),
            &Options::new(),
        )
        .unwrap();
        let real = M31LimbsVar::new_function_output(
            &cs,
            [
                new_real_limbs[0],
                new_real_limbs[1],
                new_real_limbs[2],
                new_real_limbs[3],
            ],
        )
        .unwrap();

        cs.insert_script(
            m31_limbs_add_with_reduction_gadget,
            self.imag
                .variables()
                .iter()
                .chain(rhs.imag.variables.iter())
                .copied(),
            &Options::new(),
        )
        .unwrap();
        let imag = M31LimbsVar::new_function_output(
            &cs,
            [
                new_imag_limbs[0],
                new_imag_limbs[1],
                new_imag_limbs[2],
                new_imag_limbs[3],
            ],
        )
        .unwrap();

        CM31LimbsVar { real, imag }
    }
}

fn m31_limbs_add_with_reduction_gadget(_: &mut Stack, _: &Options) -> Result<Script> {
    Ok(M31LimbsGadget::add_limbs_with_reduction())
}

#[cfg(test)]
mod test {
    use crate::builtins::cm31::CM31Var;
    use crate::builtins::cm31_limbs::CM31LimbsVar;
    use crate::builtins::table::utils::{mul_cm31, rand_cm31};
    use crate::builtins::table::TableVar;
    use crate::bvar::AllocVar;
    use crate::constraint_system::ConstraintSystem;
    use crate::test_program;
    use crate::treepp::*;
    use rand::SeedableRng;
    use rand_chacha::ChaCha20Rng;

    #[test]
    fn test_cm31_limbs_table_mul() {
        let mut prng = ChaCha20Rng::seed_from_u64(0);

        let a_val = rand_cm31(&mut prng);
        let b_val = rand_cm31(&mut prng);
        let expected = mul_cm31(a_val, b_val);

        let cs = ConstraintSystem::new_ref();

        let a = CM31Var::new_constant(&cs, a_val).unwrap();
        let a_limbs = CM31LimbsVar::from(&a);

        let b = CM31Var::new_constant(&cs, b_val).unwrap();
        let b_limbs = CM31LimbsVar::from(&b);

        let table = TableVar::new_constant(&cs, ()).unwrap();
        let res = &a_limbs * (&table, &b_limbs);

        cs.set_program_output(&res).unwrap();

        test_program(
            cs,
            script! {
                { expected.1 }
                { expected.0 }
            },
        )
        .unwrap();
    }
}
