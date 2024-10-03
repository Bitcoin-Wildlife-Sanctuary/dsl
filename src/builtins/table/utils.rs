use crate::treepp::*;

pub fn convert_m31_to_limbs(v: u32) -> [u32; 4] {
    [v & 255, (v >> 8) & 255, (v >> 16) & 255, (v >> 24) & 255]
}

pub fn convert_m31_from_limbs(v: &[u32]) -> u32 {
    v[0] + (v[1] << 8) + (v[2] << 16) + (v[3] << 24)
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
    let t0 = sqn::<2>(v as i64) * v as i64 % ((1i64 << 31) - 1);
    let t1 = sqn::<1>(t0) * t0 % ((1i64 << 31) - 1);
    let t2 = sqn::<3>(t1) * t0 % ((1i64 << 31) - 1);
    let t3 = sqn::<1>(t2) * t0 % ((1i64 << 31) - 1);
    let t4 = sqn::<8>(t3) * t3 % ((1i64 << 31) - 1);
    let t5 = sqn::<8>(t4) * t3 % ((1i64 << 31) - 1);
    (sqn::<7>(t5) * t2 % ((1i64 << 31) - 1)) as u32
}

/// Computes `v^(2*n)`.
fn sqn<const N: usize>(mut v: i64) -> i64 {
    for _ in 0..N {
        v = (v * v) % ((1i64 << 31) - 1);
    }
    v
}
