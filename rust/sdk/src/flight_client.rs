//! High-level helper mirroring the TS `PhoenixFlightClient`
//! (`ts/src/flight/client.ts`).
//!
//! A Flight builder wraps order-placing Phoenix instructions in a
//! `proxy_instruction` so the Flight program can record its fee before
//! forwarding execution to Phoenix.

use solana_instruction::Instruction;
use solana_pubkey::Pubkey;

use crate::phoenix_rise_ix::flight::{ProxyInstructionParams, create_proxy_instruction_ix};
use crate::phoenix_rise_ix::{
    PhoenixIxError, place_limit_order_discriminant, place_market_order_discriminant,
};
use crate::phoenix_rise_types::TraderKey;

/// Fields identifying a registered Flight builder.
#[derive(Debug, Clone, Copy)]
pub struct PhoenixFlightClient {
    pub builder_authority: Pubkey,
    pub builder_pda_index: u8,
    pub builder_subaccount_index: u8,
}

impl PhoenixFlightClient {
    pub fn new(
        builder_authority: Pubkey,
        builder_pda_index: u8,
        builder_subaccount_index: u8,
    ) -> Self {
        Self {
            builder_authority,
            builder_pda_index,
            builder_subaccount_index,
        }
    }

    /// The builder's own trader subaccount PDA, which receives fee credit.
    pub fn builder_trader_account(&self) -> Pubkey {
        TraderKey::derive_pda(
            &self.builder_authority,
            self.builder_pda_index,
            self.builder_subaccount_index,
        )
    }

    /// Wrap `ix` in a Flight `proxy_instruction` if it places an order; return
    /// it unchanged otherwise.
    ///
    /// Mirrors TS `PhoenixFlightClient.tryWrapFlightInstruction` at
    /// `ts/src/flight/client.ts`.
    pub fn try_wrap_order_instruction(
        &self,
        ix: Instruction,
        trader_wallet: Pubkey,
    ) -> Result<Instruction, PhoenixIxError> {
        if !is_order_placing_instruction(&ix) {
            return Ok(ix);
        }

        let inner = to_phoenix_rise_ix_instruction(ix);

        let params = ProxyInstructionParams::builder()
            .builder_authority(self.builder_authority)
            .builder_trader_account(self.builder_trader_account())
            .trader_wallet(trader_wallet)
            .inner_instruction(inner)
            .build()?;

        Ok(create_proxy_instruction_ix(params)?.into())
    }
}

/// Returns true if the instruction targets `place_limit_order` or
/// `place_market_order`. Mirrors TS `isOrderPlacingInstruction`
/// (`ts/src/flight/helper.ts`).
pub fn is_order_placing_instruction(ix: &Instruction) -> bool {
    has_discriminant(&ix.data, &place_limit_order_discriminant())
        || has_discriminant(&ix.data, &place_market_order_discriminant())
}

fn has_discriminant(data: &[u8], discriminant: &[u8; 8]) -> bool {
    data.len() >= discriminant.len() && data[..discriminant.len()] == *discriminant
}

/// Convert a `solana_instruction::Instruction` into a
/// `crate::phoenix_rise_ix::Instruction` (same wire shape; distinct types in
/// the IX crate to keep it solana-dependency-light).
fn to_phoenix_rise_ix_instruction(ix: Instruction) -> crate::phoenix_rise_ix::Instruction {
    let accounts = ix
        .accounts
        .into_iter()
        .map(|meta| crate::phoenix_rise_ix::AccountMeta {
            pubkey: meta.pubkey,
            is_signer: meta.is_signer,
            is_writable: meta.is_writable,
        })
        .collect();

    crate::phoenix_rise_ix::Instruction {
        program_id: ix.program_id,
        accounts,
        data: ix.data,
    }
}

#[cfg(test)]
mod tests {
    use solana_instruction::AccountMeta as SolAccountMeta;

    use super::*;
    use crate::phoenix_rise_ix::{LimitOrderParams, Side, create_place_limit_order_ix};

    fn build_sample_limit_ix() -> Instruction {
        let params = LimitOrderParams::builder()
            .trader(Pubkey::new_unique())
            .trader_account(Pubkey::new_unique())
            .perp_asset_map(Pubkey::new_unique())
            .orderbook(Pubkey::new_unique())
            .spline_collection(Pubkey::new_unique())
            .global_trader_index(vec![Pubkey::new_unique()])
            .active_trader_buffer(vec![Pubkey::new_unique()])
            .side(Side::Bid)
            .price_in_ticks(1000)
            .num_base_lots(100)
            .build()
            .unwrap();
        create_place_limit_order_ix(params).unwrap().into()
    }

    #[test]
    fn test_wraps_order_placing_ix() {
        let client = PhoenixFlightClient::new(Pubkey::new_unique(), 0, 0);
        let trader_wallet = Pubkey::new_unique();
        let inner = build_sample_limit_ix();
        let inner_data = inner.data.clone();

        let wrapped = client
            .try_wrap_order_instruction(inner, trader_wallet)
            .unwrap();

        // Wrapped ix targets Flight, not Phoenix.
        assert_eq!(
            wrapped.program_id,
            crate::phoenix_rise_ix::flight::FLIGHT_PROGRAM_ID
        );
        // Data = flight discriminant + inner data.
        assert_eq!(
            &wrapped.data[..8],
            &crate::phoenix_rise_ix::flight::flight_proxy_instruction_discriminant()
        );
        assert_eq!(&wrapped.data[8..], &inner_data[..]);
    }

    #[test]
    fn test_non_order_ix_passthrough() {
        let client = PhoenixFlightClient::new(Pubkey::new_unique(), 0, 0);
        let trader_wallet = Pubkey::new_unique();

        let unrelated = Instruction {
            program_id: Pubkey::new_unique(),
            accounts: vec![SolAccountMeta::new_readonly(Pubkey::new_unique(), false)],
            data: vec![0xDE, 0xAD, 0xBE, 0xEF],
        };

        let result = client
            .try_wrap_order_instruction(unrelated.clone(), trader_wallet)
            .unwrap();

        assert_eq!(result.program_id, unrelated.program_id);
        assert_eq!(result.data, unrelated.data);
        assert_eq!(result.accounts.len(), unrelated.accounts.len());
    }

    #[test]
    fn test_is_order_placing_instruction() {
        assert!(is_order_placing_instruction(&build_sample_limit_ix()));

        let fake = Instruction {
            program_id: Pubkey::new_unique(),
            accounts: vec![],
            data: vec![0u8; 8],
        };
        assert!(!is_order_placing_instruction(&fake));
    }
}
