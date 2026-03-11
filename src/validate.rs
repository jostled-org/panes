use crate::error::ConstraintError;

/// Describes why a non-negative f32 check failed.
pub(crate) enum FloatInvalid {
    Nan,
    Negative,
    Infinite,
}

/// Reject NaN, negative, or infinite f32 values.
pub(crate) fn check_f32_non_negative(value: f32) -> Result<(), FloatInvalid> {
    match value {
        v if v.is_nan() => Err(FloatInvalid::Nan),
        v if v < 0.0 => Err(FloatInvalid::Negative),
        v if v.is_infinite() => Err(FloatInvalid::Infinite),
        _ => Ok(()),
    }
}

/// Reject NaN or infinite f32 values (negative values are allowed).
pub(crate) fn check_f32_finite(value: f32) -> Result<(), FloatInvalid> {
    match value {
        v if v.is_nan() => Err(FloatInvalid::Nan),
        v if v.is_infinite() => Err(FloatInvalid::Infinite),
        _ => Ok(()),
    }
}

/// Map a `FloatInvalid` to a named `ConstraintError`.
pub(crate) fn float_invalid_to_constraint(
    name: &'static str,
    err: FloatInvalid,
) -> ConstraintError {
    match err {
        FloatInvalid::Nan => ConstraintError::IsNan(name),
        FloatInvalid::Negative => ConstraintError::IsNegative(name),
        FloatInvalid::Infinite => ConstraintError::IsInfinite(name),
    }
}
