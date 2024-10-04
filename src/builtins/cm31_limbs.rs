use crate::builtins::cm31::CM31Var;
use crate::builtins::m31::M31Var;
use crate::builtins::m31_limbs::M31LimbsVar;
use crate::builtins::table::cm31::{CM31Mult, CM31MultGadget};
use crate::builtins::table::utils::{convert_cm31_from_limbs, mul_cm31};
use crate::builtins::table::TableVar;
use crate::bvar::{AllocVar, BVar};
use crate::constraint_system::ConstraintSystemRef;
use crate::options::Options;
use crate::stack::Stack;
use crate::treepp::*;
use anyhow::Result;
use std::ops::Mul;

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

        let cs = self.cs().and(&table.cs()).and(&rhs.cs());

        let self_cm31 = convert_cm31_from_limbs(&self.value().unwrap());
        let rhs_cm31 = convert_cm31_from_limbs(&rhs.value().unwrap());

        let res = mul_cm31(self_cm31, rhs_cm31);

        let hint = CM31Mult::compute_hint_from_limbs(
            &self.real.value,
            &self.imag.value,
            &rhs.real.value,
            &rhs.imag.value,
        )
        .unwrap();
        let hint_vars = [
            M31Var::new_hint(&cs, hint.q3).unwrap(),
            M31Var::new_hint(&cs, hint.q2).unwrap(),
            M31Var::new_hint(&cs, hint.q1).unwrap(),
        ];

        let options = Options::new().with_u32("table_ref", table.variables[0] as u32);
        cs.insert_script(
            cm31_limbs_mul_gadget,
            hint_vars[0]
                .variables()
                .iter()
                .chain(hint_vars[1].variables().iter())
                .chain(hint_vars[2].variables().iter())
                .chain(self.variables().iter())
                .chain(rhs.variables().iter())
                .copied(),
            &options,
        )
        .unwrap();

        let res_var = CM31Var::new_function_output(&cs, res).unwrap();
        res_var
    }
}

fn cm31_limbs_mul_gadget(stack: &mut Stack, options: &Options) -> Result<Script> {
    let last_table_elem = options.get_u32("table_ref")?;
    let k = stack.get_relative_position(last_table_elem as usize)? - 512;

    Ok(CM31MultGadget::mult(k))
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
