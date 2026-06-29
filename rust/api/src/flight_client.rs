//! High-level helper mirroring the TS `PhoenixFlightClient`
//! (`ts/src/flight/client.ts`).
//!
//! A Flight builder wraps supported Phoenix placement instructions in a
//! `proxy_instruction` before forwarding execution to Phoenix.

use phoenix_rise_ix::flight::{ProxyInstructionParams, create_proxy_instruction_ix};
use phoenix_rise_ix::types::{AccountMeta, Instruction as RiseInstruction};
use phoenix_rise_ix::{PhoenixInstruction, PhoenixIxError};
use solana_instruction::Instruction;
use solana_pubkey::Pubkey;

use crate::trader_key::TraderKey;

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

    /// Wrap `ix` in a Flight `proxy_instruction` if Flight supports routing it;
    /// return it unchanged otherwise.
    ///
    /// Mirrors TS `PhoenixFlightClient.tryWrapFlightInstruction` at
    /// `ts/src/flight/client.ts`.
    pub fn try_wrap_order_instruction(
        &self,
        ix: Instruction,
        trader_wallet: Pubkey,
    ) -> Result<Instruction, PhoenixIxError> {
        self.try_wrap_order_instruction_with_fee_bps_override(ix, trader_wallet, None)
    }

    /// Wrap `ix` in a Flight `proxy_instruction_with_fee_override` if Flight
    /// supports routing it and `fee_bps_override` is `Some`; return it
    /// unchanged for unsupported instructions.
    pub fn try_wrap_order_instruction_with_fee_bps_override(
        &self,
        ix: Instruction,
        trader_wallet: Pubkey,
        fee_bps_override: Option<u64>,
    ) -> Result<Instruction, PhoenixIxError> {
        if !is_flight_routable_instruction(&ix) {
            return Ok(ix);
        }

        let inner = to_phoenix_rise_ix_instruction(ix);

        let mut params_builder = ProxyInstructionParams::builder()
            .builder_authority(self.builder_authority)
            .builder_trader_account(self.builder_trader_account())
            .trader_wallet(trader_wallet)
            .inner_instruction(inner);
        if let Some(fee_bps_override) = fee_bps_override {
            params_builder = params_builder.fee_bps_override(fee_bps_override);
        }
        let params = params_builder.build()?;

        Ok(create_proxy_instruction_ix(params)?.into())
    }
}

/// Returns true if the instruction targets a Flight-routable placement
/// instruction. Mirrors TS `isFlightRoutableInstruction`
/// (`ts/src/flight/helper.ts`).
pub fn is_flight_routable_instruction(ix: &Instruction) -> bool {
    has_discriminant(
        &ix.data,
        &PhoenixInstruction::PlaceLimitOrder.discriminant(),
    ) || has_discriminant(
        &ix.data,
        &PhoenixInstruction::PlaceMarketOrder.discriminant(),
    ) || has_discriminant(&ix.data, &PhoenixInstruction::PlaceStopLoss.discriminant())
        || has_discriminant(
            &ix.data,
            &PhoenixInstruction::PlacePositionConditionalOrder.discriminant(),
        )
        || has_discriminant(
            &ix.data,
            &PhoenixInstruction::PlaceAttachedConditionalOrder.discriminant(),
        )
        || has_discriminant(
            &ix.data,
            &PhoenixInstruction::PlaceLimitOrderWithConditionals.discriminant(),
        )
}

fn has_discriminant(data: &[u8], discriminant: &[u8; 8]) -> bool {
    data.len() >= discriminant.len() && data[..discriminant.len()] == *discriminant
}

/// Convert a `solana_instruction::Instruction` into a
/// `phoenix_rise_ix::types::Instruction` (same wire shape; distinct types in
/// the IX crate to keep it solana-dependency-light).
fn to_phoenix_rise_ix_instruction(ix: Instruction) -> RiseInstruction {
    let accounts = ix
        .accounts
        .into_iter()
        .map(|meta| AccountMeta {
            pubkey: meta.pubkey,
            is_signer: meta.is_signer,
            is_writable: meta.is_writable,
        })
        .collect();

    RiseInstruction {
        program_id: ix.program_id,
        accounts,
        data: ix.data,
    }
}

#[cfg(test)]
mod tests {
    use phoenix_rise_ix::limit_order::{LimitOrderParams, create_place_limit_order_ix};
    use phoenix_rise_ix::types::Side;
    use solana_instruction::AccountMeta as SolAccountMeta;

    use super::*;

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

    fn build_sample_phoenix_ix(discriminant: [u8; 8]) -> Instruction {
        Instruction {
            program_id: *phoenix_rise_ix::PHOENIX_PROGRAM_ID,
            accounts: vec![SolAccountMeta::new_readonly(Pubkey::new_unique(), false)],
            data: discriminant.to_vec(),
        }
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
            phoenix_rise_ix::flight::FLIGHT_PROGRAM_ID
        );
        // Data = flight discriminant + inner data.
        assert_eq!(
            &wrapped.data[..8],
            &phoenix_rise_ix::FlightInstruction::ProxyInstruction.discriminant()
        );
        assert_eq!(&wrapped.data[8..], &inner_data[..]);
    }

    #[test]
    fn test_wraps_order_placing_ix_with_fee_bps_override() {
        let client = PhoenixFlightClient::new(Pubkey::new_unique(), 0, 0);
        let trader_wallet = Pubkey::new_unique();
        let inner = build_sample_limit_ix();
        let inner_data = inner.data.clone();

        let wrapped = client
            .try_wrap_order_instruction_with_fee_bps_override(inner, trader_wallet, Some(5))
            .unwrap();

        assert_eq!(
            wrapped.program_id,
            phoenix_rise_ix::flight::FLIGHT_PROGRAM_ID
        );
        assert_eq!(
            &wrapped.data[..8],
            &phoenix_rise_ix::FlightInstruction::ProxyInstructionWithFeeOverride.discriminant()
        );
        assert_eq!(wrapped.data[8], 1);
        assert_eq!(&wrapped.data[9..17], &5u64.to_le_bytes());
        assert_eq!(&wrapped.data[17..], &inner_data[..]);
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
    fn test_is_flight_routable_instruction() {
        assert!(is_flight_routable_instruction(&build_sample_limit_ix()));
        assert!(is_flight_routable_instruction(&build_sample_phoenix_ix(
            PhoenixInstruction::PlaceMarketOrder.discriminant()
        )));
        assert!(!is_flight_routable_instruction(&build_sample_phoenix_ix(
            PhoenixInstruction::PlaceMarketOrderDelegated.discriminant()
        )));
        assert!(is_flight_routable_instruction(&build_sample_phoenix_ix(
            PhoenixInstruction::PlaceStopLoss.discriminant()
        )));
        assert!(is_flight_routable_instruction(&build_sample_phoenix_ix(
            PhoenixInstruction::PlacePositionConditionalOrder.discriminant()
        )));
        assert!(is_flight_routable_instruction(&build_sample_phoenix_ix(
            PhoenixInstruction::PlaceAttachedConditionalOrder.discriminant()
        )));
        assert!(is_flight_routable_instruction(&build_sample_phoenix_ix(
            PhoenixInstruction::PlaceLimitOrderWithConditionals.discriminant()
        )));

        let fake = Instruction {
            program_id: Pubkey::new_unique(),
            accounts: vec![],
            data: vec![0u8; 8],
        };
        assert!(!is_flight_routable_instruction(&fake));
    }
}
