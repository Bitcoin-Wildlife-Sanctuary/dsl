use crate::builtins::cm31::CM31Var;
use crate::builtins::hash::{bitcoin_num_to_bytes, HashVar};
use crate::builtins::m31::M31Var;
use crate::builtins::qm31::QM31Var;
use crate::builtins::str::StrVar;
use crate::bvar::{AllocVar, BVar};
use crate::constraint_system::ConstraintSystemRef;
use anyhow::Result;
use bitcoin::script::{read_scriptint, write_scriptint};
use bitcoin_circle_stark::channel::{BitcoinIntegerEncodedData, ChannelWithHint, DrawHints};
use bitcoin_circle_stark::treepp::*;
use bitcoin_circle_stark::utils::hash;
use num_traits::Zero;
use sha2::digest::Update;
use sha2::{Digest, Sha256};
use stwo_prover::core::channel::Sha256Channel;
use stwo_prover::core::fields::m31::M31;
use stwo_prover::core::vcs::sha256_hash::Sha256Hash;

impl HashVar {
    pub fn draw_digest(&mut self) -> HashVar {
        let mut sha256 = Sha256::new();
        Update::update(&mut sha256, &self.value);
        Update::update(&mut sha256, &[0x00]);
        let drawn_digest = sha256.finalize().to_vec();

        let mut sha256 = Sha256::new();
        Update::update(&mut sha256, &self.value);
        let new_digest = sha256.finalize().to_vec();

        let cs = self.cs();
        cs.insert_script(draw_digest_gadget, vec![self.variable])
            .unwrap();

        *self = HashVar::new_function_output(&cs, new_digest).unwrap();
        HashVar::new_function_output(&cs, drawn_digest).unwrap()
    }

    pub fn unpack_multi_m31(&self, n: usize, hints: &[StrVar]) -> Vec<M31Var> {
        assert!(n <= 8);
        assert!(n >= 1);

        if n == 8 {
            assert_eq!(hints.len(), 8);
        } else {
            assert_eq!(hints.len(), n + 1);
        }

        let mut m31 = vec![];
        let mut str = Option::<StrVar>::None;

        for hint in hints.iter().take(n) {
            hint.len_lessthanorequal(4);
            let (reconstructed_m31, reconstructed_str) = hint.reconstruct_for_channel_draw();
            m31.push(reconstructed_m31);

            if let Some(v) = &str {
                str = Some(v + &reconstructed_str);
            } else {
                str = Some(reconstructed_str);
            }
        }

        let mut str = str.unwrap();
        if n != 8 {
            str = &str + hints.last().unwrap()
        }
        StrVar::from(self).equalverify(&str).unwrap();

        m31
    }

    pub fn draw_felt(&mut self) -> QM31Var {
        let mut channel = Sha256Channel::default();
        channel.update_digest(Sha256Hash::from(self.value.clone()));
        let (_, hint) = channel.draw_felt_and_hints();

        let to_extract = self.draw_digest();

        let cs = self.cs();

        let hints = draw_hints_to_str_vars(&cs, hint).unwrap();

        let m31 = to_extract.unpack_multi_m31(4, &hints);

        // perform a move operation
        let qm31 = QM31Var {
            first: CM31Var {
                imag: M31Var {
                    variable: m31[1].variable,
                    value: m31[1].value,
                    cs: cs.clone(),
                },
                real: M31Var {
                    variable: m31[0].variable,
                    value: m31[0].value,
                    cs: cs.clone(),
                },
            },
            second: CM31Var {
                imag: M31Var {
                    variable: m31[3].variable,
                    value: m31[3].value,
                    cs: cs.clone(),
                },
                real: M31Var {
                    variable: m31[2].variable,
                    value: m31[2].value,
                    cs: cs.clone(),
                },
            },
        };

        qm31
    }
}

fn draw_digest_gadget() -> Script {
    script! {
        OP_DUP hash OP_SWAP
        OP_PUSHBYTES_1 OP_PUSHBYTES_0 OP_CAT hash
    }
}

impl StrVar {
    pub(crate) fn reconstruct_for_channel_draw(&self) -> (M31Var, StrVar) {
        let res = if self.value == vec![0x80] {
            (M31::zero(), vec![0x00, 0x00, 0x00, 0x80])
        } else {
            let num = read_scriptint(&self.value).unwrap();
            let abs = M31::from_u32_unchecked(num.abs() as u32);
            let abs_str = bitcoin_num_to_bytes(num.abs());

            if abs_str.len() < 4 {
                let mut str = self.value.clone();
                if str.len() < 2 {
                    str.push(0x00);
                    str.push(0x00);
                }
                if str.len() < 3 {
                    str.push(0x00);
                }

                if num < 0 {
                    str.push(0x80);
                } else {
                    str.push(0x00);
                }

                (abs, str)
            } else {
                (abs, self.value.clone())
            }
        };

        let cs = self.cs();

        cs.insert_script(reconstruct_for_channel_draw_gadget, self.variables())
            .unwrap();

        let reconstructed_str = StrVar::new_function_output(&cs, res.1).unwrap();
        let reconstructed_m31 = M31Var::new_function_output(&cs, res.0).unwrap();

        (reconstructed_m31, reconstructed_str)
    }
}

fn reconstruct_for_channel_draw_gadget() -> Script {
    script! {
        // handle 0x80 specially---it is the "negative zero", but most arithmetic opcodes refuse to work with it.
        OP_DUP OP_PUSHBYTES_1 OP_LEFT OP_EQUAL
        OP_IF
            OP_DROP
            OP_PUSHBYTES_4 OP_PUSHBYTES_0 OP_PUSHBYTES_0 OP_PUSHBYTES_0 OP_LEFT
            OP_PUSHBYTES_0 OP_TOALTSTACK
        OP_ELSE
            OP_DUP OP_ABS
            OP_DUP OP_TOALTSTACK

            OP_SIZE 4 OP_LESSTHAN
            OP_IF
                OP_DUP OP_ROT
                OP_EQUAL OP_TOALTSTACK

                // stack: abs(a)
                // altstack: abs(a), is_positive

                OP_SIZE 2 OP_LESSTHAN OP_IF OP_PUSHBYTES_2 OP_PUSHBYTES_0 OP_PUSHBYTES_0 OP_CAT OP_ENDIF
                OP_SIZE 3 OP_LESSTHAN OP_IF OP_PUSHBYTES_1 OP_PUSHBYTES_0 OP_CAT OP_ENDIF

                OP_FROMALTSTACK
                OP_IF
                    OP_PUSHBYTES_1 OP_PUSHBYTES_0
                OP_ELSE
                    OP_PUSHBYTES_1 OP_LEFT
                OP_ENDIF
                OP_CAT
            OP_ELSE
                OP_DROP
            OP_ENDIF
            OP_FROMALTSTACK
        OP_ENDIF

        // stack: str
        // altstack: abs(a)
    }
}

fn draw_hints_to_str_vars(cs: &ConstraintSystemRef, hint: DrawHints) -> Result<Vec<StrVar>> {
    let mut new_hints = vec![];
    for hint_element in hint.0.iter() {
        let data = match hint_element {
            BitcoinIntegerEncodedData::NegativeZero => {
                vec![0x80]
            }
            BitcoinIntegerEncodedData::Other(v) => {
                let mut out = [0u8; 8];
                let len = write_scriptint(&mut out, *v);
                out[0..len].to_vec()
            }
        };
        new_hints.push(StrVar::new_hint(cs, data)?);
    }
    if !hint.1.is_empty() {
        new_hints.push(StrVar::new_hint(cs, hint.1)?);
    }

    Ok(new_hints)
}

#[cfg(test)]
mod test {
    use crate::builtins::hash::HashVar;
    use crate::bvar::AllocVar;
    use crate::constraint_system::ConstraintSystem;
    use crate::test_program;
    use bitcoin_circle_stark::treepp::*;
    use rand::{Rng, SeedableRng};
    use rand_chacha::ChaCha20Rng;
    use stwo_prover::core::channel::{Channel, Sha256Channel};
    use stwo_prover::core::vcs::sha256_hash::Sha256Hash;

    #[test]
    fn test_draw_felt() {
        let mut prng = ChaCha20Rng::seed_from_u64(0);

        let mut init_state = [0u8; 32];
        init_state.iter_mut().for_each(|v| *v = prng.gen());
        let init_state = Sha256Hash::from(init_state.to_vec());

        let mut channel = Sha256Channel::default();
        channel.update_digest(init_state);
        let b = channel.draw_felt();
        let c = channel.digest;

        let cs = ConstraintSystem::new_ref();

        let mut channel_digest = HashVar::new_constant(&cs, init_state.as_ref().to_vec()).unwrap();
        let res = channel_digest.draw_felt();

        cs.set_program_output(&channel_digest).unwrap();
        cs.set_program_output(&res).unwrap();

        test_program(
            cs,
            script! {
                { c }
                { b }
            },
        )
        .unwrap();
    }
}