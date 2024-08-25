use bitcoin_script::script;
use bitcoin_scriptexec::execute_script;
use crate::compiler::Compiler;
use crate::dsl::DSL;
use crate::treepp::Script;

pub mod data_type;

pub mod functions;

pub mod dsl;

pub mod examples;

pub mod script;

pub mod stack;

pub mod compiler;

pub(crate) mod treepp {
    pub use bitcoin_script::{define_pushable, script};
    #[cfg(test)]
    pub use bitcoin_scriptexec::execute_script;

    define_pushable!();
    pub use bitcoin::ScriptBuf as Script;
}

use crate::treepp::*;

pub fn test_program(dsl: DSL) {
    let program = Compiler::compiler(dsl).unwrap();

    let mut script = script! {
        for elem in program.hint.iter() {
            { elem }
        }
        for elem in program.input.iter() {
            { elem }
        }
    }.to_bytes();
    script.extend_from_slice(program.script.as_bytes());

    let script = Script::from_bytes(script);

    let exec_result = execute_script(script);
    assert!(exec_result.success);
}