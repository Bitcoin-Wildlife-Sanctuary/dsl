use crate::bvar::{AllocVar, AllocationMode, BVar};
use crate::constraint_system::{ConstraintSystemRef, Element};
use anyhow::Result;
use bitcoin_circle_stark::treepp::*;
use std::ops::{BitAnd, BitOr, BitXor, Not};

#[derive(Debug, Clone)]
pub struct BoolVar {
    pub variable: usize,
    pub value: bool,
    pub cs: ConstraintSystemRef,
}

impl BVar for BoolVar {
    type Value = bool;

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

impl AllocVar for BoolVar {
    fn new_variable(
        cs: &ConstraintSystemRef,
        data: <Self as BVar>::Value,
        mode: AllocationMode,
    ) -> Result<Self> {
        let num = if data { 1 } else { 0 };
        Ok(Self {
            variable: cs.alloc(Element::Num(num), mode)?,
            value: data,
            cs: cs.clone(),
        })
    }
}

impl Not for &BoolVar {
    type Output = BoolVar;

    fn not(self) -> Self::Output {
        self.cs
            .insert_script(bool_var_not, self.variables())
            .unwrap();
        BoolVar::new_function_output(&self.cs, !self.value).unwrap()
    }
}

fn bool_var_not() -> Script {
    script! {
        OP_NOT
    }
}

impl BitAnd<&BoolVar> for &BoolVar {
    type Output = BoolVar;

    fn bitand(self, rhs: &BoolVar) -> Self::Output {
        self.cs
            .insert_script(bool_var_and, vec![self.variable, rhs.variable])
            .unwrap();
        BoolVar::new_function_output(&self.cs, self.value & rhs.value).unwrap()
    }
}

fn bool_var_and() -> Script {
    script! {
        OP_AND
    }
}

impl BitOr<&BoolVar> for &BoolVar {
    type Output = BoolVar;

    fn bitor(self, rhs: &BoolVar) -> Self::Output {
        self.cs
            .insert_script(bool_var_or, vec![self.variable, rhs.variable])
            .unwrap();
        BoolVar::new_function_output(&self.cs, self.value | rhs.value).unwrap()
    }
}

fn bool_var_or() -> Script {
    script! {
        OP_OR
    }
}

impl BitXor<&BoolVar> for &BoolVar {
    type Output = BoolVar;

    fn bitxor(self, rhs: &BoolVar) -> Self::Output {
        self.cs
            .insert_script(bool_var_xor, vec![self.variable, rhs.variable])
            .unwrap();
        BoolVar::new_function_output(&self.cs, self.value ^ rhs.value).unwrap()
    }
}

fn bool_var_xor() -> Script {
    script! {
        // x 0 -> x
        // x 1 -> !x
        OP_IF OP_NOT OP_ENDIF
    }
}

impl BoolVar {
    pub fn verify(self) {
        assert!(self.value);
        self.cs
            .insert_script(bool_var_verify, vec![self.variable])
            .unwrap()
    }
}

fn bool_var_verify() -> Script {
    script! {
        OP_VERIFY
    }
}
