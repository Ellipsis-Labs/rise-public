use core::fmt;
use core::marker::PhantomData;
use core::str::FromStr;

use serde::de::{self, Unexpected, Visitor};

pub trait JsonSafeInteger: Copy + fmt::Display + FromStr {
    const EXPECTING: &'static str;

    fn from_i64<E: de::Error>(value: i64) -> Result<Self, E>;
    fn from_u64<E: de::Error>(value: u64) -> Result<Self, E>;
    fn from_i128<E: de::Error>(value: i128) -> Result<Self, E>;
    fn from_u128<E: de::Error>(value: u128) -> Result<Self, E>;
}

pub fn serialize<S, T>(value: T, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
    T: JsonSafeInteger,
{
    serializer.serialize_str(&value.to_string())
}

pub fn deserialize<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: JsonSafeInteger,
{
    deserializer.deserialize_any(JsonSafeIntegerVisitor::<T>(PhantomData))
}

struct JsonSafeIntegerVisitor<T>(PhantomData<T>);

impl<'de, T> Visitor<'de> for JsonSafeIntegerVisitor<T>
where
    T: JsonSafeInteger,
{
    type Value = T;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(T::EXPECTING)
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        T::from_i64(value)
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        T::from_u64(value)
    }

    fn visit_i128<E>(self, value: i128) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        T::from_i128(value)
    }

    fn visit_u128<E>(self, value: u128) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        T::from_u128(value)
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        value
            .parse()
            .map_err(|_| E::invalid_value(Unexpected::Str(value), &T::EXPECTING))
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visit_str(&value)
    }
}

macro_rules! impl_json_safe_unsigned {
    ($ty:ty, $expecting:literal) => {
        impl JsonSafeInteger for $ty {
            const EXPECTING: &'static str = $expecting;

            fn from_i64<E: de::Error>(value: i64) -> Result<Self, E> {
                Self::try_from(value)
                    .map_err(|_| E::invalid_value(Unexpected::Signed(value), &Self::EXPECTING))
            }

            fn from_u64<E: de::Error>(value: u64) -> Result<Self, E> {
                Self::try_from(value)
                    .map_err(|_| E::invalid_value(Unexpected::Unsigned(value), &Self::EXPECTING))
            }

            fn from_i128<E: de::Error>(value: i128) -> Result<Self, E> {
                Self::try_from(value)
                    .map_err(|_| E::custom(format_args!("{value} is out of range")))
            }

            fn from_u128<E: de::Error>(value: u128) -> Result<Self, E> {
                Self::try_from(value)
                    .map_err(|_| E::custom(format_args!("{value} is out of range")))
            }
        }
    };
}

macro_rules! impl_json_safe_signed {
    ($ty:ty, $expecting:literal) => {
        impl JsonSafeInteger for $ty {
            const EXPECTING: &'static str = $expecting;

            fn from_i64<E: de::Error>(value: i64) -> Result<Self, E> {
                Self::try_from(value)
                    .map_err(|_| E::invalid_value(Unexpected::Signed(value), &Self::EXPECTING))
            }

            fn from_u64<E: de::Error>(value: u64) -> Result<Self, E> {
                Self::try_from(value)
                    .map_err(|_| E::invalid_value(Unexpected::Unsigned(value), &Self::EXPECTING))
            }

            fn from_i128<E: de::Error>(value: i128) -> Result<Self, E> {
                Self::try_from(value)
                    .map_err(|_| E::custom(format_args!("{value} is out of range")))
            }

            fn from_u128<E: de::Error>(value: u128) -> Result<Self, E> {
                Self::try_from(value)
                    .map_err(|_| E::custom(format_args!("{value} is out of range")))
            }
        }
    };
}

impl_json_safe_unsigned!(u32, "a u32 encoded as a JSON number or decimal string");
impl_json_safe_unsigned!(u64, "a u64 encoded as a JSON number or decimal string");
impl_json_safe_unsigned!(u128, "a u128 encoded as a JSON number or decimal string");
impl_json_safe_signed!(i32, "an i32 encoded as a JSON number or decimal string");
impl_json_safe_signed!(i64, "an i64 encoded as a JSON number or decimal string");
impl_json_safe_signed!(i128, "an i128 encoded as a JSON number or decimal string");
