//! Serde helpers for instruction parameters.

use core::str::FromStr;

use serde::de::Error as DeError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use solana_pubkey::Pubkey;

pub(crate) mod pubkey {
    use super::*;

    pub fn serialize<S>(value: &Pubkey, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&value.to_string())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Pubkey, D::Error>
    where
        D: Deserializer<'de>,
    {
        let encoded = String::deserialize(deserializer)?;
        Pubkey::from_str(&encoded).map_err(D::Error::custom)
    }
}

pub(crate) mod pubkey_option {
    use super::*;

    pub fn serialize<S>(value: &Option<Pubkey>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        value.as_ref().map(Pubkey::to_string).serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Pubkey>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let encoded = Option::<String>::deserialize(deserializer)?;
        encoded
            .as_deref()
            .map(Pubkey::from_str)
            .transpose()
            .map_err(D::Error::custom)
    }
}

pub(crate) mod pubkey_vec {
    use super::*;

    pub fn serialize<S>(value: &Vec<Pubkey>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        value
            .iter()
            .map(Pubkey::to_string)
            .collect::<Vec<_>>()
            .serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<Pubkey>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let encoded = Vec::<String>::deserialize(deserializer)?;
        encoded
            .iter()
            .map(|value| Pubkey::from_str(value).map_err(D::Error::custom))
            .collect()
    }
}
