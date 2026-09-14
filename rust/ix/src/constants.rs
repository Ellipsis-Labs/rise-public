//! Phoenix program constants and addresses.

#[cfg(not(target_os = "solana"))]
use std::sync::LazyLock;

use sha2::{Digest, Sha256};
use solana_pubkey::Pubkey;

use super::discriminants::EmberInstruction;
#[cfg(test)]
use super::discriminants::PhoenixInstruction;
use super::error::PhoenixIxError;

/// The Phoenix program ID (mainnet).
pub const PROD_PHOENIX_PROGRAM_ID: Pubkey =
    solana_pubkey::pubkey!("EtrnLzgbS7nMMy5fbD42kXiUzGg8XQzJ972Xtk1cjWih");

/// The Phoenix program ID (beta).
pub const BETA_PHOENIX_PROGRAM_ID: Pubkey =
    solana_pubkey::pubkey!("phDEVv4w6BcfkLrLNeXr8HhhgQxnxziVGXpGPcaadMf");

/// The Phoenix log authority address (mainnet).
pub const PROD_PHOENIX_LOG_AUTHORITY: Pubkey =
    solana_pubkey::pubkey!("GdxfTLSsdSY37G6fZoYtdGDSfgFnbT2EmRpuePZxWShS");

/// The Phoenix log authority address (beta).
pub const BETA_PHOENIX_LOG_AUTHORITY: Pubkey =
    solana_pubkey::pubkey!("8Q1zeC7qAPUhJ2ncHAW8N1TGcpMVDsSxdSBPLaE8G2ab");

/// The Phoenix global configuration address (mainnet).
pub const PROD_PHOENIX_GLOBAL_CONFIGURATION: Pubkey =
    solana_pubkey::pubkey!("2zskx2iyCvb6Stg7RBZkt1f6MrF4dpYtMG3yMvKwqtUZ");

/// The Phoenix global configuration address (beta).
pub const BETA_PHOENIX_GLOBAL_CONFIGURATION: Pubkey =
    solana_pubkey::pubkey!("3CkM38UaZW6nyTJku4ABE5jjS5AComQErrkd55LGTfxa");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PhoenixInstructionAddresses {
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    pub program_id: Pubkey,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    pub log_authority: Pubkey,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_helpers::pubkey"))]
    pub global_configuration: Pubkey,
}

pub const PROD_PHOENIX_INSTRUCTION_ADDRESSES: PhoenixInstructionAddresses =
    PhoenixInstructionAddresses {
        program_id: PROD_PHOENIX_PROGRAM_ID,
        log_authority: PROD_PHOENIX_LOG_AUTHORITY,
        global_configuration: PROD_PHOENIX_GLOBAL_CONFIGURATION,
    };

pub const BETA_PHOENIX_INSTRUCTION_ADDRESSES: PhoenixInstructionAddresses =
    PhoenixInstructionAddresses {
        program_id: BETA_PHOENIX_PROGRAM_ID,
        log_authority: BETA_PHOENIX_LOG_AUTHORITY,
        global_configuration: BETA_PHOENIX_GLOBAL_CONFIGURATION,
    };

/// Active Phoenix instruction addresses for the current process.
///
/// This is resolved once from `PHOENIX_ENV`. Set `PHOENIX_ENV=beta` before
/// first use to target the beta deployment.
#[cfg(not(target_os = "solana"))]
pub static PHOENIX_INSTRUCTION_ADDRESSES: LazyLock<PhoenixInstructionAddresses> =
    LazyLock::new(|| {
        resolve_phoenix_instruction_addresses_for_env(std::env::var("PHOENIX_ENV").ok().as_deref())
    });

#[cfg(target_os = "solana")]
pub static PHOENIX_INSTRUCTION_ADDRESSES: &PhoenixInstructionAddresses =
    &PROD_PHOENIX_INSTRUCTION_ADDRESSES;

/// Active Phoenix program ID for the current process.
///
/// This is resolved once from `PHOENIX_ENV`. Set `PHOENIX_ENV=beta` before
/// first use to target the beta deployment.
#[cfg(not(target_os = "solana"))]
pub static PHOENIX_PROGRAM_ID: LazyLock<Pubkey> =
    LazyLock::new(|| phoenix_instruction_addresses().program_id);

#[cfg(target_os = "solana")]
pub static PHOENIX_PROGRAM_ID: &Pubkey = &PROD_PHOENIX_PROGRAM_ID;

/// Active Phoenix log authority for the current process.
#[cfg(not(target_os = "solana"))]
pub static PHOENIX_LOG_AUTHORITY: LazyLock<Pubkey> =
    LazyLock::new(|| phoenix_instruction_addresses().log_authority);

#[cfg(target_os = "solana")]
pub static PHOENIX_LOG_AUTHORITY: &Pubkey = &PROD_PHOENIX_LOG_AUTHORITY;

/// Active Phoenix global configuration for the current process.
#[cfg(not(target_os = "solana"))]
pub static PHOENIX_GLOBAL_CONFIGURATION: LazyLock<Pubkey> =
    LazyLock::new(|| phoenix_instruction_addresses().global_configuration);

#[cfg(target_os = "solana")]
pub static PHOENIX_GLOBAL_CONFIGURATION: &Pubkey = &PROD_PHOENIX_GLOBAL_CONFIGURATION;

/// Resolve Phoenix instruction addresses for an explicit environment value.
///
/// `beta` selects the beta deployment. Everything else defaults to production.
pub fn resolve_phoenix_instruction_addresses_for_env(
    phoenix_env: Option<&str>,
) -> PhoenixInstructionAddresses {
    match phoenix_env
        .map(str::trim)
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("beta") => BETA_PHOENIX_INSTRUCTION_ADDRESSES,
        _ => PROD_PHOENIX_INSTRUCTION_ADDRESSES,
    }
}

/// Resolve Phoenix instruction addresses once from `PHOENIX_ENV`.
pub fn phoenix_instruction_addresses() -> PhoenixInstructionAddresses {
    *PHOENIX_INSTRUCTION_ADDRESSES
}

/// The active Phoenix program ID for the current process.
pub fn phoenix_program_id() -> Pubkey {
    *PHOENIX_PROGRAM_ID
}

/// The active Phoenix log authority for the current process.
pub fn phoenix_log_authority() -> Pubkey {
    *PHOENIX_LOG_AUTHORITY
}

/// The active Phoenix global configuration for the current process.
pub fn phoenix_global_configuration() -> Pubkey {
    *PHOENIX_GLOBAL_CONFIGURATION
}

/// The Ember program ID (for USDC -> Phoenix token conversion).
pub const EMBER_PROGRAM_ID: Pubkey =
    solana_pubkey::pubkey!("EMBERpYNE6ehWmXymZZS2skiFmCa9V5dp14e1iduM5qy");

/// USDC mint address (mainnet).
///
/// Prefer [`usdc_mint`] when the process may target the beta deployment.
pub const USDC_MINT: Pubkey =
    solana_pubkey::pubkey!("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");

/// USDC mint address (beta).
pub const BETA_USDC_MINT: Pubkey =
    solana_pubkey::pubkey!("DPTSTVhvfzQhY8pAJL5EeqfZxf9aNKTmHErfG4R3Z1SE");

/// Active USDC mint for the current process.
///
/// This is resolved once from `PHOENIX_ENV`. Set `PHOENIX_ENV=beta` before
/// first use to target the beta deployment.
#[cfg(not(target_os = "solana"))]
static ACTIVE_USDC_MINT: LazyLock<Pubkey> =
    LazyLock::new(|| resolve_usdc_mint_for_env(std::env::var("PHOENIX_ENV").ok().as_deref()));

#[cfg(target_os = "solana")]
static ACTIVE_USDC_MINT: &Pubkey = &USDC_MINT;

/// Resolve the USDC mint for an explicit environment value.
///
/// `beta` selects the beta deployment. Everything else defaults to production.
pub fn resolve_usdc_mint_for_env(phoenix_env: Option<&str>) -> Pubkey {
    match phoenix_env
        .map(str::trim)
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("beta") => BETA_USDC_MINT,
        _ => USDC_MINT,
    }
}

/// The active USDC mint for the current process.
pub fn usdc_mint() -> Pubkey {
    *ACTIVE_USDC_MINT
}

/// SPL Token program ID.
pub const SPL_TOKEN_PROGRAM_ID: Pubkey =
    solana_pubkey::pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

/// Associated Token program ID.
pub const ASSOCIATED_TOKEN_PROGRAM_ID: Pubkey =
    solana_pubkey::pubkey!("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");

/// System program ID.
pub const SYSTEM_PROGRAM_ID: Pubkey = solana_pubkey::pubkey!("11111111111111111111111111111111");

/// Compute the instruction discriminant using SHA-256.
/// Takes the first 8 bytes of SHA-256 hash of the input string.
pub fn compute_discriminant(input: &str) -> [u8; 8] {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    let result = hasher.finalize();
    let mut discriminant = [0u8; 8];
    discriminant.copy_from_slice(&result[..8]);
    discriminant
}

/// Instruction discriminant for ember deposit.
pub fn ember_deposit_discriminant() -> [u8; 8] {
    EmberInstruction::Deposit.discriminant()
}

/// Instruction discriminant for ember withdraw.
pub fn ember_withdraw_discriminant() -> [u8; 8] {
    EmberInstruction::Withdraw.discriminant()
}

/// Derives the stop loss PDA for a given trader account and asset ID.
///
/// Seeds: ["stoploss", trader_account, &asset_id.to_le_bytes()]
pub fn get_stop_loss_address(
    trader_account: &Pubkey,
    asset_id: u64,
) -> Result<Pubkey, PhoenixIxError> {
    derive_program_address(
        &[
            b"stoploss",
            trader_account.as_ref(),
            &asset_id.to_le_bytes(),
        ],
        &phoenix_program_id(),
    )
}

/// Derives the conditional orders PDA for a given trader account.
///
/// Seeds: ["conditional_orders", trader_account]
pub fn get_conditional_orders_address(trader_account: &Pubkey) -> Result<Pubkey, PhoenixIxError> {
    derive_program_address(
        &[b"conditional_orders", trader_account.as_ref()],
        &phoenix_program_id(),
    )
}

/// Derives the permission PDA for an authority and delegated signer.
///
/// Seeds: ["permission", permission_authority, delegated_key]
pub fn get_permission_address(
    permission_authority: &Pubkey,
    delegated_key: &Pubkey,
) -> Result<Pubkey, PhoenixIxError> {
    derive_program_address(
        &[
            b"permission",
            permission_authority.as_ref(),
            delegated_key.as_ref(),
        ],
        &phoenix_program_id(),
    )
}

/// Derives the spline collection PDA for a given market (orderbook) address.
///
/// Seeds: ["spline", market_address]
pub fn get_spline_collection_address(market: &Pubkey) -> Result<Pubkey, PhoenixIxError> {
    derive_program_address(&[b"spline", market.as_ref()], &phoenix_program_id())
}

/// Derives the Ember state PDA.
///
/// Seeds: [phoenix_program_id, "state"] against Ember program
pub fn get_ember_state_address() -> Result<Pubkey, PhoenixIxError> {
    let program_id = *PHOENIX_PROGRAM_ID;
    derive_program_address(&[program_id.as_ref(), b"state"], &EMBER_PROGRAM_ID)
}

/// Derives the Ember vault PDA.
///
/// Seeds: [phoenix_program_id, "vault"] against Ember program
pub fn get_ember_vault_address() -> Result<Pubkey, PhoenixIxError> {
    let program_id = *PHOENIX_PROGRAM_ID;
    derive_program_address(&[program_id.as_ref(), b"vault"], &EMBER_PROGRAM_ID)
}

/// Derives the global vault PDA for a given mint.
///
/// Seeds: ["vault", mint] against Phoenix program
pub fn get_global_vault_address(mint: &Pubkey) -> Result<Pubkey, PhoenixIxError> {
    derive_program_address(&[b"vault", mint.as_ref()], &phoenix_program_id())
}

/// Derives the native SOL authority PDA, which custodies native SOL spot
/// collateral and signs the program's own withdrawals of it.
///
/// Seeds: ["native_sol"] against Phoenix program
pub fn get_native_sol_authority_address() -> Result<Pubkey, PhoenixIxError> {
    derive_program_address(&[b"native_sol"], &phoenix_program_id())
}

/// Derives the associated token address for an owner and mint.
///
/// This follows the standard SPL ATA derivation.
pub fn get_associated_token_address(
    owner: &Pubkey,
    mint: &Pubkey,
) -> Result<Pubkey, PhoenixIxError> {
    derive_program_address(
        &[owner.as_ref(), SPL_TOKEN_PROGRAM_ID.as_ref(), mint.as_ref()],
        &ASSOCIATED_TOKEN_PROGRAM_ID,
    )
}

fn derive_program_address(seeds: &[&[u8]], program_id: &Pubkey) -> Result<Pubkey, PhoenixIxError> {
    let (pda, _bump) = Pubkey::find_program_address(seeds, program_id);
    Ok(pda)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolves_beta_instruction_addresses() {
        let addresses = resolve_phoenix_instruction_addresses_for_env(Some(" beta "));

        assert_eq!(addresses.program_id, BETA_PHOENIX_PROGRAM_ID);
        assert_eq!(addresses.log_authority, BETA_PHOENIX_LOG_AUTHORITY);
        assert_eq!(
            addresses.global_configuration,
            BETA_PHOENIX_GLOBAL_CONFIGURATION
        );
    }

    #[test]
    fn test_resolves_prod_instruction_addresses_by_default() {
        let addresses = resolve_phoenix_instruction_addresses_for_env(None);

        assert_eq!(addresses.program_id, PROD_PHOENIX_PROGRAM_ID);
        assert_eq!(addresses.log_authority, PROD_PHOENIX_LOG_AUTHORITY);
        assert_eq!(
            addresses.global_configuration,
            PROD_PHOENIX_GLOBAL_CONFIGURATION
        );
        assert_eq!(
            resolve_phoenix_instruction_addresses_for_env(Some("prod")),
            addresses
        );
        assert_eq!(
            resolve_phoenix_instruction_addresses_for_env(Some("unexpected")),
            addresses
        );
    }

    #[test]
    fn test_resolves_beta_usdc_mint() {
        assert_eq!(resolve_usdc_mint_for_env(Some(" beta ")), BETA_USDC_MINT);
        assert_ne!(BETA_USDC_MINT, USDC_MINT);
    }

    #[test]
    fn test_resolves_prod_usdc_mint_by_default() {
        assert_eq!(resolve_usdc_mint_for_env(None), USDC_MINT);
        assert_eq!(resolve_usdc_mint_for_env(Some("prod")), USDC_MINT);
        assert_eq!(resolve_usdc_mint_for_env(Some("unexpected")), USDC_MINT);
    }

    #[test]
    fn phoenix_instruction_set_covers_eternal_types_discriminants() {
        const ETERNAL_INSTRUCTIONS: &[(&str, &str)] = &[
            ("Rebirth", "rebirth"),
            ("AddMarket", "add_market"),
            ("PlaceMarketOrder", "place_market_order"),
            ("PlaceMarketOrderDelegated", "place_market_order_delegated"),
            ("PlaceLimitOrder", "place_limit_order"),
            ("PlaceMultiLimitOrder", "place_multi_limit_order"),
            ("CancelOrdersById", "cancel_orders_by_id"),
            ("CancelUpTo", "cancel_up_to"),
            ("CancelAll", "cancel_all"),
            ("ForceCancelAll", "force_cancel_all"),
            (
                "MarketClosedForceCancelAll",
                "market_closed_force_cancel_all",
            ),
            ("ForceCancelRiskIncreasing", "force_cancel_risk_increasing"),
            ("ForceCancelAfterHours", "force_cancel_after_hours"),
            ("AuthorizedForceCancel", "authorized_force_cancel"),
            ("AuthorizedForceCancelById", "authorized_force_cancel_by_id"),
            ("WithdrawFunds", "withdraw_funds"),
            ("WithdrawFundsWithFee", "withdraw_funds_with_fee"),
            ("DepositFunds", "deposit_funds"),
            ("ClaimAuthority", "claim_authority"),
            ("ClaimFees", "claim_fees"),
            ("NameSuccessor", "name_successor"),
            ("ChangeMarketStatus", "change_market_status"),
            ("ChangeExchangeStatus", "change_exchange_status"),
            ("SetExchangeStatusBits", "set_exchange_status_bits"),
            (
                "DisableExchangeCapabilities",
                "disable_exchange_capabilities",
            ),
            (
                "SetTraderCapabilitiesDelegated",
                "set_trader_capabilities_delegated",
            ),
            ("ConsumeWithdrawQueue", "consume_withdraw_queue"),
            ("RegisterTrader", "register_trader"),
            ("TransferCollateral", "transfer_collateral"),
            (
                "AuthorizedTransferCollateral",
                "authorized_transfer_collateral",
            ),
            ("CreateEscrowAccount", "create_escrow_account"),
            ("CreateEscrowRequest", "create_escrow_request"),
            ("AcceptEscrowRequest", "accept_escrow_request"),
            ("CancelEscrowRequest", "cancel_escrow_request"),
            ("UpsertEscrowRequest", "upsert_escrow_request"),
            ("RegisterSpline", "register_spline"),
            (
                "UpdateCommodityMarketState",
                "update_commodity_market_state",
            ),
            ("UpdateSplinePrice", "update_spline_price"),
            ("UpdateSplineParameters", "update_spline_parameters"),
            (
                "UpdateOraclePricesWithOrdering",
                "update_oracle_prices_with_ordering",
            ),
            ("UpdatePerpIsolatedOnly", "update_perp_isolated_only"),
            ("UpdatePerpLeverageTiers", "update_perp_leverage_tiers"),
            (
                "UpdatePerpMaxLiquidationSize",
                "update_perp_max_liquidation_size",
            ),
            ("UpdatePerpOpenInterestCap", "update_perp_open_interest_cap"),
            ("UpdatePerpUPnlRiskFactor", "update_perp_upnl_risk_factor"),
            (
                "UpdatePerpUPnlRiskFactorForWithdrawals",
                "update_perp_upnl_risk_factor_for_withdrawals",
            ),
            (
                "UpdatePerpCancelRiskFactor",
                "update_perp_cancel_risk_factor",
            ),
            (
                "UpdatePerpMarkPriceParameters",
                "update_perp_mark_price_parameters",
            ),
            ("UpdateTraderFees", "update_trader_fees"),
            ("UpdateMarketFees", "update_market_fees"),
            ("UpdateTraderState", "update_trader_state"),
            ("SetTraderCapability", "set_trader_capability"),
            ("UpdateWithdrawRateLimits", "update_withdraw_rate_limits"),
            ("UpdateWithdrawParameters", "update_withdraw_parameters"),
            ("UpdateAuthorities", "update_authorities"),
            ("LiquidateViaMarketOrder", "liquidate_via_market_order"),
            ("LiquidationTransfer", "liquidation_transfer"),
            ("CloseMatchedPositions", "close_matched_positions"),
            ("Log", "log"),
            ("InitializeArena", "initialize_arena"),
            ("RemoveEmptyArena", "remove_empty_arena"),
            ("RemoveOracle", "remove_oracle"),
            ("RemoveAllOracles", "remove_all_oracles"),
            ("CreatePermission", "create_permission"),
            ("SetPermission", "set_permission"),
            ("SetPermissionDelegated", "set_permission_delegated"),
            ("DeactivateSpline", "deactivate_spline"),
            ("PlaceStopLoss", "place_stop_loss"),
            ("ExecuteStopLoss", "execute_stop_loss"),
            ("CancelStopLoss", "cancel_stop_loss"),
            ("ForceCancelStopLoss", "force_cancel_stop_loss"),
            ("UpdateFundingParameters", "update_funding_parameters"),
            (
                "TransferCollateralChildToParent",
                "transfer_collateral_child_to_parent",
            ),
            ("UncrossCrank", "uncross_crank"),
            ("ClearExpiredOrders", "clear_expired_orders"),
            ("SyncParentToChild", "sync_parent_to_child"),
            ("DelegateTrader", "delegate_trader"),
            ("RevokePermission", "revoke_permission"),
            ("CloseMarket", "close_market"),
            ("TombstoneMarket", "tombstone_market"),
            (
                "SettleClosedMarketPosition",
                "settle_closed_market_position",
            ),
            ("DeleteTombstonedMarket", "delete_tombstoned_market"),
            ("LogEventLengths", "log_event_lengths"),
            (
                "CreateConditionalOrdersAccount",
                "create_conditional_orders_account",
            ),
            (
                "PlacePositionConditionalOrder",
                "place_position_conditional_order",
            ),
            ("CancelConditionalOrder", "cancel_conditional_order"),
            (
                "PlaceAttachedConditionalOrder",
                "place_attached_conditional_order",
            ),
            (
                "PlaceLimitOrderWithConditionals",
                "place_limit_order_with_conditionals",
            ),
            ("ExecuteConditionalOrders", "execute_conditional_orders"),
            ("PingConditionalOrders", "ping_conditional_orders"),
            ("CancelAllPlusConditional", "cancel_all_plus_conditional"),
            (
                "UpdateSplinePositionLimitsConfig",
                "update_spline_position_limits_config",
            ),
            ("CloseTraderAccount", "close_trader_account"),
            ("EnableFeature", "enable_feature"),
        ];

        assert_eq!(ETERNAL_INSTRUCTIONS.len(), 94);
        for (instruction_name, snake_case_name) in ETERNAL_INSTRUCTIONS {
            let by_instruction_name = PhoenixInstruction::from_instruction_name(instruction_name)
                .unwrap_or_else(|| panic!("missing {instruction_name}"));
            let by_snake_case_name =
                PhoenixInstruction::from_snake_case_instruction_name(snake_case_name)
                    .unwrap_or_else(|| panic!("missing {snake_case_name}"));
            assert_eq!(by_instruction_name, by_snake_case_name);
            assert_eq!(by_instruction_name.instruction_name(), *instruction_name);
            assert_eq!(
                by_instruction_name.snake_case_instruction_name(),
                *snake_case_name
            );
            assert_eq!(
                PhoenixInstruction::from_discriminant(by_instruction_name.discriminant()),
                Some(by_instruction_name)
            );
        }
    }

    #[test]
    fn test_discriminant_computation() {
        // These values should match the TypeScript SDK
        let limit_disc = crate::PhoenixInstruction::PlaceLimitOrder.discriminant();
        let market_disc = crate::PhoenixInstruction::PlaceMarketOrder.discriminant();
        let cancel_disc = crate::PhoenixInstruction::CancelOrdersById.discriminant();

        // Discriminants should be 8 bytes and non-zero
        assert_ne!(limit_disc, [0u8; 8]);
        assert_ne!(market_disc, [0u8; 8]);
        assert_ne!(cancel_disc, [0u8; 8]);

        // Each discriminant should be unique
        assert_ne!(limit_disc, market_disc);
        assert_ne!(limit_disc, cancel_disc);
        assert_ne!(market_disc, cancel_disc);
    }

    #[test]
    fn test_spline_collection_pda_derivation() {
        // Test that PDA derivation is deterministic
        let market = Pubkey::new_unique();
        let pda1 = get_spline_collection_address(&market).unwrap();
        let pda2 = get_spline_collection_address(&market).unwrap();
        assert_eq!(pda1, pda2);

        // Different markets should produce different PDAs
        let market2 = Pubkey::new_unique();
        let pda3 = get_spline_collection_address(&market2).unwrap();
        assert_ne!(pda1, pda3);
    }

    #[test]
    fn test_deposit_discriminants() {
        let deposit_disc = crate::PhoenixInstruction::DepositFunds.discriminant();
        let ember_disc = ember_deposit_discriminant();

        // Discriminants should be non-zero and unique
        assert_ne!(deposit_disc, [0u8; 8]);
        assert_ne!(ember_disc, [0u8; 8]);
        assert_ne!(deposit_disc, ember_disc);
    }

    #[test]
    fn test_register_trader_discriminant() {
        let disc = crate::PhoenixInstruction::RegisterTrader.discriminant();
        assert_ne!(disc, [0u8; 8]);
        assert_ne!(
            disc,
            crate::PhoenixInstruction::PlaceLimitOrder.discriminant()
        );
        assert_ne!(
            disc,
            crate::PhoenixInstruction::PlaceMarketOrder.discriminant()
        );
        assert_ne!(
            disc,
            crate::PhoenixInstruction::CancelOrdersById.discriminant()
        );
        assert_ne!(disc, crate::PhoenixInstruction::DepositFunds.discriminant());
        assert_ne!(
            disc,
            crate::PhoenixInstruction::WithdrawFunds.discriminant()
        );
    }

    #[test]
    fn test_withdraw_discriminants() {
        let withdraw_disc = crate::PhoenixInstruction::WithdrawFunds.discriminant();
        let ember_withdraw_disc = ember_withdraw_discriminant();
        let deposit_disc = crate::PhoenixInstruction::DepositFunds.discriminant();
        let ember_deposit_disc = ember_deposit_discriminant();

        // Discriminants should be non-zero
        assert_ne!(withdraw_disc, [0u8; 8]);
        assert_ne!(ember_withdraw_disc, [0u8; 8]);

        // All discriminants should be unique
        assert_ne!(withdraw_disc, ember_withdraw_disc);
        assert_ne!(withdraw_disc, deposit_disc);
        assert_ne!(ember_withdraw_disc, ember_deposit_disc);
    }

    #[test]
    fn test_ember_pda_derivation() {
        // Ember PDAs should be deterministic
        let state1 = get_ember_state_address().unwrap();
        let state2 = get_ember_state_address().unwrap();
        assert_eq!(state1, state2);

        let vault1 = get_ember_vault_address().unwrap();
        let vault2 = get_ember_vault_address().unwrap();
        assert_eq!(vault1, vault2);

        // State and vault should be different
        assert_ne!(state1, vault1);
    }

    #[test]
    fn test_global_vault_pda_derivation() {
        let mint = Pubkey::new_unique();
        let vault1 = get_global_vault_address(&mint).unwrap();
        let vault2 = get_global_vault_address(&mint).unwrap();
        assert_eq!(vault1, vault2);

        // Different mints should produce different vaults
        let mint2 = Pubkey::new_unique();
        let vault3 = get_global_vault_address(&mint2).unwrap();
        assert_ne!(vault1, vault3);
    }

    #[test]
    fn test_stop_loss_pda_derivation() {
        let trader_account = Pubkey::new_unique();
        let asset_id: u64 = 42;
        let pda1 = get_stop_loss_address(&trader_account, asset_id).unwrap();
        let pda2 = get_stop_loss_address(&trader_account, asset_id).unwrap();
        assert_eq!(pda1, pda2);

        // Different asset_id should produce different PDA
        let pda3 = get_stop_loss_address(&trader_account, 99).unwrap();
        assert_ne!(pda1, pda3);

        // Different trader should produce different PDA
        let trader2 = Pubkey::new_unique();
        let pda4 = get_stop_loss_address(&trader2, asset_id).unwrap();
        assert_ne!(pda1, pda4);
    }

    #[test]
    fn test_permission_pda_derivation() {
        let permission_authority = Pubkey::new_unique();
        let delegated_key = Pubkey::new_unique();
        let pda1 = get_permission_address(&permission_authority, &delegated_key).unwrap();
        let pda2 = get_permission_address(&permission_authority, &delegated_key).unwrap();
        assert_eq!(pda1, pda2);

        let other_authority = Pubkey::new_unique();
        let pda3 = get_permission_address(&other_authority, &delegated_key).unwrap();
        assert_ne!(pda1, pda3);

        let other_delegated_key = Pubkey::new_unique();
        let pda4 = get_permission_address(&permission_authority, &other_delegated_key).unwrap();
        assert_ne!(pda1, pda4);
    }

    #[test]
    fn test_ata_derivation() {
        let owner = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let ata1 = get_associated_token_address(&owner, &mint).unwrap();
        let ata2 = get_associated_token_address(&owner, &mint).unwrap();
        assert_eq!(ata1, ata2);

        // Different owner or mint should produce different ATA
        let owner2 = Pubkey::new_unique();
        let ata3 = get_associated_token_address(&owner2, &mint).unwrap();
        assert_ne!(ata1, ata3);
    }
}
