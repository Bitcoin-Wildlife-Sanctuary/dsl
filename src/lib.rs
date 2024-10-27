use crate::compiler::Compiler;
use crate::constraint_system::ConstraintSystemRef;
use anyhow::{Error, Result};
use bitcoin::hashes::Hash;
use bitcoin::opcodes::OP_TRUE;
use bitcoin::{TapLeafHash, Transaction};
use bitcoin_circle_stark::treepp::*;
use bitcoin_scriptexec::{convert_to_witness, Exec, ExecCtx, FmtStack, Options, TxTemplate};

pub mod builtins;

pub mod ldm;

pub mod bvar;

pub mod constraint_system;

pub mod examples;

pub mod stack;

pub mod compiler;

pub mod options;

pub mod script_generator;

pub fn test_program(cs: ConstraintSystemRef, expected_stack: Script) -> Result<()> {
    test_program_generic(cs, expected_stack, true)
}

pub fn test_program_without_opcat(cs: ConstraintSystemRef, expected_stack: Script) -> Result<()> {
    test_program_generic(cs, expected_stack, false)
}

fn test_program_generic(
    cs: ConstraintSystemRef,
    expected_stack: Script,
    opcat: bool,
) -> Result<()> {
    let program = Compiler::compile(cs)?;

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
    for elem in expected_final_stack.iter().rev() {
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

    let mut options = Options::default();
    if !opcat {
        options.experimental.op_cat = false;
    };

    let mut exec = Exec::new(
        ExecCtx::Tapscript,
        options,
        TxTemplate {
            tx: Transaction {
                version: bitcoin::transaction::Version::TWO,
                lock_time: bitcoin::locktime::absolute::LockTime::ZERO,
                input: vec![],
                output: vec![],
            },
            prevouts: vec![],
            input_idx: 0,
            taproot_annex_scriptleaf: Some((TapLeafHash::all_zeros(), None)),
        },
        script,
        vec![],
    )
    .expect("error creating exec");

    loop {
        if exec.exec_next().is_err() {
            break;
        }
    }
    let res = exec.result().unwrap();
    if !res.success {
        println!("{:8}", FmtStack(exec.stack().clone()));
        println!("{:?}", res.error);
    }

    println!("max stack size: {}", exec.stats().max_nb_stack_items);

    if res.success {
        Ok(())
    } else {
        Err(Error::msg("Script execution is not successful"))
    }
}
