use crate::bvar::{AllocVar, AllocationMode, BVar};
use crate::constraint_system::{ConstraintSystemRef, Element};
use anyhow::Result;
use bitcoin_circle_stark::treepp::*;
use std::ops::{Add, Sub};

#[derive(Debug, Clone)]
pub struct U8Var {
    pub variable: usize,
    pub value: u8,
    pub cs: ConstraintSystemRef,
}

impl BVar for U8Var {
    type Value = u8;

    fn cs(&self) -> ConstraintSystemRef {
        self.cs.clone()
    }

    fn variables(&self) -> Vec<usize> {
        vec![self.variable]
    }

    fn length() -> usize {
        1
    }

    fn value(&self) -> anyhow::Result<Self::Value> {
        Ok(self.value)
    }
}

impl AllocVar for U8Var {
    fn new_variable(
        cs: &ConstraintSystemRef,
        data: <Self as BVar>::Value,
        mode: AllocationMode,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            variable: cs.alloc(Element::Num(data as i32), mode)?,
            value: data,
            cs: cs.clone(),
        })
    }
}

impl Add for &U8Var {
    type Output = U8Var;

    fn add(self, rhs: Self) -> Self::Output {
        let res = self.value.checked_add(rhs.value).unwrap();

        let cs = self.cs.and(&rhs.cs);

        cs.insert_script(u8_add, [self.variable, rhs.variable])
            .unwrap();

        let res_var = U8Var::new_variable(&cs, res, AllocationMode::FunctionOutput).unwrap();
        res_var
    }
}

fn u8_add() -> Script {
    script! {
        OP_ADD
    }
}

impl Sub for &U8Var {
    type Output = U8Var;

    fn sub(self, rhs: Self) -> Self::Output {
        let res = self.value.checked_sub(rhs.value).unwrap();

        let cs = self.cs.and(&rhs.cs);

        cs.insert_script(u8_sub, [self.variable, rhs.variable])
            .unwrap();

        let res_var = U8Var::new_variable(&cs, res, AllocationMode::FunctionOutput).unwrap();
        res_var
    }
}

fn u8_sub() -> Script {
    script! {
        OP_SUB
    }
}

impl U8Var {
    pub fn check_format(&self) -> Result<()> {
        self.cs.insert_script(u8_check_format, [self.variable])
    }
}

fn u8_check_format() -> Script {
    script! {
        OP_DUP 0 OP_GREATERTHANOREQUAL OP_VERIFY
        255 OP_LESSTHANOREQUAL OP_VERIFY
    }
}

#[cfg(test)]
mod test {
    use crate::builtins::u8::U8Var;
    use crate::bvar::{AllocVar, AllocationMode};
    use crate::constraint_system::{ConstraintSystem, Element};
    use crate::test_program;
    use bitcoin_circle_stark::treepp::*;

    #[test]
    fn test_add_u8() {
        let cs = ConstraintSystem::new_ref();

        let a = U8Var::new_constant(&cs, 8).unwrap();
        let b = U8Var::new_constant(&cs, 4).unwrap();

        let c = &a + &b;
        c.check_format().unwrap();
        cs.set_program_output(&c).unwrap();
        test_program(cs, script! { 12 }).unwrap();
    }

    #[test]
    fn test_sub_u8() {
        let cs = ConstraintSystem::new_ref();
        let a = U8Var::new_constant(&cs, 8).unwrap();
        let b = U8Var::new_constant(&cs, 3).unwrap();

        let c = &a - &b;
        c.check_format().unwrap();
        cs.set_program_output(&c).unwrap();
        test_program(cs, script! { 5 }).unwrap();
    }

    #[test]
    #[should_panic]
    fn test_add_u8_overflow() {
        let cs = ConstraintSystem::new_ref();

        let a = U8Var::new_constant(&cs, 8).unwrap();
        let b = U8Var::new_constant(&cs, 248).unwrap();

        let _ = &a + &b;
    }

    #[test]
    #[should_panic]
    fn test_sub_u8_overflow() {
        let cs = ConstraintSystem::new_ref();

        let a = U8Var::new_constant(&cs, 8).unwrap();
        let b = U8Var::new_constant(&cs, 9).unwrap();

        let _ = &a - &b;
    }

    #[test]
    fn test_check_format() {
        let cs = ConstraintSystem::new_ref();

        let a = U8Var::new_constant(&cs, 8).unwrap();
        a.check_format().unwrap();
        test_program(cs, script! {}).unwrap();
    }

    #[test]
    #[should_panic]
    fn test_check_format_overflow() {
        let cs = ConstraintSystem::new_ref();

        let mut a = U8Var::new_constant(&cs, 8).unwrap();
        a.variable = cs
            .alloc(Element::Num(-1), AllocationMode::Constant)
            .unwrap();
        a.check_format().unwrap();
        test_program(cs, script! {}).unwrap();
    }
}
