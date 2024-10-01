use anyhow::{Error, Result};
use fenwick_tree::FenwickTree;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StackElementStatus {
    ABSENT,
    PRESENT,
    PULLED,
}

pub struct Stack {
    pub bitmap: Vec<StackElementStatus>,
    pub fenwick_tree: FenwickTree<isize>,
    pub size: usize,
}

impl Stack {
    pub fn new(size: usize) -> Self {
        Self {
            bitmap: vec![StackElementStatus::ABSENT; size],
            fenwick_tree: FenwickTree::with_len(size),
            size,
        }
    }

    pub fn push_to_stack(&mut self, idx: usize) -> Result<()> {
        if self.bitmap[idx] != StackElementStatus::ABSENT {
            return Err(Error::msg(
                "The stack seems to already have these elements.",
            ));
        }
        self.bitmap[idx] = StackElementStatus::PRESENT;
        self.fenwick_tree.add(idx, 1)?;
        Ok(())
    }

    pub fn is_present(&self, idx: usize) -> Result<bool> {
        Ok(matches!(self.bitmap[idx], StackElementStatus::PRESENT))
    }

    pub fn pull(&mut self, idx: usize) -> Result<()> {
        match self.bitmap[idx] {
            StackElementStatus::PRESENT => {
                self.bitmap[idx] = StackElementStatus::PULLED;
                self.fenwick_tree.add(idx, -1)?;

                Ok(())
            }
            _ => Err(Error::msg(
                "Only elements present in the stack can be pulled aside.",
            )),
        }
    }

    pub fn get_relative_position(&mut self, idx: usize) -> Result<usize> {
        if !matches!(self.bitmap[idx], StackElementStatus::PRESENT) {
            return Err(Error::msg("Only elements in the stack can have the relative position to the top of the stack."));
        }
        let sum = self.fenwick_tree.sum(idx..self.size)?;
        Ok((sum - 1) as usize)
    }

    pub fn get_num_elements_in_stack(&self) -> Result<usize> {
        Ok(self.fenwick_tree.sum(0..self.size)? as usize)
    }
}
