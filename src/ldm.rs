use crate::builtins::hash::HashVar;
use crate::bvar::{AllocVar, BVar};
use crate::constraint_system::ConstraintSystemRef;
use anyhow::Result;
use sha2::Digest;
use std::collections::HashMap;

#[derive(Default)]
pub struct LDM {
    pub name_to_id: HashMap<String, usize>,
    pub value_map: Vec<Vec<u8>>,
    pub hash_map: Vec<Vec<u8>>,

    pub cs: Option<ConstraintSystemRef>,

    pub write_hash_var: Option<HashVar>,
    pub read_hash_var: Option<HashVar>,
    pub read_log: Vec<usize>,
}

impl LDM {
    pub fn new() -> LDM {
        Self::default()
    }

    pub fn init(&mut self, cs: &ConstraintSystemRef) -> Result<()> {
        if self.cs.is_some() {
            let write_hash = self.write_hash_var.as_ref().unwrap().value.clone();
            let read_hash = self.read_hash_var.as_ref().unwrap().value.clone();

            self.cs = Some(cs.clone());

            self.write_hash_var = Some(HashVar::new_program_input(&cs, write_hash)?);
            self.read_hash_var = Some(HashVar::new_program_input(&cs, read_hash)?);
        } else {
            self.cs = Some(cs.clone());

            let default_write_hash = sha2::Sha256::digest(b"write_hash").to_vec();
            let default_read_hash = sha2::Sha256::digest(b"read_hash").to_vec();

            let write_hash_var = HashVar::new_constant(&cs, default_write_hash)?;
            let read_hash_var = HashVar::new_constant(&cs, default_read_hash)?;

            self.write_hash_var = Some(write_hash_var);
            self.read_hash_var = Some(read_hash_var);
        }

        Ok(())
    }

    pub fn write(&mut self, name: impl ToString, value: &impl BVar) -> Result<()> {
        assert!(
            self.cs.is_some(),
            "The WORMMemory is not bound to a constraint system."
        );

        let idx = self.value_map.len();
        self.name_to_id.insert(name.to_string(), idx);

        self.value_map.push(bincode::serialize(&value.value()?)?);

        let hash_var = HashVar::from(value);
        self.hash_map.push(hash_var.value.clone());
        self.write_hash_var = Some(self.write_hash_var.as_ref().unwrap() + &hash_var);

        Ok(())
    }

    pub fn read<T: AllocVar>(&mut self, name: impl ToString) -> Result<T> {
        let idx = self.name_to_id[&name.to_string()];

        let value: T::Value = bincode::deserialize(&self.value_map[idx])?;
        let v = T::new_hint(self.cs.as_ref().unwrap(), value)?;

        self.read_hash_var = Some(self.read_hash_var.as_ref().unwrap() + &HashVar::from(&v));
        self.read_log.push(idx);

        Ok(v)
    }

    pub fn save(&self) -> Result<()> {
        self.cs
            .as_ref()
            .unwrap()
            .set_program_output(self.write_hash_var.as_ref().unwrap())?;
        self.cs
            .as_ref()
            .unwrap()
            .set_program_output(self.read_hash_var.as_ref().unwrap())?;
        Ok(())
    }

    pub fn check(&self) -> Result<()> {
        let mut next_index_to_load = 0;
        let mut map = Vec::<HashVar>::new();
        let cs = self.cs.as_ref().unwrap();

        let default_write_hash = sha2::Sha256::digest(b"write_hash").to_vec();
        let default_read_hash = sha2::Sha256::digest(b"read_hash").to_vec();

        let mut recomputed_write_hash_var = HashVar::new_constant(&cs, default_write_hash)?;
        let mut recomputed_read_hash_var = HashVar::new_constant(&cs, default_read_hash)?;

        let mut read_log_iter = self.read_log.iter().peekable();

        while next_index_to_load < self.value_map.len() {
            // load the next value
            let new_hash_var = HashVar::new_hint(cs, self.hash_map[next_index_to_load].clone())?;
            next_index_to_load += 1;

            // update the current recomputed_write_hash_var
            recomputed_write_hash_var = &recomputed_write_hash_var + &new_hash_var;

            map.push(new_hash_var);

            // peek the next read_log element
            let mut next_read = read_log_iter.peek();
            while next_read.is_some() && **next_read.unwrap() < next_index_to_load {
                let id = *read_log_iter.next().unwrap();
                recomputed_read_hash_var = &recomputed_read_hash_var + &map[id];
                next_read = read_log_iter.peek();
            }
        }

        self.write_hash_var
            .as_ref()
            .unwrap()
            .equalverify(&recomputed_write_hash_var)?;
        self.read_hash_var
            .as_ref()
            .unwrap()
            .equalverify(&recomputed_read_hash_var)?;

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use crate::builtins::m31::M31Var;
    use crate::builtins::table::utils::rand_m31;
    use crate::bvar::{AllocVar, BVar};
    use crate::constraint_system::ConstraintSystem;
    use crate::ldm::LDM;
    use crate::test_program;
    use bitcoin_circle_stark::treepp::*;
    use bitcoin_script::script;
    use rand::SeedableRng;
    use rand_chacha::ChaCha20Rng;

    #[test]
    fn test_ldm() {
        let mut prng = ChaCha20Rng::seed_from_u64(0);

        let a_val = rand_m31(&mut prng);
        let b_val = rand_m31(&mut prng);

        let mut ldm = LDM::new();

        let cs = ConstraintSystem::new_ref();
        ldm.init(&cs).unwrap();

        let a = M31Var::new_constant(&cs, a_val).unwrap();
        let b = M31Var::new_constant(&cs, b_val).unwrap();

        let c = &a * &b;
        let c_val = c.value().unwrap();

        ldm.write("c", &c).unwrap();
        ldm.save().unwrap();

        test_program(
            cs,
            script! {
                { ldm.write_hash_var.as_ref().unwrap().value.clone() }
                { ldm.read_hash_var.as_ref().unwrap().value.clone() }
            },
        )
        .unwrap();

        let cs = ConstraintSystem::new_ref();
        ldm.init(&cs).unwrap();

        let c = ldm.read::<M31Var>("c").unwrap();
        assert_eq!(c.value().unwrap(), c_val);

        ldm.check().unwrap();
        ldm.save().unwrap();

        test_program(
            cs,
            script! {
                { ldm.write_hash_var.as_ref().unwrap().value.clone() }
                { ldm.read_hash_var.as_ref().unwrap().value.clone() }
            },
        )
        .unwrap();
    }
}
