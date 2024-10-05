use crate::builtins::qm31::QM31Var;
use crate::builtins::str::StrVar;
use crate::bvar::{dummy_script, AllocVar, AllocationMode, BVar};
use crate::constraint_system::{ConstraintSystemRef, Element};
use crate::options::Options;
use crate::stack::Stack;
use anyhow::Result;
use bitcoin::opcodes::all::OP_CAT;
use bitcoin::opcodes::Ordinary::OP_SHA256;
use bitcoin::script::write_scriptint;
use bitcoin_circle_stark::treepp::*;
use sha2::digest::Update;
use sha2::{Digest, Sha256};
use std::ops::Add;

pub struct HashVar {
    pub variable: usize,
    pub value: Vec<u8>,
    pub cs: ConstraintSystemRef,
}

impl BVar for HashVar {
    type Value = Vec<u8>;

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
        Ok(self.value.clone())
    }
}

impl AllocVar for HashVar {
    fn new_variable(
        cs: &ConstraintSystemRef,
        data: <Self as BVar>::Value,
        mode: AllocationMode,
    ) -> Result<Self> {
        Ok(Self {
            variable: cs.alloc(Element::Str(data.clone()), mode)?,
            value: data,
            cs: cs.clone(),
        })
    }
}

impl Add for &HashVar {
    type Output = HashVar;

    fn add(self, rhs: Self) -> HashVar {
        let cs = self.cs.and(&rhs.cs());

        let mut sha256 = Sha256::new();
        Update::update(&mut sha256, &rhs.value);
        Update::update(&mut sha256, &self.value);
        let hash = sha256.finalize().to_vec();

        cs.insert_script(hash_combine, [rhs.variable, self.variable])
            .unwrap();
        HashVar::new_function_output(&cs, hash).unwrap()
    }
}

impl Add<&QM31Var> for &HashVar {
    type Output = HashVar;

    fn add(self, rhs: &QM31Var) -> HashVar {
        let felt_hash = HashVar::from(rhs);
        self + &felt_hash
    }
}

impl<T: BVar> From<&T> for HashVar {
    fn from(v: &T) -> HashVar {
        let variables = v.variables();
        let cs = v.cs();

        let mut cur_hash = Option::<Vec<u8>>::None;
        for &variable in variables.iter().rev() {
            let mut sha256 = Sha256::new();
            match cs.get_element(variable).unwrap() {
                Element::Num(v) => {
                    Update::update(&mut sha256, &bitcoin_num_to_bytes(v as i64));
                }
                Element::Str(v) => {
                    Update::update(&mut sha256, &v);
                }
            }
            if let Some(cur_hash) = cur_hash {
                Update::update(&mut sha256, &cur_hash);
            }
            cur_hash = Some(sha256.finalize().to_vec());
        }

        let len = variables.len() as u32;
        let options = Options::new().with_u32("len", len);
        cs.insert_script_complex(hash_many, variables, &options)
            .unwrap();

        HashVar::new_function_output(&cs, cur_hash.unwrap()).unwrap()
    }
}

impl From<&HashVar> for StrVar {
    fn from(v: &HashVar) -> StrVar {
        let cs = v.cs();
        cs.insert_script(dummy_script, v.variables()).unwrap();
        StrVar::new_function_output(&cs, v.value().unwrap()).unwrap()
    }
}

fn hash_many(_: &mut Stack, options: &Options) -> Result<Script> {
    let len = options.get_u32("len")?;
    Ok(script! {
        OP_SHA256
        for _ in 0..len - 1 {
            OP_CAT OP_SHA256
        }
    })
}

fn hash_combine() -> Script {
    Script::from(vec![OP_CAT.to_u8(), OP_SHA256.to_u8()])
}

pub(crate) fn bitcoin_num_to_bytes(v: i64) -> Vec<u8> {
    let mut buf = [0u8; 8];
    let l = write_scriptint(&mut buf, v);
    buf[0..l].to_vec()
}
