use crate::builtins::m31::M31Var;
use crate::builtins::table::m31::{M31Limbs, M31LimbsGadget, M31Mult, M31MultGadget};
use crate::builtins::table::utils::{
    check_limb_format, convert_m31_from_limbs, convert_m31_to_limbs, pow2147483645, OP_256MUL,
};
use crate::builtins::table::TableVar;
use crate::bvar::{AllocVar, AllocationMode, BVar};
use crate::constraint_system::{ConstraintSystemRef, Element};
use crate::options::Options;
use crate::stack::Stack;
use crate::treepp::*;
use anyhow::Result;
use std::ops::{Add, Mul};

pub struct M31LimbsVar {
    pub variables: [usize; 4],
    pub value: [u32; 4],
    pub cs: ConstraintSystemRef,
}

impl BVar for M31LimbsVar {
    type Value = [u32; 4];

    fn cs(&self) -> ConstraintSystemRef {
        self.cs.clone()
    }

    fn variables(&self) -> Vec<usize> {
        self.variables.to_vec()
    }

    fn length() -> usize {
        4
    }

    fn value(&self) -> Result<Self::Value> {
        Ok(self.value.clone())
    }
}

impl AllocVar for M31LimbsVar {
    fn new_variable(
        cs: &ConstraintSystemRef,
        data: <Self as BVar>::Value,
        mode: AllocationMode,
    ) -> Result<Self> {
        let mut variables = [0usize; 4];
        for (v, &elem) in variables.iter_mut().zip(data.iter()) {
            *v = cs.alloc(Element::Num(elem as i32), mode)?;
        }
        Ok(Self {
            variables,
            value: data,
            cs: cs.clone(),
        })
    }
}

impl From<&M31Var> for M31LimbsVar {
    fn from(v: &M31Var) -> Self {
        let cs = v.cs();
        let num = v.value().unwrap();

        let limbs = [
            num & 0xff,
            (num >> 8) & 0xff,
            (num >> 16) & 0xff,
            (num >> 24) & 0xff,
        ];

        let res = M31LimbsVar::new_hint(&cs, limbs).unwrap();
        cs.insert_script(
            m31_to_limbs_gadget,
            v.variables().iter().chain(res.variables().iter()).copied(),
            &Options::new(),
        )
        .unwrap();
        res
    }
}

impl Mul<(&TableVar, &M31LimbsVar)> for &M31LimbsVar {
    type Output = M31Var;

    fn mul(self, rhs: (&TableVar, &M31LimbsVar)) -> Self::Output {
        let table = rhs.0;
        let rhs = rhs.1;

        let cs = self.cs().and(&table.cs()).and(&rhs.cs());

        let res = ((convert_m31_from_limbs(&self.value) as i64
            * convert_m31_from_limbs(&rhs.value) as i64)
            % ((1i64 << 31) - 1)) as u32;

        let c_limbs = M31Mult::compute_c_limbs_from_limbs(&self.value, &rhs.value).unwrap();

        let q = M31Mult::compute_q(&c_limbs).unwrap();
        let q_var = M31Var::new_hint(&cs, q).unwrap();

        let options = Options::new().with_u32("table_ref", table.variables[0] as u32);
        cs.insert_script(
            m31_limbs_mul_gadget,
            self.variables()
                .iter()
                .chain(rhs.variables().iter())
                .chain(q_var.variables().iter())
                .copied(),
            &options,
        )
        .unwrap();

        let res_var = M31Var::new_function_output(&cs, res).unwrap();
        res_var
    }
}

impl M31LimbsVar {
    pub fn inverse(&self, table: &TableVar) -> M31LimbsVar {
        let cs = self.cs();

        let inv = pow2147483645(convert_m31_from_limbs(&self.value));
        let inv_limbs = convert_m31_to_limbs(inv);

        let inv_limbs_var = M31LimbsVar::new_hint(&cs, inv_limbs).unwrap();

        let expected_one = self * (table, &inv_limbs_var);
        expected_one.is_one();

        inv_limbs_var
    }
}

impl M31LimbsVar {
    pub fn equalverify(&self, rhs: &Self) -> Result<()> {
        assert_eq!(self.value, rhs.value);

        let cs = self.cs.and(&rhs.cs());
        cs.insert_script(
            m31_limbs_equalverify_gadget,
            self.variables.iter().chain(rhs.variables.iter()).copied(),
            &Options::new(),
        )
    }
}

impl Add<&M31LimbsVar> for &M31LimbsVar {
    type Output = M31LimbsVar;

    fn add(self, rhs: &M31LimbsVar) -> Self::Output {
        let new_limbs = M31Limbs::add_limbs(&self.value, &rhs.value);

        let cs = self.cs().and(&rhs.cs());
        cs.insert_script(
            m31_limbs_add_gadget,
            self.variables().iter().chain(rhs.variables.iter()).copied(),
            &Options::new(),
        )
        .unwrap();

        M31LimbsVar::new_function_output(
            &cs,
            [new_limbs[0], new_limbs[1], new_limbs[2], new_limbs[3]],
        )
        .unwrap()
    }
}

pub(crate) fn m31_to_limbs_gadget(_: &mut Stack, _: &Options) -> Result<Script> {
    // input: m31_var, limb1..4
    Ok(script! {
        check_limb_format
        OP_256MUL OP_SWAP
        check_limb_format OP_ADD

        OP_256MUL OP_SWAP
        check_limb_format OP_ADD

        OP_256MUL OP_SWAP
        check_limb_format OP_ADD

        OP_EQUALVERIFY
    })
}

fn m31_limbs_equalverify_gadget(_: &mut Stack, _: &Options) -> Result<Script> {
    Ok(script! {
        4 OP_ROLL OP_EQUALVERIFY
        3 OP_ROLL OP_EQUALVERIFY
        OP_ROT OP_EQUALVERIFY
        OP_EQUALVERIFY
    })
}

fn m31_limbs_mul_gadget(stack: &mut Stack, options: &Options) -> Result<Script> {
    let last_table_elem = options.get_u32("table_ref")?;
    let k = stack.get_relative_position(last_table_elem as usize)? - 512;

    Ok(script! {
        OP_TOALTSTACK
        { M31MultGadget::compute_c_limbs(k) }
        OP_FROMALTSTACK
        { M31MultGadget::reduce() }
    })
}

fn m31_limbs_add_gadget(_: &mut Stack, _: &Options) -> Result<Script> {
    Ok(M31LimbsGadget::add_limbs())
}

#[cfg(test)]
mod test {
    use crate::builtins::m31::M31Var;
    use crate::builtins::m31_limbs::M31LimbsVar;
    use crate::builtins::table::m31::M31Limbs;
    use crate::builtins::table::utils::{convert_m31_to_limbs, mul_m31, rand_m31};
    use crate::builtins::table::TableVar;
    use crate::bvar::AllocVar;
    use crate::constraint_system::ConstraintSystem;
    use crate::test_program;
    use crate::treepp::*;
    use rand::SeedableRng;
    use rand_chacha::ChaCha20Rng;

    #[test]
    fn test_m31_to_limbs() {
        let mut prng = ChaCha20Rng::seed_from_u64(0);

        let a_val = rand_m31(&mut prng);

        let cs = ConstraintSystem::new_ref();

        let a = M31Var::new_constant(&cs, a_val).unwrap();
        let a_limbs = M31LimbsVar::from(&a);

        cs.set_program_output(&a_limbs).unwrap();

        test_program(
            cs,
            script! {
                { convert_m31_to_limbs(a_val).to_vec() }
            },
        )
        .unwrap();
    }

    #[test]
    fn test_m31_limbs_equalverify() {
        let mut prng = ChaCha20Rng::seed_from_u64(0);

        let a_val = rand_m31(&mut prng);
        let cs = ConstraintSystem::new_ref();

        let a = M31Var::new_constant(&cs, a_val).unwrap();
        let a_limbs = M31LimbsVar::from(&a);
        let a2_limbs = M31LimbsVar::from(&a);

        a_limbs.equalverify(&a2_limbs).unwrap();

        test_program(cs, script! {}).unwrap();
    }

    #[test]
    fn test_m31_limbs_table_mul() {
        let mut prng = ChaCha20Rng::seed_from_u64(0);

        let a_val = rand_m31(&mut prng);
        let b_val = rand_m31(&mut prng);
        let cs = ConstraintSystem::new_ref();

        let a = M31Var::new_constant(&cs, a_val).unwrap();
        let a_limbs = M31LimbsVar::from(&a);

        let b = M31Var::new_constant(&cs, b_val).unwrap();
        let b_limbs = M31LimbsVar::from(&b);

        let table = TableVar::new_constant(&cs, ()).unwrap();
        let res = &a_limbs * (&table, &b_limbs);

        cs.set_program_output(&res).unwrap();

        test_program(
            cs,
            script! {
                { mul_m31(a_val, b_val) }
            },
        )
        .unwrap();
    }

    #[test]
    fn test_m31_limbs_inverse() {
        let mut prng = ChaCha20Rng::seed_from_u64(0);

        let a_val = rand_m31(&mut prng);

        let cs = ConstraintSystem::new_ref();

        let a = M31Var::new_constant(&cs, a_val).unwrap();
        let a_limbs = M31LimbsVar::from(&a);

        let table = TableVar::new_constant(&cs, ()).unwrap();

        let a_inv_limbs = a_limbs.inverse(&table);

        let res = &a_limbs * (&table, &a_inv_limbs);

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
    fn test_m31_limbs_add() {
        let mut prng = ChaCha20Rng::seed_from_u64(0);

        let a_val = rand_m31(&mut prng);
        let b_val = rand_m31(&mut prng);

        let cs = ConstraintSystem::new_ref();

        let a_var = M31Var::new_constant(&cs, a_val).unwrap();
        let a_limbs_var = M31LimbsVar::from(&a_var);
        let b_var = M31Var::new_constant(&cs, b_val).unwrap();
        let b_limbs_var = M31LimbsVar::from(&b_var);

        let a_limbs = convert_m31_to_limbs(a_val);
        let b_limbs = convert_m31_to_limbs(b_val);
        let sum_limbs = M31Limbs::add_limbs(&a_limbs, &b_limbs);

        let sum_limbs_var = &a_limbs_var + &b_limbs_var;
        cs.set_program_output(&sum_limbs_var).unwrap();

        test_program(
            cs,
            script! {
                { sum_limbs }
            },
        )
        .unwrap();
    }
}
