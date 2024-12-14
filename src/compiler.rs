use crate::constraint_system::{ConstraintSystemRef, Element, TraceEntry};
use crate::stack::Stack;
use crate::treepp::*;
use anyhow::Result;
use bitcoin::opcodes::Ordinary::{OP_1SUB, OP_2DROP, OP_DEPTH, OP_DROP, OP_FROMALTSTACK, OP_ROLL};
use bitcoin::ScriptBuf;

pub struct CompiledProgram {
    pub input: Vec<Element>,
    pub hint: Vec<Element>,
    pub script: Script,
}

pub struct Compiler;

impl Compiler {
    pub fn compile(cs: ConstraintSystemRef) -> Result<CompiledProgram> {
        let cs = cs.0.borrow_mut();

        // step 1: count the last visit of all the memory entries
        let num_memory_entries = cs.memory_last_idx;
        let mut last_visit = vec![-1isize; num_memory_entries];

        let mut cur_time = 0;
        for trace_entry in cs.trace.iter() {
            match trace_entry {
                TraceEntry::InsertScript(_, inputs, _) => {
                    for &i in inputs.iter() {
                        last_visit[i] = cur_time;
                    }
                    cur_time += 1;
                }
                _ => {}
            }
        }

        // step 2: allocate all the inputs
        let mut input = vec![];
        if let Some(num_inputs) = cs.num_inputs {
            for i in 0..num_inputs {
                input.push(cs.memory.get(&i).unwrap().clone())
            }
        } else {
            let num_inputs = cs.memory_last_idx;
            for i in 0..num_inputs {
                input.push(cs.memory.get(&i).unwrap().clone())
            }
        }

        // step 3: initialize the stack
        let mut stack = Stack::new(cs.memory_last_idx);
        for i in 0..input.len() {
            stack.push_to_stack(i)?;
        }

        // step 4: build the output list
        let mut output = vec![];
        for trace_entry in cs.trace.iter() {
            match trace_entry {
                TraceEntry::SystemOutput(i) => {
                    output.push(*i);
                }
                _ => {}
            }
        }

        // step 5: generate the script
        let mut script = Vec::<u8>::new();
        let mut hint = Vec::<Element>::new();

        let mut cur_time = 0;

        for trace_entry in cs.trace.iter() {
            match trace_entry {
                TraceEntry::InsertScript(script_generator, inputs, options) => {
                    for (i, &input_idx) in inputs.iter().enumerate() {
                        let pos = stack.get_relative_position(input_idx)?;
                        let distance = pos + i;

                        if last_visit[input_idx] == cur_time
                            && !(i < inputs.len() - 1 && inputs[i + 1..].contains(&input_idx))
                            && !output.contains(&input_idx)
                        {
                            // roll
                            stack.pull(input_idx)?;
                            script.extend_from_slice(roll_script(distance).as_bytes());
                        } else {
                            // pick
                            script.extend_from_slice(pick_script(distance).as_bytes());
                        }
                    }

                    script
                        .extend_from_slice(script_generator.run(&mut stack, &options)?.as_bytes());

                    cur_time += 1;
                }
                TraceEntry::DeclareConstant(idx) => {
                    stack.push_to_stack(*idx)?;

                    script.extend_from_slice(
                        script! {
                            { cs.memory.get(idx).unwrap() }
                        }
                        .as_bytes(),
                    );
                }
                TraceEntry::DeclareOutput(idx) => {
                    stack.push_to_stack(*idx)?;
                }
                TraceEntry::RequestHint(idx) => {
                    hint.push(cs.memory.get(idx).unwrap().clone());
                    stack.push_to_stack(*idx)?;

                    script.push(OP_DEPTH as u8);
                    script.push(OP_1SUB as u8);
                    script.push(OP_ROLL as u8);
                }
                TraceEntry::SystemOutput(_) => {}
            }
        }

        // step 4: move the desired output to the altstack
        let mut output_list_rev = output.clone();
        output_list_rev.reverse();

        let mut output_total_len = 0;

        for (i, &idx) in output_list_rev.iter().enumerate() {
            // for each entry, roll or pick the data and then save the data to the altstack
            // - roll, if this is the last occurrence of this idx in `output_list_rev`
            // - pick, if this idx may occur another time in the remainder of `output_list_rev`
            //
            // the list is reversed with the mind that doing so may reduce the pull/roll distance and save the script length

            let pos = stack.get_relative_position(idx)?;

            if output_list_rev[i..].contains(&idx) {
                // pick
                script.extend_from_slice(
                    script! {
                        { pos } OP_PICK
                        OP_TOALTSTACK
                    }
                    .as_bytes(),
                );
            } else {
                // roll
                stack.pull(idx)?;
                script.extend_from_slice(
                    script! {
                        { pos } OP_ROLL
                        OP_TOALTSTACK
                    }
                    .as_bytes(),
                );
            }
            output_total_len += 1;
        }

        // clear all the remaining elements
        let elements_in_stack = stack.get_num_elements_in_stack()?;
        for _ in 0..elements_in_stack / 2 {
            script.push(OP_2DROP.to_u8());
        }
        if elements_in_stack % 2 == 1 {
            script.push(OP_DROP.to_u8());
        }

        // recover the output from the altstack
        for _ in 0..output_total_len {
            script.push(OP_FROMALTSTACK.to_u8());
        }

        Ok(CompiledProgram {
            input,
            script: ScriptBuf::from_bytes(script),
            hint,
        })
    }
}

fn roll_script(distance: usize) -> Script {
    if distance == 0 {
        script! {} // do nothing, it is already on the top of the stack
    } else {
        if distance == 1 {
            script! {
                OP_SWAP
            }
        } else if distance == 2 {
            script! {
                OP_ROT
            }
        } else {
            script! {
                { distance } OP_ROLL
            }
        }
    }
}

fn pick_script(distance: usize) -> Script {
    if distance == 0 {
        script! {
            OP_DUP
        }
    } else if distance == 1 {
        script! {
            OP_OVER
        }
    } else {
        script! {
            { distance } OP_PICK
        }
    }
}
