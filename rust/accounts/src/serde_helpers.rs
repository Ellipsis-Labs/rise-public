//! Serde helpers for account views.

pub(crate) fn pubkey_string(value: &[u8; 32]) -> String {
    bs58::encode(value).into_string()
}
