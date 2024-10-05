use crate::builtins::cm31_limbs::CM31LimbsVar;
use crate::builtins::m31::M31Var;
use crate::builtins::table::utils::mul_cm31;
use crate::builtins::table::TableVar;
use crate::bvar::{AllocVar, AllocationMode, BVar};
use crate::constraint_system::ConstraintSystemRef;
use crate::options::Options;
use crate::stack::Stack;
use crate::treepp::Script;
use anyhow::Result;
use std::ops::{Add, Mul, Sub};

pub struct CM31Var {
    pub imag: M31Var,
    pub real: M31Var,
}

impl BVar for CM31Var {
    type Value = (u32, u32);

    fn cs(&self) -> ConstraintSystemRef {
        self.real.cs.and(&self.imag.cs)
    }

    fn variables(&self) -> Vec<usize> {
        vec![self.imag.variable, self.real.variable]
    }

    fn length() -> usize {
        2
    }

    fn value(&self) -> Result<Self::Value> {
        Ok((self.real.value, self.imag.value))
    }
}

impl AllocVar for CM31Var {
    fn new_variable(
        cs: &ConstraintSystemRef,
        data: <Self as BVar>::Value,
        mode: AllocationMode,
    ) -> Result<Self> {
        let imag = M31Var::new_variable(cs, data.1, mode)?;
        let real = M31Var::new_variable(cs, data.0, mode)?;

        Ok(Self { imag, real })
    }
}

impl Add for &CM31Var {
    type Output = CM31Var;

    fn add(self, rhs: Self) -> Self::Output {
        let imag = &self.imag + &rhs.imag;
        let real = &self.real + &rhs.real;

        CM31Var { imag, real }
    }
}

impl Add<&M31Var> for &CM31Var {
    type Output = CM31Var;

    fn add(self, rhs: &M31Var) -> Self::Output {
        let imag = self.imag.clone().unwrap();
        let real = &self.real + rhs;

        CM31Var { imag, real }
    }
}

impl Sub for &CM31Var {
    type Output = CM31Var;

    fn sub(self, rhs: Self) -> Self::Output {
        let imag = &self.imag - &rhs.imag;
        let real = &self.real - &rhs.real;

        CM31Var { imag, real }
    }
}

impl Sub<&M31Var> for &CM31Var {
    type Output = CM31Var;

    fn sub(self, rhs: &M31Var) -> Self::Output {
        let imag = self.imag.clone().unwrap();
        let real = &self.real - rhs;

        CM31Var { imag, real }
    }
}

impl Mul for &CM31Var {
    type Output = CM31Var;

    fn mul(self, rhs: Self) -> Self::Output {
        let res = mul_cm31(self.value().unwrap(), rhs.value().unwrap());
        let cs = self.cs().and(&rhs.cs());

        cs.insert_script(
            cm31_mult_gadget,
            self.variables()
                .iter()
                .chain(rhs.variables().iter())
                .copied(),
            &Options::new(),
        )
        .unwrap();

        CM31Var::new_function_output(&cs, res).unwrap()
    }
}

impl Mul<(&TableVar, &CM31Var)> for &CM31Var {
    type Output = CM31Var;

    fn mul(self, rhs: (&TableVar, &CM31Var)) -> Self::Output {
        let table = rhs.0;
        let rhs = rhs.1;

        let self_limbs = CM31LimbsVar::from(self);
        let rhs_limbs = CM31LimbsVar::from(rhs);
        &self_limbs * (table, &rhs_limbs)
    }
}

impl CM31Var {
    pub fn inverse(&self, table: &TableVar) -> Self {
        // 1 / (a + bi) = (a - bi) / (a^2 + b^2).
        let real_squared = &self.real * (table, &self.real);
        let imag_squared = &self.imag * (table, &self.imag);
        let denom = &real_squared + &imag_squared;

        let denom_inverse = denom.inverse(table);

        let real = &self.real * (table, &denom_inverse);
        let imag = &(-&self.imag) * (table, &denom_inverse);

        Self { real, imag }
    }

    pub fn inverse_without_table(&self) -> Self {
        // 1 / (a + bi) = (a - bi) / (a^2 + b^2).
        let real_squared = &self.real * &self.real;
        let imag_squared = &self.imag * &self.imag;
        let denom = &real_squared + &imag_squared;

        let denom_inverse = denom.inverse_without_table();

        let real = &self.real * &denom_inverse;
        let imag = &(-&self.imag) * &denom_inverse;

        Self { real, imag }
    }
}

fn cm31_mult_gadget(_: &mut Stack, _: &Options) -> Result<Script> {
    Ok(rust_bitcoin_m31::cm31_mul())
}

#[cfg(test)]
mod test {
    use crate::builtins::cm31::CM31Var;
    use crate::builtins::table::utils::rand_cm31;
    use crate::builtins::table::TableVar;
    use crate::bvar::AllocVar;
    use crate::constraint_system::ConstraintSystem;
    use crate::test_program;
    use crate::treepp::*;
    use rand::SeedableRng;
    use rand_chacha::ChaCha20Rng;

    #[test]
    fn cm31_inverse() {
        let mut prng = ChaCha20Rng::seed_from_u64(0);

        let a_val = rand_cm31(&mut prng);

        let cs = ConstraintSystem::new_ref();

        let a = CM31Var::new_constant(&cs, a_val).unwrap();
        let table = TableVar::new_constant(&cs, ()).unwrap();

        let a_inv = a.inverse(&table);
        let res = &a * (&table, &a_inv);

        cs.set_program_output(&res).unwrap();

        test_program(
            cs,
            script! {
                0
                1
            },
        )
        .unwrap();
    }

    #[test]
    fn cm31_inverse_without_table() {
        let mut prng = ChaCha20Rng::seed_from_u64(0);

        let a_val = rand_cm31(&mut prng);

        let cs = ConstraintSystem::new_ref();

        let a = CM31Var::new_constant(&cs, a_val).unwrap();

        let a_inv = a.inverse_without_table();
        let res = &a * &a_inv;

        cs.set_program_output(&res).unwrap();

        test_program(
            cs,
            script! {
                0
                1
            },
        )
        .unwrap();
    }
}
