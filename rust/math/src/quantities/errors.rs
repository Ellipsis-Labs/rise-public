//! Error types for safe arithmetic operations.
//!
//! This module provides custom error types for handling arithmetic failures
//! in a type-safe manner, replacing panics with explicit error handling.

use thiserror::Error;

/// Errors that can occur during arithmetic operations on quantity types.
#[derive(Error, Debug, Clone, Copy, PartialEq, Eq)]
pub enum MathError {
    /// Division by zero was attempted.
    #[error("Division by zero")]
    DivisionByZero,

    /// Arithmetic operation would overflow the type's maximum value.
    #[error("Arithmetic overflow")]
    Overflow,

    /// Arithmetic operation would underflow below the type's minimum value.
    #[error("Arithmetic underflow")]
    Underflow,

    /// Value exceeds the defined bounds for the type.
    /// Uses i128 to properly handle both signed and unsigned values.
    #[error("Value {value} is out of bounds [{min}, {max}]")]
    OutOfBounds { value: i128, min: i128, max: i128 },
}

impl MathError {
    /// Creates an OutOfBounds error for u64 types.
    pub fn out_of_bounds_u64(value: u64, min: u64, max: u64) -> Self {
        Self::OutOfBounds {
            value: value as i128,
            min: min as i128,
            max: max as i128,
        }
    }

    /// Creates an OutOfBounds error for i64 types.
    pub fn out_of_bounds_i64(value: i64, min: i64, max: i64) -> Self {
        Self::OutOfBounds {
            value: value as i128,
            min: min as i128,
            max: max as i128,
        }
    }

    /// Creates an OutOfBounds error for i128 types.
    pub fn out_of_bounds_i128(value: i128, min: i128, max: i128) -> Self {
        Self::OutOfBounds { value, min, max }
    }

    /// Creates an OutOfBounds error for u32 types.
    pub fn out_of_bounds_u32(value: u32, min: u32, max: u32) -> Self {
        Self::OutOfBounds {
            value: value as i128,
            min: min as i128,
            max: max as i128,
        }
    }

    /// Creates an OutOfBounds error for i64 types.
    pub fn out_of_bounds_i32(value: i32, min: i32, max: i32) -> Self {
        Self::OutOfBounds {
            value: value as i128,
            min: min as i128,
            max: max as i128,
        }
    }
}
