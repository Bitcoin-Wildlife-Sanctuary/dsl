use crate::bvar::{AllocVar, AllocationMode, BVar};
use crate::constraint_system::{ConstraintSystemRef, Element};
use crate::options::Options;
use crate::stack::Stack;
use crate::treepp::*;
use anyhow::Result;
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

        let res_var = M31Var::new_variable(&cs, res, AllocationMode::FunctionOutput).unwrap();
        res_var
    }
}

fn m31_mult_gadget(_: &mut Stack, _: &Options) -> Result<Script> {
    Ok(script! {
        { rust_bitcoin_m31::m31_mul() }
    })
}
