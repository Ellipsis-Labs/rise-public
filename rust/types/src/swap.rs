//! Request/response types for the swap transaction builder endpoint.

use serde::{Deserialize, Serialize};

use crate::ix::ApiInstructionResponse;

/// Collateral assets accepted by the swap transaction builder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SwapAsset {
    /// Native SOL (lamports).
    Sol,
    /// The exchange's canonical quote token (quote atoms).
    Quote,
    /// SPL spot collateral identified by its base58 mint (mint atoms).
    Spot(String),
}

/// Request payload for /v1/ix/swap.
///
/// Venue configuration (venue list, base URLs, API keys, shared lookup
/// tables, slippage bounds) is server-side; the request only selects what to
/// swap and how to quote it.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildSwapTransactionRequest {
    /// Wallet the trader PDA derives from. Without `position_authority` it is
    /// also the swapper: sole required signer of the `swap_native`
    /// instruction, owner of every token account the swap touches, and the
    /// default fee payer.
    pub authority: String,
    /// Position authority delegated on the trader account. When provided it
    /// becomes the swapper: it signs `swap_native` and owns the staged token
    /// accounts in place of `authority`. The trader PDA still derives from
    /// `authority`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position_authority: Option<String>,
    /// Asset sold (exact-in).
    pub asset_in: SwapAsset,
    /// Asset bought. Must differ from `asset_in`.
    pub asset_out: SwapAsset,
    /// Exact-in amount in atoms of `asset_in` (lamports for SOL, quote atoms
    /// for quote, mint atoms for spot). Exactly one of `amount_atoms` /
    /// `quantity` is required.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amount_atoms: Option<u64>,
    /// Exact-in amount in whole units of `asset_in` (e.g. 0.5 SOL);
    /// converted server-side using the input asset's decimals. Exactly one
    /// of `amount_atoms` / `quantity` is required.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quantity: Option<f64>,
    /// Which of the authority's trader PDAs the swap settles collateral
    /// against; defaults to 0.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trader_pda_index: Option<u8>,
    /// Subaccount of the selected trader PDA; defaults to 0 (cross-margin).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trader_subaccount_index: Option<u8>,
    /// Fee payer for the compiled transaction; defaults to the swapper
    /// (`position_authority` when provided, else `authority`). Does not
    /// change the swap's signer, only who pays the transaction fee.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fee_payer: Option<String>,
    /// Slippage tolerance override in basis points; must not exceed the
    /// server-configured maximum.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slippage_bps: Option<u16>,
    /// Restrict planning to the venue with this id from the server config
    /// (e.g. "jupiter"). By default every configured venue is quoted and the
    /// best minimum output wins.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub venue: Option<String>,
    /// Beta quote->SOL swaps only: canonical quote collateral debited by
    /// `SwapNative` in quote atoms, decoupled from the venue amount.
    /// Defaults to the venue amount. Rejected on other deployments or
    /// directions.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub collateral_amount_atoms: Option<u64>,
    /// Priority fee (compute unit price) in micro-lamports. A v0
    /// transaction carries it as a compute-budget instruction; a v1
    /// transaction carries `ceil(price * compute_unit_limit / 1e6)` lamports
    /// in its message config. Sponsored swaps must set fees here rather than
    /// recompiling the message, since the sponsorship attestation covers the
    /// exact compiled bytes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compute_unit_price_micro_lamports: Option<u64>,
    /// Transaction format to compile: `0` (default) for a v0 message with
    /// address lookup tables, `1` for a v1 message (SIMD-0385) that inlines
    /// every address and carries the compute budget in its config instead of
    /// compute-budget instructions. Other values are rejected.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transaction_version: Option<u8>,
}

/// Response payload for /v1/ix/swap.
///
/// Carries the same plan twice: a compiled unsigned transaction for clients
/// that sign and send as-is, and the instruction/lookup-table breakdown for
/// clients that compile their own message (own blockhash, fee payer, and
/// safety instructions).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildSwapTransactionResponse {
    /// Base64 wire bytes of the unsigned transaction (placeholder
    /// signatures) in the format `transaction_version` names, compiled
    /// against `blockhash`. v0 carries the compute budget as instructions and
    /// loads the resolved lookup tables; v1 carries it in the message config.
    pub transaction: String,
    /// Message version of `transaction`: `0` or `1`.
    pub transaction_version: u8,
    /// The transaction's instructions for clients compiling their own
    /// message: compute budget first for v0, none for v1, then the swap.
    pub instructions: Vec<ApiInstructionResponse>,
    /// Address lookup tables the instructions require. A swap route does not
    /// fit in a legacy transaction, so client-side compilation must load
    /// these tables and compile a v0 message. Empty for v1.
    pub address_lookup_table_addresses: Vec<String>,
    /// Venue id that won the quote.
    pub venue: String,
    /// Venue-leg input amount in atoms of the input asset, after deducting
    /// the server-configured swap fee.
    pub amount_in_atoms: u64,
    /// `SwapNative`'s collateral `amount_in`, including the swap fee;
    /// overridden on a beta buy with `collateral_amount_atoms` set.
    pub swap_native_amount_in_atoms: u64,
    /// Venue's quoted output in atoms of the output asset.
    pub expected_amount_out_atoms: u64,
    /// Minimum output enforced on-chain, derived from the slippage
    /// tolerance.
    pub min_amount_out_atoms: u64,
    /// Venue-reported price impact in basis points.
    pub price_impact_bps: u32,
    /// Slot the venue quoted at.
    pub context_slot: u64,
    /// Compute unit limit set on the compiled transaction.
    pub compute_unit_limit: u32,
    /// Recent blockhash the transaction was compiled against.
    pub blockhash: String,
    /// Last block height at which `blockhash` (and therefore `transaction`)
    /// is valid.
    pub last_valid_block_height: u64,
    /// Provenance attestation over the compiled message, for the
    /// sponsorship server. Present only when the server has an attestation
    /// key configured. Issued for the swapper (`position_authority` when
    /// provided, else `authority`). Sign `transaction` verbatim and pass this
    /// through to POST /v1/sponsorship/transaction; any recompilation
    /// invalidates it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sponsorship_attestation: Option<String>,
    /// Unix time at which `sponsorship_attestation` expires.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attestation_expires_at: Option<i64>,
}
