use crate::treepp::*;
use rand::{Rng, RngCore};

pub fn mul_m31(a: u32, b: u32) -> u32 {
    (((a as i64) * (b as i64)) % ((1i64 << 31) - 1)) as u32
}

pub fn add_m31(a: u32, b: u32) -> u32 {
    (((a as i64) + (b as i64)) % ((1i64 << 31) - 1)) as u32
}

pub fn sub_m31(a: u32, b: u32) -> u32 {
    ((((1i64 << 31) - 1) + (a as i64) - (b as i64)) % ((1i64 << 31) - 1)) as u32
}

pub fn add_cm31(a: (u32, u32), b: (u32, u32)) -> (u32, u32) {
    let real = add_m31(a.0, b.0);
    let imag = add_m31(a.1, b.1);

    (real, imag)
}

pub fn sub_cm31(a: (u32, u32), b: (u32, u32)) -> (u32, u32) {
    let real = sub_m31(a.0, b.0);
    let imag = sub_m31(a.1, b.1);

    (real, imag)
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

pub fn mul_qm31(
    a: ((u32, u32), (u32, u32)),
    b: ((u32, u32), (u32, u32)),
) -> ((u32, u32), (u32, u32)) {
    let mut real_product = mul_cm31(a.0, b.0);
    let imag_product_shifted = mul_cm31(mul_cm31(a.1, b.1), (2, 1));

    real_product.0 = add_m31(real_product.0, imag_product_shifted.0);
    real_product.1 = add_m31(real_product.1, imag_product_shifted.1);

    let cross_term1 = mul_cm31(a.0, b.1);
    let cross_term2 = mul_cm31(a.1, b.0);

    let mut imag_product = (0, 0);
    imag_product.0 = add_m31(cross_term1.0, cross_term2.0);
    imag_product.1 = add_m31(cross_term1.1, cross_term2.1);

    (real_product, imag_product)
}

pub fn inverse_cm31(v: (u32, u32)) -> (u32, u32) {
    // 1 / (a + bi) = (a - bi) / (a^2 + b^2)

    let real_squared = mul_m31(v.0, v.0);
    let imag_squared = mul_m31(v.1, v.1);

    let denom = add_m31(real_squared, imag_squared);
    let denom_inverse = pow2147483645(denom);

    let real = mul_m31(v.0, denom_inverse);
    let imag = sub_m31(0, mul_m31(v.1, denom_inverse));

    (real, imag)
}

pub fn inverse_qm31(v: ((u32, u32), (u32, u32))) -> ((u32, u32), (u32, u32)) {
    // (a + bu)^-1 = (a - bu) / (a^2 - (2+i)b^2).

    let real_squared = mul_cm31(v.0, v.0);
    let imag_squared = mul_cm31(v.1, v.1);
    let imag_squared_times_2_plus_i = mul_cm31(imag_squared, (2, 1));

    let denom = sub_cm31(real_squared, imag_squared_times_2_plus_i);
    let denom_inverse = inverse_cm31(denom);

    let first = mul_cm31(v.0, denom_inverse);
    let second = sub_cm31((0, 0), mul_cm31(v.1, denom_inverse));

    (first, second)
}

pub fn convert_m31_to_limbs(v: u32) -> [u32; 4] {
    [v & 255, (v >> 8) & 255, (v >> 16) & 255, (v >> 24) & 255]
}

pub fn convert_m31_from_limbs(v: &[u32]) -> u32 {
    v[0] + (v[1] << 8) + (v[2] << 16) + (v[3] << 24)
}

pub fn convert_cm31_to_limbs(cm31: (u32, u32)) -> [u32; 8] {
    let real_limbs = convert_m31_to_limbs(cm31.0);
    let imag_limbs = convert_m31_to_limbs(cm31.1);

    [
        real_limbs[0],
        real_limbs[1],
        real_limbs[2],
        real_limbs[3],
        imag_limbs[0],
        imag_limbs[1],
        imag_limbs[2],
        imag_limbs[3],
    ]
}

pub fn rand_m31<R: RngCore>(prng: &mut R) -> u32 {
    prng.gen_range(0..((1i64 << 31) - 1)) as u32
}

pub fn rand_cm31<R: RngCore>(prng: &mut R) -> (u32, u32) {
    (rand_m31(prng), rand_m31(prng))
}

pub fn rand_qm31<R: RngCore>(prng: &mut R) -> ((u32, u32), (u32, u32)) {
    (rand_cm31(prng), rand_cm31(prng))
}

pub fn convert_cm31_from_limbs(v: &([u32; 4], [u32; 4])) -> (u32, u32) {
    let real = convert_m31_from_limbs(&v.0);
    let imag = convert_m31_from_limbs(&v.1);
    (real, imag)
}

pub fn check_limb_format() -> Script {
    script! {
        OP_DUP 0 OP_GREATERTHANOREQUAL OP_VERIFY
        OP_DUP 256 OP_LESSTHAN OP_VERIFY
    }
}

#[allow(non_snake_case)]
pub fn OP_256MUL() -> Script {
    #[cfg(feature = "assume-op-cat")]
    script! {
        OP_SIZE OP_NOT OP_NOTIF
        OP_PUSHBYTES_1 OP_PUSHBYTES_0 OP_SWAP OP_CAT
        OP_ENDIF
    }
    #[cfg(not(feature = "assume-op-cat"))]
    script! {
        OP_DUP OP_ADD OP_DUP OP_ADD OP_DUP OP_ADD OP_DUP OP_ADD
        OP_DUP OP_ADD OP_DUP OP_ADD OP_DUP OP_ADD OP_DUP OP_ADD
    }
}

#[allow(non_snake_case)]
pub fn OP_HINT() -> Script {
    script! {
        OP_DEPTH OP_1SUB OP_ROLL
    }
}

pub fn pow2147483645(v: u32) -> u32 {
    let t0 = mul_m31(sqn::<2>(v), v);
    let t1 = mul_m31(sqn::<1>(t0), t0);
    let t2 = mul_m31(sqn::<3>(t1), t0);
    let t3 = mul_m31(sqn::<1>(t2), t0);
    let t4 = mul_m31(sqn::<8>(t3), t3);
    let t5 = mul_m31(sqn::<8>(t4), t3);
    mul_m31(sqn::<7>(t5), t2)
}

/// Computes `v^(2*n)`.
fn sqn<const N: usize>(mut v: u32) -> u32 {
    for _ in 0..N {
        v = mul_m31(v, v);
    }
    v
}
