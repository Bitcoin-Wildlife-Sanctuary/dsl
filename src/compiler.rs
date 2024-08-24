use crate::dsl::{TraceEntry, DSL};
use crate::script::CompiledProgram;
use crate::stack::Stack;
use crate::treepp::*;
use anyhow::Result;
use bitcoin::opcodes::Ordinary::{OP_2DROP, OP_DROP};
use bitcoin::opcodes::OP_TRUE;
use bitcoin::ScriptBuf;

pub struct Compiler;

impl Compiler {
    pub fn compiler(dsl: DSL) -> Result<CompiledProgram> {
        // step 1: count the last visit of all the memory entries
        let num_memory_entries = dsl.memory_last_idx;
        let mut last_visit = vec![-1isize; num_memory_entries];

        let mut cur_time = 0;
        for trace_entry in dsl.trace.iter() {
            match trace_entry {
                TraceEntry::FunctionCall(_, inputs) => {
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
        if let Some(num_inputs) = dsl.num_inputs {
            for i in 0..num_inputs {
                input.push(dsl.memory.get(&i).unwrap().clone())
            }
        }

        // step 3: initialize the stack
        let mut stack = Stack::new(dsl.memory_last_idx);
        for (i, input_entry) in input.iter().enumerate() {
            stack.push_to_stack(i, input_entry.data.len())?;
        }

        // step 3: generate the script
        let mut script = Vec::<u8>::new();

        let mut cur_time = 0;
        let mut allocated_idx = dsl.num_inputs.unwrap_or_default();

        for trace_entry in dsl.trace.iter() {
            match trace_entry {
                TraceEntry::FunctionCall(function_name, inputs) => {
                    let function_metadata = dsl
                        .function_registry
                        .map
                        .get(&function_name.to_string())
                        .unwrap();

                    let mut deferred_ref = vec![];
                    let mut num_cloned_input_elements = 0;
                    for (&input_idx, input_type) in
                        inputs.iter().zip(function_metadata.input.iter())
                    {
                        let input_metadata = dsl
                            .data_type_registry
                            .map
                            .get(&input_type.to_string())
                            .unwrap();

                        if input_metadata.ref_only {
                            deferred_ref.push(input_idx);
                            // do not obtain the location of the ref-only element before we clone other inputs.
                        } else {
                            let len = input_metadata.num_elements;
                            let pos = stack.get_relative_position(input_idx)?;

                            if last_visit[input_idx] == cur_time {
                                // roll
                                stack.pull(input_idx)?;

                                script.extend_from_slice(
                                    script! {
                                        for _ in 0..len {
                                            { pos + num_cloned_input_elements } OP_ROLL
                                        }
                                    }
                                    .as_bytes(),
                                );

                                num_cloned_input_elements += len;
                            } else {
                                // pick
                                script.extend_from_slice(
                                    script! {
                                        for _ in 0..len {
                                            { pos + num_cloned_input_elements } OP_PICK
                                        }
                                    }
                                    .as_bytes(),
                                );

                                num_cloned_input_elements += len;
                            }
                        }
                    }

                    // It takes into the account of the elements that disappear due to pull,
                    // but it doesn't consider elements that are just copied/moved near the function stack.
                    let mut ref_positions = vec![];
                    for &input_idx in deferred_ref.iter() {
                        ref_positions.push(stack.get_relative_position(input_idx)?);
                    }

                    script.extend_from_slice((function_metadata.script)(&ref_positions).as_bytes());

                    // push the corresponding outputs
                    for output_type in function_metadata.output.iter() {
                        let num_elements = dsl
                            .data_type_registry
                            .map
                            .get(&output_type.to_string())
                            .unwrap()
                            .num_elements;
                        stack.push_to_stack(allocated_idx, num_elements)?;
                        allocated_idx += 1;
                    }

                    cur_time += 1;
                }
                TraceEntry::AllocatedConstant(idx) => {
                    let data_type = &dsl.memory.get(idx).unwrap().data_type;
                    let input_metadata = dsl
                        .data_type_registry
                        .map
                        .get(&data_type.to_string())
                        .unwrap();
                    stack.push_to_stack(*idx, input_metadata.num_elements)?;
                    allocated_idx += 1;

                    script.extend_from_slice(
                        script! {
                            { dsl.memory.get(idx).unwrap() }
                        }
                        .as_bytes(),
                    );
                }
            }
        }

        // step 4: clear the entire stack
        let elements_in_stack = stack.get_num_elements_in_stack()?;
        for _ in 0..elements_in_stack / 2 {
            script.push(OP_2DROP.to_u8());
        }
        if elements_in_stack % 2 == 1 {
            script.push(OP_DROP.to_u8());
        }
        script.push(OP_TRUE.to_u8());

        Ok(CompiledProgram {
            input,
            script: ScriptBuf::from_bytes(script),
            hint: dsl.hint,
        })
    }
}
