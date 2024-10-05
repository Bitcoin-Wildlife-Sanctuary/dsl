use crate::builtins::cm31::CM31Var;
use crate::builtins::m31::M31Var;
use crate::builtins::qm31_limbs::QM31LimbsVar;
use crate::builtins::table::TableVar;
use crate::bvar::{AllocVar, AllocationMode, BVar};
use crate::constraint_system::ConstraintSystemRef;
use anyhow::Result;
use bitcoin_circle_stark::treepp::*;
use num_traits::One;
use rust_bitcoin_m31::{m31_add_n31, m31_sub, push_m31_one, push_n31_one, qm31_swap};
use std::ops::{Add, Mul, Neg, Sub};
use stwo_prover::core::fields::m31::M31;
use stwo_prover::core::fields::qm31::QM31;
use stwo_prover::core::fields::FieldExpOps;

pub struct QM31Var {
    pub first: CM31Var,
    pub second: CM31Var,
}

impl BVar for QM31Var {
    type Value = QM31;

    fn cs(&self) -> ConstraintSystemRef {
        self.first.cs().and(&self.second.cs())
    }

    fn variables(&self) -> Vec<usize> {
        vec![
            self.second.imag.variable,
            self.second.real.variable,
            self.first.imag.variable,
            self.first.real.variable,
        ]
    }

    fn length() -> usize {
        4
    }

    fn value(&self) -> Result<Self::Value> {
        Ok(QM31(self.first.value()?, self.second.value()?))
    }
}

impl AllocVar for QM31Var {
    fn new_variable(
        cs: &ConstraintSystemRef,
        data: <Self as BVar>::Value,
        mode: AllocationMode,
    ) -> Result<Self> {
        let second = CM31Var::new_variable(cs, data.1, mode)?;
        let first = CM31Var::new_variable(cs, data.0, mode)?;

        Ok(Self { first, second })
    }
}

impl Add for &QM31Var {
    type Output = QM31Var;

    fn add(self, rhs: Self) -> Self::Output {
        let second = &self.second + &rhs.second;
        let first = &self.first + &rhs.first;

        QM31Var { first, second }
    }
}

impl Add<&CM31Var> for &QM31Var {
    type Output = QM31Var;

    fn add(self, rhs: &CM31Var) -> Self::Output {
        let second = self.second.clone().unwrap();
        let first = &self.first + rhs;

        QM31Var { first, second }
    }
}

impl Add<&M31Var> for QM31Var {
    type Output = QM31Var;

    fn add(self, rhs: &M31Var) -> Self::Output {
        let second = self.second.clone().unwrap();
        let first = &self.first + rhs;

        QM31Var { first, second }
    }
}

impl Sub for &QM31Var {
    type Output = QM31Var;

    fn sub(self, rhs: Self) -> Self::Output {
        let second = &self.second - &rhs.second;
        let first = &self.first - &rhs.first;

        QM31Var { first, second }
    }
}

impl Sub<&CM31Var> for &QM31Var {
    type Output = QM31Var;

    fn sub(self, rhs: &CM31Var) -> Self::Output {
        let second = self.second.clone().unwrap();
        let first = &self.first - rhs;

        QM31Var { first, second }
    }
}

impl Sub<&M31Var> for QM31Var {
    type Output = QM31Var;

    fn sub(self, rhs: &M31Var) -> Self::Output {
        let second = self.second.clone().unwrap();
        let first = &self.first - rhs;

        QM31Var { first, second }
    }
}

impl Mul<(&TableVar, &QM31Var)> for &QM31Var {
    type Output = QM31Var;

    fn mul(self, rhs: (&TableVar, &QM31Var)) -> Self::Output {
        let table = rhs.0;
        let rhs = rhs.1;

        let self_limbs = QM31LimbsVar::from(self);
        let rhs_limbs = QM31LimbsVar::from(rhs);
        &self_limbs * (table, &rhs_limbs)
    }
}

impl Mul for &QM31Var {
    type Output = QM31Var;

    fn mul(self, rhs: Self) -> Self::Output {
        let res = self.value().unwrap() * rhs.value().unwrap();
        let cs = self.cs().and(&rhs.cs());

        cs.insert_script(
            rust_bitcoin_m31::qm31_mul,
            self.variables()
                .iter()
                .chain(rhs.variables().iter())
                .copied(),
        )
        .unwrap();

        QM31Var::new_function_output(&cs, res).unwrap()
    }
}

impl Neg for &QM31Var {
    type Output = QM31Var;

    fn neg(self) -> Self::Output {
        let first = -(&self.first);
        let second = -(&self.second);

        QM31Var { first, second }
    }
}

impl QM31Var {
    pub fn is_one(&self) {
        assert_eq!(self.value().unwrap(), QM31::from_u32_unchecked(1, 0, 0, 0));
        self.first.is_one();
        self.second.is_zero();
    }

    pub fn add1(&self) -> QM31Var {
        let mut res = self.value().unwrap();
        res.0 .0 = res.0 .0 + M31::one();
        let cs = self.cs();

        cs.insert_script(qm31_1add_gadget, self.variables())
            .unwrap();

        QM31Var::new_function_output(&cs, res).unwrap()
    }

    pub fn sub1(&self) -> QM31Var {
        let mut res = self.value().unwrap();
        res.0 .0 = res.0 .0 - M31::one();
        let cs = self.cs();

        cs.insert_script(qm31_1sub_gadget, self.variables())
            .unwrap();

        QM31Var::new_function_output(&cs, res).unwrap()
    }

    pub fn shift_by_i(&self) -> QM31Var {
        let first = self.first.shift_by_i();
        let second = self.second.shift_by_i();

        QM31Var { first, second }
    }

    pub fn shift_by_j(&self) -> QM31Var {
        let first = self.second.clone().unwrap();
        let mut second = &self.first + &self.first;
        second.real = &second.real + &self.first.imag;
        second.imag = &second.imag - &self.first.real;

        QM31Var { first, second }
    }

    pub fn shift_by_ij(&self) -> QM31Var {
        self.shift_by_i().shift_by_j()
    }

    pub fn inverse(&self, table: &TableVar) -> QM31Var {
        let cs = self.cs();
        let res = self.value().unwrap().inverse();

        let res_var = QM31Var::new_hint(&cs, res).unwrap();
        let expected_one = &res_var * (table, self);
        expected_one.is_one();

        res_var
    }

    pub fn inverse_without_table(&self) -> QM31Var {
        let cs = self.cs();
        let res = self.value().unwrap().inverse();

        let res_var = QM31Var::new_hint(&cs, res).unwrap();
        let expected_one = &res_var * self;
        expected_one.is_one();

        res_var
    }

    pub fn conditional_swap(&self, rhs: &QM31Var, bit: &M31Var) -> (QM31Var, QM31Var) {
        assert!(bit.value.0 == 0 || bit.value.0 == 1);

        let res = if bit.value.0 == 0 {
            (self.value().unwrap(), rhs.value().unwrap())
        } else {
            (rhs.value().unwrap(), self.value().unwrap())
        };

        let cs = self.cs().and(&rhs.cs()).and(&bit.cs());

        cs.insert_script(
            qm31_conditional_swap_gadget,
            self.variables()
                .iter()
                .chain(rhs.variables().iter())
                .chain(bit.variables().iter())
                .copied(),
        )
        .unwrap();

        let res_1_var = QM31Var::new_function_output(&cs, res.0).unwrap();
        let res_2_var = QM31Var::new_function_output(&cs, res.1).unwrap();

        (res_1_var, res_2_var)
    }
}

fn qm31_1add_gadget() -> Script {
    script! {
        push_n31_one
        m31_add_n31
    }
}

fn qm31_1sub_gadget() -> Script {
    script! {
        push_m31_one
        m31_sub
    }
}

fn qm31_conditional_swap_gadget() -> Script {
    script! {
        OP_IF
            qm31_swap
        OP_ENDIF
    }
}

#[cfg(test)]
mod test {
    use crate::builtins::qm31::QM31Var;
    use crate::builtins::table::utils::rand_qm31;
    use crate::builtins::table::TableVar;
    use crate::bvar::AllocVar;
    use crate::constraint_system::ConstraintSystem;
    use crate::test_program;
    use bitcoin_circle_stark::treepp::*;
    use rand::SeedableRng;
    use rand_chacha::ChaCha20Rng;

    #[test]
    fn qm31_inverse() {
        let mut prng = ChaCha20Rng::seed_from_u64(0);

        let a_val = rand_qm31(&mut prng);

        let cs = ConstraintSystem::new_ref();

        let a = QM31Var::new_constant(&cs, a_val).unwrap();
        let table = TableVar::new_constant(&cs, ()).unwrap();

        let a_inv = a.inverse(&table);
        let res = &a * (&table, &a_inv);

        cs.set_program_output(&res).unwrap();

        test_program(
            cs,
            script! {
                0
                0
                0
                1
            },
        )
        .unwrap();
    }

    #[test]
    fn qm31_inverse_without_table() {
        let mut prng = ChaCha20Rng::seed_from_u64(0);

        let a_val = rand_qm31(&mut prng);

        let cs = ConstraintSystem::new_ref();

        let a = QM31Var::new_constant(&cs, a_val).unwrap();

        let a_inv = a.inverse_without_table();
        let res = &a * &a_inv;

        cs.set_program_output(&res).unwrap();

        test_program(
            cs,
            script! {
                0
                0
                0
                1
            },
        )
        .unwrap();
    }
}