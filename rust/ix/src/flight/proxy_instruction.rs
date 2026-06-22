//! Flight: Proxy Instruction construction.
//!
//! Wraps a Phoenix instruction so that the Flight program can record a builder
//! fee before forwarding the inner instruction. Mirrors the TS SDK's
//! `buildProxyInstructionIx`
//! (`ts/src/flight/core/ixBuilders/ProxyInstruction`).

use solana_pubkey::Pubkey;

use crate::ix::constants::PHOENIX_PROGRAM_ID;
use crate::ix::error::PhoenixIxError;
use crate::ix::flight::constants::{
    FLIGHT_PROGRAM_ID, flight_proxy_instruction_discriminant,
    flight_proxy_instruction_with_fee_override_discriminant, get_flight_builder_state_address,
    get_flight_global_state_address,
};
use crate::ix::types::{AccountMeta, Instruction};

const MAX_FEE_BPS_OVERRIDE: u64 = 10_000;

/// Parameters for the Flight `proxy_instruction` wrapper.
#[derive(Debug, Clone)]
pub struct ProxyInstructionParams {
    /// The registered builder's authority (readonly)
    builder_authority: Pubkey,
    /// The builder's Phoenix trader subaccount PDA (writable)
    builder_trader_account: Pubkey,
    /// The end trader's wallet authority (readonly)
    trader_wallet: Pubkey,
    /// The inner Phoenix instruction being wrapped. Its `program_id` must be
    /// the Phoenix program; its accounts and data are appended verbatim to the
    /// proxy instruction.
    inner_instruction: Instruction,
    /// Optional builder fee override in basis points. When present, the
    /// instruction uses Flight's `proxy_instruction_with_fee_override` variant.
    fee_bps_override: Option<u64>,
}

impl ProxyInstructionParams {
    pub fn builder() -> ProxyInstructionParamsBuilder {
        ProxyInstructionParamsBuilder::new()
    }

    pub fn builder_authority(&self) -> Pubkey {
        self.builder_authority
    }

    pub fn builder_trader_account(&self) -> Pubkey {
        self.builder_trader_account
    }

    pub fn trader_wallet(&self) -> Pubkey {
        self.trader_wallet
    }

    pub fn inner_instruction(&self) -> &Instruction {
        &self.inner_instruction
    }

    pub fn fee_bps_override(&self) -> Option<u64> {
        self.fee_bps_override
    }
}

#[derive(Default)]
pub struct ProxyInstructionParamsBuilder {
    builder_authority: Option<Pubkey>,
    builder_trader_account: Option<Pubkey>,
    trader_wallet: Option<Pubkey>,
    inner_instruction: Option<Instruction>,
    fee_bps_override: Option<u64>,
}

impl ProxyInstructionParamsBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn builder_authority(mut self, builder_authority: Pubkey) -> Self {
        self.builder_authority = Some(builder_authority);
        self
    }

    pub fn builder_trader_account(mut self, builder_trader_account: Pubkey) -> Self {
        self.builder_trader_account = Some(builder_trader_account);
        self
    }

    pub fn trader_wallet(mut self, trader_wallet: Pubkey) -> Self {
        self.trader_wallet = Some(trader_wallet);
        self
    }

    pub fn inner_instruction(mut self, inner_instruction: Instruction) -> Self {
        self.inner_instruction = Some(inner_instruction);
        self
    }

    pub fn fee_bps_override(mut self, fee_bps_override: u64) -> Self {
        self.fee_bps_override = Some(fee_bps_override);
        self
    }

    pub fn build(self) -> Result<ProxyInstructionParams, PhoenixIxError> {
        if self
            .fee_bps_override
            .is_some_and(|fee_bps_override| fee_bps_override > MAX_FEE_BPS_OVERRIDE)
        {
            return Err(PhoenixIxError::InvalidFeeBpsOverride);
        }

        Ok(ProxyInstructionParams {
            builder_authority: self
                .builder_authority
                .ok_or(PhoenixIxError::MissingField("builder_authority"))?,
            builder_trader_account: self
                .builder_trader_account
                .ok_or(PhoenixIxError::MissingField("builder_trader_account"))?,
            trader_wallet: self
                .trader_wallet
                .ok_or(PhoenixIxError::MissingField("trader_wallet"))?,
            inner_instruction: self
                .inner_instruction
                .ok_or(PhoenixIxError::MissingField("inner_instruction"))?,
            fee_bps_override: self.fee_bps_override,
        })
    }
}

/// Create a Flight proxy instruction wrapping an inner Phoenix instruction.
///
/// The inner instruction's data is appended verbatim after the Flight
/// discriminant, and its accounts are appended verbatim after the six Flight
/// accounts. When `fee_bps_override` is set, this emits Flight's
/// `proxy_instruction_with_fee_override` variant with the Borsh
/// `Option<u64>` fee prefix before the inner instruction data. Returns an
/// error if `inner_instruction.program_id` is not the Phoenix program.
pub fn create_proxy_instruction_ix(
    params: ProxyInstructionParams,
) -> Result<Instruction, PhoenixIxError> {
    if params.inner_instruction().program_id != *PHOENIX_PROGRAM_ID {
        return Err(PhoenixIxError::InvalidInnerProgram);
    }

    let inner_data = &params.inner_instruction().data;
    let data = encode_proxy_instruction_data(params.fee_bps_override(), inner_data);

    let mut accounts = Vec::with_capacity(6 + params.inner_instruction().accounts.len());
    accounts.push(AccountMeta::readonly(get_flight_global_state_address()));
    accounts.push(AccountMeta::readonly(*PHOENIX_PROGRAM_ID));
    accounts.push(AccountMeta::readonly(params.builder_authority()));
    accounts.push(AccountMeta::writable(params.builder_trader_account()));
    accounts.push(AccountMeta::readonly(get_flight_builder_state_address(
        &params.builder_authority(),
    )));
    accounts.push(AccountMeta::readonly(params.trader_wallet()));
    accounts.extend(params.inner_instruction().accounts.iter().cloned());

    Ok(Instruction {
        program_id: FLIGHT_PROGRAM_ID,
        accounts,
        data,
    })
}

fn encode_proxy_instruction_data(fee_bps_override: Option<u64>, inner_data: &[u8]) -> Vec<u8> {
    match fee_bps_override {
        Some(fee_bps_override) => {
            let prefix = flight_proxy_instruction_with_fee_override_discriminant();
            let mut data = Vec::with_capacity(prefix.len() + 1 + 8 + inner_data.len());
            data.extend_from_slice(&prefix);
            data.push(1);
            data.extend_from_slice(&fee_bps_override.to_le_bytes());
            data.extend_from_slice(inner_data);
            data
        }
        None => {
            let prefix = flight_proxy_instruction_discriminant();
            let mut data = Vec::with_capacity(prefix.len() + inner_data.len());
            data.extend_from_slice(&prefix);
            data.extend_from_slice(inner_data);
            data
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_inner_ix(data: Vec<u8>, accounts: Vec<AccountMeta>) -> Instruction {
        Instruction {
            program_id: *PHOENIX_PROGRAM_ID,
            accounts,
            data,
        }
    }

    #[test]
    fn test_rejects_non_phoenix_program() {
        let inner = Instruction {
            program_id: Pubkey::new_unique(),
            accounts: vec![],
            data: vec![],
        };
        let params = ProxyInstructionParams::builder()
            .builder_authority(Pubkey::new_unique())
            .builder_trader_account(Pubkey::new_unique())
            .trader_wallet(Pubkey::new_unique())
            .inner_instruction(inner)
            .build()
            .unwrap();

        let result = create_proxy_instruction_ix(params);
        assert!(matches!(result, Err(PhoenixIxError::InvalidInnerProgram)));
    }

    #[test]
    fn test_data_prefixed_with_discriminant() {
        let inner_data = vec![0xAA, 0xBB, 0xCC];
        let inner = make_inner_ix(inner_data.clone(), vec![]);
        let params = ProxyInstructionParams::builder()
            .builder_authority(Pubkey::new_unique())
            .builder_trader_account(Pubkey::new_unique())
            .trader_wallet(Pubkey::new_unique())
            .inner_instruction(inner)
            .build()
            .unwrap();

        let ix = create_proxy_instruction_ix(params).unwrap();
        assert_eq!(ix.program_id, FLIGHT_PROGRAM_ID);
        assert_eq!(&ix.data[..8], &flight_proxy_instruction_discriminant());
        assert_eq!(&ix.data[8..], &inner_data[..]);
    }

    #[test]
    fn test_data_with_fee_override_prefixed_with_discriminant_and_params() {
        let inner_data = vec![0xAA, 0xBB, 0xCC];
        let inner = make_inner_ix(inner_data.clone(), vec![]);
        let params = ProxyInstructionParams::builder()
            .builder_authority(Pubkey::new_unique())
            .builder_trader_account(Pubkey::new_unique())
            .trader_wallet(Pubkey::new_unique())
            .fee_bps_override(5)
            .inner_instruction(inner)
            .build()
            .unwrap();

        let ix = create_proxy_instruction_ix(params).unwrap();
        assert_eq!(ix.program_id, FLIGHT_PROGRAM_ID);
        assert_eq!(
            &ix.data[..8],
            &flight_proxy_instruction_with_fee_override_discriminant()
        );
        assert_eq!(ix.data[8], 1);
        assert_eq!(&ix.data[9..17], &5u64.to_le_bytes());
        assert_eq!(&ix.data[17..], &inner_data[..]);
    }

    #[test]
    fn test_account_layout_and_passthrough() {
        let builder_authority = Pubkey::new_unique();
        let builder_trader_account = Pubkey::new_unique();
        let trader_wallet = Pubkey::new_unique();
        let inner_a = Pubkey::new_unique();
        let inner_b = Pubkey::new_unique();
        let inner = make_inner_ix(
            vec![1, 2, 3],
            vec![
                AccountMeta::writable_signer(inner_a),
                AccountMeta::readonly(inner_b),
            ],
        );

        let params = ProxyInstructionParams::builder()
            .builder_authority(builder_authority)
            .builder_trader_account(builder_trader_account)
            .trader_wallet(trader_wallet)
            .inner_instruction(inner)
            .build()
            .unwrap();

        let ix = create_proxy_instruction_ix(params).unwrap();
        assert_eq!(ix.accounts.len(), 8);

        assert_eq!(ix.accounts[0].pubkey, get_flight_global_state_address());
        assert!(!ix.accounts[0].is_writable);

        assert_eq!(ix.accounts[1].pubkey, *PHOENIX_PROGRAM_ID);

        assert_eq!(ix.accounts[2].pubkey, builder_authority);
        assert!(!ix.accounts[2].is_writable);
        assert!(!ix.accounts[2].is_signer);

        assert_eq!(ix.accounts[3].pubkey, builder_trader_account);
        assert!(ix.accounts[3].is_writable);

        assert_eq!(
            ix.accounts[4].pubkey,
            get_flight_builder_state_address(&builder_authority)
        );
        assert!(!ix.accounts[4].is_writable);

        assert_eq!(ix.accounts[5].pubkey, trader_wallet);

        // Inner accounts preserved verbatim (flags included).
        assert_eq!(ix.accounts[6].pubkey, inner_a);
        assert!(ix.accounts[6].is_signer);
        assert!(ix.accounts[6].is_writable);
        assert_eq!(ix.accounts[7].pubkey, inner_b);
        assert!(!ix.accounts[7].is_signer);
        assert!(!ix.accounts[7].is_writable);
    }

    #[test]
    fn test_missing_inner_instruction() {
        let result = ProxyInstructionParams::builder()
            .builder_authority(Pubkey::new_unique())
            .builder_trader_account(Pubkey::new_unique())
            .trader_wallet(Pubkey::new_unique())
            .build();
        assert!(matches!(
            result,
            Err(PhoenixIxError::MissingField("inner_instruction"))
        ));
    }

    #[test]
    fn test_invalid_fee_override() {
        let result = ProxyInstructionParams::builder()
            .builder_authority(Pubkey::new_unique())
            .builder_trader_account(Pubkey::new_unique())
            .trader_wallet(Pubkey::new_unique())
            .fee_bps_override(10_001)
            .inner_instruction(make_inner_ix(vec![], vec![]))
            .build();
        assert!(matches!(result, Err(PhoenixIxError::InvalidFeeBpsOverride)));
    }
}
