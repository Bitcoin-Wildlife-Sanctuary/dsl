use crate::builtins::table::m31::{M31Limbs, M31LimbsGadget, M31Mult, M31MultGadget};
use crate::builtins::table::utils::{convert_m31_to_limbs, mul_m31};
use crate::treepp::pushable::{Builder, Pushable};
use crate::treepp::*;
use anyhow::Result;
use rust_bitcoin_m31::{m31_add, m31_sub};

pub struct CM31Mult;

impl CM31Mult {
    pub fn compute_hint_from_limbs(
        a_real: &[u32],
        a_imag: &[u32],
        b_real: &[u32],
        b_imag: &[u32],
    ) -> Result<CM31MultHint> {
        assert_eq!(a_real.len(), 4);
        assert_eq!(a_imag.len(), 4);
        assert_eq!(b_real.len(), 4);
        assert_eq!(b_imag.len(), 4);

        let a_real_b_real = M31Mult::compute_c_limbs_from_limbs(&a_real, &b_real)?;
        let q3 = M31Mult::compute_q(&a_real_b_real)?;

        let a_imag_b_imag = M31Mult::compute_c_limbs_from_limbs(&a_imag, &b_imag)?;
        let q2 = M31Mult::compute_q(&a_imag_b_imag)?;

        let a_real_imag_sum = M31Limbs::add_limbs(&a_real, &a_imag);
        let b_real_imag_sum = M31Limbs::add_limbs(&b_real, &b_imag);
        let a_real_imag_b_real_imag =
            M31Mult::compute_c_limbs_from_limbs(&a_real_imag_sum, &b_real_imag_sum)?;
        let q1 = M31Mult::compute_q(&a_real_imag_b_real_imag)?;

        Ok(CM31MultHint { q1, q2, q3 })
    }

    pub fn compute_hint(a: &[u32], b: &[u32]) -> Result<CM31MultHint> {
        assert_eq!(a.len(), 2);
        assert_eq!(b.len(), 2);

        let a_real = convert_m31_to_limbs(a[0]);
        let a_imag = convert_m31_to_limbs(a[1]);
        let b_real = convert_m31_to_limbs(b[0]);
        let b_imag = convert_m31_to_limbs(b[1]);

        Self::compute_hint_from_limbs(&a_real, &a_imag, &b_real, &b_imag)
    }
}

pub struct CM31MultGadget;

impl CM31MultGadget {
    /// Input:
    /// - CM31 element:
    /// -   h3, h2, h1
    /// -   a1, a2, a3, a4 (the real part)
    /// -   a5, a6, a7, a8 (the imaginary part)
    /// -   b1, b2, b3, b4 (the real part)
    /// -   b5, b6, b7, b8 (the imaginary part)
    pub fn mult(k: usize) -> Script {
        script! {
            // compute (b1, b2, b3, b4) + (b5, b6, b7, b8)
            // save to the altstack
            for _ in 0..8 {
                7 OP_PICK
            }
            { M31LimbsGadget::add_limbs() }
            for _ in 0..4 {
                OP_TOALTSTACK
            }

            // compute (a1, a2, a3, a4) + (a5, a6, a7, a8)
            for _ in 0..8 {
                15 OP_PICK
            }
            { M31LimbsGadget::add_limbs() }
            // pull the (b1, b2, b3, b4) + (b5, b6, b7, b8) back
            for _ in 0..4 {
                OP_FROMALTSTACK
            }

            // compute the corresponding c limbs and perform the reduction
            { M31MultGadget::compute_c_limbs(k + 3 + 4 * 4) }
            20 OP_ROLL
            { M31MultGadget::reduce() }
            OP_TOALTSTACK

            // compute the imaginary part's product
            for _ in 0..4 {
                11 OP_ROLL
            }
            { M31MultGadget::compute_c_limbs(k + 2 + 2 * 4) }
            12 OP_ROLL
            { M31MultGadget::reduce() }
            OP_TOALTSTACK

            // compute the real part's product
            { M31MultGadget::compute_c_limbs(k + 1) }
            4 OP_ROLL
            { M31MultGadget::reduce() }

            // stack: aR * bR
            // altstack: (aR + aI) * (bR + bI), aI * bI

            OP_FROMALTSTACK
            OP_2DUP
            m31_sub

            OP_ROT OP_ROT
            m31_add

            OP_FROMALTSTACK
            OP_SWAP
            m31_sub

            // follow the cm31 format: imag first, real second
            OP_SWAP
        }
    }
}

pub struct CM31MultHint {
    pub q1: u32,
    pub q2: u32,
    pub q3: u32,
}

impl Pushable for CM31MultHint {
    fn bitcoin_script_push(&self, mut builder: Builder) -> Builder {
        builder = self.q3.bitcoin_script_push(builder);
        builder = self.q2.bitcoin_script_push(builder);
        builder = self.q1.bitcoin_script_push(builder);
        builder
    }
}

pub struct CM31Limbs;

impl CM31Limbs {
    pub fn add_limbs(a: &[u32], b: &[u32]) -> Vec<u32> {
        assert_eq!(a.len(), 8);
        assert_eq!(b.len(), 8);

        let mut res = Vec::with_capacity(8);
        res.extend_from_slice(&M31Limbs::add_limbs_with_reduction(&a[0..4], &b[0..4]));
        res.extend_from_slice(&M31Limbs::add_limbs_with_reduction(&a[4..8], &b[4..8]));
        res
    }
}

pub struct CM31LimbsGadget;

impl CM31LimbsGadget {
    // a1, ..., a8
    // b1, ..., b8
    pub fn add_limbs() -> Script {
        script! {
            // pull a5, a6, a7, a8
            for _ in 0..4 {
                11 OP_ROLL
            }
            { M31LimbsGadget::add_limbs_with_reduction() }

            // move to altstack
            for _ in 0..4 {
                OP_TOALTSTACK
            }

            { M31LimbsGadget::add_limbs_with_reduction() }

            for _ in 0..4 {
                OP_FROMALTSTACK
            }
        }
    }
}

pub fn mul_cm31(a: (u32, u32), b: (u32, u32)) -> (u32, u32) {
    let a_real = a.0;
    let a_imag = a.1;

    let b_real = b.0;
    let b_imag = b.1;

    let mut c_real = mul_m31(a_real, b_real) as i64;
    c_real += (1i64 << 31) - 1;
    c_real -= mul_m31(a_imag, b_imag) as i64;
    c_real %= (1i64 << 31) - 1;

    let mut c_imag = mul_m31(a_real, b_imag) as i64;
    c_imag += mul_m31(a_imag, b_real) as i64;
    c_imag %= (1i64 << 31) - 1;

    (c_real as u32, c_imag as u32)
}

#[cfg(test)]
mod test {
    use crate::builtins::table::cm31::{
        mul_cm31, CM31Limbs, CM31LimbsGadget, CM31Mult, CM31MultGadget,
    };
    use crate::builtins::table::get_table;
    use crate::builtins::table::utils::{convert_cm31_to_limbs, convert_m31_to_limbs};
    use crate::treepp::*;
    use bitcoin_scriptexec::execute_script;
    use rand::{Rng, SeedableRng};
    use rand_chacha::ChaCha20Rng;
    use rust_bitcoin_m31::cm31_equalverify;

    #[test]
    fn test_mult() {
        let mut prng = ChaCha20Rng::seed_from_u64(0);

        let table = get_table();

        for i in 0..100 {
            let a_real = prng.gen_range(0u32..((1 << 31) - 1));
            let a_imag = prng.gen_range(0u32..((1 << 31) - 1));
            let b_real = prng.gen_range(0u32..((1 << 31) - 1));
            let b_imag = prng.gen_range(0u32..((1 << 31) - 1));

            let a = (a_real, a_imag);
            let b = (b_real, b_imag);

            let expected = mul_cm31(a, b);

            let hint = CM31Mult::compute_hint(&[a_real, a_imag], &[b_real, b_imag]).unwrap();

            let script = script! {
                { table }
                for _ in 0..i {
                    { 1 }
                }
                { hint }
                { convert_m31_to_limbs(a_real).to_vec() }
                { convert_m31_to_limbs(a_imag).to_vec() }
                { convert_m31_to_limbs(b_real).to_vec() }
                { convert_m31_to_limbs(b_imag).to_vec() }
                { CM31MultGadget::mult(i) }
                { expected.1 }
                { expected.0 }
                cm31_equalverify
                for _ in 0..i {
                    OP_DROP
                }
                for _ in 0..256 {
                    OP_2DROP
                }
                OP_DROP
                OP_TRUE
            };

            let exec_result = execute_script(script);
            assert!(exec_result.success);
        }
    }

    #[test]
    fn test_add_limbs() {
        let mut prng = ChaCha20Rng::seed_from_u64(0);

        for _ in 0..100 {
            let a_real = prng.gen_range(0u32..((1 << 31) - 1));
            let a_imag = prng.gen_range(0u32..((1 << 31) - 1));
            let b_real = prng.gen_range(0u32..((1 << 31) - 1));
            let b_imag = prng.gen_range(0u32..((1 << 31) - 1));

            let a = (a_real, a_imag);
            let b = (b_real, b_imag);

            let a_limbs = convert_cm31_to_limbs(a);
            let b_limbs = convert_cm31_to_limbs(b);

            let sum_limbs = CM31Limbs::add_limbs(&a_limbs, &b_limbs);

            let script = script! {
                for a_limb in a_limbs.iter() {
                    { *a_limb }
                }
                for b_limb in b_limbs.iter() {
                    { *b_limb }
                }
                { CM31LimbsGadget::add_limbs() }
                for sum_limb in sum_limbs.iter().rev() {
                    { *sum_limb }
                    OP_EQUALVERIFY
                }
                OP_TRUE
            };

            let exec_result = execute_script(script);
            assert!(exec_result.success);
        }
    }
}
