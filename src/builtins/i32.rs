use crate::builtins::u8::U8Var;
use crate::bvar::{AllocVar, AllocationMode, BVar};
use crate::constraint_system::{ConstraintSystemRef, Element};
use crate::options::Options;
use crate::stack::Stack;
use anyhow::Result;
use bitcoin_circle_stark::treepp::*;
use std::ops::{Add, Sub};

#[derive(Debug, Clone)]
pub struct I32Var {
    pub variable: usize,
    pub value: i32,
    pub cs: ConstraintSystemRef,
}

impl BVar for I32Var {
    type Value = i32;

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

impl AllocVar for I32Var {
    fn new_variable(
        cs: &ConstraintSystemRef,
        data: <Self as BVar>::Value,
        mode: AllocationMode,
    ) -> Result<Self> {
        assert!(data > i32::MIN);
        Ok(Self {
            variable: cs.alloc(Element::Num(data), mode)?,
            value: data,
            cs: cs.clone(),
        })
    }
}

impl Add for &I32Var {
    type Output = I32Var;

    fn add(self, rhs: Self) -> Self::Output {
        let res = self.value.checked_add(rhs.value).unwrap();
        assert!(res > i32::MIN);

        let cs = self.cs().and(&rhs.cs);

        cs.insert_script(i32_add, [self.variable, rhs.variable])
            .unwrap();

        let res_var = I32Var::new_variable(&cs, res, AllocationMode::FunctionOutput).unwrap();
        res_var
    }
}

impl Add<&U8Var> for &I32Var {
    type Output = I32Var;

    fn add(self, rhs: &U8Var) -> Self::Output {
        let res = self.value.checked_add(rhs.value as i32).unwrap();
        assert!(res > i32::MIN);

        let cs = self.cs().and(&rhs.cs);

        cs.insert_script(i32_add, [self.variable, rhs.variable])
            .unwrap();

        let res_var = I32Var::new_variable(&cs, res, AllocationMode::FunctionOutput).unwrap();
        res_var
    }
}

fn i32_add() -> Script {
    script! {
        OP_ADD
    }
}

impl Sub for &I32Var {
    type Output = I32Var;

    fn sub(self, rhs: Self) -> Self::Output {
        let res = self.value.checked_sub(rhs.value).unwrap();
        assert!(res > i32::MIN);

        let cs = self.cs().and(&rhs.cs);

        cs.insert_script(i32_sub, [self.variable, rhs.variable])
            .unwrap();

        let res_var = I32Var::new_variable(&cs, res, AllocationMode::FunctionOutput).unwrap();
        res_var
    }
}

impl Sub<&U8Var> for &I32Var {
    type Output = I32Var;

    fn sub(self, rhs: &U8Var) -> Self::Output {
        let res = self.value.checked_sub(rhs.value as i32).unwrap();
        assert!(res > i32::MIN);

        let cs = self.cs().and(&rhs.cs);

        cs.insert_script(i32_sub, [self.variable, rhs.variable])
            .unwrap();

        let res_var = I32Var::new_variable(&cs, res, AllocationMode::FunctionOutput).unwrap();
        res_var
    }
}

fn i32_sub() -> Script {
    script! {
        OP_SUB
    }
}

impl I32Var {
    pub fn check_format(&self) -> Result<()> {
        self.cs.insert_script(i32_check_format, [self.variable])
    }

    pub fn to_positive_limbs(&self, l: usize) -> Result<Vec<U8Var>> {
        assert!(l <= 8);
        assert!(self.value >= 0);

        let mut value = self.value as u32;
        let mut res = vec![];

        let n = (32 + l - 1) / l;

        for _ in 0..n {
            res.push(value & ((1 << l) - 1));
            value >>= l;
        }

        let cs = self.cs();
        let mut res_vars = vec![];
        for &v in res.iter() {
            res_vars.push(U8Var::new_hint(&cs, v as u8)?);
        }

        let mut variables = vec![self.variable];
        for res_var in res_vars.iter() {
            variables.push(res_var.variable);
        }

        cs.insert_script_complex(
            i32_to_positive_limbs_check,
            variables,
            &Options::new()
                .with_u32("n", n as u32)
                .with_u32("l", l as u32),
        )?;

        Ok(res_vars)
    }
}

fn i32_check_format() -> Script {
    script! {
        OP_ABS OP_DROP
    }
}

fn i32_to_positive_limbs_check(_: &mut Stack, options: &Options) -> Result<Script> {
    let n = options.get_u32("n")? as usize;
    let l = options.get_u32("l")? as usize;

    Ok(script! {
        for i in 0..n {
            OP_DUP 0 OP_GREATERTHANOREQUAL OP_VERIFY
            OP_DUP { 1 << l } OP_LESSTHAN OP_VERIFY

            if i != 0 {
                OP_FROMALTSTACK
                OP_ADD
            }

            if i != n - 1 {
                for _ in 0..l {
                    OP_DUP OP_ADD
                }
                OP_TOALTSTACK
            }
        }

        OP_EQUALVERIFY
    })
}

#[cfg(test)]
mod test {
    use crate::builtins::i32::I32Var;
    use crate::builtins::u8::U8Var;
    use crate::bvar::{AllocVar, AllocationMode};
    use crate::constraint_system::{ConstraintSystem, Element};
    use crate::test_program;
    use bitcoin_circle_stark::treepp::*;
    use num_traits::abs;
    use rand::{Rng, SeedableRng};
    use rand_chacha::ChaCha20Rng;

    #[test]
    fn test_add_i32() {
        let cs = ConstraintSystem::new_ref();

        let a = I32Var::new_constant(&cs, i32::MAX).unwrap();
        let b = I32Var::new_constant(&cs, -1).unwrap();

        let c = &a + &b;
        c.check_format().unwrap();
        cs.set_program_output(&c).unwrap();
        test_program(cs, script! { { i32::MAX - 1 } }).unwrap();
    }

    #[test]
    fn test_add_i32_u8() {
        let cs = ConstraintSystem::new_ref();

        let a = I32Var::new_constant(&cs, i32::MAX - 1).unwrap();
        let b = U8Var::new_constant(&cs, 1).unwrap();

        let c = &a + &b;
        c.check_format().unwrap();
        cs.set_program_output(&c).unwrap();
        test_program(cs, script! { { i32::MAX } }).unwrap();
    }

    #[test]
    fn test_sub_i32() {
        let cs = ConstraintSystem::new_ref();

        let a = I32Var::new_constant(&cs, i32::MIN + 2).unwrap();
        let b = I32Var::new_constant(&cs, -1).unwrap();

        let c = &a - &b;
        c.check_format().unwrap();
        cs.set_program_output(&c).unwrap();
        test_program(cs, script! { { i32::MIN + 3 } }).unwrap();
    }

    #[test]
    fn test_sub_i32_u8() {
        let cs = ConstraintSystem::new_ref();

        let a = I32Var::new_constant(&cs, i32::MIN + 2).unwrap();
        let b = U8Var::new_constant(&cs, 1).unwrap();

        let c = &a - &b;
        c.check_format().unwrap();
        cs.set_program_output(&c).unwrap();
        test_program(cs, script! { { i32::MIN + 1 } }).unwrap();
    }

    #[test]
    #[should_panic]
    fn test_add_i32_overflow() {
        let cs = ConstraintSystem::new_ref();

        let a = I32Var::new_constant(&cs, i32::MAX).unwrap();
        let b = I32Var::new_constant(&cs, 1).unwrap();
        let _ = &a + &b;
    }

    #[test]
    #[should_panic]
    fn test_add_i32_overflow2() {
        let cs = ConstraintSystem::new_ref();

        let a = I32Var::new_constant(&cs, i32::MAX).unwrap();
        let b = I32Var::new_constant(&cs, -1).unwrap();
        let _ = &a - &b;
    }

    #[test]
    #[should_panic]
    fn test_add_i32_u8_overflow() {
        let cs = ConstraintSystem::new_ref();

        let a = I32Var::new_constant(&cs, i32::MAX).unwrap();
        let b = U8Var::new_constant(&cs, 1).unwrap();
        let _ = &a + &b;
    }

    #[test]
    #[should_panic]
    fn test_sub_i32_overflow() {
        let cs = ConstraintSystem::new_ref();

        let a = I32Var::new_constant(&cs, i32::MIN + 1).unwrap();
        let b = I32Var::new_constant(&cs, 1).unwrap();
        let _ = &a - &b;
    }

    #[test]
    #[should_panic]
    fn test_sub_i32_overflow2() {
        let cs = ConstraintSystem::new_ref();

        let a = I32Var::new_constant(&cs, i32::MIN + 1).unwrap();
        let b = I32Var::new_constant(&cs, -1).unwrap();
        let _ = &a + &b;
    }
    #[test]
    #[should_panic]
    fn test_sub_i32_u8_overflow() {
        let cs = ConstraintSystem::new_ref();

        let a = I32Var::new_constant(&cs, i32::MIN + 1).unwrap();
        let b = U8Var::new_constant(&cs, 1).unwrap();
        let _ = &a - &b;
    }

    #[test]
    fn test_check_format() {
        let cs = ConstraintSystem::new_ref();

        let a = I32Var::new_constant(&cs, -8).unwrap();
        a.check_format().unwrap();
        test_program(cs, script! {}).unwrap();
    }

    #[test]
    #[should_panic]
    fn test_check_format_overflow() {
        let cs = ConstraintSystem::new_ref();

        let mut a = I32Var::new_constant(&cs, -8).unwrap();
        a.variable = cs
            .alloc(Element::Num(i32::MIN), AllocationMode::Constant)
            .unwrap();
        a.check_format().unwrap();
        test_program(cs, script! {}).unwrap();
    }

    #[test]
    fn test_i32_to_positive_limbs() {
        let mut prng = ChaCha20Rng::seed_from_u64(0);

        for l in 1..=8 {
            let cs = ConstraintSystem::new_ref();
            let a: i32 = abs(prng.gen::<i32>());

            let a_var = I32Var::new_constant(&cs, a).unwrap();

            let res_var = a_var.to_positive_limbs(l).unwrap();

            let mut expected = vec![];
            let n = (32 + l - 1) / l;

            let mut cur = a as u32;
            for _ in 0..n {
                expected.push(cur & ((1 << l) - 1));
                cur >>= l;
            }

            assert_eq!(res_var.len(), n);
            for i in 0..n {
                cs.set_program_output(&res_var[i]).unwrap();
            }

            test_program(
                cs,
                script! {
                    { expected }
                },
            )
            .unwrap();
        }
    }
}
