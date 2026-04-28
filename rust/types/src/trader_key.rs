//! Trader key identification and PDA derivation.

use solana_pubkey::Pubkey;

/// The Phoenix Eternal program ID (mainnet).
pub const ETERNAL_PROGRAM_ID: Pubkey =
    solana_pubkey::pubkey!("EtrnLzgbS7nMMy5fbD42kXiUzGg8XQzJ972Xtk1cjWih");

/// Subaccount index for the cross-margin (primary) account.
pub const CROSS_MARGIN_SUBACCOUNT_IDX: u8 = 0;

/// Identifies a trader on Phoenix by authority pubkey and PDA indices.
#[derive(Debug, Clone)]
pub struct TraderKey {
    /// The authority pubkey (wallet address).
    pub authority: Pubkey,
    /// The PDA index for this trader (0-255).
    pub pda_index: u8,
    /// The subaccount index (0 for cross-margin main account, 1+ for isolated
    /// subaccounts).
    pub subaccount_index: u8,
}

impl TraderKey {
    pub fn derive_pda(authority: &Pubkey, pda_index: u8, subaccount_index: u8) -> Pubkey {
        let pda_schema = [pda_index, subaccount_index];
        let (pda, _bump) = Pubkey::find_program_address(
            &[b"trader", authority.as_ref(), pda_schema.as_ref()],
            &ETERNAL_PROGRAM_ID,
        );
        pda
    }

    pub fn new(authority: Pubkey) -> Self {
        Self {
            authority,
            pda_index: 0,
            subaccount_index: 0,
        }
    }

    pub fn new_with_idx(authority: Pubkey, pda_index: u8, subaccount_index: u8) -> Self {
        Self {
            authority,
            pda_index,
            subaccount_index,
        }
    }

    pub fn from_authority(authority: Pubkey) -> Self {
        Self {
            authority,
            pda_index: 0,
            subaccount_index: 0,
        }
    }

    pub fn from_authority_with_idx(authority: Pubkey, pda_index: u8, subaccount_index: u8) -> Self {
        Self {
            authority,
            pda_index,
            subaccount_index,
        }
    }

    pub fn pda(&self) -> Pubkey {
        Self::derive_pda(&self.authority, self.pda_index, self.subaccount_index)
    }

    pub fn authority(&self) -> Pubkey {
        self.authority
    }

    pub fn authority_string(&self) -> String {
        self.authority.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trader_key_pda_derivation() {
        let authority = Pubkey::new_unique();
        let key = TraderKey::new(authority);

        let pda1 = key.pda();
        let pda2 = key.pda();
        assert_eq!(pda1, pda2);

        assert_eq!(key.authority(), authority);

        assert_eq!(key.pda_index, 0);
        assert_eq!(key.subaccount_index, 0);
    }

    #[test]
    fn test_new_with_idx() {
        let authority = Pubkey::new_unique();
        let key = TraderKey::new_with_idx(authority, 5, 3);

        assert_eq!(key.pda_index, 5);
        assert_eq!(key.subaccount_index, 3);
    }
}
