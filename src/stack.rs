use anyhow::{Error, Result};
use fenwick_tree::FenwickTree;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StackElementStatus {
    ABSENT,
    PRESENT(usize),
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

    pub fn push_to_stack(&mut self, idx: usize, num_elements: usize) -> Result<()> {
        if self.bitmap[idx] != StackElementStatus::ABSENT {
            return Err(Error::msg(
                "The stack seems to already have these elements.",
            ));
        }
        self.bitmap[idx] = StackElementStatus::PRESENT(num_elements);
        self.fenwick_tree.add(idx, num_elements as isize)?;
        Ok(())
    }

    pub fn is_present(&self, idx: usize) -> Result<bool> {
        Ok(matches!(self.bitmap[idx], StackElementStatus::PRESENT(_)))
    }

    pub fn pull(&mut self, idx: usize) -> Result<()> {
        match self.bitmap[idx] {
            StackElementStatus::PRESENT(num_elements) => {
                self.bitmap[idx] = StackElementStatus::PULLED;
                self.fenwick_tree.add(idx, -(num_elements as isize))?;

                Ok(())
            }
            _ => Err(Error::msg(
                "Only elements present in the stack can be pulled aside.",
            )),
        }
    }

    pub fn get_relative_position(&mut self, idx: usize) -> Result<usize> {
        if !matches!(self.bitmap[idx], StackElementStatus::PRESENT(_)) {
            return Err(Error::msg("Only elements in the stack can have the relative position to the top of the stack."));
        }
        let sum = self.fenwick_tree.sum(idx..self.size)?;
        Ok((sum - 1) as usize)
    }

    pub fn get_length(&mut self, idx: usize) -> Result<usize> {
        match self.bitmap[idx]{
            StackElementStatus::PRESENT(num_elements) => {
                Ok(num_elements)
            }
            _ => {
                Err(Error::msg("Only elements in the stack can have the relative position to the top of the stack."))
            }
        }
    }

    pub fn get_num_elements_in_stack(&self) -> Result<usize> {
        Ok(self.fenwick_tree.sum(0..self.size)? as usize)
    }
}

#[cfg(test)]
mod test {
    use crate::stack::Stack;

    #[test]
    fn stack_test() {
        let mut stack = Stack::new(5);
        stack.push_to_stack(0, 10).unwrap();
        assert_eq!(stack.get_relative_position(0).unwrap(), 9);
        assert!(stack.get_relative_position(1).is_err());
        assert!(stack.get_relative_position(2).is_err());
        assert!(stack.get_relative_position(3).is_err());
        assert!(stack.get_relative_position(4).is_err());

        stack.push_to_stack(1, 15).unwrap();
        assert_eq!(stack.get_relative_position(0).unwrap(), 24);
        assert_eq!(stack.get_relative_position(1).unwrap(), 14);
        assert!(stack.get_relative_position(2).is_err());
        assert!(stack.get_relative_position(3).is_err());
        assert!(stack.get_relative_position(4).is_err());

        stack.push_to_stack(2, 25).unwrap();
        assert_eq!(stack.get_relative_position(0).unwrap(), 49);
        assert_eq!(stack.get_relative_position(1).unwrap(), 39);
        assert_eq!(stack.get_relative_position(2).unwrap(), 24);
        assert!(stack.get_relative_position(3).is_err());
        assert!(stack.get_relative_position(4).is_err());

        stack.pull(1).unwrap();
        assert_eq!(stack.get_relative_position(0).unwrap(), 34);
        assert!(stack.get_relative_position(1).is_err());
        assert_eq!(stack.get_relative_position(2).unwrap(), 24);
        assert!(stack.get_relative_position(3).is_err());
        assert!(stack.get_relative_position(4).is_err());

        stack.push_to_stack(3, 2).unwrap();
        assert_eq!(stack.get_relative_position(0).unwrap(), 36);
        assert!(stack.get_relative_position(1).is_err());
        assert_eq!(stack.get_relative_position(2).unwrap(), 26);
        assert_eq!(stack.get_relative_position(3).unwrap(), 1);
        assert!(stack.get_relative_position(4).is_err());

        stack.pull(0).unwrap();
        assert!(stack.get_relative_position(0).is_err());
        assert!(stack.get_relative_position(1).is_err());
        assert_eq!(stack.get_relative_position(2).unwrap(), 26);
        assert_eq!(stack.get_relative_position(3).unwrap(), 1);
        assert!(stack.get_relative_position(4).is_err());

        stack.push_to_stack(4, 18).unwrap();
        assert!(stack.get_relative_position(0).is_err());
        assert!(stack.get_relative_position(1).is_err());
        assert_eq!(stack.get_relative_position(2).unwrap(), 44);
        assert_eq!(stack.get_relative_position(3).unwrap(), 19);
        assert_eq!(stack.get_relative_position(4).unwrap(), 17);
    }
}
