#[cfg(test)]
mod test {
    use crate::compiler::Compiler;
    use crate::dsl::{Element, MemoryEntry, DSL};
    use crate::functions::{FunctionMetadata, FunctionOutput};
    use crate::treepp::*;
    use bitcoin::ScriptBuf;
    use rand::{Rng, SeedableRng};
    use rand_chacha::ChaCha20Rng;

    fn m31_mult(dsl: &mut DSL, inputs: &[usize]) -> FunctionOutput {
        let a = dsl.get_num(inputs[0]);
        let b = dsl.get_num(inputs[1]);

        let res = (a as i64) * (b as i64) % ((1i64 << 31) - 1);

        FunctionOutput {
            new_elements: vec![MemoryEntry::new("m31", Element::Num(res as i32))],
            new_hints: vec![],
        }
    }

    fn m31_mult_gadget(_: &[usize]) -> ScriptBuf {
        script! {
            { rust_bitcoin_m31::m31_mul() }
        }
    }

    #[test]
    fn test_m31_mult() {
        let mut prng = ChaCha20Rng::seed_from_u64(0);

        let mut a_val = prng.gen_range(0..((1i64 << 31) - 1)) as i32;

        let mut dsl = DSL::new();
        dsl.add_data_type("m31", 1);
        dsl.add_function(
            "m31_mult",
            FunctionMetadata {
                trace_generator: m31_mult,
                script_generator: m31_mult_gadget,
                input: vec!["m31", "m31"],
                output: vec!["m31"],
            },
        );

        let mut a = dsl.alloc_input("m31", Element::Num(a_val));

        for _ in 0..1 {
            let b_val = prng.gen_range(0..((1i64 << 31) - 1));
            let expected = (a_val as i64) * b_val % ((1i64 << 31) - 1);

            let b = dsl.alloc_constant("m31", Element::Num(b_val as i32));

            let res = dsl.execute("m31_mult", &[a, b]);
            assert_eq!(res.len(), 1);
            let res_val = dsl.get_num(res[0]);
            assert_eq!(res_val, expected as i32);

            a = res[0];
            a_val = res_val;
        }

        let program = Compiler::compiler(dsl).unwrap();

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

        let script = Script::from_bytes(script);

        let exec_result = execute_script(script);
        assert!(exec_result.success);
    }
}
