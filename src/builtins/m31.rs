use crate::builtins::m31_limbs::{m31_to_limbs_gadget, M31LimbsVar};
use crate::builtins::table::utils::pow2147483645;
use crate::builtins::table::TableVar;
use crate::bvar::{AllocVar, AllocationMode, BVar};
use crate::constraint_system::{ConstraintSystemRef, Element};
use crate::options::Options;
use crate::stack::Stack;
use crate::treepp::*;
use anyhow::Result;
use bitcoin::opcodes::Ordinary::OP_EQUALVERIFY;
use std::ops::{Add, Mul, Sub};

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

    fn variables(&self) -> Vec<usize> {
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

impl Add for &M31Var {
    type Output = M31Var;

    fn add(self, rhs: Self) -> Self::Output {
        let res = ((self.value as i64) + (rhs.value as i64) % ((1i64 << 31) - 1)) as u32;

        let cs = self.cs.and(&rhs.cs);

        cs.insert_script(
            m31_add_gadget,
            [self.variable, rhs.variable],
            &Options::new(),
        )
        .unwrap();

        let res_var = M31Var::new_variable(&cs, res, AllocationMode::FunctionOutput).unwrap();
        res_var
    }
}

impl Sub for &M31Var {
    type Output = M31Var;

    fn sub(self, rhs: Self) -> Self::Output {
        let res = ((self.value as i64) + ((1i64 << 31) - 1)
            - (rhs.value as i64) % ((1i64 << 31) - 1)) as u32;

        let cs = self.cs.and(&rhs.cs);

        cs.insert_script(
            m31_sub_gadget,
            [self.variable, rhs.variable],
            &Options::new(),
        )
        .unwrap();

        let res_var = M31Var::new_variable(&cs, res, AllocationMode::FunctionOutput).unwrap();
        res_var
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

        M31Var::new_function_output(&cs, res).unwrap()
    }
}

impl Mul<(&TableVar, &M31Var)> for &M31Var {
    type Output = M31Var;

    fn mul(self, rhs: (&TableVar, &M31Var)) -> Self::Output {
        let table = rhs.0;
        let rhs = rhs.1;

        let self_limbs = M31LimbsVar::from(self);
        let rhs_limbs = M31LimbsVar::from(rhs);
        &self_limbs * (table, &rhs_limbs)
    }
}

impl M31Var {
    pub fn is_one(&self) {
        assert_eq!(self.value, 1);
        self.cs
            .insert_script(m31_is_one_gadget, [self.variable], &Options::new())
            .unwrap();
    }

    pub fn inverse(&self, table: &TableVar) -> Self {
        let self_limbs = M31LimbsVar::from(self);
        let inv_limbs = self_limbs.inverse(&table);

        let cs = self.cs.and(&table.cs);
        let inv = M31Var::new_hint(&cs, pow2147483645(self.value)).unwrap();

        cs.insert_script(
            m31_to_limbs_gadget,
            inv.variables()
                .iter()
                .chain(inv_limbs.variables().iter())
                .copied(),
            &Options::new(),
        )
        .unwrap();

        inv
    }

    pub fn inverse_without_table(&self) -> Self {
        let inv = M31Var::new_hint(&self.cs, pow2147483645(self.value)).unwrap();

        let res = self * &inv;
        res.is_one();

        inv
    }

    pub fn equalverify(&self, rhs: &Self) -> Result<()> {
        assert_eq!(self.value, rhs.value);

        let cs = self.cs.and(&rhs.cs());
        cs.insert_script(
            m31_equalverify_gadget,
            [self.variable, rhs.variable],
            &Options::new(),
        )
    }
}

fn m31_add_gadget(_: &mut Stack, _: &Options) -> Result<Script> {
    Ok(rust_bitcoin_m31::m31_add())
}

fn m31_sub_gadget(_: &mut Stack, _: &Options) -> Result<Script> {
    Ok(rust_bitcoin_m31::m31_sub())
}

fn m31_mult_gadget(_: &mut Stack, _: &Options) -> Result<Script> {
    Ok(rust_bitcoin_m31::m31_mul())
}

fn m31_is_one_gadget(_: &mut Stack, _: &Options) -> Result<Script> {
    Ok(script! {
        1 OP_EQUALVERIFY
    })
}

fn m31_equalverify_gadget(_: &mut Stack, _: &Options) -> Result<Script> {
    Ok(Script::from(vec![OP_EQUALVERIFY.to_u8()]))
}

#[cfg(test)]
mod test {
    use crate::builtins::m31::M31Var;
    use crate::builtins::table::utils::rand_m31;
    use crate::builtins::table::TableVar;
    use crate::bvar::AllocVar;
    use crate::constraint_system::ConstraintSystem;
    use crate::test_program;
    use crate::treepp::*;
    use rand::SeedableRng;
    use rand_chacha::ChaCha20Rng;

    #[test]
    fn test_m31_inverse() {
        let mut prng = ChaCha20Rng::seed_from_u64(0);

        let a_val = rand_m31(&mut prng);

        let cs = ConstraintSystem::new_ref();

        let a = M31Var::new_constant(&cs, a_val).unwrap();
        let table = TableVar::new_constant(&cs, ()).unwrap();

        let a_inv = a.inverse(&table);
        let res = &a * (&table, &a_inv);

        cs.set_program_output(&res).unwrap();

        test_program(
            cs,
            script! {
                1
            },
        )
        .unwrap();
    }

    #[test]
    fn test_m31_inverse_without_table() {
        let mut prng = ChaCha20Rng::seed_from_u64(0);

        let a_val = rand_m31(&mut prng);

        let cs = ConstraintSystem::new_ref();

        let a = M31Var::new_constant(&cs, a_val).unwrap();
        let a_inv = a.inverse_without_table();
        let res = &a * &a_inv;

        cs.set_program_output(&res).unwrap();

        test_program(
            cs,
            script! {
                1
            },
        )
        .unwrap();
    }
}
