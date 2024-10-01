use crate::constraint_system::ConstraintSystemRef;
use anyhow::Result;

/// This trait describes some core functionality that is common to high-level variables.
pub trait BVar {
    /// The type of the "native" value that `Self` represents in the constraint
    /// system.
    type Value: core::fmt::Debug + Eq + Clone;

    /// Returns the underlying `ConstraintSystemRef`.
    fn cs(&self) -> ConstraintSystemRef;

    /// Returns the assigned stack elements indices.
    fn variable(&self) -> Vec<usize>;

    /// Returns the length (in terms of number of elements in the stack) of the value.
    fn length() -> usize;

    /// Returns the value that is assigned to `self` in the underlying
    /// `ConstraintSystem`.
    fn value(&self) -> Result<Self::Value>;
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AllocationMode {
    INPUT,
    OUTPUT,
    CONSTANT,
    HINT,
}

pub trait AllocVar: BVar + Sized {
    fn new_variable(
        cs: &ConstraintSystemRef,
        data: <Self as BVar>::Value,
        mode: AllocationMode,
    ) -> Result<Self>;

    fn new_constant(cs: &ConstraintSystemRef, data: <Self as BVar>::Value) -> Result<Self> {
        Self::new_variable(cs, data, AllocationMode::CONSTANT)
    }

    fn new_input(cs: &ConstraintSystemRef, data: <Self as BVar>::Value) -> Result<Self> {
        Self::new_variable(cs, data, AllocationMode::INPUT)
    }

    fn new_output(cs: &ConstraintSystemRef, data: <Self as BVar>::Value) -> Result<Self> {
        Self::new_variable(cs, data, AllocationMode::OUTPUT)
    }
}
