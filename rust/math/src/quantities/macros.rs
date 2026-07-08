//! Macro definitions for generating type-safe numeric wrapper structs.
//!
//! These macros eliminate boilerplate while ensuring consistent implementation
//! of arithmetic operations with proper overflow handling and type safety.

/// Creates a basic u64 wrapper struct with safe arithmetic operations.
///
/// # Generated Methods
///
/// - `new(value: u64)`: Constructor
/// - `as_inner() -> u64`: Access underlying value
/// - `checked_add/sub`: Returns None on overflow
/// - `saturating_add/sub`: Saturates at MIN/MAX
/// - `div_ceil`: Division with ceiling rounding
///
/// # Safety Features
///
/// All arithmetic operations preserve type safety and provide
/// explicit overflow handling options.
#[macro_export]
macro_rules! basic_u64_struct {
    ($type_name:ident) => {
        #[derive(Clone, Copy, PartialOrd, Ord, Zeroable, Pod, BorshDeserialize, BorshSerialize)]
        #[repr(transparent)]
        pub struct $type_name {
            inner: u64,
        }

        impl $type_name {
            pub fn div_ceil<Divisor: WrapperNum<u64>>(self, other: Divisor) -> Self {
                match self.checked_div_ceil(other) {
                    Some(result) => result,
                    None => panic!("Overflow or division by zero in div_ceil"),
                }
            }

            pub fn checked_div_ceil<Divisor: WrapperNum<u64>>(
                self,
                other: Divisor,
            ) -> Option<Self> {
                let divisor = other.as_inner();
                if divisor == 0 {
                    None
                } else {
                    // Use built-in div_ceil method which handles overflow correctly
                    Some($type_name::new(self.inner.div_ceil(divisor)))
                }
            }
        }

        basic_num!($type_name, u64);

        impl $type_name {
            pub fn as_u128(&self) -> u128 {
                self.inner as u128
            }

            pub fn is_non_negative(&self) -> bool {
                true
            }
        }
    };
}

/// Creates a u64 wrapper with enforced value bounds.
///
/// # Overflow Prevention
///
/// By restricting values to a subset of u64's range, this macro
/// helps prevent overflow errors before they occur.
///
/// # Example
///
/// ```ignore
/// basic_u64_struct_with_bounds!(Ticks, 0, u32::MAX as u64);
/// // Ticks can only hold values from 0 to u32::MAX
/// ```
#[macro_export]
macro_rules! basic_u64_struct_with_bounds {
    ($type_name:ident, $lower_bound:expr, $upper_bound:expr) => {
        basic_u64_struct!($type_name);
        impl ScalarBounds<u64> for $type_name {
            const LOWER_BOUND: u64 = $lower_bound;
            const UPPER_BOUND: u64 = $upper_bound;
        }

        impl $type_name {
            /// Creates a new instance with bounds checking.
            /// Returns an error if the value is outside the valid range.
            ///
            /// # Example
            /// ```ignore
            /// let valid = BaseLots::new_checked(100)?;
            /// let invalid = BaseLots::new_checked(u64::MAX); // Returns Err
            /// ```
            pub fn new_checked(value: u64) -> Result<Self, $crate::quantities::MathError> {
                if !(Self::LOWER_BOUND..=Self::UPPER_BOUND).contains(&value) {
                    Err($crate::quantities::MathError::out_of_bounds_u64(
                        value,
                        Self::LOWER_BOUND,
                        Self::UPPER_BOUND,
                    ))
                } else {
                    Ok(Self::new(value))
                }
            }

            /// Creates a new instance, saturating at the bounds if necessary.
            ///
            /// # Example
            /// ```ignore
            /// let saturated = BaseLots::new_saturating(u64::MAX);
            /// assert_eq!(saturated.as_inner(), u32::MAX as u64);
            /// ```
            pub fn new_saturating(value: u64) -> Self {
                let clamped = value.clamp(Self::LOWER_BOUND, Self::UPPER_BOUND);
                Self::new(clamped)
            }
        }

        impl core::convert::TryFrom<u8> for $type_name {
            type Error = $crate::quantities::MathError;

            fn try_from(value: u8) -> Result<Self, Self::Error> {
                Self::new_checked(value as u64)
            }
        }

        impl core::convert::TryFrom<u16> for $type_name {
            type Error = $crate::quantities::MathError;

            fn try_from(value: u16) -> Result<Self, Self::Error> {
                Self::new_checked(value as u64)
            }
        }

        impl core::convert::TryFrom<u32> for $type_name {
            type Error = $crate::quantities::MathError;

            fn try_from(value: u32) -> Result<Self, Self::Error> {
                Self::new_checked(value as u64)
            }
        }

        impl core::convert::TryFrom<u128> for $type_name {
            type Error = $crate::quantities::MathError;

            fn try_from(value: u128) -> Result<Self, Self::Error> {
                if value > u64::MAX as u128 {
                    return Err($crate::quantities::MathError::Overflow);
                }
                Self::new_checked(value as u64)
            }
        }

        impl core::convert::TryFrom<usize> for $type_name {
            type Error = $crate::quantities::MathError;

            fn try_from(value: usize) -> Result<Self, Self::Error> {
                Self::new_checked(value as u64)
            }
        }
    };
}

/// Creates an unsigned u32 wrapper struct with safe arithmetic.
///
/// # Generated Methods
///
/// - `div_ceil`: Division with ceiling rounding
/// - `checked_div_ceil`: Returns None on division by zero
/// - `as_u64()`: Widen to u64
/// - `is_non_negative()`: Always returns true for unsigned
#[macro_export]
macro_rules! basic_u32_struct {
    ($type_name:ident) => {
        #[derive(Clone, Copy, PartialOrd, Ord, Zeroable, Pod, BorshDeserialize, BorshSerialize)]
        #[repr(transparent)]
        pub struct $type_name {
            inner: u32,
        }

        impl $type_name {
            pub fn div_ceil<Divisor: WrapperNum<u32>>(self, other: Divisor) -> Self {
                match self.checked_div_ceil(other) {
                    Some(result) => result,
                    None => panic!("Overflow or division by zero in div_ceil"),
                }
            }

            pub fn checked_div_ceil<Divisor: WrapperNum<u32>>(
                self,
                other: Divisor,
            ) -> Option<Self> {
                let divisor = other.as_inner();
                if divisor == 0 {
                    None
                } else {
                    // Use built-in div_ceil method which handles overflow correctly
                    Some($type_name::new(self.inner.div_ceil(divisor)))
                }
            }

            pub fn as_u64(&self) -> u64 {
                self.inner as u64
            }
        }

        basic_num!($type_name, u32);

        impl $type_name {
            pub fn as_u128(&self) -> u128 {
                self.inner as u128
            }

            pub fn is_non_negative(&self) -> bool {
                true
            }
        }
    };
}

#[macro_export]
macro_rules! basic_u32_struct_with_bounds {
    ($type_name:ident, $lower_bound:expr, $upper_bound:expr) => {
        basic_u32_struct!($type_name);
        impl ScalarBounds<u32> for $type_name {
            const LOWER_BOUND: u32 = $lower_bound;
            const UPPER_BOUND: u32 = $upper_bound;
        }

        impl $type_name {
            pub fn new_checked(value: u32) -> Result<Self, $crate::quantities::MathError> {
                if !(Self::LOWER_BOUND..=Self::UPPER_BOUND).contains(&value) {
                    Err($crate::quantities::MathError::out_of_bounds_u32(
                        value,
                        Self::LOWER_BOUND,
                        Self::UPPER_BOUND,
                    ))
                } else {
                    Ok(Self::new(value))
                }
            }

            pub fn new_saturating(value: u32) -> Self {
                let clamped = value.clamp(Self::LOWER_BOUND, Self::UPPER_BOUND);
                Self::new(clamped)
            }
        }
    };
}

#[macro_export]
macro_rules! basic_i128_struct {
    ($type_name:ident) => {
        pastey::paste! {
            #[derive(
                Clone, Copy, PartialOrd, Ord, Zeroable, Pod, BorshDeserialize, BorshSerialize,
            )]
            #[repr(transparent)]
            pub struct [<$type_name Upcasted>] {
                inner: i128,
            }

            basic_num!([<$type_name Upcasted>], i128);

            impl $type_name {
                pub fn upcast(&self) -> [<$type_name Upcasted>] {
                    [<$type_name Upcasted>] { inner: self.inner as i128 }
                }
            }

            impl [<$type_name Upcasted>] {
                pub fn downcast(&self) -> Option<$type_name> {
                    if self.inner > i64::MAX as i128 || self.inner < i64::MIN as i128 {
                        None
                    } else {
                        Some($type_name::new(self.inner as i64))
                    }
                }

                /// Checked conversion to u128. Returns error if negative.
                pub fn checked_as_u128(&self) -> Result<u128, $crate::quantities::MathError> {
                    if self.inner < 0 {
                        Err($crate::quantities::MathError::Underflow)
                    } else {
                        Ok(self.inner as u128)
                    }
                }

                pub fn is_non_negative(&self) -> bool {
                    self.inner >= 0
                }
            }
        }
    };
}

/// Creates a signed i64 wrapper struct with safe arithmetic.
///
/// Includes additional methods for signed operations:
/// - `abs()`: Absolute value
/// - `signum()`: Sign of the number
/// - `neg()`: Negation operator
/// - `is_non_negative()`: Check if >= 0
#[macro_export]
macro_rules! basic_i64_struct {
    ($type_name:ident) => {
        #[derive(Clone, Copy, PartialOrd, Ord, Zeroable, Pod, BorshDeserialize, BorshSerialize)]
        #[repr(transparent)]
        pub struct $type_name {
            inner: i64,
        }

        basic_num!($type_name, i64);

        impl $type_name {
            pub fn abs(self) -> Self {
                $type_name::new(
                    self.inner
                        .checked_abs()
                        .expect("Overflow in abs for signed 64-bit wrapper"),
                )
            }

            pub fn as_i128(&self) -> i128 {
                self.inner as i128
            }

            /// Checked conversion to u128. Returns error if negative.
            pub fn checked_as_u128(&self) -> Result<u128, $crate::quantities::MathError> {
                if self.inner < 0 {
                    Err($crate::quantities::MathError::Underflow)
                } else {
                    Ok(self.inner as u128)
                }
            }

            /// Absolute value as u128. Always succeeds since u128 can represent
            /// abs(i64::MIN).
            pub fn abs_as_u128(&self) -> u128 {
                self.inner.unsigned_abs() as u128
            }

            pub fn signum(&self) -> Self {
                $type_name::new(self.inner.signum())
            }

            pub fn is_non_negative(&self) -> bool {
                self.inner >= 0
            }
        }

        impl Neg for $type_name {
            type Output = Self;

            fn neg(self) -> Self {
                $type_name::new(
                    self.inner
                        .checked_neg()
                        .expect("Overflow in neg for signed 64-bit wrapper"),
                )
            }
        }
    };
}

#[macro_export]
macro_rules! basic_i64_struct_with_bounds {
    ($type_name:ident, $lower_bound:expr, $upper_bound:expr) => {
        basic_i64_struct!($type_name);
        impl ScalarBounds<i64> for $type_name {
            const LOWER_BOUND: i64 = $lower_bound;
            const UPPER_BOUND: i64 = $upper_bound;
        }

        impl $type_name {
            /// Creates a new instance with bounds checking.
            /// Returns an error if the value is outside the valid range.
            ///
            /// # Example
            /// ```ignore
            /// let valid = BaseLots::new_checked(100)?;
            /// let invalid = BaseLots::new_checked(i64::MAX); // Returns Err
            /// ```
            pub fn new_checked(value: i64) -> Result<Self, $crate::quantities::MathError> {
                if !(Self::LOWER_BOUND..=Self::UPPER_BOUND).contains(&value) {
                    Err($crate::quantities::MathError::out_of_bounds_i64(
                        value,
                        Self::LOWER_BOUND,
                        Self::UPPER_BOUND,
                    ))
                } else {
                    Ok(Self::new(value))
                }
            }

            /// Creates a new instance, saturating at the bounds if necessary.
            ///
            /// # Example
            /// ```ignore
            /// let saturated = BaseLots::new_saturating(i64::MAX);
            /// assert_eq!(saturated.as_inner(), u32::MAX as u64);
            /// ```
            pub fn new_saturating(value: i64) -> Self {
                let clamped = value.clamp(Self::LOWER_BOUND, Self::UPPER_BOUND);
                Self::new(clamped)
            }
        }

        impl core::convert::TryFrom<i8> for $type_name {
            type Error = $crate::quantities::MathError;

            fn try_from(value: i8) -> Result<Self, Self::Error> {
                Self::new_checked(i64::from(value))
            }
        }

        impl core::convert::TryFrom<i16> for $type_name {
            type Error = $crate::quantities::MathError;

            fn try_from(value: i16) -> Result<Self, Self::Error> {
                Self::new_checked(i64::from(value))
            }
        }

        impl core::convert::TryFrom<i32> for $type_name {
            type Error = $crate::quantities::MathError;

            fn try_from(value: i32) -> Result<Self, Self::Error> {
                Self::new_checked(i64::from(value))
            }
        }

        impl core::convert::TryFrom<i128> for $type_name {
            type Error = $crate::quantities::MathError;

            fn try_from(value: i128) -> Result<Self, Self::Error> {
                let converted = i64::try_from(value).map_err(|_| {
                    $crate::quantities::MathError::out_of_bounds_i128(
                        value,
                        $type_name::LOWER_BOUND as i128,
                        $type_name::UPPER_BOUND as i128,
                    )
                })?;
                Self::new_checked(converted)
            }
        }
    };
}

/// Creates a signed i32 wrapper struct with safe arithmetic.
///
/// Includes additional methods for signed operations:
/// - `abs()`: Absolute value
/// - `signum()`: Sign of the number
/// - `neg()`: Negation operator
/// - `is_non_negative()`: Check if >= 0
#[macro_export]
macro_rules! basic_i32_struct {
    ($type_name:ident) => {
        #[derive(Clone, Copy, PartialOrd, Ord, Zeroable, Pod, BorshDeserialize, BorshSerialize)]
        #[repr(transparent)]
        pub struct $type_name {
            inner: i32,
        }

        basic_num!($type_name, i32);

        impl $type_name {
            pub fn as_i64(&self) -> i64 {
                self.inner as i64
            }

            /// Checked conversion to u128. Returns error if negative.
            pub fn checked_as_u128(&self) -> Result<u128, $crate::quantities::MathError> {
                if self.inner < 0 {
                    Err($crate::quantities::MathError::Underflow)
                } else {
                    Ok(self.inner as u128)
                }
            }

            pub fn abs(self) -> Self {
                $type_name::new(
                    self.inner
                        .checked_abs()
                        .expect("Overflow in abs for signed 32-bit wrapper"),
                )
            }

            pub fn signum(&self) -> Self {
                $type_name::new(self.inner.signum())
            }

            pub fn is_non_negative(&self) -> bool {
                self.inner >= 0
            }
        }

        impl Neg for $type_name {
            type Output = Self;

            fn neg(self) -> Self {
                $type_name::new(
                    self.inner
                        .checked_neg()
                        .expect("Overflow in neg for signed 32-bit wrapper"),
                )
            }
        }
    };
}

#[macro_export]
macro_rules! basic_i32_struct_with_bounds {
    ($type_name:ident, $lower_bound:expr, $upper_bound:expr) => {
        basic_i32_struct!($type_name);
        impl ScalarBounds<i32> for $type_name {
            const LOWER_BOUND: i32 = $lower_bound;
            const UPPER_BOUND: i32 = $upper_bound;
        }

        impl $type_name {
            /// Creates a new instance with bounds checking.
            /// Returns an error if the value is outside the valid range.
            ///
            /// # Example
            /// ```ignore
            /// let valid = BaseLots::new_checked(100)?;
            /// let invalid = BaseLots::new_checked(i32::MAX); // Returns Err
            /// ```
            pub fn new_checked(value: i32) -> Result<Self, $crate::quantities::MathError> {
                if !(Self::LOWER_BOUND..=Self::UPPER_BOUND).contains(&value) {
                    Err($crate::quantities::MathError::out_of_bounds_i32(
                        value,
                        Self::LOWER_BOUND,
                        Self::UPPER_BOUND,
                    ))
                } else {
                    Ok(Self::new(value))
                }
            }

            /// Creates a new instance, saturating at the bounds if necessary.
            ///
            /// # Example
            /// ```ignore
            /// let saturated = BaseLots::new_saturating(i64::MAX);
            /// assert_eq!(saturated.as_inner(), u32::MAX as u64);
            /// ```
            pub fn new_saturating(value: i32) -> Self {
                let clamped = value.clamp(Self::LOWER_BOUND, Self::UPPER_BOUND);
                Self::new(clamped)
            }
        }
    };
}

#[macro_export]
macro_rules! basic_num {
    ($type_name:ident, $inner_type:ty) => {
        impl WrapperNum<$inner_type> for $type_name {
            type Inner = $inner_type;

            fn new(value: $inner_type) -> Self {
                $type_name { inner: value }
            }

            fn as_inner(&self) -> $inner_type {
                self.inner
            }
        }

        impl $type_name {
            pub const MAX: Self = $type_name {
                inner: <$inner_type>::MAX,
            };
            pub const MIN: Self = $type_name {
                inner: <$inner_type>::MIN,
            };
            pub const ONE: Self = $type_name { inner: 1 };
            pub const ZERO: Self = $type_name { inner: 0 };

            pub const fn new(value: $inner_type) -> Self {
                $type_name { inner: value }
            }

            pub const fn new_const(value: $inner_type) -> Self {
                $type_name { inner: value }
            }

            pub const fn as_inner(&self) -> $inner_type {
                self.inner
            }

            pub fn is_zero(&self) -> bool {
                *self == Self::ZERO
            }

            pub fn saturating_add(self, other: Self) -> Self {
                $type_name::new(self.inner.saturating_add(other.inner))
            }

            pub fn saturating_sub(self, other: Self) -> Self {
                $type_name::new(self.inner.saturating_sub(other.inner))
            }

            pub fn checked_add(self, other: Self) -> Option<Self> {
                self.inner.checked_add(other.inner).map($type_name::new)
            }

            pub fn checked_sub(self, other: Self) -> Option<Self> {
                self.inner.checked_sub(other.inner).map($type_name::new)
            }

            pub fn wrapping_add(self, other: Self) -> Self {
                $type_name::new(self.inner.wrapping_add(other.inner))
            }

            pub fn wrapping_sub(self, other: Self) -> Self {
                $type_name::new(self.inner.wrapping_sub(other.inner))
            }

            pub fn wrapping_mul(self, other: Self) -> Self {
                $type_name::new(self.inner.wrapping_mul(other.inner))
            }

            pub fn unchecked_div<
                Divisor: WrapperNum<$inner_type>,
                Quotient: WrapperNum<$inner_type>,
            >(
                self,
                other: Divisor,
            ) -> Quotient {
                Quotient::new(self.inner / other.as_inner())
            }

            pub fn checked_div(self, other: Self) -> Option<Self> {
                if other.inner == 0 {
                    None
                } else {
                    Some($type_name::new(self.inner / other.inner))
                }
            }

            pub fn checked_div_generic<
                Divisor: WrapperNum<$inner_type>,
                Quotient: WrapperNum<$inner_type>,
            >(
                self,
                other: Divisor,
            ) -> Option<Quotient> {
                let divisor = other.as_inner();
                if divisor == 0 {
                    None
                } else {
                    Some(Quotient::new(self.inner / divisor))
                }
            }

            pub fn leading_zeros(&self) -> u32 {
                self.inner.leading_zeros()
            }
        }

        impl Debug for $type_name {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(f, "{}({})", stringify!($type_name), self.inner)
            }
        }

        impl Display for $type_name {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(f, "{}", self.inner)
            }
        }

        impl Mul for $type_name {
            type Output = Self;

            fn mul(self, other: Self) -> Self {
                $type_name::new(self.inner * other.inner)
            }
        }

        impl Sum<$type_name> for $type_name {
            fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
                iter.fold($type_name::ZERO, |acc, x| acc + x)
            }
        }

        impl Add for $type_name {
            type Output = Self;

            fn add(self, other: Self) -> Self {
                $type_name::new(self.inner + other.inner)
            }
        }

        impl AddAssign for $type_name {
            fn add_assign(&mut self, other: Self) {
                *self = *self + other;
            }
        }

        impl Sub for $type_name {
            type Output = Self;

            fn sub(self, other: Self) -> Self {
                $type_name::new(self.inner - other.inner)
            }
        }

        impl SubAssign for $type_name {
            fn sub_assign(&mut self, other: Self) {
                *self = *self - other;
            }
        }

        // Only implement Default if not already derived
        impl Default for $type_name {
            fn default() -> Self {
                Self::ZERO
            }
        }

        impl PartialEq for $type_name {
            fn eq(&self, other: &Self) -> bool {
                self.inner == other.inner
            }
        }

        impl From<$type_name> for $inner_type {
            fn from(x: $type_name) -> $inner_type {
                x.inner
            }
        }

        impl From<$inner_type> for $type_name {
            fn from(x: $inner_type) -> $type_name {
                $type_name::new(x)
            }
        }

        impl From<$type_name> for f64 {
            fn from(x: $type_name) -> f64 {
                x.inner as f64
            }
        }

        impl Eq for $type_name {}

        #[cfg(feature = "serde")]
        impl serde::Serialize for $type_name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                $crate::quantities::serde_numeric::serialize(self.inner, serializer)
            }
        }

        #[cfg(feature = "serde")]
        impl<'de> serde::Deserialize<'de> for $type_name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                $crate::quantities::serde_numeric::deserialize(deserializer).map($type_name::new)
            }
        }

        // Below should only be used in tests.
        impl PartialEq<$inner_type> for $type_name {
            fn eq(&self, other: &$inner_type) -> bool {
                self.inner == *other
            }
        }

        impl PartialEq<$type_name> for $inner_type {
            fn eq(&self, other: &$type_name) -> bool {
                *self == other.inner
            }
        }
    };
}

/// Defines type-safe multiplication and division between wrapper types.
///
/// # Type Safety
///
/// This macro ensures dimensional correctness in multiplication:
/// - `Type1 * Type2 -> ResultType`
/// - `ResultType / Type1 -> Type2`
/// - `ResultType / Type2 -> Type1`
///
/// # Example
///
/// ```ignore
/// allow_multiply!(BaseUnits, BaseLotsPerBaseUnit, BaseLots);
/// // BaseUnits * BaseLotsPerBaseUnit = BaseLots
/// // BaseLots / BaseUnits = BaseLotsPerBaseUnit
/// ```
#[macro_export]
macro_rules! allow_multiply {
    ($type_1:ident, $type_2:ident, $type_result:ident) => {
        impl Mul<$type_2> for $type_1 {
            type Output = $type_result;

            fn mul(self, other: $type_2) -> $type_result {
                $type_result::new(self.inner * other.inner)
            }
        }

        impl Mul<$type_1> for $type_2 {
            type Output = $type_result;

            fn mul(self, other: $type_1) -> $type_result {
                $type_result::new(self.inner * other.inner)
            }
        }

        impl Div<$type_1> for $type_result {
            type Output = $type_2;

            #[track_caller]
            fn div(self, other: $type_1) -> $type_2 {
                $type_2::new(self.inner / other.inner)
            }
        }

        impl Div<$type_2> for $type_result {
            type Output = $type_1;

            #[track_caller]
            fn div(self, other: $type_2) -> $type_1 {
                $type_1::new(self.inner / other.inner)
            }
        }

        // Generate checked division methods with unique names based on the types
        pastey::paste! {
            impl $type_result {
                #[doc = concat!(
                    "Safely divides this `", stringify!($type_result), "` by a `", stringify!($type_1), "`.\n",
                    "\n",
                    "Returns `Some(", stringify!($type_2), ")` if the division is valid, or `None` if the divisor is zero.\n",
                    "\n",
                    "# Example\n",
                    "```ignore\n",
                    "let result = ", stringify!($type_result), "::new(100);\n",
                    "let divisor = ", stringify!($type_1), "::new(10);\n",
                    "assert_eq!(result.", stringify!([<checked_div_by_ $type_1:snake>]), "(divisor), Some(", stringify!($type_2), "::new(10)));\n",
                    "\n",
                    "let zero = ", stringify!($type_1), "::new(0);\n",
                    "assert_eq!(result.", stringify!([<checked_div_by_ $type_1:snake>]), "(zero), None);\n",
                    "```"
                )]
                pub fn [<checked_div_by_ $type_1:snake>](self, other: $type_1) -> Option<$type_2> {
                    if other.inner == 0 {
                        None
                    } else {
                        Some($type_2::new(self.inner / other.inner))
                    }
                }

                #[doc = concat!(
                    "Safely divides this `", stringify!($type_result), "` by a `", stringify!($type_2), "`.\n",
                    "\n",
                    "Returns `Some(", stringify!($type_1), ")` if the division is valid, or `None` if the divisor is zero.\n",
                    "\n",
                    "# Example\n",
                    "```ignore\n",
                    "let result = ", stringify!($type_result), "::new(100);\n",
                    "let divisor = ", stringify!($type_2), "::new(10);\n",
                    "assert_eq!(result.", stringify!([<checked_div_by_ $type_2:snake>]), "(divisor), Some(", stringify!($type_1), "::new(10)));\n",
                    "\n",
                    "let zero = ", stringify!($type_2), "::new(0);\n",
                    "assert_eq!(result.", stringify!([<checked_div_by_ $type_2:snake>]), "(zero), None);\n",
                    "```"
                )]
                pub fn [<checked_div_by_ $type_2:snake>](self, other: $type_2) -> Option<$type_1> {
                    if other.inner == 0 {
                        None
                    } else {
                        Some($type_1::new(self.inner / other.inner))
                    }
                }
            }
        }

        // Generate checked multiplication methods
        pastey::paste! {
            impl $type_1 {
                #[doc = concat!(
                    "Safely multiplies this `", stringify!($type_1), "` by a `", stringify!($type_2), "`.\n",
                    "\n",
                    "Returns `Some(", stringify!($type_result), ")` if the multiplication doesn't overflow, or `None` if it does.\n"
                )]
                pub fn [<checked_mul_ $type_2:snake>](self, rhs: $type_2) -> Option<$type_result> {
                    self.inner.checked_mul(rhs.inner).map($type_result::new)
                }
            }

            impl $type_2 {
                #[doc = concat!(
                    "Safely multiplies this `", stringify!($type_2), "` by a `", stringify!($type_1), "`.\n",
                    "\n",
                    "Returns `Some(", stringify!($type_result), ")` if the multiplication doesn't overflow, or `None` if it does.\n"
                )]
                pub fn [<checked_mul_ $type_1:snake>](self, rhs: $type_1) -> Option<$type_result> {
                    self.inner.checked_mul(rhs.inner).map($type_result::new)
                }
            }
        }
    };
}

/// Enables safe addition/subtraction between unsigned and signed 64-bit types.
///
/// # Overflow Handling
///
/// This macro carefully handles the interaction between signed and unsigned
/// types to prevent underflow when subtracting larger unsigned values.
///
/// # Panic Safety
///
/// Will panic on underflow when adding negative signed values to unsigned
/// types that would result in negative values.
#[macro_export]
macro_rules! allow_add_64bit {
    ($unsigned_type:ident, $signed_type:ident) => {
        impl $unsigned_type {
            /// Infallible conversion to the signed type. Safe for pairs where the
            /// unsigned bounds fit in the signed representation.
            pub fn as_signed(&self) -> $signed_type {
                $signed_type::new(self.inner as i64)
            }

            /// Checked conversion to the signed type. Returns an error if the value
            /// exceeds i64::MAX.
            pub fn checked_as_signed(&self) -> Result<$signed_type, $crate::quantities::MathError> {
                if self.inner <= i64::MAX as u64 {
                    Ok($signed_type::new(self.inner as i64))
                } else {
                    Err($crate::quantities::MathError::Overflow)
                }
            }
        }

        impl $signed_type {
            /// Checked conversion to the unsigned type. Errors if the value is
            /// negative or exceeds the unsigned type's upper bound.
            pub fn checked_as_unsigned(
                &self,
            ) -> Result<$unsigned_type, $crate::quantities::MathError> {
                if self.inner < 0 {
                    Err($crate::quantities::MathError::Underflow)
                } else if (self.inner as u64)
                    > <$unsigned_type as $crate::quantities::ScalarBounds<u64>>::UPPER_BOUND
                {
                    Err($crate::quantities::MathError::Overflow)
                } else {
                    Ok($unsigned_type::new(self.inner as u64))
                }
            }

            /// Absolute value converted to unsigned.
            pub fn abs_as_unsigned(&self) -> $unsigned_type {
                $unsigned_type::new(self.inner.unsigned_abs())
            }
        }

        impl Add<$signed_type> for $unsigned_type {
            type Output = $unsigned_type;

            fn add(self, other: $signed_type) -> $unsigned_type {
                if other.is_non_negative() {
                    $unsigned_type::new(self.inner + other.inner as u64)
                } else {
                    // For negative signed values, we need to handle potential underflow
                    let abs_value = other.inner.unsigned_abs();
                    if self.inner >= abs_value {
                        $unsigned_type::new(self.inner - abs_value)
                    } else {
                        // Maintain backward compatibility - panic on underflow
                        panic!("Underflow in add operation");
                    }
                }
            }
        }

        impl $unsigned_type {
            /// Safe addition with signed type that returns None on underflow.
            pub fn checked_add_signed(self, other: $signed_type) -> Option<$unsigned_type> {
                if other.is_non_negative() {
                    self.inner
                        .checked_add(other.inner as u64)
                        .map($unsigned_type::new)
                } else {
                    let abs_value = other.inner.unsigned_abs();
                    if self.inner >= abs_value {
                        Some($unsigned_type::new(self.inner - abs_value))
                    } else {
                        None // Underflow
                    }
                }
            }

            /// Saturating addition with signed type that saturates at 0 on underflow.
            pub fn saturating_add_signed(self, other: $signed_type) -> $unsigned_type {
                if other.is_non_negative() {
                    $unsigned_type::new(self.inner.saturating_add(other.inner as u64))
                } else {
                    let abs_value = other.inner.unsigned_abs();
                    $unsigned_type::new(self.inner.saturating_sub(abs_value))
                }
            }
        }

        impl Sub<$signed_type> for $unsigned_type {
            type Output = $unsigned_type;

            fn sub(self, other: $signed_type) -> $unsigned_type {
                if other.is_non_negative() {
                    $unsigned_type::new(self.inner - other.inner.unsigned_abs())
                } else {
                    // Subtracting a negative = adding the absolute value
                    let abs_value = other.inner.unsigned_abs();
                    match self.inner.checked_add(abs_value) {
                        Some(result) => $unsigned_type::new(result),
                        None => panic!("Overflow in sub operation"),
                    }
                }
            }
        }

        impl Add<$unsigned_type> for $signed_type {
            type Output = $signed_type;

            fn add(self, other: $unsigned_type) -> $signed_type {
                let other_signed = other
                    .checked_as_signed()
                    .expect("Overflow in add: unsigned value exceeds signed bounds");
                let sum = self
                    .inner
                    .checked_add(other_signed.as_inner())
                    .expect("Overflow in add: result exceeds signed bounds");
                $signed_type::new(sum)
            }
        }

        impl Sub<$unsigned_type> for $signed_type {
            type Output = $signed_type;

            fn sub(self, other: $unsigned_type) -> $signed_type {
                let other_signed = other
                    .checked_as_signed()
                    .expect("Overflow in sub: unsigned value exceeds signed bounds");
                let diff = self
                    .inner
                    .checked_sub(other_signed.as_inner())
                    .expect("Overflow in sub: result exceeds signed bounds");
                $signed_type::new(diff)
            }
        }

        impl AddAssign<$signed_type> for $unsigned_type {
            fn add_assign(&mut self, other: $signed_type) {
                *self = *self + other;
            }
        }

        impl SubAssign<$signed_type> for $unsigned_type {
            fn sub_assign(&mut self, other: $signed_type) {
                *self = *self - other;
            }
        }

        impl AddAssign<$unsigned_type> for $signed_type {
            fn add_assign(&mut self, other: $unsigned_type) {
                *self = *self + other;
            }
        }

        impl SubAssign<$unsigned_type> for $signed_type {
            fn sub_assign(&mut self, other: $unsigned_type) {
                *self = *self - other;
            }
        }
    };
}

/// Enables safe addition/subtraction between unsigned and signed 64-bit types
/// where the unsigned type may exceed the signed type's representable range.
#[macro_export]
macro_rules! allow_checked_add_64bit {
    ($unsigned_type:ident, $signed_type:ident) => {
        impl $unsigned_type {
            /// Checked conversion to the signed type. Returns overflow if the
            /// unsigned value cannot fit in i64.
            pub fn checked_as_signed(&self) -> Result<$signed_type, $crate::quantities::MathError> {
                if self.inner <= i64::MAX as u64 {
                    Ok($signed_type::new(self.inner as i64))
                } else {
                    Err($crate::quantities::MathError::Overflow)
                }
            }
        }

        impl $signed_type {
            /// Checked conversion to the unsigned type. Errors if the value is
            /// negative or exceeds the unsigned type's upper bound.
            pub fn checked_as_unsigned(
                &self,
            ) -> Result<$unsigned_type, $crate::quantities::MathError> {
                if self.inner < 0 {
                    Err($crate::quantities::MathError::Underflow)
                } else if (self.inner as u64)
                    > <$unsigned_type as $crate::quantities::ScalarBounds<u64>>::UPPER_BOUND
                {
                    Err($crate::quantities::MathError::Overflow)
                } else {
                    Ok($unsigned_type::new(self.inner as u64))
                }
            }

            /// Absolute value converted to unsigned.
            pub fn abs_as_unsigned(&self) -> $unsigned_type {
                $unsigned_type::new(self.inner.unsigned_abs())
            }
        }

        impl Add<$signed_type> for $unsigned_type {
            type Output = $unsigned_type;

            fn add(self, other: $signed_type) -> $unsigned_type {
                if other.is_non_negative() {
                    $unsigned_type::new(self.inner + other.inner as u64)
                } else {
                    let abs_value = other.inner.unsigned_abs();
                    if self.inner >= abs_value {
                        $unsigned_type::new(self.inner - abs_value)
                    } else {
                        panic!("Underflow in add operation");
                    }
                }
            }
        }

        impl $unsigned_type {
            /// Safe addition with signed type that returns None on underflow.
            pub fn checked_add_signed(self, other: $signed_type) -> Option<$unsigned_type> {
                if other.is_non_negative() {
                    self.inner
                        .checked_add(other.inner as u64)
                        .map($unsigned_type::new)
                } else {
                    let abs_value = other.inner.unsigned_abs();
                    if self.inner >= abs_value {
                        Some($unsigned_type::new(self.inner - abs_value))
                    } else {
                        None
                    }
                }
            }

            /// Saturating addition with signed type that saturates at 0 on underflow.
            pub fn saturating_add_signed(self, other: $signed_type) -> $unsigned_type {
                if other.is_non_negative() {
                    $unsigned_type::new(self.inner.saturating_add(other.inner as u64))
                } else {
                    let abs_value = other.inner.unsigned_abs();
                    $unsigned_type::new(self.inner.saturating_sub(abs_value))
                }
            }
        }

        impl Sub<$signed_type> for $unsigned_type {
            type Output = $unsigned_type;

            fn sub(self, other: $signed_type) -> $unsigned_type {
                if other.is_non_negative() {
                    $unsigned_type::new(self.inner - other.inner.unsigned_abs())
                } else {
                    // Subtracting a negative = adding the absolute value
                    let abs_value = other.inner.unsigned_abs();
                    match self.inner.checked_add(abs_value) {
                        Some(result) => $unsigned_type::new(result),
                        None => panic!("Overflow in sub operation"),
                    }
                }
            }
        }

        impl Add<$unsigned_type> for $signed_type {
            type Output = $signed_type;

            fn add(self, other: $unsigned_type) -> $signed_type {
                let other_signed = other
                    .checked_as_signed()
                    .expect("Overflow in add: unsigned value exceeds signed bounds");
                let sum = self
                    .inner
                    .checked_add(other_signed.as_inner())
                    .expect("Overflow in add: result exceeds signed bounds");
                $signed_type::new(sum)
            }
        }

        impl Sub<$unsigned_type> for $signed_type {
            type Output = $signed_type;

            fn sub(self, other: $unsigned_type) -> $signed_type {
                let other_signed = other
                    .checked_as_signed()
                    .expect("Overflow in sub: unsigned value exceeds signed bounds");
                let diff = self
                    .inner
                    .checked_sub(other_signed.as_inner())
                    .expect("Overflow in sub: result exceeds signed bounds");
                $signed_type::new(diff)
            }
        }

        impl AddAssign<$signed_type> for $unsigned_type {
            fn add_assign(&mut self, other: $signed_type) {
                *self = *self + other;
            }
        }

        impl SubAssign<$signed_type> for $unsigned_type {
            fn sub_assign(&mut self, other: $signed_type) {
                *self = *self - other;
            }
        }

        impl AddAssign<$unsigned_type> for $signed_type {
            fn add_assign(&mut self, other: $unsigned_type) {
                *self = *self + other;
            }
        }

        impl SubAssign<$unsigned_type> for $signed_type {
            fn sub_assign(&mut self, other: $unsigned_type) {
                *self = *self - other;
            }
        }
    };
}

/// Enables safe addition/subtraction between unsigned and signed types where
/// the signed representation is bounded by i32::MAX.
#[macro_export]
macro_rules! allow_checked_add_32bit {
    ($unsigned_type:ident, $signed_type:ident) => {
        impl $unsigned_type {
            /// Checked conversion to the signed type. Returns overflow if the
            /// unsigned value cannot fit in i32.
            pub fn checked_as_signed(&self) -> Result<$signed_type, $crate::quantities::MathError> {
                if self.inner <= i32::MAX as u64 {
                    Ok($signed_type::new(self.inner as i64))
                } else {
                    Err($crate::quantities::MathError::Overflow)
                }
            }
        }

        impl $signed_type {
            /// Checked conversion to the unsigned type. Errors if the value is
            /// negative or exceeds the unsigned type's upper bound.
            pub fn checked_as_unsigned(
                &self,
            ) -> Result<$unsigned_type, $crate::quantities::MathError> {
                if self.inner < 0 {
                    Err($crate::quantities::MathError::Underflow)
                } else if (self.inner as u64)
                    > <$unsigned_type as $crate::quantities::ScalarBounds<u64>>::UPPER_BOUND
                {
                    Err($crate::quantities::MathError::Overflow)
                } else {
                    Ok($unsigned_type::new(self.inner as u64))
                }
            }

            /// Absolute value converted to unsigned.
            pub fn abs_as_unsigned(&self) -> $unsigned_type {
                $unsigned_type::new(self.inner.unsigned_abs())
            }
        }

        impl Add<$signed_type> for $unsigned_type {
            type Output = $unsigned_type;

            fn add(self, other: $signed_type) -> $unsigned_type {
                if other.is_non_negative() {
                    $unsigned_type::new(self.inner + other.inner as u64)
                } else {
                    let abs_value = other.inner.unsigned_abs();
                    if self.inner >= abs_value {
                        $unsigned_type::new(self.inner - abs_value)
                    } else {
                        panic!("Underflow in add operation");
                    }
                }
            }
        }

        impl $unsigned_type {
            /// Safe addition with signed type that returns None on underflow.
            pub fn checked_add_signed(self, other: $signed_type) -> Option<$unsigned_type> {
                if other.is_non_negative() {
                    self.inner
                        .checked_add(other.inner as u64)
                        .map($unsigned_type::new)
                } else {
                    let abs_value = other.inner.unsigned_abs();
                    if self.inner >= abs_value {
                        Some($unsigned_type::new(self.inner - abs_value))
                    } else {
                        None
                    }
                }
            }

            /// Saturating addition with signed type that saturates at 0 on underflow.
            pub fn saturating_add_signed(self, other: $signed_type) -> $unsigned_type {
                if other.is_non_negative() {
                    $unsigned_type::new(self.inner.saturating_add(other.inner as u64))
                } else {
                    let abs_value = other.inner.unsigned_abs();
                    $unsigned_type::new(self.inner.saturating_sub(abs_value))
                }
            }
        }

        impl Sub<$signed_type> for $unsigned_type {
            type Output = $unsigned_type;

            fn sub(self, other: $signed_type) -> $unsigned_type {
                if other.is_non_negative() {
                    $unsigned_type::new(self.inner - other.inner.unsigned_abs())
                } else {
                    // Subtracting a negative = adding the absolute value
                    let abs_value = other.inner.unsigned_abs();
                    match self.inner.checked_add(abs_value) {
                        Some(result) => $unsigned_type::new(result),
                        None => panic!("Overflow in sub operation"),
                    }
                }
            }
        }

        impl Add<$unsigned_type> for $signed_type {
            type Output = $signed_type;

            fn add(self, other: $unsigned_type) -> $signed_type {
                let other_signed = other
                    .checked_as_signed()
                    .expect("Overflow in add: unsigned value exceeds signed bounds");
                let sum = self
                    .inner
                    .checked_add(other_signed.as_inner())
                    .expect("Overflow in add: result exceeds signed bounds");
                $signed_type::new(sum)
            }
        }

        impl Sub<$unsigned_type> for $signed_type {
            type Output = $signed_type;

            fn sub(self, other: $unsigned_type) -> $signed_type {
                let other_signed = other
                    .checked_as_signed()
                    .expect("Overflow in sub: unsigned value exceeds signed bounds");
                let diff = self
                    .inner
                    .checked_sub(other_signed.as_inner())
                    .expect("Overflow in sub: result exceeds signed bounds");
                $signed_type::new(diff)
            }
        }

        impl AddAssign<$signed_type> for $unsigned_type {
            fn add_assign(&mut self, other: $signed_type) {
                *self = *self + other;
            }
        }

        impl SubAssign<$signed_type> for $unsigned_type {
            fn sub_assign(&mut self, other: $signed_type) {
                *self = *self - other;
            }
        }

        impl AddAssign<$unsigned_type> for $signed_type {
            fn add_assign(&mut self, other: $unsigned_type) {
                *self = *self + other;
            }
        }

        impl SubAssign<$unsigned_type> for $signed_type {
            fn sub_assign(&mut self, other: $unsigned_type) {
                *self = *self - other;
            }
        }
    };
}
