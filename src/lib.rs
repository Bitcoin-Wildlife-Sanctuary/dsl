use crate::compiler::Compiler;
use crate::dsl::DSL;
use crate::treepp::Script;
use anyhow::{Error, Result};
use bitcoin::opcodes::all::OP_RETURN;
use bitcoin::opcodes::OP_TRUE;
use bitcoin_script::script;
use bitcoin_scriptexec::{convert_to_witness, execute_script};

pub mod data_type;

pub mod functions;

pub mod dsl;

pub mod examples;

pub mod script;

pub mod stack;

pub mod compiler;

pub(crate) mod treepp {
    pub use bitcoin_script::{define_pushable, script};

    define_pushable!();
    pub use bitcoin::ScriptBuf as Script;
}

use crate::treepp::*;

pub fn test_program(dsl: DSL, expected_stack: Script) -> Result<()> {
    let program = Compiler::compiler(dsl)?;

    let mut script = script! {
        for elem in program.hint.iter() {
            { elem }
        }
        for elem in program.input.iter() {
            { elem }
        }
    }
    .to_bytes();
    script.extend_from_slice(program.script.as_bytes());

    let expected_final_stack = convert_to_witness(expected_stack)
        .map_err(|x| anyhow::Error::msg(format!("final stack parsing error: {:?}", x)))?;
    for elem in expected_final_stack.iter() {
        script.extend_from_slice(
            script! {
                { elem.to_vec() }
                OP_EQUALVERIFY
            }
            .as_bytes(),
        );
    }

    script.push(OP_TRUE.to_u8());

    let script = Script::from_bytes(script);

    println!("script size: {}", script.len());

    let exec_result = execute_script(script);

    println!("max stack size: {}", exec_result.stats.max_nb_stack_items);

    if exec_result.success {
        Ok(())
    } else {
        Err(Error::msg("Script execution is not successful"))
    }
}
