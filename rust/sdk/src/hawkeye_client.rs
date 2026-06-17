use base64::Engine as _;
use solana_compute_budget_interface::ComputeBudgetInstruction;
use solana_instruction::Instruction;
use solana_pubkey::Pubkey;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_rpc_client_api::config::RpcSimulateTransactionConfig;
use solana_rpc_client_api::response::RpcSimulateTransactionResult;
use solana_transaction::Transaction;
use thiserror::Error;

use crate::PhoenixMetadata;
use crate::phoenix_rise_ix::{
    HAWKEYE_PROGRAM_ID, HAWKEYE_SIMULATION_COMPUTE_UNIT_LIMIT, HAWKEYE_SIMULATION_FEE_PAYER,
    HawkeyeReturnData, HawkeyeReturnDataError, ViewAssetReturn, ViewBboReturn, ViewFundingReturn,
    ViewLiquidationPriceReturn, ViewMarginReturn, decode_hawkeye_return,
    decode_hawkeye_return_data,
};
use crate::tx_builder::{PhoenixTxBuilder, PhoenixTxBuilderError};

#[derive(Debug, Clone)]
pub struct HawkeyeSimulation<T> {
    pub value: T,
    pub logs: Vec<String>,
    pub units_consumed: Option<u64>,
    pub raw_return_data: Vec<u8>,
}

#[derive(Debug, Error)]
pub enum PhoenixHawkeyeClientError {
    #[error("Phoenix transaction builder error: {0}")]
    Builder(#[from] PhoenixTxBuilderError),

    #[error("RPC error: {0}")]
    Rpc(String),

    #[error("Hawkeye simulation failed: {0:?}")]
    SimulationFailed(solana_transaction_error::TransactionError),

    #[error("Hawkeye simulation did not set return data")]
    MissingReturnData,

    #[error("Hawkeye simulation returned data for program {actual}, expected {expected}")]
    WrongReturnProgram { actual: String, expected: Pubkey },

    #[error("Hawkeye simulation returned unsupported return-data encoding")]
    UnsupportedReturnDataEncoding,

    #[error("Hawkeye builder produced {actual} instructions, expected exactly 1")]
    UnexpectedInstructionCount { actual: usize },

    #[error("Failed to base64 decode Hawkeye return data: {0}")]
    Base64(#[from] base64::DecodeError),

    #[error("Hawkeye return-data decode error: {0}")]
    Decode(#[from] HawkeyeReturnDataError),
}

pub struct PhoenixHawkeyeClient<'a> {
    rpc_client: RpcClient,
    builder: PhoenixTxBuilder<'a>,
    fee_payer: Pubkey,
    compute_unit_limit: u32,
}

impl<'a> PhoenixHawkeyeClient<'a> {
    pub fn new(rpc_url: impl Into<String>, metadata: &'a PhoenixMetadata) -> Self {
        Self {
            rpc_client: RpcClient::new(rpc_url.into()),
            builder: PhoenixTxBuilder::new(metadata),
            fee_payer: HAWKEYE_SIMULATION_FEE_PAYER,
            compute_unit_limit: HAWKEYE_SIMULATION_COMPUTE_UNIT_LIMIT,
        }
    }

    pub fn with_fee_payer(mut self, fee_payer: Pubkey) -> Self {
        self.fee_payer = fee_payer;
        self
    }

    pub fn with_compute_unit_limit(mut self, compute_unit_limit: u32) -> Self {
        self.compute_unit_limit = compute_unit_limit;
        self
    }

    pub async fn simulate_instruction(
        &self,
        instruction: Instruction,
    ) -> Result<HawkeyeSimulation<HawkeyeReturnData>, PhoenixHawkeyeClientError> {
        let result = self.simulate_raw(instruction).await?;
        let bytes = decode_simulation_return_data(&result, HAWKEYE_PROGRAM_ID)?;
        let value = decode_hawkeye_return_data(&bytes)?;
        Ok(HawkeyeSimulation {
            value,
            logs: result.logs.unwrap_or_default(),
            units_consumed: result.units_consumed,
            raw_return_data: bytes,
        })
    }

    pub async fn view_margin(
        &self,
        authority: Pubkey,
        pda_index: u8,
        subaccount_index: u8,
    ) -> Result<HawkeyeSimulation<ViewMarginReturn>, PhoenixHawkeyeClientError> {
        let instruction = only_instruction(self.builder.build_hawkeye_view_margin(
            authority,
            pda_index,
            subaccount_index,
        )?)?;
        self.simulate_typed(instruction, "view_margin").await
    }

    pub async fn view_margin_for_trader(
        &self,
        trader: Pubkey,
    ) -> Result<HawkeyeSimulation<ViewMarginReturn>, PhoenixHawkeyeClientError> {
        let instruction =
            only_instruction(self.builder.build_hawkeye_view_margin_for_trader(trader)?)?;
        self.simulate_typed(instruction, "view_margin").await
    }

    pub async fn view_margin_for_asset(
        &self,
        authority: Pubkey,
        pda_index: u8,
        subaccount_index: u8,
        asset_id: u32,
    ) -> Result<HawkeyeSimulation<ViewAssetReturn>, PhoenixHawkeyeClientError> {
        let instruction = only_instruction(self.builder.build_hawkeye_view_margin_for_asset(
            authority,
            pda_index,
            subaccount_index,
            asset_id,
        )?)?;
        self.simulate_typed(instruction, "view_margin_for_asset")
            .await
    }

    pub async fn view_margin_for_asset_for_trader(
        &self,
        trader: Pubkey,
        asset_id: u32,
    ) -> Result<HawkeyeSimulation<ViewAssetReturn>, PhoenixHawkeyeClientError> {
        let instruction = only_instruction(
            self.builder
                .build_hawkeye_view_margin_for_asset_for_trader(trader, asset_id)?,
        )?;
        self.simulate_typed(instruction, "view_margin_for_asset")
            .await
    }

    pub async fn view_liquidation_price(
        &self,
        authority: Pubkey,
        pda_index: u8,
        subaccount_index: u8,
        asset_id: u32,
    ) -> Result<HawkeyeSimulation<ViewLiquidationPriceReturn>, PhoenixHawkeyeClientError> {
        let instruction = only_instruction(self.builder.build_hawkeye_view_liquidation_price(
            authority,
            pda_index,
            subaccount_index,
            asset_id,
        )?)?;
        self.simulate_typed(instruction, "view_liquidation_price")
            .await
    }

    pub async fn view_liquidation_price_for_trader(
        &self,
        trader: Pubkey,
        asset_id: u32,
    ) -> Result<HawkeyeSimulation<ViewLiquidationPriceReturn>, PhoenixHawkeyeClientError> {
        let instruction = only_instruction(
            self.builder
                .build_hawkeye_view_liquidation_price_for_trader(trader, asset_id)?,
        )?;
        self.simulate_typed(instruction, "view_liquidation_price")
            .await
    }

    pub async fn view_bbo(
        &self,
        symbol: &str,
    ) -> Result<HawkeyeSimulation<ViewBboReturn>, PhoenixHawkeyeClientError> {
        let instruction = only_instruction(self.builder.build_hawkeye_view_bbo(symbol)?)?;
        self.simulate_typed(instruction, "view_bbo").await
    }

    pub async fn view_funding(
        &self,
        authority: Pubkey,
        pda_index: u8,
        subaccount_index: u8,
        asset_id: u32,
    ) -> Result<HawkeyeSimulation<ViewFundingReturn>, PhoenixHawkeyeClientError> {
        let instruction = only_instruction(self.builder.build_hawkeye_view_funding(
            authority,
            pda_index,
            subaccount_index,
            asset_id,
        )?)?;
        self.simulate_typed(instruction, "view_funding").await
    }

    pub async fn view_funding_for_trader(
        &self,
        trader: Pubkey,
        asset_id: u32,
    ) -> Result<HawkeyeSimulation<ViewFundingReturn>, PhoenixHawkeyeClientError> {
        let instruction = only_instruction(
            self.builder
                .build_hawkeye_view_funding_for_trader(trader, asset_id)?,
        )?;
        self.simulate_typed(instruction, "view_funding").await
    }

    async fn simulate_typed<T: bytemuck::Pod + Copy>(
        &self,
        instruction: Instruction,
        label: &'static str,
    ) -> Result<HawkeyeSimulation<T>, PhoenixHawkeyeClientError> {
        let result = self.simulate_raw(instruction).await?;
        let bytes = decode_simulation_return_data(&result, HAWKEYE_PROGRAM_ID)?;
        let value = decode_hawkeye_return::<T>(&bytes, label)?;
        Ok(HawkeyeSimulation {
            value,
            logs: result.logs.unwrap_or_default(),
            units_consumed: result.units_consumed,
            raw_return_data: bytes,
        })
    }

    async fn simulate_raw(
        &self,
        instruction: Instruction,
    ) -> Result<RpcSimulateTransactionResult, PhoenixHawkeyeClientError> {
        let compute_budget_ix =
            ComputeBudgetInstruction::set_compute_unit_limit(self.compute_unit_limit);
        let instructions = [compute_budget_ix, instruction];
        let transaction = Transaction::new_with_payer(&instructions, Some(&self.fee_payer));
        let config = RpcSimulateTransactionConfig {
            sig_verify: false,
            replace_recent_blockhash: true,
            commitment: Some(self.rpc_client.commitment()),
            encoding: None,
            accounts: None,
            min_context_slot: None,
            inner_instructions: false,
        };
        let response = self
            .rpc_client
            .simulate_transaction_with_config(&transaction, config)
            .await
            .map_err(|error| PhoenixHawkeyeClientError::Rpc(error.to_string()))?;
        if let Some(err) = response.value.err {
            return Err(PhoenixHawkeyeClientError::SimulationFailed(err));
        }
        Ok(response.value)
    }
}

fn only_instruction(
    mut instructions: Vec<Instruction>,
) -> Result<Instruction, PhoenixHawkeyeClientError> {
    if instructions.len() != 1 {
        return Err(PhoenixHawkeyeClientError::UnexpectedInstructionCount {
            actual: instructions.len(),
        });
    }
    Ok(instructions.pop().expect("checked one instruction"))
}

fn decode_simulation_return_data(
    result: &RpcSimulateTransactionResult,
    expected_program_id: Pubkey,
) -> Result<Vec<u8>, PhoenixHawkeyeClientError> {
    let return_data = result
        .return_data
        .as_ref()
        .ok_or(PhoenixHawkeyeClientError::MissingReturnData)?;
    if return_data.program_id != expected_program_id.to_string() {
        return Err(PhoenixHawkeyeClientError::WrongReturnProgram {
            actual: return_data.program_id.clone(),
            expected: expected_program_id,
        });
    }
    if return_data.data.1 != solana_transaction_status_client_types::UiReturnDataEncoding::Base64 {
        return Err(PhoenixHawkeyeClientError::UnsupportedReturnDataEncoding);
    }
    Ok(base64::engine::general_purpose::STANDARD.decode(&return_data.data.0)?)
}
