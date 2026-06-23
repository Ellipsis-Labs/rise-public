use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum AccountDeserializeError {
    #[error("{account} account is too short: expected at least {expected} bytes, got {actual}")]
    TooShort {
        account: &'static str,
        expected: usize,
        actual: usize,
    },

    #[error("{account} account has invalid discriminant: expected {expected:?}, got {actual:?}")]
    InvalidDiscriminant {
        account: &'static str,
        expected: [u8; 8],
        actual: [u8; 8],
    },

    #[error("{account} account has invalid data: {message}")]
    InvalidData {
        account: &'static str,
        message: String,
    },
}

impl AccountDeserializeError {
    pub(crate) fn too_short(account: &'static str, expected: usize, actual: usize) -> Self {
        Self::TooShort {
            account,
            expected,
            actual,
        }
    }

    pub(crate) fn invalid_data(account: &'static str, message: impl Into<String>) -> Self {
        Self::InvalidData {
            account,
            message: message.into(),
        }
    }
}
