use crate::options::Options;
use crate::stack::Stack;
use anyhow::Result;
use bitcoin_circle_stark::treepp::Script;

#[derive(Clone, Debug)]
pub enum ScriptGenerator {
    Simple(fn() -> Script),
    Complex(fn(&mut Stack, &Options) -> Result<Script>),
}

impl ScriptGenerator {
    pub fn run(&self, stack: &mut Stack, options: &Options) -> Result<Script> {
        match self {
            ScriptGenerator::Simple(f) => Ok(f()),
            ScriptGenerator::Complex(f) => f(stack, options),
        }
    }
}
