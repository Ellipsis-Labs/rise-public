use super::PERP_ASSET_MAP_ACCOUNT;
use crate::common::PhoenixAccountDecodeError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AssetSymbol {
    bytes: [u8; 16],
    len: u8,
}

impl AssetSymbol {
    pub(crate) fn try_from_bytes(bytes: [u8; 16]) -> Result<Self, PhoenixAccountDecodeError> {
        Self::try_from_bytes_for_account(bytes, PERP_ASSET_MAP_ACCOUNT)
    }

    pub(crate) fn try_from_bytes_for_account(
        bytes: [u8; 16],
        account: &'static str,
    ) -> Result<Self, PhoenixAccountDecodeError> {
        let len = bytes
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(bytes.len());
        if !bytes[..len].iter().all(u8::is_ascii) {
            return Err(PhoenixAccountDecodeError::InvalidData {
                account,
                reason: "asset symbol contains non-ASCII bytes",
            });
        }
        Ok(Self {
            bytes,
            len: len as u8,
        })
    }

    #[inline(always)]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.len as usize]
    }

    #[inline(always)]
    pub fn as_str(&self) -> &str {
        core::str::from_utf8(self.as_bytes())
            .expect("AssetSymbol is validated as ASCII during decode")
    }

    #[inline(always)]
    pub fn matches(&self, symbol: &str) -> bool {
        self.as_bytes() == symbol.as_bytes()
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for AssetSymbol {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}
