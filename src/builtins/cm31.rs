use crate::builtins::m31::M31Var;
use crate::bvar::{AllocVar, AllocationMode, BVar};
use crate::constraint_system::ConstraintSystemRef;
use anyhow::Result;
use std::ops::Add;

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
