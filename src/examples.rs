#[cfg(test)]
mod test {
    use crate::dsl::{Element, ElementType, MemoryEntry, DSL};
    use crate::functions::{FunctionMetadata, FunctionOutput};
    use crate::test_program;
    use crate::treepp::*;
    use anyhow::Result;
    use bitcoin::ScriptBuf;
    use rand::{Rng, SeedableRng};
    use rand_chacha::ChaCha20Rng;

    fn m31_mult(dsl: &mut DSL, inputs: &[usize]) -> Result<FunctionOutput> {
        let a = dsl.get_num(inputs[0])?;
        let b = dsl.get_num(inputs[1])?;

        let res = (a as i64) * (b as i64) % ((1i64 << 31) - 1);

        Ok(FunctionOutput {
            new_elements: vec![MemoryEntry::new("m31", Element::Num(res as i32))],
            new_hints: vec![],
        })
    }

    fn m31_mult_gadget(_: &[usize]) -> Result<ScriptBuf> {
        Ok(script! {
            { rust_bitcoin_m31::m31_mul() }
        })
    }

    #[test]
    fn test_m31_mult() {
        let mut prng = ChaCha20Rng::seed_from_u64(0);

        let mut a_val = prng.gen_range(0..((1i64 << 31) - 1)) as i32;

        let mut dsl = DSL::new();
        dsl.add_data_type("m31", ElementType::Num).unwrap();
        dsl.add_function(
            "m31_mult",
            FunctionMetadata {
                trace_generator: m31_mult,
                script_generator: m31_mult_gadget,
                input: vec!["m31", "m31"],
                output: vec!["m31"],
            },
        )
        .unwrap();

        let mut a = dsl.alloc_input("m31", Element::Num(a_val)).unwrap();

        for _ in 0..10 {
            let b_val = prng.gen_range(0..((1i64 << 31) - 1));
            let expected = (a_val as i64) * b_val % ((1i64 << 31) - 1);

            let b = dsl
                .alloc_constant("m31", Element::Num(b_val as i32))
                .unwrap();

            let res = dsl.execute("m31_mult", &[a, b]).unwrap();
            assert_eq!(res.len(), 1);
            let res_val = dsl.get_num(res[0]).unwrap();
            assert_eq!(res_val, expected as i32);

            a = res[0];
            a_val = res_val;
        }

        dsl.set_program_output("m31", a).unwrap();

        test_program(
            dsl,
            script! {
                { a_val }
            },
        )
        .unwrap();
    }
}
