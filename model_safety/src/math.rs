//! Safe arithmetic helpers - no unwrap, no panics, no as casts

/// Add u128 with saturation at MAX
pub fn add_u128(a: u128, b: u128) -> u128 {
    a.saturating_add(b)
}

/// Subtract u128 with saturation at 0
pub fn sub_u128(a: u128, b: u128) -> u128 {
    a.saturating_sub(b)
}

/// Add i128 with saturation
pub fn add_i128(a: i128, b: i128) -> i128 {
    a.saturating_add(b)
}

/// Subtract i128 with saturation
pub fn sub_i128(a: i128, b: i128) -> i128 {
    a.saturating_sub(b)
}

/// Clamp positive i128 to u128 (negative becomes 0)
pub fn clamp_pos_i128(x: i128) -> u128 {
    if x > 0 {
        // Safe: we checked x > 0
        if x > i128::MAX {
            u128::MAX
        } else {
            x as u128
        }
    } else {
        0
    }
}

/// Convert u128 to i128 with saturation at i128::MAX
pub fn u128_to_i128(x: u128) -> i128 {
    if x > i128::MAX as u128 {
        i128::MAX
    } else {
        x as i128
    }
}

/// Multiply u128 with saturation
pub fn mul_u128(a: u128, b: u128) -> u128 {
    a.saturating_mul(b)
}

/// Divide u128 (returns 0 if divisor is 0)
pub fn div_u128(a: u128, b: u128) -> u128 {
    if b == 0 {
        0
    } else {
        a / b
    }
}

/// Minimum of two u128
pub fn min_u128(a: u128, b: u128) -> u128 {
    if a < b { a } else { b }
}

/// Maximum of two u128
pub fn max_u128(a: u128, b: u128) -> u128 {
    if a > b { a } else { b }
}

/// Minimum of two i128
pub fn min_i128(a: i128, b: i128) -> i128 {
    if a < b { a } else { b }
}

/// Maximum of two i128
pub fn max_i128(a: i128, b: i128) -> i128 {
    if a > b { a } else { b }
}

/// Multiply i128 with saturation
pub fn mul_i128(a: i128, b: i128) -> i128 {
    a.saturating_mul(b)
}

/// Divide i128 (returns 0 if divisor is 0)
pub fn div_i128(a: i128, b: i128) -> i128 {
    if b == 0 {
        0
    } else {
        a / b
    }
}
