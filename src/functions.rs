use crate::dsl::{MemoryEntry, DSL};
use crate::treepp::Script;
use anyhow::Result;
use std::collections::HashMap;
use crate::options::Options;

pub struct FunctionRegistry {
    pub map: HashMap<String, AcceptableFunctionMetadata>,
}

impl FunctionRegistry {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }
}

pub struct FunctionMetadata {
    pub trace_generator: fn(&mut DSL, &[usize]) -> Result<FunctionOutput>,
    pub script_generator: fn(&[usize]) -> Result<Script>,
    pub input: Vec<&'static str>,
    pub output: Vec<&'static str>,
}

pub struct FunctionOutput {
    pub new_elements: Vec<MemoryEntry>,
    pub new_hints: Vec<MemoryEntry>,
}

pub struct FunctionWithOptionsMetadata {
    pub trace_generator: fn(&mut DSL, &[usize], &Options) -> Result<FunctionOutput>,
    pub script_generator: fn(&[usize], &Options) -> Result<Script>,
    pub input: Vec<&'static str>,
    pub output: Vec<&'static str>,
}

pub enum AcceptableFunctionMetadata {
    FunctionWithoutOptions(FunctionMetadata),
    FunctionWithOptions(FunctionWithOptionsMetadata),
}

impl Into<AcceptableFunctionMetadata> for FunctionMetadata {
    fn into(self) -> AcceptableFunctionMetadata {
        AcceptableFunctionMetadata::FunctionWithoutOptions(self)
    }
}

impl Into<AcceptableFunctionMetadata> for FunctionWithOptionsMetadata {
    fn into(self) -> AcceptableFunctionMetadata {
        AcceptableFunctionMetadata::FunctionWithOptions(self)
    }
}