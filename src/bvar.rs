use crate::constraint_system::ConstraintSystemRef;
use anyhow::Result;
use serde::de::DeserializeOwned;
use serde::Serialize;

/// This trait describes some core functionality that is common to high-level variables.
pub trait BVar {
    /// The type of the "native" value that `Self` represents in the constraint
    /// system.
    type Value: core::fmt::Debug + Eq + Clone + Serialize + DeserializeOwned;

    /// Returns the underlying `ConstraintSystemRef`.
    fn cs(&self) -> ConstraintSystemRef;

    /// Returns the assigned stack elements indices.
    fn variables(&self) -> Vec<usize>;

    /// Returns the length (in terms of number of elements in the stack) of the value.
    fn length() -> usize;

    /// Returns the value that is assigned to `self` in the underlying
    /// `ConstraintSystem`.
    fn value(&self) -> Result<Self::Value>;
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AllocationMode {
    ProgramInput,
    FunctionOutput,
    Constant,
    Hint,
}

pub trait AllocVar: BVar + Sized {
    fn new_variable(
        cs: &ConstraintSystemRef,
        data: <Self as BVar>::Value,
        mode: AllocationMode,
    ) -> Result<Self>;

    fn new_constant(cs: &ConstraintSystemRef, data: <Self as BVar>::Value) -> Result<Self> {
        Self::new_variable(cs, data, AllocationMode::Constant)
    }

    fn new_program_input(cs: &ConstraintSystemRef, data: <Self as BVar>::Value) -> Result<Self> {
        Self::new_variable(cs, data, AllocationMode::ProgramInput)
    }

    fn new_function_output(cs: &ConstraintSystemRef, data: <Self as BVar>::Value) -> Result<Self> {
        Self::new_variable(cs, data, AllocationMode::FunctionOutput)
    }

    fn new_hint(cs: &ConstraintSystemRef, data: <Self as BVar>::Value) -> Result<Self> {
        Self::new_variable(cs, data, AllocationMode::Hint)
    }
}