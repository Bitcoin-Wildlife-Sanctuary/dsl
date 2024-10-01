use crate::bvar::{AllocationMode, BVar};
use crate::options::Options;
use crate::stack::Stack;
use crate::treepp::pushable::{Builder, Pushable};
use crate::treepp::Script;
use anyhow::{Error, Result};
use indexmap::IndexMap;
use std::cell::RefCell;
use std::cmp::PartialEq;
use std::rc::Rc;

/// A shared reference to a constraint system that can be stored in high level
/// variables.
#[derive(Clone, Debug)]
pub struct ConstraintSystemRef(pub(crate) Rc<RefCell<ConstraintSystem>>);

impl PartialEq for &ConstraintSystemRef {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for &ConstraintSystemRef {}

impl ConstraintSystemRef {
    pub fn and(&self, other: &Self) -> Self {
        assert_eq!(self, other);
        self.clone()
    }

    pub fn alloc(&self, data: Element, mode: AllocationMode) -> Result<usize> {
        self.0.borrow_mut().alloc(data, mode)
    }

    pub fn insert_script(
        &self,
        script_generator: fn(&mut Stack, &Options) -> Result<Script>,
        input_idxs: impl IntoIterator<Item = usize>,
        options: &Options,
    ) -> Result<()> {
        self.0
            .borrow_mut()
            .insert_script(script_generator, input_idxs, options)
    }

    pub fn set_execution_output(&self, var: &impl BVar) -> Result<()> {
        self.0.borrow_mut().set_execution_output(var)
    }
}

#[derive(Debug)]
pub struct ConstraintSystem {
    pub memory: IndexMap<usize, Element>,
    pub memory_last_idx: usize,
    pub trace: Vec<TraceEntry>,
    pub num_inputs: Option<usize>,
}

#[derive(Clone, Debug)]
pub enum Element {
    Num(i32),
    Str(Vec<u8>),
}

impl Pushable for &Element {
    fn bitcoin_script_push(&self, builder: Builder) -> Builder {
        match self {
            Element::Num(v) => v.bitcoin_script_push(builder),
            Element::Str(v) => v.bitcoin_script_push(builder),
        }
    }
}

#[derive(Clone, Debug)]
pub enum TraceEntry {
    InsertScript(
        fn(&mut Stack, &Options) -> Result<Script>,
        Vec<usize>,
        Options,
    ),
    DeclareConstant(usize),
    DeclareOutput(usize),
    RequestHint(usize),
    SystemOutput(usize),
}

impl ConstraintSystem {
    pub fn new() -> Self {
        Self {
            memory: IndexMap::new(),
            memory_last_idx: 0,
            trace: vec![],
            num_inputs: None,
        }
    }

    pub fn new_ref() -> ConstraintSystemRef {
        let sys = Self::new();
        ConstraintSystemRef(Rc::new(RefCell::new(sys)))
    }

    pub fn alloc(&mut self, data: Element, mode: AllocationMode) -> Result<usize> {
        if mode != AllocationMode::INPUT {
            if self.num_inputs.is_none() {
                self.num_inputs = Some(self.memory_last_idx);
            }
        } else {
            if self.num_inputs.is_some() {
                return Err(Error::msg(
                    "Inputs can only be allocated before any execution or allocation for constants or hints",
                ));
            }
        }

        let idx = self.memory_last_idx;
        self.memory_last_idx += 1;

        if self.memory.get(&idx).is_some() {
            return Err(Error::msg("Memory is corrupted"));
        }
        self.memory.insert(idx, data);

        if mode == AllocationMode::CONSTANT {
            self.trace.push(TraceEntry::DeclareConstant(idx));
        } else if mode == AllocationMode::HINT {
            self.trace.push(TraceEntry::RequestHint(idx));
        } else if mode == AllocationMode::OUTPUT {
            self.trace.push(TraceEntry::DeclareOutput(idx));
        }

        Ok(idx)
    }

    pub fn set_execution_output(&mut self, var: &impl BVar) -> Result<()> {
        let indices = var.variable();
        for &index in indices.iter() {
            if self.memory.get(&index).is_none() {
                return Err(Error::msg(
                    "Could not find the memory entry with the given index",
                ));
            }
            self.trace.push(TraceEntry::SystemOutput(index));
        }
        Ok(())
    }

    pub fn get_num(&mut self, idx: usize) -> Result<i32> {
        match self.memory.get(&idx) {
            Some(Element::Num(v)) => Ok(*v),
            _ => Err(Error::msg(
                "Cannot read the requested data in memory as a number",
            )),
        }
    }

    pub fn get_str(&mut self, idx: usize) -> Result<&[u8]> {
        match self.memory.get(&idx) {
            Some(Element::Str(v)) => Ok(v.as_slice()),
            _ => Err(Error::msg(
                "Cannot read the requested data in memory as a string",
            )),
        }
    }

    pub fn insert_script(
        &mut self,
        script_generator: fn(&mut Stack, &Options) -> Result<Script>,
        input_idxs: impl IntoIterator<Item = usize>,
        options: &Options,
    ) -> Result<()> {
        if self.num_inputs.is_none() {
            self.num_inputs = Some(self.memory_last_idx);
        }

        self.trace.push(TraceEntry::InsertScript(
            script_generator,
            input_idxs.into_iter().collect(),
            options.clone(),
        ));

        Ok(())
    }
}
